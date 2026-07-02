// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! `ISerializationPort` — hexagonal port for post-fx stack save / load.
//!
//! The `PostStack` driver and the editor inspector can persist the current
//! pipeline (effect toggles, quality settings, LUT path) so a user can save a
//! post-fx profile and restore it across sessions. The port abstracts the
//! concrete wire format (JSON, YAML, binary) and storage backend (file,
//! PlayerPrefs, cloud save).

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::postfx::error::{PostFxError, PostFxResult};

/// Serializable snapshot of a post-fx stack configuration.
///
/// Engine-agnostic by design: it carries only logical state, not Unity
/// references. The port is responsible for translating this snapshot into
/// the engine's runtime representation on load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostFxStackSnapshot {
    /// Format version of this snapshot. Bumped on breaking changes.
    #[serde(default = "default_version")]
    pub version: u32,
    /// Human-readable name of the snapshot (e.g. `"Cinematic-Moody"`).
    #[serde(default)]
    pub name: String,
    /// Currently loaded LUT path (or empty for identity).
    #[serde(default)]
    pub lut_path: String,
    /// Bloom intensity, normalised to `[0, 1]`.
    #[serde(default)]
    pub bloom_intensity: f32,
    /// SSAO toggle.
    #[serde(default)]
    pub ssao_enabled: bool,
    /// Vignette toggle.
    #[serde(default)]
    pub vignette_enabled: bool,
}

fn default_version() -> u32 {
    1
}

impl Default for PostFxStackSnapshot {
    fn default() -> Self {
        Self {
            version: 1,
            name: String::new(),
            lut_path: String::new(),
            bloom_intensity: 0.0,
            ssao_enabled: false,
            vignette_enabled: false,
        }
    }
}

/// Hexagonal port: save / load post-fx stack snapshots.
pub trait PostFxSerializationPort {
    /// Stable format identifier (e.g. `"postfx-json-v1"`).
    fn format_id(&self) -> &str;
    /// Serialize `snapshot` to `destination` (backend-specific).
    fn save(&self, snapshot: &PostFxStackSnapshot, destination: &str) -> PostFxResult<()>;
    /// Load and deserialize a snapshot from `destination`.
    fn load(&self, destination: &str) -> PostFxResult<PostFxStackSnapshot>;
}

/// Default JSON-on-disk adapter.
#[derive(Debug, Default, Clone, Copy)]
pub struct JsonFilePostFxSerialization;

impl JsonFilePostFxSerialization {
    /// Format identifier used by tests and debug prints.
    pub const FORMAT_ID: &'static str = "postfx-json-v1";
}

impl PostFxSerializationPort for JsonFilePostFxSerialization {
    fn format_id(&self) -> &str {
        Self::FORMAT_ID
    }

    fn save(&self, snapshot: &PostFxStackSnapshot, destination: &str) -> PostFxResult<()> {
        if destination.is_empty() {
            return Err(PostFxError::InvalidPassDescriptor(
                "destination must not be empty".into(),
            ));
        }
        let json = serde_json::to_string_pretty(snapshot)?;
        if let Some(parent) = Path::new(destination).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(destination, json)?;
        Ok(())
    }

    fn load(&self, destination: &str) -> PostFxResult<PostFxStackSnapshot> {
        if destination.is_empty() {
            return Err(PostFxError::InvalidPassDescriptor(
                "destination must not be empty".into(),
            ));
        }
        let bytes = fs::read(destination).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => {
                PostFxError::InvalidLut(format!("snapshot file not found: {destination}"))
            }
            _ => PostFxError::Io(e),
        })?;
        if bytes.is_empty() {
            return Err(PostFxError::InvalidLut(format!(
                "snapshot file is empty: {destination}"
            )));
        }
        let snapshot: PostFxStackSnapshot = serde_json::from_slice(&bytes).map_err(|e| {
            PostFxError::InvalidLut(format!("could not deserialize snapshot {destination}: {e}"))
        })?;
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn tmp_path(name: &str) -> String {
        let mut p = env::temp_dir();
        p.push(format!("postfx-snap-{}-{}.json", name, std::process::id()));
        p.to_string_lossy().to_string()
    }

    #[test]
    fn json_adapter_round_trips_through_file() {
        let path = tmp_path("rt");
        let _ = std::fs::remove_file(&path);
        let port = JsonFilePostFxSerialization;
        assert_eq!(port.format_id(), "postfx-json-v1");

        let original = PostFxStackSnapshot {
            name: "Cinematic-Moody".into(),
            lut_path: "LUTs/CinematicMoody.cube".into(),
            bloom_intensity: 0.7,
            ssao_enabled: true,
            vignette_enabled: true,
            ..Default::default()
        };
        port.save(&original, &path).unwrap();
        let recovered = port.load(&path).unwrap();
        assert_eq!(recovered, original);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn json_adapter_load_missing_file_raises() {
        let port = JsonFilePostFxSerialization;
        let missing = tmp_path("missing");
        let _ = std::fs::remove_file(&missing);
        let res = port.load(&missing);
        assert!(res.is_err());
    }
}
