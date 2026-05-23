//! Coordinate types.
//!
//! World coordinates are fixed-point `i64` at [`FIXED_SCALE`] so all positional math
//! is deterministic across machines and replays. Consumers that need real-world
//! units divide by [`FIXED_SCALE`] only at the rendering boundary.

use serde::{Deserialize, Serialize};

/// Fixed-point scale: `10^6`. World coords are `i64` numerators over this denominator.
pub const FIXED_SCALE: i64 = 1_000_000;

/// World-space position in fixed-point `i64` units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorldCoord {
    /// Fixed-point X.
    pub x: i64,
    /// Fixed-point Y (vertical, world-up).
    pub y: i64,
    /// Fixed-point Z.
    pub z: i64,
}

/// Integer chunk-grid coordinates. Each unit corresponds to one dense 16³ leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    /// Chunk-grid X.
    pub cx: i32,
    /// Chunk-grid Y.
    pub cy: i32,
    /// Chunk-grid Z.
    pub cz: i32,
}

/// Convert a fixed-point world position to chunk-grid coordinates given the per-axis
/// chunk edge in voxels and the per-voxel fixed-point span.
#[must_use]
pub fn to_chunk_coord(world: WorldCoord, voxel_span: i64, edge: i32) -> ChunkCoord {
    let edge64 = i64::from(edge);
    ChunkCoord {
        cx: (world.x.div_euclid(voxel_span * edge64)) as i32,
        cy: (world.y.div_euclid(voxel_span * edge64)) as i32,
        cz: (world.z.div_euclid(voxel_span * edge64)) as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-PHENO-VOXEL-COORD-000 — origin maps to origin chunk.
    #[test]
    fn origin_maps_to_origin_chunk() {
        let w = WorldCoord { x: 0, y: 0, z: 0 };
        let c = to_chunk_coord(w, 1_000_000, 16);
        assert_eq!(
            c,
            ChunkCoord {
                cx: 0,
                cy: 0,
                cz: 0
            }
        );
    }

    /// FR-PHENO-VOXEL-COORD-001 — negative coords use Euclidean division so the
    /// chunk lattice has no gaps at the origin (regression target for the classic
    /// `-1 / 16 == 0` integer-truncation bug).
    #[test]
    fn negative_coords_use_euclidean_division() {
        let span: i64 = 1_000_000;
        let edge: i32 = 16;
        let w = WorldCoord {
            x: -span,
            y: 0,
            z: 0,
        };
        let c = to_chunk_coord(w, span, edge);
        assert_eq!(c.cx, -1);
    }
}
