//! Concrete `TerrainLod` impl. Ported from C# `TerrainLod.cs`.
//!
//! Default thresholds: 50 / 150 / 400 world units. Default resolutions:
//! 64 / 32 / 16 quads per side.

use serde::{Deserialize, Serialize};

use crate::terrain::error::TerrainResult;
use crate::terrain::lod::{LodBase, LodTier};

/// Default LOD config for terrain chunks.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TerrainLod {
    near: f32,
    mid: f32,
    cull: f32,
    near_resolution: i32,
    mid_resolution: i32,
    far_resolution: i32,
}

impl Default for TerrainLod {
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

impl TerrainLod {
    /// Create with default thresholds and resolutions.
    pub fn new() -> Self { Self::default() }

    /// Grid resolution for the `Near` tier. Must be `> 0`.
    pub fn near_resolution(&self) -> i32 { self.near_resolution }
    /// Set the near resolution. Caller is responsible for `> 0`.
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
    pub fn select_resolution(&self, distance: f32) -> TerrainResult<i32> {
        Ok(match self.select_tier(distance)? {
            LodTier::Near => self.near_resolution,
            LodTier::Mid => self.mid_resolution,
            LodTier::Far => self.far_resolution,
            LodTier::Culled => 0,
        })
    }
}

impl LodBase for TerrainLod {
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
        let lod = TerrainLod::new();
        assert_eq!(lod.near_distance(), 50.0);
        assert_eq!(lod.mid_distance(), 150.0);
        assert_eq!(lod.cull_distance(), 400.0);
    }

    #[test]
    fn default_resolutions_are_expected() {
        let lod = TerrainLod::new();
        assert_eq!(lod.near_resolution(), 64);
        assert_eq!(lod.mid_resolution(), 32);
        assert_eq!(lod.far_resolution(), 16);
    }

    #[test]
    fn select_resolution_near_distance_returns_near_resolution() {
        let lod = TerrainLod::new();
        assert_eq!(lod.select_resolution(25.0).unwrap(), 64);
    }

    #[test]
    fn select_resolution_mid_distance_returns_mid_resolution() {
        let lod = TerrainLod::new();
        assert_eq!(lod.select_resolution(100.0).unwrap(), 32);
    }

    #[test]
    fn select_resolution_far_distance_returns_far_resolution() {
        let lod = TerrainLod::new();
        assert_eq!(lod.select_resolution(250.0).unwrap(), 16);
    }

    #[test]
    fn select_resolution_culled_returns_zero() {
        let lod = TerrainLod::new();
        assert_eq!(lod.select_resolution(500.0).unwrap(), 0);
    }

    #[test]
    fn validate_thresholds_default_does_not_throw() {
        let lod = TerrainLod::new();
        assert!(lod.validate_thresholds().is_ok());
    }
}
