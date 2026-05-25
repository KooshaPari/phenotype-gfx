//! Shape hint registry: maps sprite/asset name prefixes to voxelization hints.
//!
//! Ported from WSM3D `AssetShapeRegistry.cs`. The registry stores an ordered
//! list of `(prefix, ShapeHint)` pairs; the first matching prefix wins.
//! Matching is case-insensitive and checks three positions: start-of-name,
//! `_prefix`, and `prefix_` — exactly the same three checks WSM3D uses.
//!
//! # Built-in hints
//!
//! [`ShapeHintRegistry::with_wsm3d_defaults`] pre-populates the same 47 prefix
//! entries as `AssetShapeRegistry._prefixHints`. Consumers that want a clean
//! slate can call [`ShapeHintRegistry::new`] instead.
//!
//! # Usage
//!
//! ```rust
//! use phenotype_voxel::shape_hints::{ShapeHint, ShapeHintRegistry};
//!
//! let reg = ShapeHintRegistry::with_wsm3d_defaults();
//! assert_eq!(reg.get("tree_oak"), ShapeHint::Cylinder);
//! assert_eq!(reg.get("wall_stone"), ShapeHint::LongX);
//! assert_eq!(reg.get("unknown_widget"), ShapeHint::Auto);
//! ```

use serde::{Deserialize, Serialize};

/// Voxelization shape hint. Controls which inflation strategy the voxelizer
/// picks for a given asset. Mirrors `WorldSphereMod.Voxel.ShapeHint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ShapeHint {
    /// Revolve around Y — good for trees, barrels, pillars.
    Cylinder,
    /// Perlin-noise depth variation — good for rocks, boulders.
    OrganicBlob,
    /// Extruded along the X axis — good for walls, roads, bridges.
    LongX,
    /// Extruded along the Z axis.
    LongZ,
    /// Tall narrow extrusion — good for towers, masts.
    Tall,
    /// Flat slab — good for humanoids, animals seen from the side.
    Flat,
    /// Left-right mirror before extrusion — good for vehicles, boats.
    Mirror,
    /// No hint; the voxelizer chooses a sensible default (balloon).
    #[default]
    Auto,
}

/// A single registry entry: a lowercase prefix and the hint it implies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeHintEntry {
    /// Lowercase prefix string.
    pub prefix: String,
    /// Hint associated with this prefix.
    pub hint: ShapeHint,
}

/// Registry that maps asset name prefixes to [`ShapeHint`] values.
///
/// Lookups are O(n) over the entry list. The list is typically short (< 100
/// entries) so this is faster than a `HashMap` for the expected use pattern of
/// one lookup per sprite voxelization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShapeHintRegistry {
    entries: Vec<ShapeHintEntry>,
}

impl ShapeHintRegistry {
    /// Create an empty registry. All lookups return [`ShapeHint::Auto`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry pre-populated with the 47 WSM3D default prefix→hint
    /// mappings from `AssetShapeRegistry._prefixHints`.
    pub fn with_wsm3d_defaults() -> Self {
        let mut reg = Self::new();
        let defaults: &[(&str, ShapeHint)] = &[
            ("human", ShapeHint::Flat),
            ("dwarf", ShapeHint::Flat),
            ("elf", ShapeHint::Flat),
            ("orc", ShapeHint::Flat),
            ("goblin", ShapeHint::Flat),
            ("tree", ShapeHint::Cylinder),
            ("bush", ShapeHint::Cylinder),
            ("flower", ShapeHint::Cylinder),
            ("barrel", ShapeHint::Cylinder),
            ("pot", ShapeHint::Cylinder),
            ("rock", ShapeHint::OrganicBlob),
            ("stone", ShapeHint::OrganicBlob),
            ("boulder", ShapeHint::OrganicBlob),
            ("mountain", ShapeHint::OrganicBlob),
            ("animal", ShapeHint::Flat),
            ("wolf", ShapeHint::Flat),
            ("bird", ShapeHint::Cylinder),
            ("eagle", ShapeHint::Cylinder),
            ("fish", ShapeHint::Cylinder),
            ("snake", ShapeHint::Cylinder),
            ("spider", ShapeHint::Cylinder),
            ("sheep", ShapeHint::Cylinder),
            ("horse", ShapeHint::Cylinder),
            ("cow", ShapeHint::Cylinder),
            ("rabbit", ShapeHint::Cylinder),
            ("crab", ShapeHint::Cylinder),
            ("zombie", ShapeHint::Cylinder),
            ("skeleton", ShapeHint::Cylinder),
            ("wall", ShapeHint::LongX),
            ("barracks", ShapeHint::LongX),
            ("bunker", ShapeHint::LongX),
            ("dock", ShapeHint::LongX),
            ("road", ShapeHint::Flat),
            ("bridge", ShapeHint::LongX),
            ("path", ShapeHint::Flat),
            ("tower", ShapeHint::Tall),
            ("lighthouse", ShapeHint::Tall),
            ("mast", ShapeHint::Tall),
            ("obelisk", ShapeHint::Tall),
            ("pillar", ShapeHint::Cylinder),
            ("boat", ShapeHint::Mirror),
            ("ship", ShapeHint::Mirror),
            ("wagon", ShapeHint::Mirror),
            ("cart", ShapeHint::Mirror),
            ("car", ShapeHint::Mirror),
            ("tank", ShapeHint::Mirror),
            ("vehicle", ShapeHint::Mirror),
        ];
        for (prefix, hint) in defaults {
            reg.register(*prefix, *hint);
        }
        reg
    }

    /// Append a new prefix→hint mapping. Later entries have lower priority than
    /// earlier ones (first-match wins).
    pub fn register(&mut self, prefix: impl Into<String>, hint: ShapeHint) {
        self.entries.push(ShapeHintEntry {
            prefix: prefix.into().to_lowercase(),
            hint,
        });
    }

    /// Look up the hint for `asset_id`. Returns [`ShapeHint::Auto`] if no entry
    /// matches.
    ///
    /// Matching rules (case-insensitive, mirrors WSM3D):
    /// - `asset_id` starts with the prefix, **or**
    /// - `asset_id` contains `_<prefix>`, **or**
    /// - `asset_id` contains `<prefix>_`.
    pub fn get(&self, asset_id: &str) -> ShapeHint {
        if asset_id.is_empty() {
            return ShapeHint::Auto;
        }
        let lower = asset_id.to_lowercase();
        for entry in &self.entries {
            let p = &entry.prefix;
            if lower.starts_with(p.as_str())
                || lower.contains(&format!("_{p}"))
                || lower.contains(&format!("{p}_"))
            {
                return entry.hint;
            }
        }
        ShapeHint::Auto
    }

    /// Remove all entries from the registry.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Number of registered entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` if no entries have been registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> ShapeHintRegistry {
        ShapeHintRegistry::with_wsm3d_defaults()
    }

    /// FR-PHENO-VOXEL-SHAPEHINT-001 — empty string returns Auto.
    #[test]
    fn empty_string_returns_auto() {
        assert_eq!(defaults().get(""), ShapeHint::Auto);
    }

    /// FR-PHENO-VOXEL-SHAPEHINT-002 — unknown prefix returns Auto.
    #[test]
    fn unknown_returns_auto() {
        assert_eq!(defaults().get("completely_unknown_asset"), ShapeHint::Auto);
    }

    /// FR-PHENO-VOXEL-SHAPEHINT-003 — prefix at start of name is matched.
    #[test]
    fn prefix_at_start_matched() {
        assert_eq!(defaults().get("tree_oak"), ShapeHint::Cylinder);
        assert_eq!(defaults().get("human_warrior"), ShapeHint::Flat);
        assert_eq!(defaults().get("tower_guard"), ShapeHint::Tall);
    }

    /// FR-PHENO-VOXEL-SHAPEHINT-004 — prefix in `_prefix` position is matched.
    #[test]
    fn prefix_after_underscore_matched() {
        assert_eq!(defaults().get("big_tree"), ShapeHint::Cylinder);
        assert_eq!(defaults().get("stone_wall"), ShapeHint::OrganicBlob); // "stone" fires first
        assert_eq!(defaults().get("old_tower"), ShapeHint::Tall);
    }

    /// FR-PHENO-VOXEL-SHAPEHINT-005 — prefix in `prefix_` position is matched.
    #[test]
    fn prefix_before_underscore_matched() {
        assert_eq!(defaults().get("wall_segment"), ShapeHint::LongX);
        assert_eq!(defaults().get("boat_small"), ShapeHint::Mirror);
    }

    /// FR-PHENO-VOXEL-SHAPEHINT-006 — case-insensitive matching.
    #[test]
    fn case_insensitive() {
        assert_eq!(defaults().get("TREE_OAK"), ShapeHint::Cylinder);
        assert_eq!(defaults().get("Human_Warrior"), ShapeHint::Flat);
    }

    /// FR-PHENO-VOXEL-SHAPEHINT-007 — custom registration overrides later defaults.
    #[test]
    fn custom_registration_first_wins() {
        let mut reg = ShapeHintRegistry::new();
        reg.register("tree", ShapeHint::Flat); // override the Cylinder default
        reg.register("tree", ShapeHint::Cylinder);
        // First match wins; custom Flat entry was inserted first.
        assert_eq!(reg.get("tree_oak"), ShapeHint::Flat);
    }

    /// FR-PHENO-VOXEL-SHAPEHINT-008 — with_wsm3d_defaults has the expected count.
    #[test]
    fn wsm3d_defaults_count() {
        assert_eq!(defaults().len(), 47);
    }

    /// FR-PHENO-VOXEL-SHAPEHINT-009 — clear empties the registry.
    #[test]
    fn clear_empties() {
        let mut reg = defaults();
        reg.clear();
        assert!(reg.is_empty());
        assert_eq!(reg.get("tree"), ShapeHint::Auto);
    }
}
