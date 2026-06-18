//! `IMaterialRegistry` — water material asset registry.
//!
//! Ported from C# `Ports/IMaterialRegistry.cs`. The trait surface is the same
//! as the C# interface; methods take `&mut self` where the recording mock
//! needs to log calls (the previous E0596 fix).
//!
//! `WaterMaterial` is a deprecated pass-through, so the registry uses the
//! material's `id: u64` as the registry key (the C# equivalent derived a
//! Guid from `material.GetHashCode()`).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::water::error::{WaterError, WaterResult};
use crate::water::rendering::water_material::WaterMaterial;

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

/// Returns the next fresh material handle. Called by [`WaterMaterial::new`].
pub(crate) fn next_handle() -> u64 { NEXT_HANDLE.fetch_add(1, Ordering::Relaxed) }

/// Hexagonal port: registry of water materials.
pub trait IMaterialRegistry {
    /// Returns all materials currently registered.
    fn list(&mut self) -> Vec<WaterMaterial>;
    /// Looks up a material by id.
    fn find(&mut self, id: u64) -> Option<WaterMaterial>;
    /// Registers a material. If an entry with the same id already exists, it
    /// is replaced.
    fn register(&mut self, material: WaterMaterial) -> WaterResult<()>;
    /// Removes a material by id. Returns `true` if a material was removed.
    fn unregister(&mut self, id: u64) -> bool;
}

/// In-memory adapter backed by a `HashMap<u64, WaterMaterial>`.
#[derive(Debug, Default, Clone)]
pub struct InMemoryWaterMaterialRegistry {
    by_id: HashMap<u64, WaterMaterial>,
}

impl InMemoryWaterMaterialRegistry {
    /// New empty registry.
    pub fn new() -> Self { Self::default() }
    /// Number of registered materials.
    pub fn len(&self) -> usize { self.by_id.len() }
    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool { self.by_id.is_empty() }
}

impl IMaterialRegistry for InMemoryWaterMaterialRegistry {
    fn list(&mut self) -> Vec<WaterMaterial> { self.by_id.values().cloned().collect() }
    fn find(&mut self, id: u64) -> Option<WaterMaterial> { self.by_id.get(&id).cloned() }
    fn register(&mut self, material: WaterMaterial) -> WaterResult<()> {
        if material.label().trim().is_empty() {
            return Err(WaterError::NullMaterial);
        }
        self.by_id.insert(material.id(), material);
        Ok(())
    }
    fn unregister(&mut self, id: u64) -> bool { self.by_id.remove(&id).is_some() }
}

/// Recording mock used by domain tests. Each method call is logged to a list
/// the test can replay.
#[derive(Debug, Default, Clone)]
pub struct RecordingWaterMaterialRegistry {
    by_id: HashMap<u64, WaterMaterial>,
    calls: Vec<String>,
}

impl RecordingWaterMaterialRegistry {
    /// New empty recording mock.
    pub fn new() -> Self { Self::default() }
    /// Sequence of method names invoked on this mock.
    pub fn calls(&self) -> &[String] { &self.calls }
    /// Reset the call log (keeps the registry contents intact).
    pub fn reset_calls(&mut self) { self.calls.clear(); }
}

impl IMaterialRegistry for RecordingWaterMaterialRegistry {
    fn list(&mut self) -> Vec<WaterMaterial> {
        self.calls.push("List".to_string());
        self.by_id.values().cloned().collect()
    }
    fn find(&mut self, id: u64) -> Option<WaterMaterial> {
        self.calls.push(format!("Find({})", id));
        self.by_id.get(&id).cloned()
    }
    fn register(&mut self, material: WaterMaterial) -> WaterResult<()> {
        if material.label().trim().is_empty() {
            return Err(WaterError::NullMaterial);
        }
        self.calls.push(format!("Register({})", material.id()));
        self.by_id.insert(material.id(), material);
        Ok(())
    }
    fn unregister(&mut self, id: u64) -> bool {
        self.calls.push(format!("Unregister({})", id));
        self.by_id.remove(&id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::water::rendering::water_shader::WaterShader;

    #[test]
    fn in_memory_register_and_find_round_trip() {
        let shader = WaterShader::new("Phenotype/Water");
        let mut reg = InMemoryWaterMaterialRegistry::new();
        let a = WaterMaterial::new("Deep", &shader);
        let b = WaterMaterial::new("Shallow", &shader);
        let a_id = a.id();
        let b_id = b.id();
        reg.register(a).unwrap();
        reg.register(b).unwrap();
        let found_a = reg.find(a_id).unwrap();
        let found_b = reg.find(b_id).unwrap();
        assert_eq!(found_a.label(), "Deep");
        assert_eq!(found_b.label(), "Shallow");
        assert!(reg.find(999_999).is_none());
    }

    #[test]
    fn in_memory_unregister_removes_entry() {
        let shader = WaterShader::new("Phenotype/Water");
        let mut reg = InMemoryWaterMaterialRegistry::new();
        let m = WaterMaterial::new("Dirt", &shader);
        let id = m.id();
        reg.register(m).unwrap();
        assert!(reg.unregister(id));
        assert!(!reg.unregister(id));
    }

    #[test]
    fn recording_mock_captures_call_sequence() {
        let shader = WaterShader::new("Phenotype/Water");
        let mut mock = RecordingWaterMaterialRegistry::new();
        let m = WaterMaterial::new("Sand", &shader);
        let id = m.id();
        mock.register(m).unwrap();
        mock.find(id);
        mock.unregister(id);
        assert_eq!(mock.calls(), &[
            format!("Register({})", id),
            format!("Find({})", id),
            format!("Unregister({})", id),
        ]);
    }
}
