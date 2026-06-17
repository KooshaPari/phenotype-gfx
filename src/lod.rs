//! LOD system: chunk render planning, scale-budget primitives, LOD ring plan.
//!
//! Folded from `civis-platform-wt/crates/voxel/src/lod.rs` (chunk render
//! planning) and `scale_budget.rs` (MVP resident config, extent budget, LOD
//! ring plan, sim-LOD gestalt aggregator — FR-CIV-SCALE-001..004).
//!
//! All types here are pure Rust, engine-agnostic: no Bevy, no ECS, no GPU.
//! Engine adapters frustum-cull chunks then call into this module.

use core::cmp::Ordering;

use serde::{Deserialize, Serialize};

use phenotype_voxel::{select_lod, ChunkCoord, ChunkId, LodLevel, LodPolicy, VoxelScaleMultiplier};

use crate::streaming::{ring_distance, WindowPolicy};

// ============================================================================
// Chunk render planning (from lod.rs)
// ============================================================================

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

/// Select the mesh detail level for a chunk at the given camera distance.
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
/// Returns `None` when `in_frustum == false`. The caller frustum-culls
/// first; this function handles LOD selection only.
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

// ============================================================================
// FR-CIV-SCALE-001 — MVP resident working set (from scale_budget.rs)
// ============================================================================

/// MVP resident working set constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MvpResidentConfig {
    /// Base voxel edge length in metres.
    pub base_voxel_m: u32,
    /// CA chunk edge length in voxels.
    pub ca_chunk_voxels: u32,
    /// MVP world-edge length in chunks.
    pub mvp_chunks_per_side: u32,
}

impl MvpResidentConfig {
    /// FR-CIV-SCALE-001 stable identifier.
    pub const FR_ID: &'static str = "FR-CIV-SCALE-001";

    /// MVP defaults: 4 m/voxel, 256³ CA chunk, 1 chunk/side.
    pub const MVP: Self = Self {
        base_voxel_m: 4,
        ca_chunk_voxels: 256,
        mvp_chunks_per_side: 1,
    };

    /// World-edge length of the MVP in chunks.
    #[must_use]
    pub const fn mvp_world_side_chunks(&self) -> u32 {
        self.mvp_chunks_per_side
    }

    /// World-edge length of the MVP in metres.
    #[must_use]
    pub const fn mvp_world_side_m(&self) -> u32 {
        self.mvp_chunks_per_side
            .saturating_mul(self.ca_chunk_voxels)
            .saturating_mul(self.base_voxel_m)
    }

    /// Active streaming-window diameter in chunks.
    #[must_use]
    pub const fn mvp_active_window_chunks(&self, policy: WindowPolicy) -> u32 {
        let m1 = policy.mesh_ring as u32;
        let c1 = policy.coarse_ring as u32;
        let r = if m1 > c1 { m1 } else { c1 };
        r.saturating_mul(2).saturating_add(1)
    }

    /// Worst-case resident chunk count for the MVP at the given policy.
    #[must_use]
    pub const fn mvp_max_resident_chunks(&self, policy: WindowPolicy) -> u32 {
        let side = self.mvp_active_window_chunks(policy);
        match side.checked_mul(side) {
            Some(s2) => s2.saturating_mul(side),
            None => u32::MAX,
        }
    }
}

impl Default for MvpResidentConfig {
    fn default() -> Self {
        Self::MVP
    }
}

/// Stripped-down streaming config used by the budget validator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamConfigLite {
    /// Chunks-per-side of the streaming layer's active budget.
    pub active_window_side: u32,
    /// `WindowPolicy` the streaming layer is using.
    pub policy: WindowPolicy,
}

/// The MVP resident budget — validates a streaming config against MVP targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MvpResidentBudget {
    /// MVP world numbers.
    pub config: MvpResidentConfig,
    /// Maximum chunks the active budget permits in RAM.
    pub active_budget: u32,
}

impl MvpResidentBudget {
    /// FR-CIV-SCALE-001 stable identifier.
    pub const FR_ID: &'static str = "FR-CIV-SCALE-001";

    /// MVP defaults: active_budget = 256 chunks.
    pub const MVP: Self = Self {
        config: MvpResidentConfig::MVP,
        active_budget: 256,
    };

    /// Maximum chunks the MVP working set can hold at the given policy.
    #[must_use]
    pub fn mvp_max_chunks(&self, policy: WindowPolicy) -> u32 {
        self.config.mvp_max_resident_chunks(policy)
    }

    /// True if the config's active window hosts the MVP and fits within budget.
    #[must_use]
    pub fn fits(&self, cfg: StreamConfigLite) -> bool {
        let mvp_worst = self.mvp_max_chunks(cfg.policy);
        let sl_capacity = match cfg.active_window_side.checked_mul(cfg.active_window_side) {
            Some(s2) => match s2.checked_mul(cfg.active_window_side) {
                Some(s3) => s3,
                None => return false,
            },
            None => return false,
        };
        mvp_worst <= sl_capacity && sl_capacity <= self.active_budget
    }
}

impl Default for MvpResidentBudget {
    fn default() -> Self {
        Self::MVP
    }
}

// ============================================================================
// FR-CIV-SCALE-002 — No fixed world-size cap
// ============================================================================

/// World-extent budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtentBudget {
    /// Fixed-side world (legacy tier).
    Bounded {
        /// Side length of the world in chunks. Must be `> 0`.
        side_chunks: u32,
    },
    /// Unbounded world — the FR-CIV-SCALE-002 final target.
    Unbounded,
}

impl ExtentBudget {
    /// FR-CIV-SCALE-002 stable identifier.
    pub const FR_ID: &'static str = "FR-CIV-SCALE-002";

    /// Legacy WORLD_DIMS_SMALL (256 chunks/side).
    pub const SMALL: Self = Self::Bounded { side_chunks: 256 };
    /// Legacy WORLD_DIMS_MEDIUM.
    pub const MEDIUM: Self = Self::Bounded { side_chunks: 512 };
    /// Legacy WORLD_DIMS_LARGE.
    pub const LARGE: Self = Self::Bounded { side_chunks: 1024 };
    /// Legacy WORLD_DIMS_HUGE.
    pub const HUGE: Self = Self::Bounded { side_chunks: 2048 };
    /// FR-CIV-SCALE-002 final target.
    pub const FINAL: Self = Self::Unbounded;

    /// True if this budget is the unbounded variant.
    #[must_use]
    pub const fn is_unbounded(&self) -> bool {
        matches!(self, Self::Unbounded)
    }

    /// True if this budget is a bounded variant.
    #[must_use]
    pub const fn is_bounded(&self) -> bool {
        matches!(self, Self::Bounded { .. })
    }

    /// Validate a chunk coord against the budget.
    pub fn validate(&self, coord: ChunkCoord) -> Result<(), ExtentError> {
        match *self {
            Self::Unbounded => Ok(()),
            Self::Bounded { side_chunks } => {
                if side_chunks == 0 {
                    return Err(ExtentError::ZeroSide);
                }
                let half = (side_chunks / 2) as i64;
                let cx = coord.cx as i64;
                let cy = coord.cy as i64;
                let cz = coord.cz as i64;
                if (-half..half).contains(&cx)
                    && (-half..half).contains(&cy)
                    && (-half..half).contains(&cz)
                {
                    Ok(())
                } else {
                    Err(ExtentError::OutOfExtent { coord, side_chunks })
                }
            }
        }
    }
}

impl Default for ExtentBudget {
    fn default() -> Self {
        Self::FINAL
    }
}

/// Errors from [`ExtentBudget::validate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtentError {
    /// `side_chunks` was 0.
    ZeroSide,
    /// Coord was outside the bounded world's half-open extent.
    OutOfExtent {
        /// The coord that was rejected.
        coord: ChunkCoord,
        /// The side the budget was configured with.
        side_chunks: u32,
    },
}

impl core::fmt::Display for ExtentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ZeroSide => f.write_str("ExtentBudget::Bounded side_chunks must be > 0"),
            Self::OutOfExtent { coord, side_chunks } => write!(
                f,
                "coord ({}, {}, {}) is outside the bounded world (side_chunks = {})",
                coord.cx, coord.cy, coord.cz, side_chunks
            ),
        }
    }
}

impl std::error::Error for ExtentError {}

// ============================================================================
// FR-CIV-SCALE-003 — LOD ring plan + horizon-fade seam
// ============================================================================

/// A chunk's role in the LOD ring layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RingRole {
    /// Inside the inner mesh ring. Mesh at LOD 0, full alpha.
    Inner,
    /// Inside the horizon-fade seam. Blend weight `0 < w ≤ 255`.
    Seam {
        /// Blend weight (0..=255). Higher = more inner-LOD.
        weight: u8,
    },
    /// Past the seam, inside the render budget. Coarser LOD.
    Outer,
    /// Past the render budget. Not meshed.
    Frozen,
}

impl RingRole {
    /// True if this role is the inner mesh ring.
    #[must_use]
    pub const fn is_inner(self) -> bool { matches!(self, Self::Inner) }
    /// True if this role is in the horizon-fade seam band.
    #[must_use]
    pub const fn is_seam(self) -> bool { matches!(self, Self::Seam { .. }) }
    /// True if this role is the outer (coarse-LOD) ring.
    #[must_use]
    pub const fn is_outer(self) -> bool { matches!(self, Self::Outer) }
    /// True if this role is frozen (renderer does not see it).
    #[must_use]
    pub const fn is_frozen(self) -> bool { matches!(self, Self::Frozen) }
}

/// LOD ring plan — the renderer's view of the streaming window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LodRingPlan {
    /// The `WindowPolicy` this plan was derived from.
    pub policy: WindowPolicy,
    /// Outermost ring the renderer still draws. Must be `≥ policy.mesh_ring`.
    pub coarse_render_ring: u8,
}

/// Errors from [`LodRingPlan::checked`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlanError {
    /// `coarse_render_ring < mesh_ring`.
    CoarseBelowMesh {
        /// The coarse_render_ring that was rejected.
        coarse_render_ring: u8,
        /// The mesh_ring of the policy.
        mesh_ring: u8,
    },
}

impl core::fmt::Display for PlanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CoarseBelowMesh { coarse_render_ring, mesh_ring } => write!(
                f,
                "LodRingPlan coarse_render_ring ({coarse_render_ring}) must be ≥ policy.mesh_ring ({mesh_ring})"
            ),
        }
    }
}

impl std::error::Error for PlanError {}

impl LodRingPlan {
    /// FR-CIV-SCALE-003 stable identifier.
    pub const FR_ID: &'static str = "FR-CIV-SCALE-003";

    /// Default plan: `coarse_render_ring = mesh_ring + 1`.
    pub const fn default_for(policy: WindowPolicy) -> Self {
        let crr = policy.mesh_ring.saturating_add(1);
        Self { policy, coarse_render_ring: crr }
    }

    /// Construct with an explicit `coarse_render_ring`.
    pub fn checked(policy: WindowPolicy, coarse_render_ring: u8) -> Result<Self, PlanError> {
        if coarse_render_ring < policy.mesh_ring {
            return Err(PlanError::CoarseBelowMesh {
                coarse_render_ring,
                mesh_ring: policy.mesh_ring,
            });
        }
        Ok(Self { policy, coarse_render_ring })
    }

    /// Classify a chunk's render role given its coord and the camera anchor.
    #[must_use]
    pub fn role(&self, coord: ChunkCoord, anchor: ChunkCoord) -> RingRole {
        let ring = ring_distance(coord, anchor, self.policy.vy_weight);
        let mesh = self.policy.mesh_ring as u32;
        let seam = self.policy.seam_chunks as u32;
        let crr = self.coarse_render_ring as u32;
        if ring <= mesh {
            RingRole::Inner
        } else if ring > crr {
            RingRole::Frozen
        } else if ring <= mesh.saturating_add(seam) {
            let steps_out = ring - mesh;
            RingRole::Seam { weight: self.seam_blend(steps_out) }
        } else {
            RingRole::Outer
        }
    }

    /// Seam-blend weight (0..=255) for `steps_out` chunks past the inner ring.
    #[must_use]
    pub const fn seam_blend(&self, steps_out: u32) -> u8 {
        let seam = self.policy.seam_chunks as u32;
        if seam == 0 || steps_out == 0 { return 255; }
        if steps_out >= seam { return (255 / seam) as u8; }
        let numerator = 255u32.saturating_mul(seam - steps_out);
        (numerator / seam) as u8
    }

    /// Inner ring count (chunks in `Inner` role) per side.
    #[must_use]
    pub const fn inner_side_chunks(&self) -> u32 {
        (self.policy.mesh_ring as u32).saturating_mul(2).saturating_add(1)
    }

    /// Seam band count (chunks in `Seam` role) per side.
    #[must_use]
    pub const fn seam_side_chunks(&self) -> u32 {
        let outer = (self.policy.mesh_ring as u32)
            .saturating_add(self.policy.seam_chunks as u32)
            .saturating_mul(2)
            .saturating_add(1);
        outer.saturating_sub(self.inner_side_chunks())
    }
}

impl Default for LodRingPlan {
    fn default() -> Self {
        Self::default_for(WindowPolicy::default())
    }
}

// ============================================================================
// FR-CIV-SCALE-004 — Sim-LOD gestalt aggregator
// ============================================================================

/// Per-cohort totals fed to the gestalt aggregator.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CohortTotals {
    /// Mass in the cohort. Non-negative.
    pub mass: f32,
    /// Agent count. Non-negative integer in `f32`.
    pub agents: f32,
    /// Number of chunks that contributed this tick.
    pub chunks: u32,
}

impl CohortTotals {
    /// Empty cohort (no chunks contributed).
    pub const EMPTY: Self = Self { mass: 0.0, agents: 0.0, chunks: 0 };

    /// True if no chunks contributed.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.chunks == 0 }
}

impl Default for CohortTotals {
    fn default() -> Self { Self::EMPTY }
}

/// A gestalt summary — per-tick output of [`SimLodAggregator::fold`].
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Gestalt {
    /// Total mass across all contributing cohorts.
    pub total_mass: f32,
    /// Total agent count.
    pub total_agents: f32,
    /// Number of contributing chunks.
    pub total_chunks: u32,
    /// Number of cohorts that contributed.
    pub cohort_count: u32,
}

impl Gestalt {
    /// Empty gestalt (no cohorts).
    pub const EMPTY: Self = Self {
        total_mass: 0.0,
        total_agents: 0.0,
        total_chunks: 0,
        cohort_count: 0,
    };

    /// Mass per chunk. `0.0` if no chunks.
    #[must_use]
    pub fn mass_per_chunk(&self) -> f32 {
        if self.total_chunks == 0 { 0.0 } else { self.total_mass / (self.total_chunks as f32) }
    }

    /// Agents per chunk. `0.0` if no chunks.
    #[must_use]
    pub fn agents_per_chunk(&self) -> f32 {
        if self.total_chunks == 0 { 0.0 } else { self.total_agents / (self.total_chunks as f32) }
    }
}

/// Sim-LOD gestalt aggregator — folds per-cohort totals deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SimLodAggregator {
    /// Schema version; bumped when the summation algorithm changes.
    pub schema_version: u16,
}

impl SimLodAggregator {
    /// FR-CIV-SCALE-004 stable identifier.
    pub const FR_ID: &'static str = "FR-CIV-SCALE-004";
    /// Current schema version.
    pub const SCHEMA_VERSION: u16 = 1;
    /// Default aggregator.
    pub const DEFAULT: Self = Self { schema_version: Self::SCHEMA_VERSION };

    /// Fold an unsorted input slice (sorts by mass/agents/chunks before summing).
    pub fn fold(&self, inputs: &[CohortTotals]) -> Gestalt {
        let mut sorted: Vec<CohortTotals> = inputs.to_vec();
        sorted.sort_by(|a, b| {
            a.mass.partial_cmp(&b.mass).unwrap_or(Ordering::Equal)
                .then_with(|| a.agents.partial_cmp(&b.agents).unwrap_or(Ordering::Equal))
                .then_with(|| a.chunks.cmp(&b.chunks))
        });
        self.fold_sorted(&sorted)
    }

    /// Fold a pre-sorted slice in input order.
    pub fn fold_sorted(&self, inputs: &[CohortTotals]) -> Gestalt {
        let mut total_mass: f32 = 0.0;
        let mut total_agents: f32 = 0.0;
        let mut total_chunks: u32 = 0;
        for c in inputs {
            total_mass += c.mass;
            total_agents += c.agents;
            total_chunks = total_chunks.saturating_add(c.chunks);
        }
        Gestalt { total_mass, total_agents, total_chunks, cohort_count: inputs.len() as u32 }
    }

    /// Bound on mass divergence from inputs (`len * f32::EPSILON * max_mass`).
    #[must_use]
    pub fn mass_divergence_bound(inputs: &[CohortTotals]) -> Option<f32> {
        if inputs.is_empty() { return None; }
        let max_mass = inputs.iter().map(|c| c.mass.abs()).fold(0.0_f32, f32::max);
        Some(inputs.len() as f32 * f32::EPSILON * max_mass)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- chunk render planning ----

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
        assert!(plan_chunk_render(ChunkId(3), 32.0, false, VoxelScaleMultiplier::default(), policy).is_none());
        let plan = plan_chunk_render(ChunkId(7), 1.0e6, true, VoxelScaleMultiplier::default(), policy)
            .expect("visible chunk");
        assert_eq!(plan.chunk_id, ChunkId(7));
        assert_eq!(plan.lod, LodLevel(policy.max_level));
    }

    // ---- FR-CIV-SCALE-001 ----

    #[test]
    fn fr_civ_scale_001_mvp_world_edge_is_one_ca_chunk() {
        let cfg = MvpResidentConfig::MVP;
        assert_eq!(cfg.mvp_world_side_chunks(), 1);
        assert_eq!(cfg.mvp_world_side_m(), 1024);
    }

    #[test]
    fn fr_civ_scale_001_budget_fits_default_policy() {
        let budget = MvpResidentBudget::MVP;
        let cfg = StreamConfigLite { active_window_side: 5, policy: WindowPolicy::default() };
        assert!(budget.fits(cfg));
    }

    // ---- FR-CIV-SCALE-002 ----

    #[test]
    fn fr_civ_scale_002_default_extent_is_unbounded() {
        assert!(ExtentBudget::default().is_unbounded());
        assert!(ExtentBudget::SMALL.is_bounded());
    }

    fn coord(cx: i32, cy: i32, cz: i32) -> ChunkCoord { ChunkCoord { cx, cy, cz } }

    #[test]
    fn fr_civ_scale_002_bounded_rejects_out_of_extent() {
        let budget = ExtentBudget::SMALL;
        assert!(budget.validate(coord(0, 0, 0)).is_ok());
        assert!(matches!(budget.validate(coord(128, 0, 0)), Err(ExtentError::OutOfExtent { .. })));
    }

    // ---- FR-CIV-SCALE-003 ----

    #[test]
    fn fr_civ_scale_003_default_plan_role_layout() {
        let plan = LodRingPlan::default();
        let anchor = coord(0, 0, 0);
        assert_eq!(plan.role(coord(0, 0, 0), anchor), RingRole::Inner);
        assert_eq!(plan.role(coord(1, 0, 0), anchor), RingRole::Inner);
        assert!(plan.role(coord(2, 0, 0), anchor).is_seam());
        assert_eq!(plan.role(coord(3, 0, 0), anchor), RingRole::Frozen);
    }

    #[test]
    fn fr_civ_scale_003_checked_rejects_coarse_below_mesh() {
        let policy = WindowPolicy { mesh_ring: 2, ..WindowPolicy::default() };
        assert!(LodRingPlan::checked(policy, 1).is_err());
    }

    // ---- FR-CIV-SCALE-004 ----

    #[test]
    fn fr_civ_scale_004_empty_input_yields_empty_gestalt() {
        let g = SimLodAggregator::DEFAULT.fold(&[]);
        assert_eq!(g, Gestalt::EMPTY);
    }

    #[test]
    fn fr_civ_scale_004_fold_is_order_independent() {
        let agg = SimLodAggregator::DEFAULT;
        let a = [
            CohortTotals { mass: 1.0, agents: 2.0, chunks: 4 },
            CohortTotals { mass: 2.0, agents: 1.0, chunks: 3 },
            CohortTotals { mass: 3.0, agents: 3.0, chunks: 2 },
        ];
        let b = [a[2], a[0], a[1]];
        assert_eq!(agg.fold(&a), agg.fold(&b));
    }
}
