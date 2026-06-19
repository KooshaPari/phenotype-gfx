//! `ISerializationPort` — terrain save/load.
//!
//! Ported from C# `Ports/ISerializationPort.cs`. Format version is `1`.

use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::terrain::error::TerrainError;

/// Serializable snapshot of a terrain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainSnapshot {
    /// Format version of this snapshot. Bumped on breaking changes.
    #[serde(default = "default_version")]
    pub version: i32,
    /// Width of the height-field in tiles.
    #[serde(default)]
    pub width: usize,
    /// Height of the height-field in tiles.
    #[serde(default)]
    pub height: usize,
    /// Flat elevation array, length = `width * height`.
    #[serde(default)]
    pub elevations: Vec<f32>,
    /// Ids of the materials referenced by this terrain.
    #[serde(default)]
    pub material_ids: Vec<String>,
}

fn default_version() -> i32 { 1 }

/// Hexagonal port: save / load terrain snapshots.
pub trait ISerializationPort {
    /// Format identifier (e.g. `"terrain-json-v1"`).
    fn format_id(&self) -> &'static str;
    /// Serializes `snapshot` to `destination` (file path / key / etc.).
    fn save(&self, snapshot: &TerrainSnapshot, destination: &str) -> Result<(), TerrainError>;
    /// Loads a snapshot from `destination`.
    fn load(&self, destination: &str) -> Result<TerrainSnapshot, TerrainError>;
}

/// Default JSON-on-disk adapter.
#[derive(Debug, Default, Clone, Copy)]
pub struct JsonFileSerializationPort;

impl JsonFileSerializationPort {
    /// New JSON adapter.
    pub fn new() -> Self { Self }
}

impl ISerializationPort for JsonFileSerializationPort {
    fn format_id(&self) -> &'static str { "terrain-json-v1" }

    fn save(&self, snapshot: &TerrainSnapshot, destination: &str) -> Result<(), TerrainError> {
        if destination.trim().is_empty() {
            return Err(TerrainError::OutOfBounds {
                msg: "Destination must not be empty.".to_string(),
            });
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

    fn load(&self, destination: &str) -> Result<TerrainSnapshot, TerrainError> {
        if destination.trim().is_empty() {
            return Err(TerrainError::OutOfBounds {
                msg: "Destination must not be empty.".to_string(),
            });
        }
        if !Path::new(destination).exists() {
            return Err(TerrainError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Snapshot file not found: {}", destination),
            )));
        }
        let json = fs::read_to_string(destination)?;
        if json.trim().is_empty() {
            return Err(TerrainError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("Snapshot file is empty: {}", destination),
            )));
        }
        let snap: TerrainSnapshot = serde_json::from_str(&json)?;
        Ok(snap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn tmp_path(name: &str) -> String {
        env::temp_dir().join(format!("terrain-snap-{}.json", name)).to_string_lossy().into_owned()
    }

    #[test]
    fn json_adapter_round_trips_through_file() {
        let path = tmp_path(&uuid::Uuid::new_v4().simple().to_string());
        let port = JsonFileSerializationPort::new();
        assert_eq!(port.format_id(), "terrain-json-v1");

        let original = TerrainSnapshot {
            version: 1,
            width: 4,
            height: 4,
            elevations: (0..16).map(|i| i as f32).collect(),
            material_ids: vec!["grass".to_string(), "rock".to_string()],
        };
        port.save(&original, &path).unwrap();
        let recovered = port.load(&path).unwrap();
        assert_eq!(recovered, original);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn json_adapter_load_missing_file_raises() {
        let port = JsonFileSerializationPort::new();
        let missing = tmp_path(&format!("missing-{}", uuid::Uuid::new_v4().simple()));
        let err = port.load(&missing).unwrap_err();
        // Compare by display string since Io is not PartialEq natively.
        assert!(err.to_string().contains("Snapshot file not found"));
    }

    #[test]
    fn json_adapter_load_empty_file_raises() {
        let port = JsonFileSerializationPort::new();
        let path = tmp_path(&format!("empty-{}", uuid::Uuid::new_v4().simple()));
        fs::write(&path, "").unwrap();
        let err = port.load(&path).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("empty"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn json_adapter_save_empty_destination_raises() {
        let port = JsonFileSerializationPort::new();
        let snap = TerrainSnapshot { version: 1, width: 1, height: 1, elevations: vec![0.0], material_ids: vec![] };
        assert!(port.save(&snap, "").is_err());
    }

    #[test]
    fn json_adapter_load_empty_destination_raises() {
        let port = JsonFileSerializationPort::new();
        assert!(port.load("").is_err());
    }
}
