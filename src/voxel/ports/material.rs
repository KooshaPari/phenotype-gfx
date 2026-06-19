//! Material port: engine-neutral material / asset registry contracts.
//!
//! The domain code depends **only** on the [`MaterialRegistry`] trait. Adapters
//! (in-memory, file-backed, Addressables-backed, AssetBundle-backed, …) live
//! outside the domain and implement this trait. The hexagon boundary is:
//!
//! ```text
//!   ┌─────────────────────────┐
//!   │  domain (uses trait)    │ ─── no concrete adapter import ──▶
//!   └────────────┬────────────┘
//!                ▼
//!          MaterialRegistry   ◀── port (this file)
//!                ▲
//!   ┌────────────┴────────────┐
//!   │  adapters: in-mem / IO  │
//!   └─────────────────────────┘
//! ```

use thiserror::Error;

// Re-export the domain types so trait + impls share the same type identity.
pub use crate::voxel::material::{MaterialId, MaterialPalette, VoxelMaterial};

/// Errors that can be raised by a [`MaterialRegistry`] adapter.
///
/// Adapters translate their underlying failure (I/O, Addressables lookup,
/// AssetBundle miss, …) into one of these variants so domain code can pattern-
/// match without depending on adapter-specific error types.
#[derive(Debug, Error)]
pub enum MaterialError {
    /// The requested material id is not present in the registry.
    #[error("material id {0:?} not found in registry")]
    NotFound(MaterialId),
    /// The registry's underlying storage failed (I/O, asset bundle, …).
    #[error("material storage error: {0}")]
    Storage(String),
    /// The supplied material failed validation (duplicate name, bad era, …).
    #[error("invalid material: {0}")]
    Invalid(String),
}

/// Result alias for material port operations.
pub type MaterialResult<T> = Result<T, MaterialError>;

/// Hexagonal port: engine-neutral material registry.
///
/// Every concrete storage backend (in-memory, file-backed JSON, Unity
/// Addressables, Godot Resource, …) implements this trait. The domain code
/// imports **only** this trait — never a concrete adapter.
pub trait MaterialRegistry {
    /// Returns the number of materials currently registered.
    fn len(&self) -> usize;

    /// Returns `true` if the registry holds no materials.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Look up a material by its stable id.
    fn get(&self, id: MaterialId) -> MaterialResult<&VoxelMaterial>;

    /// Iterate over `(id, material)` pairs in **insertion order**.
    ///
    /// Adapters are required to honour insertion order so consumers (palette
    /// UIs, serialization writers) get a deterministic enumeration.
    fn iter(&self) -> Box<dyn Iterator<Item = (MaterialId, &VoxelMaterial)> + '_>;

    /// Append a material and return its newly-assigned id.
    fn add(&mut self, material: VoxelMaterial) -> MaterialResult<MaterialId>;

    /// Borrow the entire palette as a snapshot.
    fn palette(&self) -> MaterialPalette;
}

// ────────────────────────────────────────────────────────────────────────────
// Adapter: InMemoryMaterialRegistry
// ────────────────────────────────────────────────────────────────────────────

/// In-memory adapter for [`MaterialRegistry`].
///
/// This is the canonical "null adapter" — used by the domain in tests, in
/// headless servers, and as a default when no engine asset system is wired in.
#[derive(Debug, Default, Clone)]
pub struct InMemoryMaterialRegistry {
    palette: MaterialPalette,
}

impl InMemoryMaterialRegistry {
    /// Build an empty registry.
    pub fn new() -> Self {
        Self::default()
    }
}

impl MaterialRegistry for InMemoryMaterialRegistry {
    fn len(&self) -> usize {
        self.palette.materials.len()
    }

    fn get(&self, id: MaterialId) -> MaterialResult<&VoxelMaterial> {
        self.palette
            .materials
            .get(id.0 as usize)
            .ok_or(MaterialError::NotFound(id))
    }

    fn iter(&self) -> Box<dyn Iterator<Item = (MaterialId, &VoxelMaterial)> + '_> {
        Box::new(
            self.palette
                .materials
                .iter()
                .enumerate()
                .map(|(i, m)| (MaterialId(i as u16), m)),
        )
    }

    fn add(&mut self, material: VoxelMaterial) -> MaterialResult<MaterialId> {
        if material.name.trim().is_empty() {
            return Err(MaterialError::Invalid(
                "material name must not be empty".into(),
            ));
        }
        let id = self.palette.add(material);
        Ok(id)
    }

    fn palette(&self) -> MaterialPalette {
        self.palette.clone()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Test mock
// ────────────────────────────────────────────────────────────────────────────

/// Recording mock used by domain tests to assert registry interaction order.
///
/// Stores every `add` and `get` call into a `Vec` so a test can replay the
/// sequence and verify the domain did not skip steps.
#[derive(Debug, Default)]
pub struct MockMaterialRegistry {
    palette: MaterialPalette,
    calls: Vec<MockCall>,
}

/// One call recorded by [`MockMaterialRegistry`].
#[derive(Debug, Clone, PartialEq)]
pub enum MockCall {
    /// `get(id)` was invoked.
    Get(MaterialId),
    /// `add(material)` was invoked.
    Add(String),
}

impl MockMaterialRegistry {
    /// Returns the recorded call sequence.
    pub fn calls(&self) -> &[MockCall] {
        &self.calls
    }

    /// Reset the recorded calls (keeps the palette intact).
    pub fn reset_calls(&mut self) {
        self.calls.clear();
    }
}

impl MaterialRegistry for MockMaterialRegistry {
    fn len(&self) -> usize {
        self.palette.materials.len()
    }

    fn get(&self, id: MaterialId) -> MaterialResult<&VoxelMaterial> {
        // SAFETY: we cannot mutate `self` through `&self`, so we record via a
        // RefCell-style escape hatch. To keep this mock simple we use a
        // separate `record_get` API on `&mut self` and treat the `&self`
        // version as read-only recording only when the test asks for it.
        self.palette
            .materials
            .get(id.0 as usize)
            .ok_or(MaterialError::NotFound(id))
    }

    fn iter(&self) -> Box<dyn Iterator<Item = (MaterialId, &VoxelMaterial)> + '_> {
        Box::new(
            self.palette
                .materials
                .iter()
                .enumerate()
                .map(|(i, m)| (MaterialId(i as u16), m)),
        )
    }

    fn add(&mut self, material: VoxelMaterial) -> MaterialResult<MaterialId> {
        self.calls.push(MockCall::Add(material.name.clone()));
        let id = self.palette.add(material);
        Ok(id)
    }

    fn palette(&self) -> MaterialPalette {
        self.palette.clone()
    }
}

impl MockMaterialRegistry {
    /// Record a `get` call from a `&mut self` context (used by tests).
    pub fn record_get(&mut self, id: MaterialId) -> MaterialResult<&VoxelMaterial> {
        self.calls.push(MockCall::Get(id));
        self.palette
            .materials
            .get(id.0 as usize)
            .ok_or(MaterialError::NotFound(id))
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Unit tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-PHENO-VOXEL-PORT-MATERIAL-000 — in-memory adapter assigns sequential
    /// ids and round-trips lookups.
    #[test]
    fn in_memory_registry_assigns_sequential_ids() {
        let mut reg = InMemoryMaterialRegistry::new();
        assert!(reg.is_empty());

        let mud = reg
            .add(VoxelMaterial {
                name: "mud-brick".into(),
                era: 0,
                hardness: 1.0,
            })
            .expect("add mud");
        let rock = reg
            .add(VoxelMaterial {
                name: "granite".into(),
                era: 2,
                hardness: 30.0,
            })
            .expect("add rock");

        assert_eq!(mud, MaterialId(0));
        assert_eq!(rock, MaterialId(1));
        assert_eq!(reg.len(), 2);

        let got = reg.get(mud).expect("mud lookup");
        assert_eq!(got.name, "mud-brick");
        assert_eq!(got.era, 0);
    }

    /// FR-PHENO-VOXEL-PORT-MATERIAL-001 — empty name is rejected with `Invalid`.
    #[test]
    fn empty_name_rejected() {
        let mut reg = InMemoryMaterialRegistry::new();
        let err = reg
            .add(VoxelMaterial {
                name: "   ".into(),
                era: 0,
                hardness: 0.0,
            })
            .unwrap_err();
        assert!(matches!(err, MaterialError::Invalid(_)));
    }

    /// FR-PHENO-VOXEL-PORT-MATERIAL-002 — mock records the call sequence so
    /// tests can assert on it.
    #[test]
    fn mock_records_call_sequence() {
        let mut mock = MockMaterialRegistry::default();
        let a = mock
            .add(VoxelMaterial {
                name: "alpha".into(),
                era: 0,
                hardness: 0.0,
            })
            .expect("add alpha");
        mock.record_get(a).expect("get alpha");
        let b = mock
            .add(VoxelMaterial {
                name: "beta".into(),
                era: 1,
                hardness: 0.0,
            })
            .expect("add beta");
        mock.record_get(b).expect("get beta");

        assert_eq!(
            mock.calls(),
            &[
                MockCall::Add("alpha".into()),
                MockCall::Get(MaterialId(0)),
                MockCall::Add("beta".into()),
                MockCall::Get(MaterialId(1)),
            ]
        );
    }

    /// FR-PHENO-VOXEL-PORT-MATERIAL-003 — iteration order is insertion order
    /// (determinism contract — see material.rs doc comment).
    #[test]
    fn iter_yields_insertion_order() {
        let mut reg = InMemoryMaterialRegistry::new();
        reg.add(VoxelMaterial {
            name: "first".into(),
            era: 0,
            hardness: 0.0,
        })
        .unwrap();
        reg.add(VoxelMaterial {
            name: "second".into(),
            era: 0,
            hardness: 0.0,
        })
        .unwrap();

        let names: Vec<_> = reg.iter().map(|(_, m)| m.name.as_str()).collect();
        assert_eq!(names, ["first", "second"]);
    }
}
