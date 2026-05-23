//! Level-of-detail policy.
//!
//! Captures the WSM3D lesson that LOD thresholds must compose with
//! `VoxelScaleMultiplier` or actors collapse into the impostor tier prematurely.

use serde::{Deserialize, Serialize};

/// Discrete LOD level. 0 = highest detail (per-voxel), higher = coarser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LodLevel(pub u8);

/// `VoxelScaleMultiplier` newtype so consumers cannot accidentally combine it with
/// raw scalars from elsewhere. WSM3D-lineage invariant: default of 8.0.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct VoxelScaleMultiplier(pub f32);

impl Default for VoxelScaleMultiplier {
    fn default() -> Self {
        Self(crate::DEFAULT_VOXEL_SCALE_MULTIPLIER)
    }
}

/// Policy parameters for [`select_lod`]. Distances are in voxel-edge-multiples
/// (so the policy is scale-invariant — composes correctly with
/// [`VoxelScaleMultiplier`]).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LodPolicy {
    /// Distance (in voxel-edges) below which we render LOD 0.
    pub near_voxel_edges: f32,
    /// Distance (in voxel-edges) above which we render LOD `max_level`.
    pub far_voxel_edges: f32,
    /// Highest LOD index permitted.
    pub max_level: u8,
}

impl Default for LodPolicy {
    fn default() -> Self {
        Self {
            near_voxel_edges: 64.0,
            far_voxel_edges: 512.0,
            max_level: 4,
        }
    }
}

/// Compute the LOD level for a viewer at the given world-space distance, in metres,
/// given the world's [`VoxelScaleMultiplier`] and the [`LodPolicy`].
#[must_use]
pub fn select_lod(
    distance_metres: f32,
    scale: VoxelScaleMultiplier,
    policy: LodPolicy,
) -> LodLevel {
    if scale.0 <= 0.0 {
        return LodLevel(policy.max_level);
    }
    let in_edges = distance_metres / scale.0;
    if in_edges <= policy.near_voxel_edges {
        LodLevel(0)
    } else if in_edges >= policy.far_voxel_edges {
        LodLevel(policy.max_level)
    } else {
        // Linear interpolation between near and far.
        let t = (in_edges - policy.near_voxel_edges)
            / (policy.far_voxel_edges - policy.near_voxel_edges);
        let level = (t * f32::from(policy.max_level)).round() as u8;
        LodLevel(level.min(policy.max_level))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-PHENO-VOXEL-LOD-000 — LOD selection is scale-invariant: doubling the
    /// `VoxelScaleMultiplier` halves the effective distance, so LOD tier should stay
    /// the same.
    #[test]
    fn lod_is_scale_invariant() {
        let p = LodPolicy::default();
        let lod_a = select_lod(64.0 * 8.0, VoxelScaleMultiplier(8.0), p);
        let lod_b = select_lod(64.0 * 16.0, VoxelScaleMultiplier(16.0), p);
        assert_eq!(lod_a, lod_b);
    }

    /// FR-PHENO-VOXEL-LOD-001 — far distances clamp to `max_level`.
    #[test]
    fn far_distances_clamp_to_max() {
        let p = LodPolicy::default();
        let lod = select_lod(1.0e6, VoxelScaleMultiplier::default(), p);
        assert_eq!(lod, LodLevel(p.max_level));
    }
}
