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
    let w = if vy_weight == 0 { 1u32 } else { vy_weight as u32 };
    let dx = (coord.cx - anchor.cx).unsigned_abs();
    let dz = (coord.cz - anchor.cz).unsigned_abs();
    let dy = (coord.cy - anchor.cy).unsigned_abs() * w;
    // Manual max-of-three to stay const fn (u32::max not const-stable yet).
    let m = if dx > dz { dx } else { dz };
    if m > dy { m } else { dy }
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
        matches!(self, Self::Resident | Self::Meshed | Self::Fading { .. } | Self::Evicting)
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
        mesh_ring: u8, sim_ring: u8, coarse_ring: u8, seam_chunks: u8,
        vy_weight: u8, sim_lod_step: u8, prefetch_ring: u8,
        forward_cone_cos_theta: i8, fade_ticks: u8,
    ) -> Result<Self, PolicyError> {
        if vy_weight == 0 { return Err(PolicyError::ZeroVyWeight); }
        if sim_lod_step == 0 { return Err(PolicyError::ZeroSimLodStep); }
        if sim_ring > coarse_ring { return Err(PolicyError::SimRingAboveCoarseRing); }
        Ok(Self {
            mesh_ring, sim_ring, coarse_ring, seam_chunks, vy_weight,
            sim_lod_step, prefetch_ring, forward_cone_cos_theta, fade_ticks,
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
                ChunkState::Fading { ticks_remaining: self.fade_ticks }
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
            SimCohort::CoarseSim { step_multiplier: self.sim_lod_step }
        } else {
            SimCohort::Frozen
        }
    }

    /// True if `coord` is in the prefetch cone.
    #[must_use]
    pub const fn in_prefetch_cone(
        &self, coord: ChunkCoord, anchor: ChunkCoord, forward_q7: [i32; 3],
    ) -> bool {
        if self.prefetch_ring == 0 { return false; }
        let ring = ring_distance(coord, anchor, self.vy_weight);
        if ring <= self.mesh_ring as u32 { return true; }
        if ring > (self.mesh_ring as u32).saturating_add(self.prefetch_ring as u32) { return false; }
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
        Self { ring: ring_distance(coord, anchor, vy_weight), lru_pos }
    }
}

impl Ord for EvictionKey {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // Larger ring = evict first → invert ring comparison.
        other.ring.cmp(&self.ring).then(self.lru_pos.cmp(&other.lru_pos))
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

    fn coord(cx: i32, cy: i32, cz: i32) -> ChunkCoord { ChunkCoord { cx, cy, cz } }

    #[test]
    fn ring_distance_horizontal_is_chebyshev() {
        let anchor = coord(0, 0, 0);
        assert_eq!(ring_distance(coord(3, 0, 1), anchor, 2), 3);
    }

    #[test]
    fn ring_distance_vertical_uses_vy_weight() {
        let anchor = coord(0, 0, 0);
        // dy=1, vy_weight=2 → effective dy=2 > dx=1 → distance=2
        assert_eq!(ring_distance(coord(1, 1, 0), anchor, 2), 2);
    }

    #[test]
    fn window_policy_classify_inner_is_meshed() {
        let policy = WindowPolicy::default(); // mesh_ring=1
        let anchor = coord(0, 0, 0);
        assert_eq!(policy.classify(coord(1, 0, 0), anchor), ChunkState::Meshed);
        assert!(matches!(policy.classify(coord(2, 0, 0), anchor), ChunkState::Resident));
        assert_eq!(policy.classify(coord(5, 0, 0), anchor), ChunkState::Unloaded);
    }

    #[test]
    fn window_policy_sim_cohort_bands() {
        let policy = WindowPolicy::default(); // sim_ring=1, coarse_ring=2
        let anchor = coord(0, 0, 0);
        assert_eq!(policy.sim_cohort(coord(1, 0, 0), anchor), SimCohort::FullSim);
        assert!(matches!(policy.sim_cohort(coord(2, 0, 0), anchor), SimCohort::CoarseSim { .. }));
        assert_eq!(policy.sim_cohort(coord(3, 0, 0), anchor), SimCohort::Frozen);
    }

    #[test]
    fn eviction_key_far_evicts_before_near() {
        let anchor = coord(0, 0, 0);
        let near = EvictionKey::new(coord(1, 0, 0), anchor, 2, 0);
        let far = EvictionKey::new(coord(5, 0, 0), anchor, 2, 0);
        assert!(far < near, "far must evict before near");
    }

    #[test]
    fn policy_checked_rejects_zero_vy_weight() {
        assert!(WindowPolicy::checked(1, 1, 2, 1, 0, 2, 0, 0, 0).is_err());
    }

    #[test]
    fn policy_checked_rejects_sim_ring_above_coarse() {
        assert!(WindowPolicy::checked(1, 3, 2, 1, 2, 2, 0, 0, 0).is_err());
    }
}
