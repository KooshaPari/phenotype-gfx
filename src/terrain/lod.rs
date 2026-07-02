//! LOD base: distance-based tier selection.
//!
//! Ported from C# `LodBase.cs` (abstract class) + `LodTier` enum. The C#
//! abstract class becomes a Rust trait; concrete impls (`TerrainLod`,
//! `WaterLod`) supply the distance thresholds.
//!
//! The C# `LodBase` lived in the terrain repo but is also used by the water
//! module (`WaterLod`). To avoid duplication, the abstract shape lives here
//! and the water module re-exports the same `LodTier` + `LodBase` trait.

use crate::terrain::error::{TerrainError, TerrainResult};

/// Resolution tier returned by LOD selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodTier {
    /// Camera is within near distance — highest grid density.
    Near,
    /// Camera is within mid distance.
    Mid,
    /// Camera is within far distance — coarsest rendered grid.
    Far,
    /// Camera is beyond the cull distance — mesh should not be rendered.
    Culled,
}

/// LOD base: distance-based tier selection. Concrete impls supply the
/// distance thresholds (near, mid, cull) and a per-tier resolution.
pub trait LodBase {
    /// Distance at which the mesh transitions from `Near` to `Mid` quality.
    fn near_distance(&self) -> f32;
    /// Set the near distance threshold.
    fn set_near_distance(&mut self, value: f32);
    /// Distance at which the mesh transitions from `Mid` to `Far` quality.
    fn mid_distance(&self) -> f32;
    /// Set the mid distance threshold.
    fn set_mid_distance(&mut self, value: f32);
    /// Distance at which the mesh transitions from `Far` to `Culled`.
    fn cull_distance(&self) -> f32;
    /// Set the cull distance threshold.
    fn set_cull_distance(&mut self, value: f32);

    /// Returns the LOD tier appropriate for the given camera distance.
    fn select_tier(&self, distance: f32) -> TerrainResult<LodTier> {
        if distance < 0.0 {
            return Err(TerrainError::InvalidDistance { value: distance });
        }
        if distance < self.near_distance() {
            Ok(LodTier::Near)
        } else if distance < self.mid_distance() {
            Ok(LodTier::Mid)
        } else if distance < self.cull_distance() {
            Ok(LodTier::Far)
        } else {
            Ok(LodTier::Culled)
        }
    }

    /// Validates that the configured thresholds are strictly increasing:
    /// `near < mid < cull`.
    fn validate_thresholds(&self) -> TerrainResult<()> {
        if self.near_distance() >= self.mid_distance() {
            return Err(TerrainError::InvalidThresholds {
                msg: format!(
                    "NearDistance ({}) must be less than MidDistance ({})",
                    self.near_distance(),
                    self.mid_distance()
                ),
            });
        }
        if self.mid_distance() >= self.cull_distance() {
            return Err(TerrainError::InvalidThresholds {
                msg: format!(
                    "MidDistance ({}) must be less than CullDistance ({})",
                    self.mid_distance(),
                    self.cull_distance()
                ),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestLod {
        near: f32,
        mid: f32,
        cull: f32,
    }

    impl LodBase for TestLod {
        fn near_distance(&self) -> f32 {
            self.near
        }
        fn set_near_distance(&mut self, v: f32) {
            self.near = v;
        }
        fn mid_distance(&self) -> f32 {
            self.mid
        }
        fn set_mid_distance(&mut self, v: f32) {
            self.mid = v;
        }
        fn cull_distance(&self) -> f32 {
            self.cull
        }
        fn set_cull_distance(&mut self, v: f32) {
            self.cull = v;
        }
    }

    #[test]
    fn select_tier_near_returns_near() {
        let lod = TestLod {
            near: 10.0,
            mid: 50.0,
            cull: 100.0,
        };
        assert_eq!(lod.select_tier(5.0).unwrap(), LodTier::Near);
        assert_eq!(lod.select_tier(0.0).unwrap(), LodTier::Near);
    }

    #[test]
    fn select_tier_mid_returns_mid() {
        let lod = TestLod {
            near: 10.0,
            mid: 50.0,
            cull: 100.0,
        };
        assert_eq!(lod.select_tier(25.0).unwrap(), LodTier::Mid);
        assert_eq!(lod.select_tier(10.0).unwrap(), LodTier::Mid);
    }

    #[test]
    fn select_tier_far_returns_far() {
        let lod = TestLod {
            near: 10.0,
            mid: 50.0,
            cull: 100.0,
        };
        assert_eq!(lod.select_tier(75.0).unwrap(), LodTier::Far);
        assert_eq!(lod.select_tier(50.0).unwrap(), LodTier::Far);
    }

    #[test]
    fn select_tier_culled_returns_culled() {
        let lod = TestLod {
            near: 10.0,
            mid: 50.0,
            cull: 100.0,
        };
        assert_eq!(lod.select_tier(100.0).unwrap(), LodTier::Culled);
        assert_eq!(lod.select_tier(150.0).unwrap(), LodTier::Culled);
    }

    #[test]
    fn select_tier_negative_throws() {
        let lod = TestLod {
            near: 10.0,
            mid: 50.0,
            cull: 100.0,
        };
        assert!(lod.select_tier(-1.0).is_err());
    }

    #[test]
    fn validate_thresholds_valid_does_not_throw() {
        let lod = TestLod {
            near: 10.0,
            mid: 50.0,
            cull: 100.0,
        };
        assert!(lod.validate_thresholds().is_ok());
    }

    #[test]
    fn validate_thresholds_invalid_near_mid_throws() {
        let lod = TestLod {
            near: 50.0,
            mid: 50.0,
            cull: 100.0,
        };
        assert!(lod.validate_thresholds().is_err());
    }

    #[test]
    fn validate_thresholds_invalid_mid_cull_throws() {
        let lod = TestLod {
            near: 10.0,
            mid: 100.0,
            cull: 100.0,
        };
        assert!(lod.validate_thresholds().is_err());
    }
}
