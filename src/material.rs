//! Voxel materials and palettes. Engine-agnostic; renderers translate
//! [`MaterialId`] into engine-specific PBR materials at the renderer boundary.

use serde::{Deserialize, Serialize};

/// Stable per-world material identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MaterialId(pub u16);

/// Mesh-neutral material description. The renderer pairs `MaterialId` with its own
/// shader / texture set; this struct only carries the source-of-truth metadata that
/// the simulation cares about (era, hardness for destruction, conductivity, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelMaterial {
    /// Human-readable id (mod-friendly).
    pub name: String,
    /// Era index — used by `civ-build` grammars and `civ-diffusion` adoption curves.
    pub era: u16,
    /// Destructibility hardness; consumers like `civ-tactics` use this for damage.
    pub hardness: f32,
}

/// Palette of materials currently in use. Iteration order is the insertion order of
/// the underlying `Vec` so consumers get a deterministic enumeration.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialPalette {
    /// Materials indexed by their [`MaterialId`].
    pub materials: Vec<VoxelMaterial>,
}

impl MaterialPalette {
    /// Append a new material. Returns its newly-assigned [`MaterialId`].
    pub fn add(&mut self, material: VoxelMaterial) -> MaterialId {
        let id = u16::try_from(self.materials.len()).expect("palette overflow");
        self.materials.push(material);
        MaterialId(id)
    }

    /// Look up a material by id.
    #[must_use]
    pub fn get(&self, id: MaterialId) -> Option<&VoxelMaterial> {
        self.materials.get(id.0 as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-PHENO-VOXEL-MATERIAL-000 — palette assigns sequential ids.
    #[test]
    fn palette_assigns_sequential_ids() {
        let mut p = MaterialPalette::default();
        let a = p.add(VoxelMaterial {
            name: "mud-brick".into(),
            era: 0,
            hardness: 1.0,
        });
        let b = p.add(VoxelMaterial {
            name: "reinforced-concrete".into(),
            era: 4,
            hardness: 50.0,
        });
        assert_eq!(a, MaterialId(0));
        assert_eq!(b, MaterialId(1));
        assert_eq!(p.get(a).unwrap().name, "mud-brick");
        assert_eq!(p.get(b).unwrap().name, "reinforced-concrete");
    }
}
