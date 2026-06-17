//! single-core home; real algorithms fold in here (see ADR-0001).
//!
//! LOD tier selection and chunk render planning.
//!
//! Folded from `civis-platform-wt/crates/voxel/src/lod.rs`.
//! Depends only on `phenotype_voxel` kernel types — no Bevy, no ECS.
//! Engine adapters frustum-cull chunks and call into this module for
//! the mesh detail level and render plan.
//!
//! Note: `scale_budget` (MvpResidentBudget, LodRingPlan, SimLodAggregator etc.)
//! is deferred to PR3 because it depends on `window::WindowPolicy` which
//! is also a fold candidate from `civis-platform-wt/crates/voxel/src/window/`.

use phenotype_voxel::{select_lod, ChunkId, LodLevel, LodPolicy, VoxelScaleMultiplier};

/// Render planning output for a visible chunk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChunkRenderPlan {
    /// Chunk identifier.
    pub chunk_id: ChunkId,
    /// Selected mesh detail level.
    pub lod: LodLevel,
    /// Distance in world metres from the camera to the chunk center.
    pub distance_metres: f32,
}

/// Select the mesh detail level for a chunk at the given distance.
#[must_use]
pub fn select_mesh_detail_level(
    distance_metres: f32,
    scale: VoxelScaleMultiplier,
    policy: LodPolicy,
) -> LodLevel {
    select_lod(distance_metres, scale, policy)
}

/// Build a render plan for a frustum-culled chunk.
///
/// Returns `None` if the chunk is outside the frustum (`in_frustum == false`).
/// The caller frustum-culls first; this function handles LOD selection only.
#[must_use]
pub fn plan_chunk_render(
    chunk_id: ChunkId,
    distance_metres: f32,
    in_frustum: bool,
    scale: VoxelScaleMultiplier,
    policy: LodPolicy,
) -> Option<ChunkRenderPlan> {
    if !in_frustum {
        return None;
    }
    Some(ChunkRenderPlan {
        chunk_id,
        lod: select_mesh_detail_level(distance_metres, scale, policy),
        distance_metres,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_selection_tracks_scale_invariance() {
        let policy = LodPolicy::default();
        let lod_a = select_mesh_detail_level(64.0 * 8.0, VoxelScaleMultiplier(8.0), policy);
        let lod_b = select_mesh_detail_level(64.0 * 16.0, VoxelScaleMultiplier(16.0), policy);
        assert_eq!(lod_a, lod_b);
    }

    #[test]
    fn plan_is_culled_before_lod_selection() {
        let policy = LodPolicy::default();
        assert!(plan_chunk_render(
            ChunkId(3),
            32.0,
            false,
            VoxelScaleMultiplier::default(),
            policy
        )
        .is_none());
        let plan = plan_chunk_render(
            ChunkId(7),
            1.0e6,
            true,
            VoxelScaleMultiplier::default(),
            policy,
        )
        .expect("visible chunk");
        assert_eq!(plan.chunk_id, ChunkId(7));
        assert_eq!(plan.lod, LodLevel(policy.max_level));
    }
}
