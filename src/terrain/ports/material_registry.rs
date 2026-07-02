//! `IMaterialRegistry` — terrain material asset registry.
//!
//! Ported from C# `Ports/IMaterialRegistry.cs`. The trait surface is the same
//! as the C# interface; methods take `&mut self` where the recording mock
//! needs to log calls (the previous E0596 fix).

use std::collections::HashMap;
use uuid::Uuid;

use crate::terrain::error::TerrainError;
use crate::terrain::materials::TerrainMaterial;

/// Hexagonal port: registry of terrain materials.
pub trait IMaterialRegistry {
    /// Returns all materials currently registered.
    fn list(&mut self) -> Vec<TerrainMaterial>;
    /// Looks up a material by id.
    fn find(&mut self, id: Uuid) -> Option<TerrainMaterial>;
    /// Registers a material. If an entry with the same id already exists, it
    /// is replaced.
    fn register(&mut self, material: TerrainMaterial) -> Result<(), TerrainError>;
    /// Removes a material by id. Returns `true` if a material was removed.
    fn unregister(&mut self, id: Uuid) -> bool;
}

/// In-memory adapter backed by a `HashMap<Uuid, TerrainMaterial>`.
#[derive(Debug, Default, Clone)]
pub struct InMemoryTerrainMaterialRegistry {
    by_id: HashMap<Uuid, TerrainMaterial>,
}

impl InMemoryTerrainMaterialRegistry {
    /// New empty registry.
    pub fn new() -> Self {
        Self::default()
    }
    /// Number of registered materials.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }
    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

impl IMaterialRegistry for InMemoryTerrainMaterialRegistry {
    fn list(&mut self) -> Vec<TerrainMaterial> {
        self.by_id.values().cloned().collect()
    }
    fn find(&mut self, id: Uuid) -> Option<TerrainMaterial> {
        self.by_id.get(&id).cloned()
    }
    fn register(&mut self, material: TerrainMaterial) -> Result<(), TerrainError> {
        if material.name().trim().is_empty() {
            return Err(TerrainError::NullMaterial);
        }
        self.by_id.insert(material.id(), material);
        Ok(())
    }
    fn unregister(&mut self, id: Uuid) -> bool {
        self.by_id.remove(&id).is_some()
    }
}

/// Recording mock used by domain tests. Each method call is logged to a list
/// the test can replay.
#[derive(Debug, Default, Clone)]
pub struct RecordingTerrainMaterialRegistry {
    by_id: HashMap<Uuid, TerrainMaterial>,
    calls: Vec<String>,
}

impl RecordingTerrainMaterialRegistry {
    /// New empty recording mock.
    pub fn new() -> Self {
        Self::default()
    }
    /// Sequence of method names invoked on this mock.
    pub fn calls(&self) -> &[String] {
        &self.calls
    }
    /// Reset the call log (keeps the registry contents intact).
    pub fn reset_calls(&mut self) {
        self.calls.clear();
    }
}

impl IMaterialRegistry for RecordingTerrainMaterialRegistry {
    fn list(&mut self) -> Vec<TerrainMaterial> {
        self.calls.push("List".to_string());
        self.by_id.values().cloned().collect()
    }
    fn find(&mut self, id: Uuid) -> Option<TerrainMaterial> {
        self.calls.push(format!("Find({})", id));
        self.by_id.get(&id).cloned()
    }
    fn register(&mut self, material: TerrainMaterial) -> Result<(), TerrainError> {
        if material.name().trim().is_empty() {
            return Err(TerrainError::NullMaterial);
        }
        self.calls.push(format!("Register({})", material.id()));
        self.by_id.insert(material.id(), material);
        Ok(())
    }
    fn unregister(&mut self, id: Uuid) -> bool {
        self.calls.push(format!("Unregister({})", id));
        self.by_id.remove(&id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_register_and_find_round_trip() {
        let mut reg = InMemoryTerrainMaterialRegistry::new();
        let a = TerrainMaterial::new("Grass").unwrap();
        let b = TerrainMaterial::new("Rock").unwrap();
        reg.register(a.clone()).unwrap();
        reg.register(b.clone()).unwrap();
        let found_a = reg.find(a.id()).unwrap();
        let found_b = reg.find(b.id()).unwrap();
        assert_eq!(found_a.name(), "Grass");
        assert_eq!(found_b.name(), "Rock");
        assert!(reg.find(Uuid::new_v4()).is_none());
    }

    #[test]
    fn in_memory_unregister_removes_entry() {
        let mut reg = InMemoryTerrainMaterialRegistry::new();
        let m = TerrainMaterial::new("Dirt").unwrap();
        reg.register(m.clone()).unwrap();
        assert!(reg.unregister(m.id()));
        assert!(!reg.unregister(m.id()));
    }

    #[test]
    fn in_memory_list_returns_all() {
        let mut reg = InMemoryTerrainMaterialRegistry::new();
        reg.register(TerrainMaterial::new("A").unwrap()).unwrap();
        reg.register(TerrainMaterial::new("B").unwrap()).unwrap();
        assert_eq!(reg.list().len(), 2);
    }

    #[test]
    fn recording_mock_captures_call_sequence() {
        let mut mock = RecordingTerrainMaterialRegistry::new();
        let m = TerrainMaterial::new("Sand").unwrap();
        mock.register(m.clone()).unwrap();
        mock.find(m.id());
        mock.unregister(m.id());
        assert_eq!(
            mock.calls(),
            &[
                format!("Register({})", m.id()),
                format!("Find({})", m.id()),
                format!("Unregister({})", m.id()),
            ]
        );
    }

    #[test]
    fn recording_mock_reset_calls_keeps_state() {
        let mut mock = RecordingTerrainMaterialRegistry::new();
        let m = TerrainMaterial::new("Clay").unwrap();
        mock.register(m.clone()).unwrap();
        mock.reset_calls();
        assert!(mock.calls().is_empty());
        // State is intact
        assert!(mock.find(m.id()).is_some());
    }
}
