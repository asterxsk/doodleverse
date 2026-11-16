use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::ObjectId;
use crate::objects::{
    Pose, ShapeObject, StatementObject, StoredObject, StrokeGeometry, StrokeStyle,
};

/// Commands that mutate the shared document. Local camera, selection, appearance, and AI drafts
/// are not document commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DocumentCommand {
    CreateStroke {
        pose: Pose,
        style: StrokeStyle,
        geometry: StrokeGeometry,
    },
    CreateStatement {
        pose: Pose,
        statement: StatementObject,
    },
    CreateShape {
        pose: Pose,
        shape: ShapeObject,
    },
    MoveObjects {
        ids: Vec<ObjectId>,
        dx: f64,
        dy: f64,
    },
    DeleteObjects {
        ids: Vec<ObjectId>,
    },
    GroupSelection {
        ids: Vec<ObjectId>,
    },
    Ungroup {
        ids: Vec<ObjectId>,
    },
    CommitImport {
        objects: Vec<StoredObject>,
    },
    Undo,
    Redo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    EmptySelection,
    UnknownObject,
    InvalidObject,
    GroupNeedsTwoObjects,
    NotTopLevel,
    NotAGroup,
    LimitExceeded,
    WouldCycle,
    NothingToDo,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentOutcome {
    pub changed: Vec<ObjectId>,
    pub created: Vec<ObjectId>,
    pub deleted: Vec<ObjectId>,
}

impl DocumentOutcome {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            changed: Vec::new(),
            created: Vec::new(),
            deleted: Vec::new(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.created.is_empty() && self.deleted.is_empty()
    }
}

/// Field-level before/after pair used by local undo. Local-only undo never records remote updates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectDelta {
    pub id: ObjectId,
    pub before: Option<StoredObject>,
    pub after: Option<StoredObject>,
}

#[derive(Debug, Error, PartialEq)]
pub enum HistoryError {
    #[error("nothing to undo")]
    NothingToUndo,
    #[error("nothing to redo")]
    NothingToRedo,
}
