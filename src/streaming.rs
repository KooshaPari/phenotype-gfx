//! Streaming window policy: ring-based chunk lifecycle, eviction ordering.
//!
//! Folded from `civis-platform-wt/crates/voxel/src/window/mod.rs`.
//! Pure Rust — no IO, no engine types, no GPU. Every function is a pure
//! function of `(coord, anchor, policy)` for deterministic replay.
//!
//! Provides:
//! - [`ring_distance`] — Chebyshev distance with vertical weight.
//! - [`WindowPolicy`] — named, serialisable ring-radius config.
//! - [`ChunkState`] — lifecycle state machine for a chunk.
//! - [`SimCohort`] — sim tick cohort derived from ring distance.
//! - [`EvictionKey`] — comparator for eviction ordering under budget pressure.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use crate::voxel::ChunkCoord;

// ============================================================================
// ring_distance — the core metric
// ============================================================================

/// Chebyshev distance with a vertical weight.
///
/// `ring_distance(coord, anchor, vy_weight) = max(|Δx|, |Δy| * vy_weight, |Δz|)`.
///
/// Worlds are mostly flat heightfields; a vertical step costs more than a
/// horizontal step. With `vy_weight = 1` the metric is a pure Chebyshev cube;
/// with `vy_weight = 2` a 1-chunk vertical step is equivalent to 2 horizontal.
///
/// `vy_weight = 0` is treated as 1 (defensive; `WindowPolicy::checked` rejects it).
#[must_use]
pub const fn ring_distance(coord: ChunkCoord, anchor: ChunkCoord, vy_weight: u8) -> u32 {
    let w = if vy_weight == 0 {
        1u32
    } else {
        vy_weight as u32
    };
    let dx = (coord.cx - anchor.cx).unsigned_abs();
    let dz = (coord.cz - anchor.cz).unsigned_abs();
    let dy = (coord.cy - anchor.cy).unsigned_abs() * w;
    // Manual max-of-three to stay const fn (u32::max not const-stable yet).
    let m = if dx > dz { dx } else { dz };
    if m > dy {
        m
    } else {
        dy
    }
}

// ============================================================================
// ChunkState — lifecycle state machine
// ============================================================================

/// Lifecycle state for a chunk in the streaming window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChunkState {
    /// Not in the resident set. Will regen from seed if requested.
    Unloaded,
    /// In the resident set, not yet meshed (or mesh already despawned).
    Resident,
    /// In the resident set, mesh is alive (engine entity spawned).
    Meshed,
    /// Mesh is alive but alpha is being lowered for a ring shrink.
    Fading {
        /// Ticks remaining in the fade ramp (1..=`fade_ticks`).
        ticks_remaining: u8,
    },
    /// Marked for eviction this tick; mesh despawn scheduled.
    Evicting,
    /// Removed from resident set; persisted to disk if dirty. Terminal
    /// (coord re-enters the cycle via `Resident` after regen).
    Evicted,
}

impl ChunkState {
    /// True if the chunk holds a live mesh in the renderer.
    #[must_use]
    pub const fn has_mesh(self) -> bool {
        matches!(self, Self::Meshed | Self::Fading { .. })
    }

    /// True if the chunk occupies RAM (counted against the active budget).
    #[must_use]
    pub const fn is_resident(self) -> bool {
        matches!(
            self,
            Self::Resident | Self::Meshed | Self::Fading { .. } | Self::Evicting
        )
    }
}

// ============================================================================
// SimCohort — sim tick cohort
// ============================================================================

/// Sim-LOD cohort, derived from ring distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SimCohort {
    /// Every tick, per-voxel CA, full agent tick.
    FullSim,
    /// Every `step_multiplier`-th tick, statistical gestalt only.
    CoarseSim {
        /// Tick-rate divisor vs. full sim.
        step_multiplier: u8,
    },
    /// No sim tick; mass conserved trivially.
    Frozen,
}

// ============================================================================
// WindowPolicy — the named ring-radius config
// ============================================================================

/// Streaming-window policy.
///
/// All fields are `u8`/`i8` so the struct is `Copy`, serialisable, and
/// round-trips bit-identically through bincode for replay/manifest persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WindowPolicy {
    /// Innermost ring fully meshed at LOD 0.
    pub mesh_ring: u8,
    /// Innermost ring running full-sim cadence.
    pub sim_ring: u8,
    /// Outermost ring on coarse-sim. Must be `≥ sim_ring`.
    pub coarse_ring: u8,
    /// Width of the horizon-fade seam between adjacent rings, in chunks.
    pub seam_chunks: u8,
    /// Vertical weight for the ring-distance metric (default 2 for heightfields).
    pub vy_weight: u8,
    /// Coarse-sim tick divisor (e.g. 2 = every other tick).
    pub sim_lod_step: u8,
    /// How many rings past `mesh_ring` the prefetch cone reaches (0 = disabled).
    pub prefetch_ring: u8,
    /// Forward-cone half-angle for prefetch, Q0.7 signed (0 = hemisphere).
    pub forward_cone_cos_theta: i8,
    /// Fade ramp length in ticks (0 = instant despawn on ring exit).
    pub fade_ticks: u8,
}

impl Default for WindowPolicy {
    fn default() -> Self {
        Self {
            mesh_ring: 1,
            sim_ring: 1,
            coarse_ring: 2,
            seam_chunks: 1,
            vy_weight: 2,
            sim_lod_step: 2,
            prefetch_ring: 0,
            forward_cone_cos_theta: 0,
            fade_ticks: 0,
        }
    }
}

/// Errors from [`WindowPolicy::checked`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    /// `vy_weight` was 0.
    ZeroVyWeight,
    /// `sim_lod_step` was 0.
    ZeroSimLodStep,
    /// `sim_ring > coarse_ring`.
    SimRingAboveCoarseRing,
    /// `forward_cone_cos_theta` outside Q0.7 signed range.
    ForwardConeOutOfRange,
}

impl core::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::ZeroVyWeight => "vy_weight must be ≥ 1",
            Self::ZeroSimLodStep => "sim_lod_step must be ≥ 1",
            Self::SimRingAboveCoarseRing => "sim_ring must be ≤ coarse_ring",
            Self::ForwardConeOutOfRange => "forward_cone_cos_theta must be in -128..=127",
        })
    }
}

impl std::error::Error for PolicyError {}

impl WindowPolicy {
    /// Construct with explicit invariants validated.
    #[allow(clippy::too_many_arguments)]
    pub fn checked(
        mesh_ring: u8,
        sim_ring: u8,
        coarse_ring: u8,
        seam_chunks: u8,
        vy_weight: u8,
        sim_lod_step: u8,
        prefetch_ring: u8,
        forward_cone_cos_theta: i8,
        fade_ticks: u8,
    ) -> Result<Self, PolicyError> {
        if vy_weight == 0 {
            return Err(PolicyError::ZeroVyWeight);
        }
        if sim_lod_step == 0 {
            return Err(PolicyError::ZeroSimLodStep);
        }
        if sim_ring > coarse_ring {
            return Err(PolicyError::SimRingAboveCoarseRing);
        }
        Ok(Self {
            mesh_ring,
            sim_ring,
            coarse_ring,
            seam_chunks,
            vy_weight,
            sim_lod_step,
            prefetch_ring,
            forward_cone_cos_theta,
            fade_ticks,
        })
    }

    /// Classify a chunk's lifecycle state (pure function of coord, anchor, policy).
    #[must_use]
    pub const fn classify(&self, coord: ChunkCoord, anchor: ChunkCoord) -> ChunkState {
        let ring = ring_distance(coord, anchor, self.vy_weight);
        if ring <= self.mesh_ring as u32 {
            ChunkState::Meshed
        } else if ring <= (self.mesh_ring as u32).saturating_add(self.seam_chunks as u32) {
            if self.fade_ticks == 0 {
                ChunkState::Resident
            } else {
                ChunkState::Fading {
                    ticks_remaining: self.fade_ticks,
                }
            }
        } else {
            ChunkState::Unloaded
        }
    }

    /// Derive the sim cohort from ring distance.
    #[must_use]
    pub const fn sim_cohort(&self, coord: ChunkCoord, anchor: ChunkCoord) -> SimCohort {
        let ring = ring_distance(coord, anchor, self.vy_weight);
        if ring <= self.sim_ring as u32 {
            SimCohort::FullSim
        } else if ring <= self.coarse_ring as u32 {
            SimCohort::CoarseSim {
                step_multiplier: self.sim_lod_step,
            }
        } else {
            SimCohort::Frozen
        }
    }

    /// True if `coord` is in the prefetch cone.
    #[must_use]
    pub const fn in_prefetch_cone(
        &self,
        coord: ChunkCoord,
        anchor: ChunkCoord,
        forward_q7: [i32; 3],
    ) -> bool {
        if self.prefetch_ring == 0 {
            return false;
        }
        let ring = ring_distance(coord, anchor, self.vy_weight);
        if ring <= self.mesh_ring as u32 {
            return true;
        }
        if ring > (self.mesh_ring as u32).saturating_add(self.prefetch_ring as u32) {
            return false;
        }
        let dx = coord.cx - anchor.cx;
        let dy = (coord.cy - anchor.cy) * (self.vy_weight as i32);
        let dz = coord.cz - anchor.cz;
        let dot_q14 = forward_q7[0] * dx + forward_q7[1] * dy + forward_q7[2] * dz;
        let l1 = dx.abs() + dy.abs() + dz.abs();
        let cos_q7 = self.forward_cone_cos_theta as i32;
        if cos_q7 > 0 {
            dot_q14 > cos_q7.saturating_mul(l1).saturating_mul(128)
        } else {
            dot_q14 > 0
        }
    }
}

// ============================================================================
// EvictionKey — comparator for eviction ordering
// ============================================================================

/// Eviction comparator. Smaller key = evicted first.
///
/// Primary key: ring distance (larger ring evicts first — far chunks go
/// before near ones). Tie-breaker: LRU position (smaller lru_pos = colder).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EvictionKey {
    /// Ring distance from the current anchor.
    pub ring: u32,
    /// LRU position within the ring (smaller = colder).
    pub lru_pos: u32,
}

impl EvictionKey {
    /// Build an eviction key for a chunk.
    #[must_use]
    pub const fn new(coord: ChunkCoord, anchor: ChunkCoord, vy_weight: u8, lru_pos: u32) -> Self {
        Self {
            ring: ring_distance(coord, anchor, vy_weight),
            lru_pos,
        }
    }
}

impl Ord for EvictionKey {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // Larger ring = evict first → invert ring comparison.
        other
            .ring
            .cmp(&self.ring)
            .then(self.lru_pos.cmp(&other.lru_pos))
    }
}

impl PartialOrd for EvictionKey {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coord(cx: i32, cy: i32, cz: i32) -> ChunkCoord {
        ChunkCoord { cx, cy, cz }
    }

    fn default_anchor() -> ChunkCoord {
        coord(0, 0, 0)
    }

    // ========================================================================
    // ring_distance tests
    // ========================================================================

    #[test]
    fn ring_distance_same_coord_is_zero() {
        let c = coord(5, 3, 7);
        assert_eq!(ring_distance(c, c, 2), 0);
    }

    #[test]
    fn ring_distance_horizontal_is_chebyshev() {
        let anchor = default_anchor();
        // dx=3, dy=0, dz=1 → max(3, 0, 1) = 3
        assert_eq!(ring_distance(coord(3, 0, 1), anchor, 2), 3);
    }

    #[test]
    fn ring_distance_only_z_axis() {
        let anchor = default_anchor();
        assert_eq!(ring_distance(coord(0, 0, 4), anchor, 2), 4);
    }

    #[test]
    fn ring_distance_only_y_axis() {
        let anchor = default_anchor();
        // dy=3, vy_weight=2 → effective=6
        assert_eq!(ring_distance(coord(0, 3, 0), anchor, 2), 6);
    }

    #[test]
    fn ring_distance_only_x_axis() {
        let anchor = default_anchor();
        assert_eq!(ring_distance(coord(7, 0, 0), anchor, 1), 7);
    }

    #[test]
    fn ring_distance_vertical_uses_vy_weight() {
        let anchor = default_anchor();
        // dy=1, vy_weight=2 → effective dy=2 > dx=1 → distance=2
        assert_eq!(ring_distance(coord(1, 1, 0), anchor, 2), 2);
    }

    #[test]
    fn ring_distance_vy_weight_zero_fallback_to_one() {
        let anchor = default_anchor();
        // vy_weight=0 treated as 1; dy=1 → effective=1
        assert_eq!(ring_distance(coord(0, 1, 0), anchor, 0), 1);
    }

    #[test]
    fn ring_distance_vy_weight_zero_with_horizontal() {
        let anchor = default_anchor();
        // dx=5, vy_weight=0 (treated as 1), dy=1 → effective dy=1, max=5
        assert_eq!(ring_distance(coord(5, 1, 0), anchor, 0), 5);
    }

    #[test]
    fn ring_distance_vy_weight_one_pure_chebyshev() {
        let anchor = default_anchor();
        // vy_weight=1, dx=3, dy=5, dz=2 → max(3, 5, 2) = 5
        assert_eq!(ring_distance(coord(3, 5, 2), anchor, 1), 5);
    }

    #[test]
    fn ring_distance_negative_coords() {
        let anchor = coord(-5, -3, -10);
        // target at (-2, 0, -6) → dx=3, dy=3, dz=4, vy_weight=1 → max=4
        assert_eq!(ring_distance(coord(-2, 0, -6), anchor, 1), 4);
    }

    #[test]
    fn ring_distance_all_axes_contribute() {
        let anchor = default_anchor();
        // dx=3, dy=2, dz=4, vy_weight=1 → max(3, 2, 4) = 4
        assert_eq!(ring_distance(coord(3, 2, 4), anchor, 1), 4);
    }

    #[test]
    fn ring_distance_symmetry() {
        let a = coord(1, 2, 3);
        let b = coord(4, 6, 8);
        assert_eq!(ring_distance(a, b, 2), ring_distance(b, a, 2));
    }

    // ========================================================================
    // ChunkState — has_mesh / is_resident
    // ========================================================================

    #[test]
    fn chunk_state_meshed_has_mesh_and_is_resident() {
        let s = ChunkState::Meshed;
        assert!(s.has_mesh());
        assert!(s.is_resident());
    }

    #[test]
    fn chunk_state_fading_has_mesh_and_is_resident() {
        let s = ChunkState::Fading { ticks_remaining: 3 };
        assert!(s.has_mesh());
        assert!(s.is_resident());
    }

    #[test]
    fn chunk_state_resident_no_mesh_but_is_resident() {
        let s = ChunkState::Resident;
        assert!(!s.has_mesh());
        assert!(s.is_resident());
    }

    #[test]
    fn chunk_state_evicting_no_mesh_but_is_resident() {
        let s = ChunkState::Evicting;
        assert!(!s.has_mesh());
        assert!(s.is_resident());
    }

    #[test]
    fn chunk_state_unloaded_no_mesh_not_resident() {
        let s = ChunkState::Unloaded;
        assert!(!s.has_mesh());
        assert!(!s.is_resident());
    }

    #[test]
    fn chunk_state_evicted_no_mesh_not_resident() {
        let s = ChunkState::Evicted;
        assert!(!s.has_mesh());
        assert!(!s.is_resident());
    }

    #[test]
    fn chunk_state_fading_ticks_remaining_1() {
        let s = ChunkState::Fading { ticks_remaining: 1 };
        assert!(s.has_mesh());
        assert!(s.is_resident());
    }

    #[test]
    fn chunk_state_fading_ticks_remaining_255() {
        let s = ChunkState::Fading { ticks_remaining: 255 };
        assert!(s.has_mesh());
        assert!(s.is_resident());
    }

    // ========================================================================
    // WindowPolicy — classify
    // ========================================================================

    #[test]
    fn classify_inner_ring_is_meshed() {
        let policy = WindowPolicy::default(); // mesh_ring=1, seam_chunks=1
        let anchor = default_anchor();
        // ring=0 (same chunk) → meshed
        assert_eq!(policy.classify(coord(0, 0, 0), anchor), ChunkState::Meshed);
        // ring=1 (mesh_ring) → meshed
        assert_eq!(policy.classify(coord(1, 0, 0), anchor), ChunkState::Meshed);
    }

    #[test]
    fn classify_seam_ring_no_fade_is_resident() {
        let policy = WindowPolicy::default(); // fade_ticks=0, mesh_ring=1, seam_chunks=1
        let anchor = default_anchor();
        // ring=2 → mesh_ring(1)+seam_chunks(1)=2 → seam zone → Resident (fade_ticks=0)
        assert_eq!(policy.classify(coord(2, 0, 0), anchor), ChunkState::Resident);
    }

    #[test]
    fn classify_beyond_seam_is_unloaded() {
        let policy = WindowPolicy::default(); // mesh_ring=1, seam_chunks=1
        let anchor = default_anchor();
        // ring=5 → well beyond mesh_ring+seam_chunks=2 → Unloaded
        assert_eq!(policy.classify(coord(5, 0, 0), anchor), ChunkState::Unloaded);
    }

    #[test]
    fn classify_with_fade_ticks_returns_fading() {
        let policy = WindowPolicy {
            mesh_ring: 1,
            seam_chunks: 1,
            fade_ticks: 10,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        // ring=2 → seam zone, fade_ticks>0 → Fading
        match policy.classify(coord(2, 0, 0), anchor) {
            ChunkState::Fading { ticks_remaining } => assert_eq!(ticks_remaining, 10),
            other => panic!("expected Fading, got {other:?}"),
        }
    }

    #[test]
    fn classify_boundary_exactly_at_mesh_ring() {
        let policy = WindowPolicy {
            mesh_ring: 3,
            seam_chunks: 1,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        // ring=3 → meshed (≤ mesh_ring)
        assert_eq!(policy.classify(coord(3, 0, 0), anchor), ChunkState::Meshed);
    }

    #[test]
    fn classify_boundary_exactly_at_seam_edge() {
        let policy = WindowPolicy {
            mesh_ring: 2,
            seam_chunks: 2,
            fade_ticks: 0,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        // ring=4 → mesh_ring(2)+seam_chunks(2)=4 → seam edge → Resident
        assert_eq!(policy.classify(coord(4, 0, 0), anchor), ChunkState::Resident);
        // ring=5 → beyond seam → Unloaded
        assert_eq!(policy.classify(coord(5, 0, 0), anchor), ChunkState::Unloaded);
    }

    #[test]
    fn classify_zero_seam_chunks_skips_seam() {
        let policy = WindowPolicy {
            mesh_ring: 2,
            seam_chunks: 0,
            fade_ticks: 0,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        // ring=2 → meshed (≤ mesh_ring)
        assert_eq!(policy.classify(coord(2, 0, 0), anchor), ChunkState::Meshed);
        // ring=3 → beyond mesh_ring+0 → Unloaded (no seam)
        assert_eq!(policy.classify(coord(3, 0, 0), anchor), ChunkState::Unloaded);
    }

    #[test]
    fn classify_with_vertical_offset() {
        let policy = WindowPolicy {
            mesh_ring: 4,
            vy_weight: 2,
            seam_chunks: 1,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        // dy=1, vy_weight=2 → ring=2 ≤ mesh_ring(4) → Meshed
        assert_eq!(policy.classify(coord(0, 1, 0), anchor), ChunkState::Meshed);
        // dy=3, vy_weight=2 → ring=6 > mesh_ring(4)+seam(1)=5 → Unloaded
        assert_eq!(policy.classify(coord(0, 3, 0), anchor), ChunkState::Unloaded);
    }

    // ========================================================================
    // WindowPolicy — sim_cohort
    // ========================================================================

    #[test]
    fn sim_cohort_inner_is_full_sim() {
        let policy = WindowPolicy::default(); // sim_ring=1
        let anchor = default_anchor();
        assert_eq!(
            policy.sim_cohort(coord(1, 0, 0), anchor),
            SimCohort::FullSim
        );
    }

    #[test]
    fn sim_cohort_same_chunk_is_full_sim() {
        let policy = WindowPolicy::default();
        let anchor = default_anchor();
        assert_eq!(policy.sim_cohort(coord(0, 0, 0), anchor), SimCohort::FullSim);
    }

    #[test]
    fn sim_cohort_coarse_band() {
        let policy = WindowPolicy::default(); // sim_ring=1, coarse_ring=2
        let anchor = default_anchor();
        // ring=2 → >sim_ring(1), ≤coarse_ring(2) → CoarseSim
        match policy.sim_cohort(coord(2, 0, 0), anchor) {
            SimCohort::CoarseSim { step_multiplier } => assert_eq!(step_multiplier, 2),
            other => panic!("expected CoarseSim, got {other:?}"),
        }
    }

    #[test]
    fn sim_cohort_frozen_outside_coarse_ring() {
        let policy = WindowPolicy::default(); // coarse_ring=2
        let anchor = default_anchor();
        assert_eq!(
            policy.sim_cohort(coord(3, 0, 0), anchor),
            SimCohort::Frozen
        );
    }

    #[test]
    fn sim_cohort_boundary_exactly_at_sim_ring() {
        let policy = WindowPolicy {
            sim_ring: 5,
            coarse_ring: 8,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        // ring=5 → FullSim
        assert_eq!(
            policy.sim_cohort(coord(5, 0, 0), anchor),
            SimCohort::FullSim
        );
    }

    #[test]
    fn sim_cohort_boundary_exactly_at_coarse_ring() {
        let policy = WindowPolicy {
            sim_ring: 3,
            coarse_ring: 6,
            sim_lod_step: 4,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        // ring=6 → CoarseSim
        match policy.sim_cohort(coord(6, 0, 0), anchor) {
            SimCohort::CoarseSim { step_multiplier } => assert_eq!(step_multiplier, 4),
            other => panic!("expected CoarseSim at coarse_ring boundary, got {other:?}"),
        }
    }

    #[test]
    fn sim_cohort_custom_step_multiplier() {
        let policy = WindowPolicy {
            sim_ring: 1,
            coarse_ring: 3,
            sim_lod_step: 8,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        match policy.sim_cohort(coord(2, 0, 0), anchor) {
            SimCohort::CoarseSim { step_multiplier } => assert_eq!(step_multiplier, 8),
            other => panic!("expected CoarseSim with step=8, got {other:?}"),
        }
    }

    // ========================================================================
    // WindowPolicy — in_prefetch_cone
    // ========================================================================

    #[test]
    fn prefetch_cone_disabled_returns_false() {
        let policy = WindowPolicy {
            prefetch_ring: 0,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        // Even the anchor itself returns false when prefetch is disabled
        assert!(!policy.in_prefetch_cone(coord(0, 0, 0), anchor, [128, 0, 0]));
    }

    #[test]
    fn prefetch_cone_inside_mesh_ring_always_true() {
        let policy = WindowPolicy {
            prefetch_ring: 3,
            mesh_ring: 2,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        // ring=1 ≤ mesh_ring(2) → always true regardless of direction
        assert!(policy.in_prefetch_cone(coord(1, 0, 0), anchor, [128, 0, 0]));
    }

    #[test]
    fn prefetch_cone_outside_prefetch_ring_returns_false() {
        let policy = WindowPolicy {
            prefetch_ring: 2,
            mesh_ring: 1,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        // ring=4 > mesh_ring(1)+prefetch_ring(2)=3 → false
        assert!(!policy.in_prefetch_cone(coord(4, 0, 0), anchor, [128, 0, 0]));
    }

    #[test]
    fn prefetch_cone_positive_cos_theta_in_cone() {
        // cos_q7=1 is minimal positive. threshold = 1 * l1 * 128.
        // With forward [256, 0, 0] (overscaled i32, valid in API), dot_q14=256*dx.
        // Condition: 256*dx > 1*dx*128 = 128*dx → 256>128 → true.
        let policy = WindowPolicy {
            prefetch_ring: 5,
            mesh_ring: 1,
            forward_cone_cos_theta: 1,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        assert!(policy.in_prefetch_cone(coord(3, 0, 0), anchor, [256, 0, 0]));
    }

    #[test]
    fn prefetch_cone_positive_cos_theta_opposite_direction() {
        // cos_q7=64, forward along +X, chunk at -X → dot_q14 is negative → false.
        let policy = WindowPolicy {
            prefetch_ring: 5,
            mesh_ring: 1,
            forward_cone_cos_theta: 64,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        assert!(!policy.in_prefetch_cone(coord(-3, 0, 0), anchor, [128, 0, 0]));
    }

    #[test]
    fn prefetch_cone_standard_forward_along_axis_with_cos_zero() {
        // cos_q7=0 → else branch: dot_q14 > 0. Standard forward [128,0,0], chunk at +X.
        let policy = WindowPolicy {
            prefetch_ring: 5,
            mesh_ring: 1,
            forward_cone_cos_theta: 0,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        assert!(policy.in_prefetch_cone(coord(3, 0, 0), anchor, [128, 0, 0]));
        assert!(!policy.in_prefetch_cone(coord(-3, 0, 0), anchor, [128, 0, 0]));
    }

    #[test]
    fn prefetch_cone_cos_theta_blocks_near_perpendicular() {
        // cos_q7=127 (~0.99), forward along +X. Perpendicular chunk at (0,0,3).
        // dx=0, dz=3, dy=0 → dot=0, l1=3 → threshold=127*3*128=48768 → 0 > 48768 → false.
        let policy = WindowPolicy {
            prefetch_ring: 5,
            mesh_ring: 1,
            forward_cone_cos_theta: 127,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        assert!(!policy.in_prefetch_cone(coord(0, 0, 3), anchor, [128, 0, 0]));
    }

    #[test]
    fn prefetch_cone_zero_cos_theta_requires_positive_dot() {
        let policy = WindowPolicy {
            prefetch_ring: 5,
            mesh_ring: 1,
            forward_cone_cos_theta: 0,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        // Forward along +X, chunk at +X → dot > 0 → true
        assert!(policy.in_prefetch_cone(coord(2, 0, 0), anchor, [128, 0, 0]));
        // Chunk at -X → dot < 0 → false
        assert!(!policy.in_prefetch_cone(coord(-2, 0, 0), anchor, [128, 0, 0]));
    }

    // ========================================================================
    // WindowPolicy — checked constructor
    // ========================================================================

    #[test]
    fn policy_checked_rejects_zero_vy_weight() {
        assert_eq!(
            WindowPolicy::checked(1, 1, 2, 1, 0, 2, 0, 0, 0),
            Err(PolicyError::ZeroVyWeight)
        );
    }

    #[test]
    fn policy_checked_rejects_zero_sim_lod_step() {
        assert_eq!(
            WindowPolicy::checked(1, 1, 2, 1, 2, 0, 0, 0, 0),
            Err(PolicyError::ZeroSimLodStep)
        );
    }

    #[test]
    fn policy_checked_rejects_sim_ring_above_coarse() {
        assert_eq!(
            WindowPolicy::checked(1, 3, 2, 1, 2, 2, 0, 0, 0),
            Err(PolicyError::SimRingAboveCoarseRing)
        );
    }

    #[test]
    fn policy_checked_accepts_valid_config() {
        let result = WindowPolicy::checked(1, 1, 2, 1, 2, 2, 0, 0, 0);
        assert!(result.is_ok());
        let p = result.unwrap();
        assert_eq!(p.mesh_ring, 1);
        assert_eq!(p.vy_weight, 2);
    }

    #[test]
    fn policy_checked_sim_ring_equals_coarse_ring_is_valid() {
        assert!(WindowPolicy::checked(1, 3, 3, 1, 2, 2, 0, 0, 0).is_ok());
    }

    #[test]
    fn policy_checked_all_fields_preserved() {
        let p = WindowPolicy::checked(5, 2, 10, 3, 3, 4, 6, -64, 12).unwrap();
        assert_eq!(p.mesh_ring, 5);
        assert_eq!(p.sim_ring, 2);
        assert_eq!(p.coarse_ring, 10);
        assert_eq!(p.seam_chunks, 3);
        assert_eq!(p.vy_weight, 3);
        assert_eq!(p.sim_lod_step, 4);
        assert_eq!(p.prefetch_ring, 6);
        assert_eq!(p.forward_cone_cos_theta, -64);
        assert_eq!(p.fade_ticks, 12);
    }

    // ========================================================================
    // WindowPolicy — default values
    // ========================================================================

    #[test]
    fn default_policy_has_expected_values() {
        let p = WindowPolicy::default();
        assert_eq!(p.mesh_ring, 1);
        assert_eq!(p.sim_ring, 1);
        assert_eq!(p.coarse_ring, 2);
        assert_eq!(p.seam_chunks, 1);
        assert_eq!(p.vy_weight, 2);
        assert_eq!(p.sim_lod_step, 2);
        assert_eq!(p.prefetch_ring, 0);
        assert_eq!(p.forward_cone_cos_theta, 0);
        assert_eq!(p.fade_ticks, 0);
    }

    // ========================================================================
    // PolicyError — Display
    // ========================================================================

    #[test]
    fn policy_error_display_messages() {
        assert_eq!(
            PolicyError::ZeroVyWeight.to_string(),
            "vy_weight must be ≥ 1"
        );
        assert_eq!(
            PolicyError::ZeroSimLodStep.to_string(),
            "sim_lod_step must be ≥ 1"
        );
        assert_eq!(
            PolicyError::SimRingAboveCoarseRing.to_string(),
            "sim_ring must be ≤ coarse_ring"
        );
        assert_eq!(
            PolicyError::ForwardConeOutOfRange.to_string(),
            "forward_cone_cos_theta must be in -128..=127"
        );
    }

    // ========================================================================
    // EvictionKey — ordering and construction
    // ========================================================================

    #[test]
    fn eviction_key_far_evicts_before_near() {
        let anchor = default_anchor();
        let near = EvictionKey::new(coord(1, 0, 0), anchor, 2, 0);
        let far = EvictionKey::new(coord(5, 0, 0), anchor, 2, 0);
        assert!(far < near, "far must evict before near");
    }

    #[test]
    fn eviction_key_same_ring_colder_lru_evicts_first() {
        let anchor = default_anchor();
        let cold = EvictionKey::new(coord(3, 0, 0), anchor, 1, 0);
        let warm = EvictionKey::new(coord(3, 0, 0), anchor, 1, 10);
        assert!(cold < warm, "colder LRU (lower pos) must evict first");
    }

    #[test]
    fn eviction_key_same_ring_same_lru_are_equal() {
        let anchor = default_anchor();
        let a = EvictionKey::new(coord(3, 0, 0), anchor, 1, 5);
        let b = EvictionKey::new(coord(3, 0, 0), anchor, 1, 5);
        assert_eq!(a, b);
        assert_eq!(a.cmp(&b), core::cmp::Ordering::Equal);
    }

    #[test]
    fn eviction_key_constructed_with_correct_ring() {
        let anchor = default_anchor();
        let k = EvictionKey::new(coord(3, 2, 0), anchor, 2, 7);
        // dx=3, dy=2*2=4, dz=0 → ring=4
        assert_eq!(k.ring, 4);
        assert_eq!(k.lru_pos, 7);
    }

    #[test]
    fn eviction_key_sort_order_is_eviction_priority() {
        let anchor = default_anchor();
        let mut keys = vec![
            EvictionKey::new(coord(1, 0, 0), anchor, 1, 10), // ring=1, lru=10
            EvictionKey::new(coord(5, 0, 0), anchor, 1, 0),  // ring=5, lru=0
            EvictionKey::new(coord(3, 0, 0), anchor, 1, 5),  // ring=3, lru=5
            EvictionKey::new(coord(3, 0, 0), anchor, 1, 15), // ring=3, lru=15
        ];
        keys.sort();
        // Eviction order: far→near, same ring cold→warm
        // ring=5 (0) → ring=3 (5) → ring=3 (15) → ring=1 (10)
        assert_eq!(keys[0].ring, 5);
        assert_eq!(keys[1].ring, 3);
        assert_eq!(keys[1].lru_pos, 5);
        assert_eq!(keys[2].ring, 3);
        assert_eq!(keys[2].lru_pos, 15);
        assert_eq!(keys[3].ring, 1);
    }

    #[test]
    fn eviction_key_partial_ord_consistent() {
        let anchor = default_anchor();
        let a = EvictionKey::new(coord(2, 0, 0), anchor, 1, 0);
        let b = EvictionKey::new(coord(4, 0, 0), anchor, 1, 0);
        // b has larger ring → b < a (b evicts first)
        assert!(b < a);
        assert!(a > b);
        assert_eq!(a.partial_cmp(&b), Some(core::cmp::Ordering::Greater));
        assert_eq!(b.partial_cmp(&a), Some(core::cmp::Ordering::Less));
    }

    // ========================================================================
    // Edge cases: state machine transitions
    // ========================================================================

    #[test]
    fn chunk_state_clone_and_debug() {
        let s = ChunkState::Fading { ticks_remaining: 5 };
        let s2 = s;
        assert_eq!(s, s2);
        let dbg = format!("{:?}", s);
        assert!(dbg.contains("Fading"));
        assert!(dbg.contains("ticks_remaining: 5"));
    }

    #[test]
    fn chunk_state_partial_eq_across_variants() {
        assert_ne!(ChunkState::Meshed, ChunkState::Resident);
        assert_ne!(ChunkState::Unloaded, ChunkState::Evicted);
        assert_ne!(
            ChunkState::Fading { ticks_remaining: 1 },
            ChunkState::Fading { ticks_remaining: 2 }
        );
        assert_eq!(
            ChunkState::Fading { ticks_remaining: 5 },
            ChunkState::Fading { ticks_remaining: 5 }
        );
    }

    #[test]
    fn window_policy_is_copy_and_debug() {
        let p1 = WindowPolicy::default();
        let p2 = p1; // Copy
        assert_eq!(p1, p2);
        let dbg = format!("{:?}", p1);
        assert!(dbg.contains("WindowPolicy"));
    }

    #[test]
    fn eviction_key_is_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let anchor = default_anchor();
        let k1 = EvictionKey::new(coord(3, 0, 0), anchor, 1, 5);
        let k2 = EvictionKey::new(coord(3, 0, 0), anchor, 1, 5);
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        k1.hash(&mut h1);
        k2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    // ========================================================================
    // Edge cases: full ring buffer lifecycle simulation
    // ========================================================================

    #[test]
    fn ring_buffer_full_lifecycle_classify_sequence() {
        // Simulate a player moving: anchor shifts, chunks change state.
        let policy = WindowPolicy::default();
        let anchor_a = coord(0, 0, 0);
        let anchor_b = coord(5, 0, 0); // player moved +5 along X

        // Chunk at (3, 0, 0): near anchor_a (ring=3→unloaded), near anchor_b (ring=2→seam→resident)
        assert_eq!(
            policy.classify(coord(3, 0, 0), anchor_a),
            ChunkState::Unloaded
        );
        assert_eq!(
            policy.classify(coord(3, 0, 0), anchor_b),
            ChunkState::Resident
        );

        // Chunk at (6, 0, 0): far from anchor_a (ring=6→unloaded), far from anchor_b (ring=1→meshed)
        assert_eq!(
            policy.classify(coord(6, 0, 0), anchor_a),
            ChunkState::Unloaded
        );
        assert_eq!(
            policy.classify(coord(6, 0, 0), anchor_b),
            ChunkState::Meshed
        );
    }

    #[test]
    fn eviction_ordering_under_budget_pressure() {
        // Simulate multiple chunks competing for eviction under memory pressure.
        let anchor = default_anchor();
        let vy_weight = 2;
        let mut keys = vec![
            EvictionKey::new(coord(8, 0, 0), anchor, vy_weight, 100), // ring=8, lru=100
            EvictionKey::new(coord(3, 0, 0), anchor, vy_weight, 0),   // ring=3, lru=0 (cold)
            EvictionKey::new(coord(3, 0, 0), anchor, vy_weight, 50),  // ring=3, lru=50 (warm)
            EvictionKey::new(coord(5, 0, 0), anchor, vy_weight, 200), // ring=5, lru=200
        ];
        keys.sort();
        // Eviction: ring=8 first, then ring=5, then ring=3 cold, then ring=3 warm
        assert_eq!(keys[0].ring, 8); // evict first
        assert_eq!(keys[1].ring, 5);
        assert_eq!(keys[2].ring, 3);
        assert_eq!(keys[2].lru_pos, 0); // cold
        assert_eq!(keys[3].ring, 3);
        assert_eq!(keys[3].lru_pos, 50); // warm — evict last
    }

    #[test]
    fn classify_empty_region_all_unloaded() {
        // Policy with no chunks meshed: all coords return Unloaded.
        let policy = WindowPolicy {
            mesh_ring: 0,
            seam_chunks: 0,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        // ring=0 → mesh_ring(0) → Meshed for same chunk
        assert_eq!(policy.classify(coord(0, 0, 0), anchor), ChunkState::Meshed);
        // ring=1 → >0 → Unloaded
        assert_eq!(policy.classify(coord(1, 0, 0), anchor), ChunkState::Unloaded);
    }

    #[test]
    fn classify_very_large_ring_distance() {
        let policy = WindowPolicy {
            mesh_ring: 1,
            seam_chunks: 1,
            ..WindowPolicy::default()
        };
        let anchor = default_anchor();
        // Huge offset → ring=1000 → Unloaded
        assert_eq!(policy.classify(coord(1000, 500, 1000), anchor), ChunkState::Unloaded);
    }

    #[test]
    fn double_evict_same_key_is_stable() {
        // Creating the same key twice yields equal values — idempotent.
        let anchor = default_anchor();
        let k1 = EvictionKey::new(coord(4, 1, 2), anchor, 2, 10);
        let k2 = EvictionKey::new(coord(4, 1, 2), anchor, 2, 10);
        assert_eq!(k1, k2);
        assert_eq!(k1.cmp(&k2), core::cmp::Ordering::Equal);
        // Sorting multiple identical keys is stable
        let mut v = vec![k1, k2, k1];
        v.sort();
        assert!(v.windows(2).all(|w| w[0] == w[1]));
    }

    #[test]
    fn sim_cohort_and_classify_consistency() {
        // For a chunk inside sim_ring, classify should produce Meshed or Resident
        // and sim_cohort should produce FullSim.
        let policy = WindowPolicy::default();
        let anchor = default_anchor();
        let c = coord(1, 0, 0); // ring=1 ≤ sim_ring=1, ≤ mesh_ring=1
        assert_eq!(policy.classify(c, anchor), ChunkState::Meshed);
        assert_eq!(policy.sim_cohort(c, anchor), SimCohort::FullSim);
    }

    #[test]
    fn negative_coord_ring_distance_symmetric() {
        let a = coord(-10, -5, -3);
        let b = coord(-7, -2, 1);
        // dx=3, dy=3, dz=4, vy_weight=2 → max(3, 6, 4) = 6
        assert_eq!(ring_distance(a, b, 2), 6);
        assert_eq!(ring_distance(b, a, 2), 6);
    }
}
