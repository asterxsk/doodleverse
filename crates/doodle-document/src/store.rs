use doodle_types::{
    BoardSnapshot, CommandError, DocumentOutcome, MAX_GROUP_DEPTH, MAX_OBJECTS,
    MAX_TOTAL_STROKE_POINTS, ObjectDelta, ObjectId, ObjectKind, Pose, Rect, StatementObject,
    StoredObject, StrokeGeometry, StrokeStyle,
};
use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;
use yrs::types::ToJson;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{
    Any, Doc, Map, MapPrelim, MapRef, Out, ReadTxn, StateVector, Transact, TransactionMut, Update,
};

/// Transaction origin for user-issued local commands.
pub const ORIGIN_LOCAL: u8 = 1;
/// Transaction origin for updates arriving from a peer or file import application.
pub const ORIGIN_REMOTE: u8 = 2;

const OBJECTS_ROOT: &str = "objects";

#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("invalid command: {0:?}")]
    Command(CommandError),
    #[error("document limit exceeded")]
    Limit,
    #[error("failed to encode object data")]
    Encode,
    #[error("failed to decode object data")]
    Decode,
    #[error("invalid remote update")]
    Update,
    #[error("projection is invalid: {0:?}")]
    Projection(ProjectionError),
}

impl From<CommandError> for DocumentError {
    fn from(value: CommandError) -> Self {
        Self::Command(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ProjectionError {
    #[error("too many objects")]
    TooManyObjects,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedObject {
    pub object: StoredObject,
    pub parent: Option<ObjectId>,
    /// Derived bounds: leaf geometry bounds, or the union of all descendant leaves for a group.
    pub bounds: Option<Rect>,
    pub depth: u16,
    /// Top-level ancestor used for selection, deletion, and occupancy.
    pub top_id: ObjectId,
}

impl ProjectedObject {
    #[must_use]
    pub fn is_group(&self) -> bool {
        self.object.kind.is_group()
    }

    #[must_use]
    pub fn is_top_level(&self) -> bool {
        self.parent.is_none()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoardProjection {
    /// Deterministic render order: order key, then object id.
    pub objects: Vec<ProjectedObject>,
    pub revision: u64,
    pub total_stroke_points: usize,
    /// Lookup index into `objects`. Public only so test fixtures can build a projection literal.
    #[doc(hidden)]
    pub index: HashMap<ObjectId, usize>,
}

impl BoardProjection {
    #[must_use]
    pub fn get(&self, id: ObjectId) -> Option<&ProjectedObject> {
        self.index
            .get(&id)
            .and_then(|index| self.objects.get(*index))
    }

    pub fn top_level(&self) -> impl Iterator<Item = &ProjectedObject> {
        self.objects.iter().filter(|object| object.is_top_level())
    }

    pub fn leaves(&self) -> impl Iterator<Item = &ProjectedObject> {
        self.objects.iter().filter(|object| !object.is_group())
    }

    #[must_use]
    pub fn content_bounds(&self) -> Option<Rect> {
        self.leaves()
            .filter_map(|object| object.bounds)
            .reduce(Rect::union)
    }

    /// Occupied regions for import placement: every top-level object with visible bounds.
    #[must_use]
    pub fn occupied_regions(&self) -> Vec<Rect> {
        self.top_level()
            .filter_map(|object| object.bounds)
            .collect()
    }

    #[must_use]
    pub fn top_ancestor(&self, id: ObjectId) -> Option<ObjectId> {
        self.get(id).map(|object| object.top_id)
    }

    /// Selected top-level object IDs, in deterministic projection order.
    #[must_use]
    pub fn expand_to_top_level(&self, ids: &[ObjectId]) -> Vec<ObjectId> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        for id in ids {
            if let Some(object) = self.get(*id)
                && seen.insert(object.top_id)
            {
                result.push(object.top_id);
            }
        }
        result.sort_by_key(|id| {
            self.get(*id)
                .map_or((i64::MAX, *id), |object| (object.object.order_key, *id))
        });
        result
    }

    #[must_use]
    pub fn leaf_descendants(&self, id: ObjectId) -> Vec<ObjectId> {
        if self.get(id).is_some_and(|object| !object.is_group()) {
            return vec![id];
        }
        self.objects
            .iter()
            .filter(|object| !object.is_group() && is_descendant_chain(self, object.object.id, id))
            .map(|object| object.object.id)
            .collect()
    }

    #[must_use]
    pub fn subtree_ids(&self, id: ObjectId) -> Vec<ObjectId> {
        let mut result = vec![id];
        for object in &self.objects {
            if object.object.id != id && is_descendant_chain(self, object.object.id, id) {
                result.push(object.object.id);
            }
        }
        result
    }
}

fn is_descendant_chain(
    projection: &BoardProjection,
    candidate: ObjectId,
    ancestor: ObjectId,
) -> bool {
    let mut current = projection.get(candidate).and_then(|object| object.parent);
    let mut steps = 0;
    while let Some(parent) = current {
        if parent == ancestor {
            return true;
        }
        steps += 1;
        if steps > MAX_GROUP_DEPTH {
            return false;
        }
        current = projection.get(parent).and_then(|object| object.parent);
    }
    false
}

#[derive(Debug, Clone)]
struct UndoEntry {
    deltas: Vec<ObjectDelta>,
    label: &'static str,
}

pub struct BoardDocument {
    doc: Doc,
    objects: MapRef,
    undo: VecDeque<UndoEntry>,
    redo: Vec<UndoEntry>,
    revision: u64,
}

impl BoardDocument {
    #[must_use]
    pub fn new() -> Self {
        let doc = Doc::new();
        let objects = doc.get_or_insert_map(OBJECTS_ROOT);
        Self {
            doc,
            objects,
            undo: VecDeque::new(),
            redo: Vec::new(),
            revision: 0,
        }
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    #[must_use]
    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    /// Label of the command that [`Self::undo`] would reverse, if any.
    #[must_use]
    pub fn undo_label(&self) -> Option<&'static str> {
        self.undo.back().map(|entry| entry.label)
    }

    /// Label of the command that [`Self::redo`] would reapply, if any.
    #[must_use]
    pub fn redo_label(&self) -> Option<&'static str> {
        self.redo.last().map(|entry| entry.label)
    }

    // ---------------------------------------------------------------------
    // Commands
    // ---------------------------------------------------------------------

    pub fn create_stroke(
        &mut self,
        pose: Pose,
        style: StrokeStyle,
        geometry: StrokeGeometry,
    ) -> Result<DocumentOutcome, DocumentError> {
        let object = StoredObject {
            id: ObjectId::new(),
            parent: None,
            order_key: self.next_order_key(),
            pose,
            kind: ObjectKind::Stroke(doodle_types::StrokeObject { style, geometry }),
        };
        if !object.is_valid() {
            return Err(CommandError::InvalidObject.into());
        }
        self.insert_new_objects(vec![object], "stroke")
    }

    pub fn create_statement(
        &mut self,
        pose: Pose,
        statement: StatementObject,
    ) -> Result<DocumentOutcome, DocumentError> {
        let object = StoredObject {
            id: ObjectId::new(),
            parent: None,
            order_key: self.next_order_key(),
            pose,
            kind: ObjectKind::Statement(statement),
        };
        if !object.is_valid() {
            return Err(CommandError::InvalidObject.into());
        }
        self.insert_new_objects(vec![object], "statement")
    }

    pub fn commit_import(
        &mut self,
        objects: Vec<StoredObject>,
    ) -> Result<DocumentOutcome, DocumentError> {
        self.insert_new_objects(objects, "import")
    }

    fn insert_new_objects(
        &mut self,
        objects: Vec<StoredObject>,
        label: &'static str,
    ) -> Result<DocumentOutcome, DocumentError> {
        if objects.is_empty() {
            return Err(CommandError::NothingToDo.into());
        }
        let existing = self.read_all()?;
        if existing.len() + objects.len() > MAX_OBJECTS {
            return Err(DocumentError::Limit);
        }
        let mut seen = HashSet::new();
        for object in &objects {
            if !object.is_valid() {
                return Err(CommandError::InvalidObject.into());
            }
            if !seen.insert(object.id) || existing.contains_key(&object.id) {
                return Err(CommandError::InvalidObject.into());
            }
        }
        for object in &objects {
            if let Some(parent) = object.parent
                && !seen.contains(&parent)
                && !existing.contains_key(&parent)
            {
                return Err(CommandError::InvalidObject.into());
            }
        }
        let new_by_id: HashMap<ObjectId, &StoredObject> =
            objects.iter().map(|object| (object.id, object)).collect();
        for object in &objects {
            let mut depth = 0usize;
            let mut current = object.parent;
            let mut visited = HashSet::new();
            visited.insert(object.id);
            while let Some(parent_id) = current {
                if !visited.insert(parent_id) {
                    return Err(CommandError::WouldCycle.into());
                }
                let parent = new_by_id
                    .get(&parent_id)
                    .copied()
                    .or_else(|| existing.get(&parent_id));
                let Some(parent) = parent else {
                    return Err(CommandError::InvalidObject.into());
                };
                if !parent.kind.is_group() {
                    return Err(CommandError::InvalidObject.into());
                }
                depth += 1;
                if depth > MAX_GROUP_DEPTH {
                    return Err(CommandError::LimitExceeded.into());
                }
                current = parent.parent;
            }
        }
        let prepared = objects
            .iter()
            .map(Prepared::from_object)
            .collect::<Result<Vec<_>, _>>()?;
        let before: HashMap<ObjectId, Option<StoredObject>> =
            objects.iter().map(|object| (object.id, None)).collect();
        {
            let mut txn = self.doc.transact_mut_with(ORIGIN_LOCAL);
            for object in &prepared {
                self.write_prepared(&mut txn, object)?;
            }
        }
        self.revision += 1;
        let delta = self.build_deltas(before)?;
        self.push_undo(delta, label);
        let ids = objects.iter().map(|object| object.id).collect();
        Ok(DocumentOutcome {
            changed: Vec::new(),
            created: ids,
            deleted: Vec::new(),
        })
    }

    pub fn move_objects(
        &mut self,
        ids: &[ObjectId],
        dx: f64,
        dy: f64,
    ) -> Result<DocumentOutcome, DocumentError> {
        if !dx.is_finite() || !dy.is_finite() {
            return Err(CommandError::InvalidObject.into());
        }
        let projection = self.project()?;
        let top = projection.expand_to_top_level(ids);
        if top.is_empty() {
            return Err(CommandError::EmptySelection.into());
        }
        let mut leaf_ids = Vec::new();
        for id in &top {
            leaf_ids.extend(projection.leaf_descendants(*id));
        }
        leaf_ids.sort_unstable();
        leaf_ids.dedup();
        if leaf_ids.is_empty() {
            return Ok(DocumentOutcome::empty());
        }
        let before = self.read_selected(&leaf_ids)?;
        let mut moved = Vec::new();
        {
            let mut txn = self.doc.transact_mut_with(ORIGIN_LOCAL);
            for id in &leaf_ids {
                if let Some(mut object) = before.get(id).cloned().flatten() {
                    let original = object.clone();
                    object.pose = Pose::new(object.pose.x + dx, object.pose.y + dy);
                    if !object.pose.is_finite() {
                        continue;
                    }
                    self.write_changed(&mut txn, &original, &object)?;
                    moved.push(*id);
                }
            }
        }
        if moved.is_empty() {
            return Ok(DocumentOutcome::empty());
        }
        self.revision += 1;
        let delta = self.build_deltas(before)?;
        self.push_undo(delta, "move");
        Ok(DocumentOutcome {
            changed: moved,
            created: Vec::new(),
            deleted: Vec::new(),
        })
    }

    pub fn delete_objects(&mut self, ids: &[ObjectId]) -> Result<DocumentOutcome, DocumentError> {
        let projection = self.project()?;
        let top = projection.expand_to_top_level(ids);
        if top.is_empty() {
            return Err(CommandError::EmptySelection.into());
        }
        let mut to_delete = Vec::new();
        for id in &top {
            to_delete.extend(projection.subtree_ids(*id));
        }
        to_delete.sort_unstable();
        to_delete.dedup();
        let before = self.read_selected(&to_delete)?;
        if before.values().all(Option::is_none) {
            return Err(CommandError::UnknownObject.into());
        }
        {
            let mut txn = self.doc.transact_mut_with(ORIGIN_LOCAL);
            for id in &to_delete {
                self.objects.remove(&mut txn, &id.to_string());
            }
        }
        self.revision += 1;
        let delta = self.build_deltas(before)?;
        self.push_undo(delta, "delete");
        Ok(DocumentOutcome {
            changed: Vec::new(),
            created: Vec::new(),
            deleted: to_delete,
        })
    }

    pub fn group_selection(&mut self, ids: &[ObjectId]) -> Result<DocumentOutcome, DocumentError> {
        let projection = self.project()?;
        if projection.objects.len() >= MAX_OBJECTS {
            return Err(DocumentError::Limit);
        }
        let top = projection.expand_to_top_level(ids);
        if top.len() < 2 {
            return Err(CommandError::GroupNeedsTwoObjects.into());
        }
        let mut selected = Vec::new();
        for id in &top {
            let object = projection.get(*id).ok_or(CommandError::UnknownObject)?;
            if object.parent.is_some() {
                return Err(CommandError::NotTopLevel.into());
            }
            selected.push(object.clone());
        }
        let max_depth = selected
            .iter()
            .map(|object| {
                projection
                    .subtree_ids(object.object.id)
                    .into_iter()
                    .filter_map(|id| projection.get(id).map(|o| o.depth))
                    .max()
                    .unwrap_or(object.depth)
            })
            .max()
            .unwrap_or(0);
        if usize::from(max_depth) + 1 > MAX_GROUP_DEPTH {
            return Err(CommandError::LimitExceeded.into());
        }
        let group_id = ObjectId::new();
        let order = selected
            .iter()
            .map(|object| object.object.order_key)
            .max()
            .unwrap_or(0)
            .saturating_add(16);
        let group = StoredObject {
            id: group_id,
            parent: None,
            order_key: order,
            pose: Pose::default(),
            kind: ObjectKind::Group,
        };
        let mut before: HashMap<ObjectId, Option<StoredObject>> = HashMap::new();
        before.insert(group_id, None);
        for object in &selected {
            before.insert(object.object.id, Some(object.object.clone()));
        }
        {
            let mut txn = self.doc.transact_mut_with(ORIGIN_LOCAL);
            let prepared = Prepared::from_object(&group)?;
            self.write_prepared(&mut txn, &prepared)?;
            for object in &selected {
                let mut child = object.object.clone();
                child.parent = Some(group_id);
                self.write_changed(&mut txn, &object.object, &child)?;
            }
        }
        self.revision += 1;
        let delta = self.build_deltas(before)?;
        self.push_undo(delta, "group");
        Ok(DocumentOutcome {
            changed: selected.iter().map(|o| o.object.id).collect(),
            created: vec![group_id],
            deleted: Vec::new(),
        })
    }

    pub fn ungroup(&mut self, ids: &[ObjectId]) -> Result<DocumentOutcome, DocumentError> {
        let projection = self.project()?;
        let mut groups: Vec<ObjectId> = Vec::new();
        for id in ids {
            let Some(object) = projection.get(*id) else {
                continue;
            };
            if !object.is_group() {
                return Err(CommandError::NotAGroup.into());
            }
            if object.parent.is_some() {
                return Err(CommandError::NotTopLevel.into());
            }
            if !groups.contains(&object.object.id) {
                groups.push(object.object.id);
            }
        }
        if groups.is_empty() {
            return Err(CommandError::EmptySelection.into());
        }
        groups.sort_unstable();
        let mut before: HashMap<ObjectId, Option<StoredObject>> = HashMap::new();
        let mut children_updates: Vec<StoredObject> = Vec::new();
        for group_id in &groups {
            if let Some(group) = projection.get(*group_id) {
                before.insert(*group_id, Some(group.object.clone()));
            }
            let children: Vec<&ProjectedObject> = projection
                .objects
                .iter()
                .filter(|object| object.parent == Some(*group_id))
                .collect();
            for child in children {
                before
                    .entry(child.object.id)
                    .or_insert_with(|| Some(child.object.clone()));
                let mut updated = child.object.clone();
                updated.parent = projection.get(*group_id).and_then(|group| group.parent);
                children_updates.push(updated);
            }
        }
        if children_updates.is_empty() && before.is_empty() {
            return Err(CommandError::NothingToDo.into());
        }
        {
            let mut txn = self.doc.transact_mut_with(ORIGIN_LOCAL);
            for object in &children_updates {
                if let Some(original) = before.get(&object.id).and_then(Option::as_ref) {
                    self.write_changed(&mut txn, original, object)?;
                } else {
                    let prepared = Prepared::from_object(object)?;
                    self.write_prepared(&mut txn, &prepared)?;
                }
            }
            for group_id in &groups {
                self.objects.remove(&mut txn, &group_id.to_string());
            }
        }
        self.revision += 1;
        let delta = self.build_deltas(before)?;
        self.push_undo(delta, "ungroup");
        Ok(DocumentOutcome {
            changed: children_updates.iter().map(|object| object.id).collect(),
            created: Vec::new(),
            deleted: groups,
        })
    }

    pub fn undo(&mut self) -> Result<DocumentOutcome, DocumentError> {
        let Some(entry) = self.undo.pop_back() else {
            return Err(CommandError::NothingToDo.into());
        };
        let outcome = self.apply_deltas(&entry.deltas, false)?;
        self.redo.push(entry);
        Ok(outcome)
    }

    pub fn redo(&mut self) -> Result<DocumentOutcome, DocumentError> {
        let Some(entry) = self.redo.pop() else {
            return Err(CommandError::NothingToDo.into());
        };
        let outcome = self.apply_deltas(&entry.deltas, true)?;
        self.undo.push_back(entry);
        Ok(outcome)
    }

    fn apply_deltas(
        &mut self,
        deltas: &[ObjectDelta],
        forward: bool,
    ) -> Result<DocumentOutcome, DocumentError> {
        let mut changed = Vec::new();
        let mut created = Vec::new();
        let mut deleted = Vec::new();
        {
            let mut txn = self.doc.transact_mut_with(ORIGIN_LOCAL);
            for delta in deltas {
                let target = if forward { &delta.after } else { &delta.before };
                let source = if forward { &delta.before } else { &delta.after };
                let key = delta.id.to_string();
                let current = match self.objects.get(&txn, &key) {
                    Some(Out::YMap(map)) => read_object(&map, &txn, delta.id),
                    _ => None,
                };
                match target {
                    Some(object) => {
                        if current.is_some() && source.is_none() {
                            // The object already exists; never overwrite a peer's version by
                            // replaying a whole-object creation delta.
                            continue;
                        }
                        if current.is_none() && source.is_some() {
                            // A peer removed an object this command only moved or regrouped.
                            // Undoing the local command must not resurrect it.
                            continue;
                        }
                        let fields = if current.is_some() {
                            changed_fields(source.as_ref(), object)
                        } else {
                            WriteFields::all()
                        };
                        if fields.is_empty() {
                            continue;
                        }
                        self.write_fields(&mut txn, object, fields)?;
                        changed.push(object.id);
                        if delta.before.is_none() {
                            created.push(object.id);
                        }
                    }
                    None => {
                        if current.is_some() {
                            self.objects.remove(&mut txn, &key);
                            deleted.push(delta.id);
                            changed.push(delta.id);
                        }
                    }
                }
            }
        }
        self.revision += 1;
        Ok(DocumentOutcome {
            changed,
            created,
            deleted,
        })
    }

    fn push_undo(&mut self, deltas: Vec<ObjectDelta>, label: &'static str) {
        if deltas.iter().all(|delta| delta.before == delta.after) {
            return;
        }
        self.redo.clear();
        self.undo.push_back(UndoEntry { deltas, label });
        while self.undo.len() > doodle_types::MAX_UNDO_STEPS {
            self.undo.pop_front();
        }
    }

    // ---------------------------------------------------------------------
    // Projection and snapshots
    // ---------------------------------------------------------------------

    pub fn project(&self) -> Result<BoardProjection, DocumentError> {
        let raw = self.read_all()?;
        if raw.len() > MAX_OBJECTS {
            return Err(DocumentError::Projection(ProjectionError::TooManyObjects));
        }
        Ok(project_objects(raw, self.revision))
    }

    #[must_use]
    pub fn snapshot_visible(&self) -> BoardSnapshot {
        let Ok(projection) = self.project() else {
            return BoardSnapshot::new(Vec::new());
        };
        BoardSnapshot::new(
            projection
                .objects
                .iter()
                .map(|projected| {
                    let mut object = projected.object.clone();
                    object.parent = projected.parent;
                    object
                })
                .collect(),
        )
    }

    // ---------------------------------------------------------------------
    // Sync surface (used by networking in a later milestone)
    // ---------------------------------------------------------------------

    #[must_use]
    pub fn state_vector_v1(&self) -> Vec<u8> {
        self.doc.transact().state_vector().encode_v1()
    }

    #[must_use]
    pub fn encode_full_update_v1(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.encode_diff_v1(&StateVector::default())
    }

    pub fn encode_diff_v1(&self, remote_state_vector: &[u8]) -> Result<Vec<u8>, DocumentError> {
        let state_vector =
            StateVector::decode_v1(remote_state_vector).map_err(|_| DocumentError::Update)?;
        let txn = self.doc.transact();
        Ok(txn.encode_diff_v1(&state_vector))
    }

    /// Applies a remote CRDT update. Remote transactions are never captured by local undo.
    pub fn apply_remote_update_v1(&mut self, update: &[u8]) -> Result<(), DocumentError> {
        let decoded = Update::decode_v1(update).map_err(|_| DocumentError::Update)?;
        let mut txn = self.doc.transact_mut_with(ORIGIN_REMOTE);
        txn.apply_update(decoded)
            .map_err(|_| DocumentError::Update)?;
        drop(txn);
        self.revision += 1;
        Ok(())
    }

    /// Debug/test helper used to inspect raw CRDT JSON without going through the projection.
    #[must_use]
    pub fn to_json(&self) -> Any {
        let txn = self.doc.transact();
        self.objects.to_json(&txn)
    }

    // ---------------------------------------------------------------------
    // Internal write/read helpers
    // ---------------------------------------------------------------------

    fn next_order_key(&self) -> i64 {
        let Ok(raw) = self.read_all() else {
            return 0;
        };
        raw.values()
            .map(|object| object.order_key)
            .max()
            .unwrap_or(0)
            .saturating_add(16)
    }

    fn read_all(&self) -> Result<HashMap<ObjectId, StoredObject>, DocumentError> {
        let txn = self.doc.transact();
        let mut result = HashMap::new();
        for (key, out) in self.objects.iter(&txn) {
            let Ok(id) = key.parse::<ObjectId>() else {
                continue;
            };
            let Out::YMap(map) = out else {
                continue;
            };
            if let Some(object) = read_object(&map, &txn, id) {
                result.insert(id, object);
            }
        }
        Ok(result)
    }

    fn read_selected(
        &self,
        ids: &[ObjectId],
    ) -> Result<HashMap<ObjectId, Option<StoredObject>>, DocumentError> {
        let txn = self.doc.transact();
        let mut result = HashMap::new();
        for id in ids {
            let value = match self.objects.get(&txn, &id.to_string()) {
                Some(Out::YMap(map)) => read_object(&map, &txn, *id),
                _ => None,
            };
            result.insert(*id, value);
        }
        Ok(result)
    }

    fn build_deltas(
        &self,
        before: HashMap<ObjectId, Option<StoredObject>>,
    ) -> Result<Vec<ObjectDelta>, DocumentError> {
        let ids: Vec<ObjectId> = before.keys().copied().collect();
        let after = self.read_selected(&ids)?;
        let mut deltas = Vec::with_capacity(before.len());
        for id in ids {
            let before_object = before.get(&id).cloned().flatten();
            let after_object = after.get(&id).cloned().flatten();
            deltas.push(ObjectDelta {
                id,
                before: before_object,
                after: after_object,
            });
        }
        deltas.sort_by_key(|delta| delta.id);
        Ok(deltas)
    }

    fn object_map(&self, txn: &mut TransactionMut<'_>, key: &str) -> MapRef {
        if let Some(Out::YMap(map)) = self.objects.get(txn, key) {
            map
        } else {
            self.objects.insert(txn, key, MapPrelim::default())
        }
    }

    fn write_changed(
        &self,
        txn: &mut TransactionMut<'_>,
        before: &StoredObject,
        after: &StoredObject,
    ) -> Result<(), DocumentError> {
        self.write_fields(txn, after, changed_fields(Some(before), after))
    }

    /// Writes only the fields a command actually changed. Rewriting an unchanged field would
    /// stamp it with a fresh CRDT clock and let a local move win over a concurrent peer edit of
    /// that same field.
    fn write_fields(
        &self,
        txn: &mut TransactionMut<'_>,
        object: &StoredObject,
        fields: WriteFields,
    ) -> Result<(), DocumentError> {
        if fields.is_empty() {
            return Ok(());
        }
        let key = object.id.to_string();
        let map = self.object_map(txn, &key);
        if fields.kind_data {
            let prepared = Prepared::from_object(object)?;
            map.insert(txn, "kind", prepared.kind.as_str());
            map.insert(txn, "data", prepared.kind_bytes);
        }
        if fields.parent {
            map.insert(
                txn,
                "parent",
                object.parent.map(|id| id.to_string()).unwrap_or_default(),
            );
        }
        if fields.order {
            map.insert(txn, "order", object.order_key);
        }
        if fields.pose_x {
            map.insert(txn, "pose_x", object.pose.x);
        }
        if fields.pose_y {
            map.insert(txn, "pose_y", object.pose.y);
        }
        Ok(())
    }

    fn write_prepared(
        &self,
        txn: &mut TransactionMut<'_>,
        prepared: &Prepared,
    ) -> Result<(), DocumentError> {
        let key = prepared.id.to_string();
        let map = self.object_map(txn, &key);
        map.insert(txn, "kind", prepared.kind.as_str());
        map.insert(
            txn,
            "parent",
            prepared.parent.map(|id| id.to_string()).unwrap_or_default(),
        );
        map.insert(txn, "order", prepared.order_key);
        map.insert(txn, "pose_x", prepared.pose.x);
        map.insert(txn, "pose_y", prepared.pose.y);
        map.insert(txn, "data", prepared.kind_bytes.clone());
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct WriteFields {
    kind_data: bool,
    parent: bool,
    order: bool,
    pose_x: bool,
    pose_y: bool,
}

impl WriteFields {
    const fn all() -> Self {
        Self {
            kind_data: true,
            parent: true,
            order: true,
            pose_x: true,
            pose_y: true,
        }
    }

    const fn is_empty(self) -> bool {
        !(self.kind_data || self.parent || self.order || self.pose_x || self.pose_y)
    }
}

fn changed_fields(before: Option<&StoredObject>, after: &StoredObject) -> WriteFields {
    let Some(before) = before else {
        return WriteFields::all();
    };
    WriteFields {
        kind_data: before.kind != after.kind,
        parent: before.parent != after.parent,
        order: before.order_key != after.order_key,
        pose_x: before.pose.x != after.pose.x,
        pose_y: before.pose.y != after.pose.y,
    }
}

impl Default for BoardDocument {
    fn default() -> Self {
        Self::new()
    }
}

struct Prepared {
    id: ObjectId,
    parent: Option<ObjectId>,
    order_key: i64,
    pose: Pose,
    kind: KindTag,
    kind_bytes: Vec<u8>,
}

#[derive(Clone, Copy)]
enum KindTag {
    Stroke,
    Group,
    Statement,
}

impl KindTag {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stroke => "stroke",
            Self::Group => "group",
            Self::Statement => "statement",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "stroke" => Some(Self::Stroke),
            "group" => Some(Self::Group),
            "statement" => Some(Self::Statement),
            _ => None,
        }
    }
}

impl Prepared {
    fn from_object(object: &StoredObject) -> Result<Self, DocumentError> {
        let kind = match &object.kind {
            ObjectKind::Stroke(_) => KindTag::Stroke,
            ObjectKind::Group => KindTag::Group,
            ObjectKind::Statement(_) => KindTag::Statement,
        };
        let kind_bytes = postcard::to_allocvec(&object.kind).map_err(|_| DocumentError::Encode)?;
        Ok(Self {
            id: object.id,
            parent: object.parent,
            order_key: object.order_key,
            pose: object.pose,
            kind,
            kind_bytes,
        })
    }
}

fn read_object<T: ReadTxn>(map: &MapRef, txn: &T, id: ObjectId) -> Option<StoredObject> {
    let kind_string = match map.get(txn, "kind")? {
        Out::Any(Any::String(value)) => value.to_string(),
        _ => return None,
    };
    let kind_tag = KindTag::from_str(&kind_string)?;
    let parent = match map.get(txn, "parent") {
        Some(Out::Any(Any::String(value))) if !value.is_empty() => value.parse().ok(),
        _ => None,
    };
    let order_key = match map.get(txn, "order") {
        Some(Out::Any(Any::Number(value))) => value.as_i64()?,
        _ => return None,
    };
    let pose_x = match map.get(txn, "pose_x") {
        Some(Out::Any(Any::Number(value))) => value.as_f64()?,
        _ => 0.0,
    };
    let pose_y = match map.get(txn, "pose_y") {
        Some(Out::Any(Any::Number(value))) => value.as_f64()?,
        _ => 0.0,
    };
    let kind_bytes = match map.get(txn, "data") {
        Some(Out::Any(Any::Buffer(value))) => value.to_vec(),
        _ => return None,
    };
    let kind: ObjectKind = postcard::from_bytes(&kind_bytes).ok()?;
    let matches = matches!(
        (&kind_tag, &kind),
        (KindTag::Stroke, ObjectKind::Stroke(_))
            | (KindTag::Group, ObjectKind::Group)
            | (KindTag::Statement, ObjectKind::Statement(_))
    );
    if !matches {
        return None;
    }
    Some(StoredObject {
        id,
        parent,
        order_key,
        pose: Pose::new(pose_x, pose_y),
        kind,
    })
}

fn project_objects(raw: HashMap<ObjectId, StoredObject>, revision: u64) -> BoardProjection {
    let mut ids: Vec<ObjectId> = raw.keys().copied().collect();
    ids.sort_unstable();
    let mut parents: HashMap<ObjectId, Option<ObjectId>> = HashMap::with_capacity(ids.len());
    for id in &ids {
        let object = &raw[id];
        let valid_parent = object
            .parent
            .filter(|parent| raw.get(parent).is_some_and(|value| value.kind.is_group()));
        parents.insert(*id, valid_parent);
    }

    // Cut cycles and over-deep chains with a bounded walk per object. A single pass cuts the
    // edge at the depth limit for every overly deep branch; repeat until stable so a cut that
    // re-roots one node cannot leave another chain over the cap.
    let mut changed = true;
    let mut passes = 0;
    while changed && passes < MAX_GROUP_DEPTH {
        changed = false;
        passes += 1;
        for id in &ids {
            let mut chain: Vec<ObjectId> = Vec::new();
            let mut positions: HashMap<ObjectId, usize> = HashMap::new();
            let mut current = Some(*id);
            while let Some(node) = current {
                if let Some(&start) = positions.get(&node) {
                    if let Some(max) = chain[start..].iter().max()
                        && parents.get(max).copied().flatten().is_some()
                    {
                        parents.insert(*max, None);
                        changed = true;
                    }
                    break;
                }
                positions.insert(node, chain.len());
                chain.push(node);
                if chain.len() > MAX_GROUP_DEPTH {
                    if let Some(&deep) = chain.get(MAX_GROUP_DEPTH)
                        && parents.get(&deep).copied().flatten().is_some()
                    {
                        parents.insert(deep, None);
                        changed = true;
                    }
                    break;
                }
                current = parents.get(&node).copied().flatten();
            }
        }
    }

    // Resolve depth and top-level ancestor bottom-up, memoized across chains.
    let mut depths: HashMap<ObjectId, u16> = HashMap::new();
    let mut tops: HashMap<ObjectId, ObjectId> = HashMap::new();
    for id in &ids {
        if depths.contains_key(id) {
            continue;
        }
        let mut chain: Vec<ObjectId> = Vec::new();
        let mut local: HashSet<ObjectId> = HashSet::new();
        let mut current = *id;
        while !depths.contains_key(&current) {
            if !local.insert(current) {
                parents.insert(current, None);
                break;
            }
            chain.push(current);
            match parents.get(&current).copied().flatten() {
                Some(parent) => current = parent,
                None => break,
            }
        }
        if let std::collections::hash_map::Entry::Vacant(slot) = depths.entry(current) {
            slot.insert(0);
            tops.insert(current, current);
        }
        for node in chain.into_iter().rev() {
            if depths.contains_key(&node) {
                continue;
            }
            match parents
                .get(&node)
                .copied()
                .flatten()
                .filter(|parent| depths.contains_key(parent))
            {
                Some(parent) => {
                    let depth = depths[&parent]
                        .saturating_add(1)
                        .min(MAX_GROUP_DEPTH as u16);
                    depths.insert(node, depth);
                    let top = tops.get(&parent).copied().unwrap_or(parent);
                    tops.insert(node, top);
                }
                None => {
                    depths.insert(node, 0);
                    tops.insert(node, node);
                }
            }
        }
    }

    let mut group_bounds: HashMap<ObjectId, Rect> = HashMap::new();
    let mut total_stroke_points = 0usize;
    for id in &ids {
        let object = &raw[id];
        if let ObjectKind::Stroke(stroke) = &object.kind {
            total_stroke_points = total_stroke_points.saturating_add(stroke.geometry.points.len());
        }
        if let Some(bounds) = object.bounds() {
            let mut current = parents.get(id).copied().flatten();
            let mut steps = 0;
            while let Some(parent) = current {
                group_bounds
                    .entry(parent)
                    .and_modify(|existing| *existing = existing.union(bounds))
                    .or_insert(bounds);
                current = parents.get(&parent).copied().flatten();
                steps += 1;
                if steps > MAX_GROUP_DEPTH {
                    break;
                }
            }
        }
    }

    let mut objects: Vec<ProjectedObject> = ids
        .into_iter()
        .map(|id| {
            let object = raw[&id].clone();
            let parent = parents.get(&id).copied().flatten();
            let bounds = if object.kind.is_group() {
                group_bounds.get(&id).copied()
            } else {
                object.bounds()
            };
            ProjectedObject {
                bounds,
                parent,
                depth: depths.get(&id).copied().unwrap_or(0),
                top_id: tops.get(&id).copied().unwrap_or(id),
                object,
            }
        })
        .collect();
    objects.sort_by(|a, b| {
        a.object
            .order_key
            .cmp(&b.object.order_key)
            .then_with(|| a.object.id.cmp(&b.object.id))
    });
    let index = objects
        .iter()
        .enumerate()
        .map(|(index, object)| (object.object.id, index))
        .collect();
    BoardProjection {
        objects,
        revision,
        total_stroke_points: total_stroke_points.min(MAX_TOTAL_STROKE_POINTS + 1),
        index,
    }
}

/// Reads a board file snapshot into a fresh document using new CRDT values but the same app IDs.
#[cfg(test)]
mod tests {
    use super::*;
    use doodle_types::{StrokeObject, StrokePoint};

    fn stroke(offset: f64) -> StoredObject {
        StoredObject {
            id: ObjectId::new(),
            parent: None,
            order_key: (offset as i64).saturating_mul(16),
            pose: Pose::new(offset, 0.0),
            kind: ObjectKind::Stroke(StrokeObject {
                style: StrokeStyle::default(),
                geometry: StrokeGeometry::new(vec![
                    StrokePoint {
                        x: 0.0,
                        y: 0.0,
                        width: 6.0,
                    },
                    StrokePoint {
                        x: 20.0,
                        y: 0.0,
                        width: 6.0,
                    },
                ]),
            }),
        }
    }

    fn doc_with_strokes(count: usize) -> BoardDocument {
        let mut doc = BoardDocument::new();
        for index in 0..count {
            let object = stroke(index as f64);
            doc.commit_import(vec![object]).expect("import");
        }
        doc
    }

    #[test]
    fn projection_orders_and_bounds() {
        let doc = doc_with_strokes(3);
        let projection = doc.project().expect("projection");
        assert_eq!(projection.objects.len(), 3);
        let bounds = projection.content_bounds().expect("bounds");
        assert!(bounds.width() > 0.0);
        assert_eq!(projection.total_stroke_points, 6);
    }

    #[test]
    fn group_and_ungroup_preserve_leaf_positions() {
        let mut doc = doc_with_strokes(2);
        let ids: Vec<ObjectId> = doc
            .project()
            .expect("projection")
            .objects
            .iter()
            .map(|object| object.object.id)
            .collect();
        let before: HashMap<_, _> = doc
            .project()
            .expect("projection")
            .leaves()
            .map(|object| (object.object.id, object.object.pose))
            .collect();
        let outcome = doc.group_selection(&ids).expect("group");
        let group_id = outcome.created[0];
        let projection = doc.project().expect("projection");
        let group = projection.get(group_id).expect("group");
        assert!(group.is_group());
        assert!(group.bounds.is_some());
        let after: HashMap<_, _> = projection
            .leaves()
            .map(|object| (object.object.id, object.object.pose))
            .collect();
        assert_eq!(
            before, after,
            "grouping must preserve leaf identity and pose"
        );
        doc.ungroup(&[group_id]).expect("ungroup");
        let projection = doc.project().expect("projection");
        assert!(projection.get(group_id).is_none());
        assert!(projection.top_level().all(|object| !object.is_group()));
        let ungrouped: HashMap<_, _> = projection
            .leaves()
            .map(|object| (object.object.id, object.object.pose))
            .collect();
        assert_eq!(
            before, ungrouped,
            "ungrouping must preserve leaf identity and pose"
        );
    }

    #[test]
    fn moving_group_moves_leaf_descendants() {
        let mut doc = doc_with_strokes(2);
        let ids: Vec<ObjectId> = doc
            .project()
            .expect("projection")
            .objects
            .iter()
            .map(|object| object.object.id)
            .collect();
        let group = doc.group_selection(&ids).expect("group").created[0];
        let before: HashMap<_, _> = doc
            .project()
            .expect("projection")
            .leaves()
            .map(|object| (object.object.id, object.object.pose))
            .collect();
        doc.move_objects(&[group], 10.0, 5.0).expect("move");
        let after: HashMap<_, _> = doc
            .project()
            .expect("projection")
            .leaves()
            .map(|object| (object.object.id, object.object.pose))
            .collect();
        for (id, pose) in before {
            let moved = after[&id];
            assert_eq!(moved.x, pose.x + 10.0);
            assert_eq!(moved.y, pose.y + 5.0);
        }
        assert_eq!(doc.project().expect("projection").leaves().count(), 2);
    }

    #[test]
    fn delete_group_removes_descendants() {
        let mut doc = doc_with_strokes(2);
        let ids: Vec<ObjectId> = doc
            .project()
            .expect("projection")
            .objects
            .iter()
            .map(|object| object.object.id)
            .collect();
        let group = doc.group_selection(&ids).expect("group").created[0];
        doc.delete_objects(&[group]).expect("delete");
        assert!(doc.project().expect("projection").objects.is_empty());
        doc.undo().expect("undo");
        assert_eq!(doc.project().expect("projection").objects.len(), 3);
    }

    #[test]
    fn undo_redo_round_trip() {
        let mut doc = doc_with_strokes(1);
        let id = doc.project().expect("projection").objects[0].object.id;
        doc.move_objects(&[id], 25.0, 0.0).expect("move");
        let moved = doc.project().expect("projection").objects[0].object.pose;
        doc.undo().expect("undo");
        let original = doc.project().expect("projection").objects[0].object.pose;
        assert_eq!(original, Pose::new(0.0, 0.0));
        doc.redo().expect("redo");
        assert_eq!(
            doc.project().expect("projection").objects[0].object.pose,
            moved
        );
    }

    #[test]
    fn replicas_converge_with_diff_updates() {
        let mut a = doc_with_strokes(2);
        let mut b = BoardDocument::new();
        let sv = b.state_vector_v1();
        let diff = a.encode_diff_v1(&sv).expect("diff");
        b.apply_remote_update_v1(&diff).expect("apply");
        assert_eq!(normalized(&a), normalized(&b));

        let object = stroke(9.0);
        a.commit_import(vec![object]).expect("import");
        let sv = b.state_vector_v1();
        let diff = a.encode_diff_v1(&sv).expect("diff");
        b.apply_remote_update_v1(&diff).expect("apply");
        assert_eq!(normalized(&a), normalized(&b));
    }

    #[test]
    fn undo_move_does_not_undo_remote_grouping() {
        let mut a = doc_with_strokes(2);
        let mut b = BoardDocument::new();
        b.apply_remote_update_v1(&a.encode_full_update_v1())
            .expect("sync");
        let ids: Vec<ObjectId> = a
            .project()
            .expect("projection")
            .objects
            .iter()
            .map(|object| object.object.id)
            .collect();
        let moved_id = ids[0];
        a.move_objects(&[moved_id], 10.0, 0.0).expect("move");
        let group = b.group_selection(&ids).expect("group").created[0];
        let diff = b.encode_diff_v1(&a.state_vector_v1()).expect("diff");
        a.apply_remote_update_v1(&diff).expect("apply grouping");
        assert_eq!(
            a.project()
                .expect("projection")
                .get(moved_id)
                .unwrap()
                .parent,
            Some(group)
        );
        a.undo().expect("undo move");
        let after = a.project().expect("projection");
        let moved = after.get(moved_id).expect("still present");
        assert_eq!(moved.parent, Some(group), "undo must not leave the group");
    }

    #[test]
    fn undo_move_does_not_resurrect_a_remote_delete() {
        let mut a = doc_with_strokes(2);
        let mut b = BoardDocument::new();
        b.apply_remote_update_v1(&a.encode_full_update_v1())
            .expect("sync");
        let id = a.project().expect("projection").objects[0].object.id;
        a.move_objects(&[id], 12.0, 4.0).expect("move");
        b.delete_objects(&[id]).expect("delete");
        let diff = b.encode_diff_v1(&a.state_vector_v1()).expect("diff");
        a.apply_remote_update_v1(&diff).expect("apply delete");
        assert!(a.project().expect("projection").get(id).is_none());
        a.undo().expect("undo move");
        assert!(
            a.project().expect("projection").get(id).is_none(),
            "undoing a local move must not resurrect a peer-deleted object"
        );
    }

    #[test]
    fn over_deep_local_hierarchy_is_rejected() {
        let mut doc = BoardDocument::new();
        let mut objects = Vec::new();
        let mut parent = None;
        for index in 0..66 {
            let object = StoredObject {
                id: ObjectId::new(),
                parent,
                order_key: index,
                pose: Pose::default(),
                kind: ObjectKind::Group,
            };
            parent = Some(object.id);
            objects.push(object);
        }
        let result = doc.commit_import(objects);
        assert!(result.is_err(), "depth 65 must be rejected");
        assert!(doc.project().expect("projection").objects.is_empty());
    }

    #[test]
    fn deep_raw_projection_stays_bounded() {
        let mut raw = HashMap::new();
        let mut parent = None;
        for index in 0..10_000 {
            let id = ObjectId::new();
            raw.insert(
                id,
                StoredObject {
                    id,
                    parent,
                    order_key: index,
                    pose: Pose::default(),
                    kind: ObjectKind::Group,
                },
            );
            parent = Some(id);
        }
        let projection = project_objects(raw, 0);
        assert_eq!(projection.objects.len(), 10_000);
        assert!(
            projection
                .objects
                .iter()
                .all(|object| object.depth <= MAX_GROUP_DEPTH as u16)
        );
    }

    #[test]
    fn remote_updates_do_not_enter_local_undo() {
        let a = doc_with_strokes(1);
        let mut b = BoardDocument::new();
        let diff = a.encode_full_update_v1();
        b.apply_remote_update_v1(&diff).expect("apply");
        assert_eq!(b.undo_depth(), 0);
        b.commit_import(vec![stroke(4.0)]).expect("import");
        assert_eq!(b.undo_depth(), 1);
        b.undo().expect("undo");
        let projection = b.project().expect("projection");
        assert_eq!(projection.objects.len(), 1);
    }

    #[test]
    fn export_snapshot_has_no_deleted_objects() {
        let mut doc = doc_with_strokes(2);
        let id = doc.project().expect("projection").objects[0].object.id;
        doc.delete_objects(&[id]).expect("delete");
        let snapshot = doc.snapshot_visible();
        assert_eq!(snapshot.objects.len(), 1);
        assert_eq!(snapshot.validate(), Ok(()));
    }

    #[test]
    fn cycle_projection_is_deterministic_and_safe() {
        let a = ObjectId::new();
        let b = ObjectId::new();
        let mut first = StoredObject {
            id: a,
            parent: Some(b),
            order_key: 0,
            pose: Pose::default(),
            kind: ObjectKind::Group,
        };
        let mut second = StoredObject {
            id: b,
            parent: Some(a),
            order_key: 1,
            pose: Pose::default(),
            kind: ObjectKind::Group,
        };
        let mut raw = HashMap::new();
        raw.insert(a, first.clone());
        raw.insert(b, second.clone());
        let projection = project_objects(raw, 0);
        assert_eq!(projection.objects.len(), 2);
        let roots = projection.top_level().count();
        assert_eq!(roots, 1);
        // Same input, same output.
        let mut raw = HashMap::new();
        first.parent = Some(b);
        second.parent = Some(a);
        raw.insert(a, first);
        raw.insert(b, second);
        let second_projection = project_objects(raw, 0);
        assert_eq!(projection, second_projection);
    }

    #[test]
    fn invalid_local_parent_is_rejected() {
        let mut doc = BoardDocument::new();
        let object = stroke(0.0);
        let missing = ObjectId::new();
        let mut bad = object.clone();
        bad.parent = Some(missing);
        let result = doc.commit_import(vec![bad]);
        assert!(matches!(
            result,
            Err(DocumentError::Command(CommandError::InvalidObject))
        ));
        assert!(doc.project().expect("projection").objects.is_empty());
    }

    #[test]
    fn missing_parent_in_raw_data_is_projected_to_root() {
        let object = stroke(0.0);
        let mut bad = object.clone();
        bad.parent = Some(ObjectId::new());
        let mut raw = HashMap::new();
        raw.insert(bad.id, bad);
        let projection = project_objects(raw, 0);
        assert_eq!(projection.objects.len(), 1);
        assert_eq!(
            projection.objects[0].top_id,
            projection.objects[0].object.id
        );
    }

    #[test]
    fn imported_object_can_be_moved_and_deleted_with_undo() {
        let mut doc = BoardDocument::new();
        let object = stroke(1.0);
        let id = object.id;
        doc.commit_import(vec![object]).expect("import");
        doc.move_objects(&[id], -1.0, 2.0).expect("move");
        doc.delete_objects(&[id]).expect("delete");
        doc.undo().expect("undo delete");
        assert!(doc.project().expect("projection").get(id).is_some());
    }

    fn normalized(doc: &BoardDocument) -> Vec<(ObjectId, Option<ObjectId>, i64, Pose, ObjectKind)> {
        doc.project()
            .expect("projection")
            .objects
            .iter()
            .map(|projected| {
                (
                    projected.object.id,
                    projected.parent,
                    projected.object.order_key,
                    projected.object.pose,
                    projected.object.kind.clone(),
                )
            })
            .collect()
    }
}
