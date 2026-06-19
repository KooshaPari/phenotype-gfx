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
        Self(crate::voxel::DEFAULT_VOXEL_SCALE_MULTIPLIER)
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

    /// FR-PHENO-VOXEL-LOD-002 — distances at or below `near_voxel_edges` always
    /// resolve to LOD 0 (highest detail), regardless of scale.
    #[test]
    fn near_boundary_is_lod_zero() {
        let p = LodPolicy::default();
        let scale = VoxelScaleMultiplier::default(); // 8.0
                                                     // Exactly at the near threshold in voxel-edges: near_voxel_edges * scale
        let at_threshold = p.near_voxel_edges * scale.0;
        assert_eq!(select_lod(at_threshold, scale, p), LodLevel(0));
        // One unit inside the threshold.
        assert_eq!(select_lod(at_threshold - 1.0, scale, p), LodLevel(0));
        // Origin.
        assert_eq!(select_lod(0.0, scale, p), LodLevel(0));
    }

    /// FR-PHENO-VOXEL-LOD-003 — distances at or beyond `far_voxel_edges` always
    /// resolve to `max_level`.
    #[test]
    fn far_boundary_is_max_level() {
        let p = LodPolicy::default();
        let scale = VoxelScaleMultiplier::default();
        let at_far = p.far_voxel_edges * scale.0;
        assert_eq!(select_lod(at_far, scale, p), LodLevel(p.max_level));
        assert_eq!(select_lod(at_far + 1.0, scale, p), LodLevel(p.max_level));
    }

    /// FR-PHENO-VOXEL-LOD-004 — LOD level increases monotonically with distance
    /// in the linear interpolation region.
    #[test]
    fn lod_increases_with_distance() {
        let p = LodPolicy::default();
        let scale = VoxelScaleMultiplier::default();
        // Sample five points from just past near to just before far.
        let near = p.near_voxel_edges * scale.0;
        let far = p.far_voxel_edges * scale.0;
        let samples: Vec<LodLevel> = (1..=5)
            .map(|i| {
                let t = i as f32 / 6.0;
                select_lod(near + t * (far - near), scale, p)
            })
            .collect();
        for pair in samples.windows(2) {
            assert!(
                pair[0] <= pair[1],
                "LOD not monotone: {:?} > {:?}",
                pair[0],
                pair[1]
            );
        }
        // First sample must be strictly greater than LOD 0 (we're past the near edge).
        assert!(samples[0] > LodLevel(0));
        // Last sample must be near max_level (at the far edge boundary).
        assert!(samples.last().unwrap().0 >= p.max_level - 1);
    }

    /// FR-PHENO-VOXEL-LOD-005 — zero or negative `VoxelScaleMultiplier` returns
    /// `max_level` safely (guard against divide-by-zero from the WSM3D lesson).
    #[test]
    fn zero_scale_returns_max_level() {
        let p = LodPolicy::default();
        assert_eq!(
            select_lod(100.0, VoxelScaleMultiplier(0.0), p),
            LodLevel(p.max_level)
        );
        assert_eq!(
            select_lod(100.0, VoxelScaleMultiplier(-1.0), p),
            LodLevel(p.max_level)
        );
    }

    /// FR-PHENO-VOXEL-LOD-006 — custom policy with a single level (max_level=0)
    /// always returns LOD 0 regardless of distance.
    #[test]
    fn single_level_policy_always_zero() {
        let p = LodPolicy {
            near_voxel_edges: 10.0,
            far_voxel_edges: 100.0,
            max_level: 0,
        };
        let scale = VoxelScaleMultiplier(1.0);
        assert_eq!(select_lod(0.0, scale, p), LodLevel(0));
        assert_eq!(select_lod(50.0, scale, p), LodLevel(0));
        assert_eq!(select_lod(1_000.0, scale, p), LodLevel(0));
    }
}
