//! Concrete [`LodBase`] impl for the water surface mesh.
//!
//! Ported from C# `Rendering/WaterLod.cs`. Default thresholds: 50 / 150 / 400
//! world units. Default per-tier resolutions: 64 / 32 / 16 quads per side.

use crate::terrain::lod::{LodBase, LodTier};
use crate::water::error::WaterResult;

/// Default LOD config for water surface mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterLod {
    near: f32,
    mid: f32,
    cull: f32,
    near_resolution: i32,
    mid_resolution: i32,
    far_resolution: i32,
}

impl Default for WaterLod {
    fn default() -> Self {
        Self {
            near: 50.0,
            mid: 150.0,
            cull: 400.0,
            near_resolution: 64,
            mid_resolution: 32,
            far_resolution: 16,
        }
    }
}

impl WaterLod {
    /// Create a water LOD with default thresholds and resolutions.
    pub fn new() -> Self { Self::default() }

    /// Grid resolution for the `Near` tier.
    pub fn near_resolution(&self) -> i32 { self.near_resolution }
    /// Set the near resolution.
    pub fn set_near_resolution(&mut self, v: i32) { self.near_resolution = v; }
    /// Grid resolution for the `Mid` tier.
    pub fn mid_resolution(&self) -> i32 { self.mid_resolution }
    /// Set the mid resolution.
    pub fn set_mid_resolution(&mut self, v: i32) { self.mid_resolution = v; }
    /// Grid resolution for the `Far` tier.
    pub fn far_resolution(&self) -> i32 { self.far_resolution }
    /// Set the far resolution.
    pub fn set_far_resolution(&mut self, v: i32) { self.far_resolution = v; }

    /// Returns the grid resolution for the given distance; `0` when culled.
    pub fn select_resolution(&self, distance: f32) -> WaterResult<i32> {
        Ok(match self.select_tier(distance)? {
            LodTier::Near => self.near_resolution,
            LodTier::Mid => self.mid_resolution,
            LodTier::Far => self.far_resolution,
            LodTier::Culled => 0,
        })
    }
}

impl LodBase for WaterLod {
    fn near_distance(&self) -> f32 { self.near }
    fn set_near_distance(&mut self, v: f32) { self.near = v; }
    fn mid_distance(&self) -> f32 { self.mid }
    fn set_mid_distance(&mut self, v: f32) { self.mid = v; }
    fn cull_distance(&self) -> f32 { self.cull }
    fn set_cull_distance(&mut self, v: f32) { self.cull = v; }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_thresholds_are_expected() {
        let lod = WaterLod::new();
        assert_eq!(lod.near_distance(), 50.0);
        assert_eq!(lod.mid_distance(), 150.0);
        assert_eq!(lod.cull_distance(), 400.0);
    }

    #[test]
    fn default_resolutions_are_expected() {
        let lod = WaterLod::new();
        assert_eq!(lod.near_resolution(), 64);
        assert_eq!(lod.mid_resolution(), 32);
        assert_eq!(lod.far_resolution(), 16);
    }

    #[test]
    fn default_resolutions_are_monotonically_decreasing() {
        let lod = WaterLod::new();
        assert!(lod.near_resolution() > lod.mid_resolution());
        assert!(lod.mid_resolution() > lod.far_resolution());
        assert!(lod.far_resolution() > 0);
    }

    #[test]
    fn select_resolution_near_distance_returns_near_resolution() {
        let lod = WaterLod::new();
        assert_eq!(lod.select_resolution(0.0).unwrap(), 64);
        assert_eq!(lod.select_resolution(25.0).unwrap(), 64);
    }

    #[test]
    fn select_resolution_mid_distance_returns_mid_resolution() {
        let lod = WaterLod::new();
        assert_eq!(lod.select_resolution(75.0).unwrap(), 32);
    }

    #[test]
    fn select_resolution_far_distance_returns_far_resolution() {
        let lod = WaterLod::new();
        assert_eq!(lod.select_resolution(200.0).unwrap(), 16);
    }

    #[test]
    fn select_resolution_beyond_cull_returns_zero() {
        let lod = WaterLod::new();
        assert_eq!(lod.select_resolution(500.0).unwrap(), 0);
        assert_eq!(lod.select_resolution(1000.0).unwrap(), 0);
    }

    #[test]
    fn select_tier_thresholds_default() {
        let lod = WaterLod::new();
        let cases = [
            (0.0, LodTier::Near),
            (25.0, LodTier::Near),
            (49.9, LodTier::Near),
            (50.0, LodTier::Mid),
            (100.0, LodTier::Mid),
            (149.9, LodTier::Mid),
            (150.0, LodTier::Far),
            (300.0, LodTier::Far),
            (399.9, LodTier::Far),
            (400.0, LodTier::Culled),
            (999.0, LodTier::Culled),
        ];
        for (d, expected) in cases {
            assert_eq!(lod.select_tier(d).unwrap(), expected, "at d={d}");
        }
    }

    #[test]
    fn monotonic_farther_distance_never_finer_resolution() {
        let lod = WaterLod::new();
        let distances = [0.0, 10.0, 49.0, 50.0, 100.0, 149.0, 150.0, 300.0, 399.0, 400.0, 800.0];
        let mut prev = i32::MAX;
        for d in distances {
            let res = lod.select_resolution(d).unwrap();
            assert!(res <= prev, "At d={d} resolution {res} is finer than previous {prev}");
            prev = res;
        }
    }

    #[test]
    fn custom_thresholds_respected_by_select_tier() {
        let lod = WaterLod {
            near: 10.0,
            mid: 30.0,
            cull: 60.0,
            ..WaterLod::default()
        };
        assert_eq!(lod.select_tier(5.0).unwrap(), LodTier::Near);
        assert_eq!(lod.select_tier(15.0).unwrap(), LodTier::Mid);
        assert_eq!(lod.select_tier(40.0).unwrap(), LodTier::Far);
        assert_eq!(lod.select_tier(60.0).unwrap(), LodTier::Culled);
    }

    #[test]
    fn custom_resolutions_respected_by_select_resolution() {
        let lod = WaterLod {
            near_resolution: 128,
            mid_resolution: 64,
            far_resolution: 32,
            ..WaterLod::default()
        };
        assert_eq!(lod.select_resolution(0.0).unwrap(), 128);
        assert_eq!(lod.select_resolution(75.0).unwrap(), 64);
        assert_eq!(lod.select_resolution(200.0).unwrap(), 32);
        assert_eq!(lod.select_resolution(500.0).unwrap(), 0);
    }

    #[test]
    fn valid_thresholds_does_not_throw() {
        let lod = WaterLod::new();
        assert!(lod.validate_thresholds().is_ok());
    }

    #[test]
    fn invalid_thresholds_near_ge_mid_raises() {
        let lod = WaterLod { near: 200.0, mid: 100.0, ..WaterLod::default() };
        assert!(lod.validate_thresholds().is_err());
    }

    #[test]
    fn invalid_thresholds_mid_ge_cull_raises() {
        let lod = WaterLod { mid: 500.0, cull: 300.0, ..WaterLod::default() };
        assert!(lod.validate_thresholds().is_err());
    }

    #[test]
    fn negative_distance_raises() {
        let lod = WaterLod::new();
        assert!(lod.select_tier(-1.0).is_err());
        assert!(lod.select_resolution(-0.001).is_err());
    }
}
