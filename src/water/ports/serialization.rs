//! `ISerializationPort` — water save/load.
//!
//! Ported from C# `Ports/ISerializationPort.cs`. Format version is `1`.
//! `MockSerializationPort` is dropped per the migration audit (YAGNI).

use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::water::error::{WaterError, WaterResult};

/// Serializable snapshot of a body of water.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaterSnapshot {
    /// Format version of this snapshot. Bumped on breaking changes.
    #[serde(default = "default_version")]
    pub version: i32,
    /// Human-readable name.
    #[serde(default)]
    pub name: String,
    /// Sea level in world units.
    #[serde(default)]
    pub sea_level: f32,
    /// Names of the materials referenced by this body of water.
    #[serde(default)]
    pub material_names: Vec<String>,
}

fn default_version() -> i32 { 1 }

/// Hexagonal port: save / load water snapshots.
pub trait ISerializationPort {
    /// Format identifier.
    fn format_id(&self) -> &'static str;
    /// Save the snapshot to `destination`.
    fn save(&self, snapshot: &WaterSnapshot, destination: &str) -> WaterResult<()>;
    /// Load a snapshot from `destination`.
    fn load(&self, destination: &str) -> WaterResult<WaterSnapshot>;
}

/// Default JSON-on-disk adapter.
#[derive(Debug, Default, Clone, Copy)]
pub struct JsonFileSerializationPort;

impl JsonFileSerializationPort {
    /// New JSON adapter.
    pub fn new() -> Self { Self }
}

impl ISerializationPort for JsonFileSerializationPort {
    fn format_id(&self) -> &'static str { "water-json-v1" }

    fn save(&self, snapshot: &WaterSnapshot, destination: &str) -> WaterResult<()> {
        if destination.trim().is_empty() {
            return Err(WaterError::OutOfBounds {
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

    fn load(&self, destination: &str) -> WaterResult<WaterSnapshot> {
        if destination.trim().is_empty() {
            return Err(WaterError::OutOfBounds {
                msg: "Destination must not be empty.".to_string(),
            });
        }
        if !Path::new(destination).exists() {
            return Err(WaterError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Snapshot file not found: {}", destination),
            )));
        }
        let json = fs::read_to_string(destination)?;
        if json.trim().is_empty() {
            return Err(WaterError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("Snapshot file is empty: {}", destination),
            )));
        }
        let snap: WaterSnapshot = serde_json::from_str(&json)?;
        Ok(snap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn tmp_path(name: &str) -> String {
        env::temp_dir().join(format!("water-snap-{}.json", name)).to_string_lossy().into_owned()
    }

    #[test]
    fn json_adapter_round_trips_through_file() {
        let path = tmp_path(&uuid::Uuid::new_v4().simple().to_string());
        let port = JsonFileSerializationPort::new();
        assert_eq!(port.format_id(), "water-json-v1");

        let original = WaterSnapshot {
            version: 1,
            name: "Lake".to_string(),
            sea_level: 0.0,
            material_names: vec!["deep".to_string(), "shallow".to_string()],
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
    fn save_empty_destination_raises() {
        let port = JsonFileSerializationPort::new();
        let snap = WaterSnapshot { version: 1, name: "X".to_string(), sea_level: 0.0, material_names: vec![] };
        assert!(port.save(&snap, "").is_err());
    }
}
