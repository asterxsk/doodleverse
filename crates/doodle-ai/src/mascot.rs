use serde::Deserialize;

const HOVER_DURATION_MS: u64 = 600;
const BUSY_PHASE_MS: u64 = 3000;
const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MascotMood {
    Neutral,
    Idle,
    Hover,
    Talking,
    Thinking,
    Working,
}

impl MascotMood {
    #[must_use]
    pub fn state_name(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Idle => "idle",
            Self::Hover => "hover",
            Self::Talking => "talking",
            Self::Thinking => "thinking",
            Self::Working => "working",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MascotInput {
    pub reduced_motion: bool,
    pub talking: bool,
    /// Milliseconds since a pointer/focus entry, or `None` when not hovered.
    pub hover_elapsed_ms: Option<u64>,
    /// Milliseconds since the active request began, or `None` when idle.
    pub busy_elapsed_ms: Option<u64>,
}

/// Confirmed presentation priority: reduced motion, talking, active hover jump, busy phase, idle.
#[must_use]
pub fn resolve_mascot(input: MascotInput) -> MascotMood {
    if input.reduced_motion {
        return MascotMood::Neutral;
    }
    if input.talking {
        return MascotMood::Talking;
    }
    if input
        .hover_elapsed_ms
        .is_some_and(|elapsed| elapsed < HOVER_DURATION_MS)
    {
        return MascotMood::Hover;
    }
    if let Some(elapsed) = input.busy_elapsed_ms {
        return if (elapsed / BUSY_PHASE_MS).is_multiple_of(2) {
            MascotMood::Thinking
        } else {
            MascotMood::Working
        };
    }
    MascotMood::Idle
}

#[derive(Debug, Clone, Deserialize)]
pub struct AtlasManifest {
    pub schema_version: u32,
    pub name: String,
    pub atlas: AtlasInfo,
    pub states: std::collections::HashMap<String, AtlasState>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AtlasInfo {
    pub file: String,
    pub frame_width: u32,
    pub frame_height: u32,
    pub columns: u32,
    pub rows: u32,
    pub width: u32,
    pub height: u32,
    pub frame_count: u32,
    pub premultiplied_alpha: bool,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct AtlasState {
    pub first_frame: u32,
    pub frame_count: u32,
    pub duration_ms: u32,
    #[serde(rename = "loop")]
    pub looping: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtlasFrame {
    /// Normalized UV rectangle: `[u0, v0, u1, v1]`.
    pub uv: [f32; 4],
    pub frame_index: u32,
}

pub fn parse_manifest(json: &str) -> Result<AtlasManifest, String> {
    let manifest: AtlasManifest =
        serde_json::from_str(json).map_err(|error| format!("invalid motion manifest: {error}"))?;
    if manifest.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported motion manifest schema {}",
            manifest.schema_version
        ));
    }
    if manifest.atlas.frame_width == 0
        || manifest.atlas.frame_height == 0
        || manifest.atlas.columns == 0
        || manifest.atlas.rows == 0
        || manifest.atlas.width == 0
        || manifest.atlas.height == 0
    {
        return Err("atlas dimensions must be non-zero".into());
    }
    if manifest.atlas.frame_count > manifest.atlas.columns * manifest.atlas.rows {
        return Err("atlas frame count exceeds its grid".into());
    }
    for (name, state) in &manifest.states {
        if state.frame_count == 0 && state.duration_ms != 0 {
            return Err(format!("state {name} has no frames"));
        }
        let end = state.first_frame.saturating_add(state.frame_count);
        if end > manifest.atlas.frame_count {
            return Err(format!("state {name} references missing atlas frames"));
        }
        if state.looping && state.duration_ms == 0 {
            return Err(format!("looping state {name} has no duration"));
        }
    }
    Ok(manifest)
}

/// Frame index for a mood at `elapsed_ms` since that mood became visible.
#[must_use]
pub fn frame_for_mood(mood: MascotMood, elapsed_ms: u64, manifest: &AtlasManifest) -> Option<u32> {
    let state = manifest.states.get(mood.state_name())?;
    if state.frame_count == 0 {
        return Some(state.first_frame.min(manifest.atlas.frame_count));
    }
    let local = if state.duration_ms == 0 {
        0
    } else if state.looping {
        let phase = elapsed_ms % u64::from(state.duration_ms);
        (phase * u64::from(state.frame_count) / u64::from(state.duration_ms))
            .min(u64::from(state.frame_count - 1))
    } else {
        let phase = elapsed_ms.min(u64::from(state.duration_ms));
        (phase * u64::from(state.frame_count) / u64::from(state.duration_ms))
            .min(u64::from(state.frame_count - 1))
    };
    Some(state.first_frame + local as u32)
}

#[must_use]
pub fn frame_uv(frame_index: u32, manifest: &AtlasManifest) -> Option<AtlasFrame> {
    if frame_index >= manifest.atlas.frame_count {
        return None;
    }
    let columns = manifest.atlas.columns.max(1);
    let column = frame_index % columns;
    let row = frame_index / columns;
    if row >= manifest.atlas.rows {
        return None;
    }
    let u0 = (column * manifest.atlas.frame_width) as f32 / manifest.atlas.width as f32;
    let v0 = (row * manifest.atlas.frame_height) as f32 / manifest.atlas.height as f32;
    let u1 = ((column + 1) * manifest.atlas.frame_width) as f32 / manifest.atlas.width as f32;
    let v1 = ((row + 1) * manifest.atlas.frame_height) as f32 / manifest.atlas.height as f32;
    Some(AtlasFrame {
        uv: [u0, v0, u1, v1],
        frame_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = include_str!("../../../assets/mascot/dibby-motion.json");

    fn manifest() -> AtlasManifest {
        parse_manifest(MANIFEST).expect("bundled manifest parses")
    }

    #[test]
    fn priority_matches_contract() {
        let busy = Some(1000);
        assert_eq!(
            resolve_mascot(MascotInput {
                reduced_motion: true,
                talking: true,
                hover_elapsed_ms: Some(0),
                busy_elapsed_ms: busy,
            }),
            MascotMood::Neutral
        );
        assert_eq!(
            resolve_mascot(MascotInput {
                talking: true,
                hover_elapsed_ms: Some(0),
                busy_elapsed_ms: busy,
                ..MascotInput::default()
            }),
            MascotMood::Talking
        );
        assert_eq!(
            resolve_mascot(MascotInput {
                hover_elapsed_ms: Some(599),
                busy_elapsed_ms: busy,
                ..MascotInput::default()
            }),
            MascotMood::Hover
        );
        assert_eq!(
            resolve_mascot(MascotInput {
                hover_elapsed_ms: Some(600),
                busy_elapsed_ms: busy,
                ..MascotInput::default()
            }),
            MascotMood::Thinking
        );
        assert_eq!(
            resolve_mascot(MascotInput {
                busy_elapsed_ms: Some(3000),
                ..MascotInput::default()
            }),
            MascotMood::Working
        );
        assert_eq!(
            resolve_mascot(MascotInput {
                busy_elapsed_ms: Some(6000),
                ..MascotInput::default()
            }),
            MascotMood::Thinking
        );
        assert_eq!(resolve_mascot(MascotInput::default()), MascotMood::Idle);
    }

    #[test]
    fn atlas_frames_stay_in_bounds() {
        let manifest = manifest();
        assert_eq!(manifest.name, "Dibby");
        for mood in [
            MascotMood::Neutral,
            MascotMood::Idle,
            MascotMood::Hover,
            MascotMood::Talking,
            MascotMood::Thinking,
            MascotMood::Working,
        ] {
            for elapsed in [0, 1, 599, 600, 2400, 3000, 6000, 99_999] {
                let index = frame_for_mood(mood, elapsed, &manifest).expect("frame");
                let uv = frame_uv(index, &manifest).expect("uv");
                assert!(uv.uv[0] >= 0.0 && uv.uv[2] <= 1.0);
                assert!(uv.uv[1] >= 0.0 && uv.uv[3] <= 1.0);
                assert!(uv.uv[0] < uv.uv[2] && uv.uv[1] < uv.uv[3]);
            }
        }
        assert_eq!(frame_for_mood(MascotMood::Neutral, 500, &manifest), Some(0));
        assert_eq!(
            frame_for_mood(MascotMood::Hover, 10_000, &manifest),
            Some(72)
        );
    }

    #[test]
    fn invalid_manifest_is_rejected() {
        assert!(parse_manifest("{}").is_err());
        let mut value: serde_json::Value = serde_json::from_str(MANIFEST).expect("json");
        value["schema_version"] = serde_json::json!(1);
        assert!(parse_manifest(&value.to_string()).is_err());
        let mut value: serde_json::Value = serde_json::from_str(MANIFEST).expect("json");
        value["states"]["idle"]["first_frame"] = serde_json::json!(170);
        assert!(parse_manifest(&value.to_string()).is_err());
    }
}
