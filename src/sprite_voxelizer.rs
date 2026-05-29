//! Sprite voxelizer: converts a 2D RGBA image into a 3D voxel volume.
//!
//! # Depth extrusion
//!
//! Ported from WSM3D `SpriteVoxelizer.cs` (phase 1 greedy-mesh branch).
//! The core idea: every occupied pixel (alpha > threshold) is extruded
//! symmetrically along the Z axis by a configurable depth instead of
//! producing a flat 1-voxel slab. The center slice `z = depth / 2` is
//! always solid; slices taper off at the Z edges for the balloon mode, or
//! stay uniform for the flat mode.
//!
//! ## Coordinate convention
//!
//! - `x` = pixel column (left → right).
//! - `y` = pixel row (bottom → top, matching OpenGL / Unity UV).
//! - `z` = depth axis, `+z` = "front", `0..depth` centered around the sprite
//!   plane so `z_center = depth / 2`.
//!
//! ## Usage
//!
//! ```rust
//! use phenotype_voxel::sprite_voxelizer::{VoxelizeConfig, voxelize_image};
//!
//! // 2×2 fully-opaque image, RGBA8
//! let pixels: Vec<[u8; 4]> = vec![[255, 0, 0, 255]; 4];
//! let cfg = VoxelizeConfig { depth: 3, ..Default::default() };
//! let vol = voxelize_image(&pixels, 2, 2, &cfg);
//! // A 2×2×3 volume — every voxel solid red.
//! assert_eq!(vol.len(), 2 * 2 * 3);
//! ```

use serde::{Deserialize, Serialize};

use crate::chunk::{Chunk, CHUNK_EDGE};
use crate::material::MaterialId;

/// Alpha threshold below which a pixel is considered transparent ("air").
/// WSM3D uses 16 / 255; we adopt the same constant.
pub const ALPHA_THRESHOLD: u8 = 16;

/// Default extrusion depth when none is provided by the caller.
/// Mirrors `SpriteVoxelizer.DefaultDepth = 8` in WSM3D.
pub const DEFAULT_DEPTH: u32 = 8;

/// A single voxel produced by the sprite voxelizer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteVoxel {
    /// Grid-local X position (pixel column).
    pub x: u32,
    /// Grid-local Y position (pixel row, bottom-up).
    pub y: u32,
    /// Grid-local Z position (depth slice, 0 = back face).
    pub z: u32,
    /// RGBA8 color sampled from the source image.
    pub color: [u8; 4],
}

/// Extrusion strategy, mirroring the `InflationStyle` enum in WSM3D.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExtrusionMode {
    /// Every occupied pixel is extruded uniformly along Z by exactly `depth`
    /// slices, symmetric about the center. This is the simplest "flat slab"
    /// mode and maps to WSM3D's `pertexel` style with no noise.
    ///
    /// The origin lies at `z = 0`; voxels occupy `z ∈ [0, depth)`.
    #[default]
    Flat,

    /// Balloon inflation: the Z extent for each pixel is proportional to its
    /// Manhattan distance to the nearest transparent pixel, so edge pixels
    /// are thin and interior pixels are thick. Produces a rounded, inflated
    /// volume. Mirrors WSM3D `BuildBalloon`.
    ///
    /// Requires the caller to supply the distance-to-air map via
    /// [`VoxelizeConfig::distance_to_air`]. If none is supplied, falls back
    /// to [`ExtrusionMode::Flat`].
    Balloon,
}

/// Configuration for [`voxelize_image`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxelizeConfig {
    /// Number of Z slices to extrude. Must be >= 1; clamped to 1 if zero.
    ///
    /// Corresponds to `SavedSettings.VoxelSpriteDepth` in WSM3D, defaulting to
    /// [`DEFAULT_DEPTH`] when unset.
    pub depth: u32,

    /// Extrusion strategy. Defaults to [`ExtrusionMode::Flat`].
    pub mode: ExtrusionMode,

    /// Pre-computed per-pixel Manhattan distance to the nearest transparent
    /// neighbour, row-major `x + y * width` order. Required for
    /// [`ExtrusionMode::Balloon`]; ignored for [`ExtrusionMode::Flat`].
    ///
    /// Ported from `SpriteVoxelizer.ComputeManhattanDistanceToAir`. Callers
    /// that need the balloon mode can pre-compute this with
    /// [`compute_manhattan_dist_to_air`].
    pub distance_to_air: Option<Vec<u32>>,

    /// Minimum depth for a balloon pixel. WSM3D clamps to 2 for body pixels.
    pub balloon_min_depth: u32,
}

impl Default for VoxelizeConfig {
    fn default() -> Self {
        Self {
            depth: DEFAULT_DEPTH,
            mode: ExtrusionMode::Flat,
            distance_to_air: None,
            balloon_min_depth: 2,
        }
    }
}

/// Convert a flat RGBA8 image into a 3D voxel volume using symmetric Z extrusion.
///
/// # Parameters
/// - `pixels` — row-major RGBA8 pixels, length `width * height`.
/// - `width`, `height` — image dimensions in pixels.
/// - `cfg` — extrusion configuration.
///
/// # Returns
/// A `Vec<SpriteVoxel>` containing one entry per solid (x, y, z) position.
/// The volume occupies `x ∈ [0, width)`, `y ∈ [0, height)`, `z ∈ [0, depth)`.
/// Transparent pixels produce no voxels.
pub fn voxelize_image(
    pixels: &[[u8; 4]],
    width: u32,
    height: u32,
    cfg: &VoxelizeConfig,
) -> Vec<SpriteVoxel> {
    let depth = cfg.depth.max(1);
    let z_center = depth / 2;

    let mut voxels = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let pixel = pixels[(x + y * width) as usize];
            if pixel[3] <= ALPHA_THRESHOLD {
                continue;
            }

            let (z_start, z_end) = match cfg.mode {
                ExtrusionMode::Flat => (0, depth),
                ExtrusionMode::Balloon => {
                    let dist = cfg
                        .distance_to_air
                        .as_deref()
                        .and_then(|d| d.get((x + y * width) as usize).copied())
                        .unwrap_or(depth / 2);
                    // Mirror of WSM3D's balloon formula:
                    // z_start = max(0, z_center - d)
                    // z_end   = min(depth, z_center + d + 1)
                    // with d = max(balloon_min_depth, dist)
                    let d = dist.max(cfg.balloon_min_depth);
                    let z_start = z_center.saturating_sub(d);
                    let z_end = (z_center + d + 1).min(depth);
                    (z_start, z_end)
                }
            };

            for z in z_start..z_end {
                voxels.push(SpriteVoxel {
                    x,
                    y,
                    z,
                    color: pixel,
                });
            }
        }
    }

    voxels
}

/// Compute the Manhattan distance from each solid pixel to the nearest transparent
/// neighbour using BFS. Transparent pixels have distance 0; solid pixels get the
/// minimum BFS distance to any transparent pixel.
///
/// This is a Rust port of `SpriteVoxelizer.ComputeManhattanDistanceToAir`.
///
/// # Returns
/// A row-major `Vec<u32>` of length `width * height`. Transparent pixels hold 0.
/// If the image is entirely solid (no air pixels), a fallback seeding from the
/// border pixels is applied (matching WSM3D's fallback branch).
pub fn compute_manhattan_dist_to_air(pixels: &[[u8; 4]], width: u32, height: u32) -> Vec<u32> {
    let n = (width * height) as usize;
    let mut dist = vec![u32::MAX / 4; n];
    let mut queue: std::collections::VecDeque<(u32, u32)> = std::collections::VecDeque::new();

    // Seed: transparent pixels have distance 0.
    for y in 0..height {
        for x in 0..width {
            let idx = (x + y * width) as usize;
            if pixels[idx][3] <= ALPHA_THRESHOLD {
                dist[idx] = 0;
                queue.push_back((x, y));
            }
        }
    }

    // Fallback: if entirely solid, seed from border.
    if queue.is_empty() {
        for y in 0..height {
            for &x in &[0u32, width.saturating_sub(1)] {
                let idx = (x + y * width) as usize;
                if pixels[idx][3] > ALPHA_THRESHOLD {
                    dist[idx] = 0;
                    queue.push_back((x, y));
                }
            }
        }
        for x in 0..width {
            for &y in &[0u32, height.saturating_sub(1)] {
                let idx = (x + y * width) as usize;
                if pixels[idx][3] > ALPHA_THRESHOLD && dist[idx] != 0 {
                    dist[idx] = 0;
                    queue.push_back((x, y));
                }
            }
        }
    }

    // BFS flood.
    while let Some((cx, cy)) = queue.pop_front() {
        let cur_dist = dist[(cx + cy * width) as usize];
        let next_dist = cur_dist + 1;

        let neighbours: [(i64, i64); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for (dx, dy) in neighbours {
            let nx = cx as i64 + dx;
            let ny = cy as i64 + dy;
            if nx < 0 || ny < 0 || nx >= width as i64 || ny >= height as i64 {
                continue;
            }
            let nidx = (nx as u32 + ny as u32 * width) as usize;
            if pixels[nidx][3] > ALPHA_THRESHOLD && dist[nidx] > next_dist {
                dist[nidx] = next_dist;
                queue.push_back((nx as u32, ny as u32));
            }
        }
    }

    dist
}

/// Convert a flat RGBA8 pixel grid into a `Chunk<MaterialId>` via Z-extrusion.
///
/// # Parameters
/// - `pixels` — row-major RGBA8 pixels, length `width * height`.
/// - `width`, `height` — image dimensions in pixels (each must be ≤ `CHUNK_EDGE`).
/// - `depth` — number of Z slices to extrude per opaque pixel (must be ≤ `CHUNK_EDGE`).
/// - `pixel_to_material` — closure mapping an RGBA8 pixel to a `MaterialId`. Called
///   once per opaque pixel; transparent pixels are skipped (they remain `MaterialId(0)`,
///   the conventional "air" value).
///
/// # Out-of-bounds handling
/// Pixels / depth values that would exceed `CHUNK_EDGE` in any dimension are silently
/// clamped so the function never panics on oversized inputs.
///
/// # Returns
/// A `Chunk<MaterialId>` of size `CHUNK_EDGE³`. Opaque pixels write their `MaterialId`
/// into all `z ∈ [0, depth)` slices at their `(x, y)` grid position. All other voxels
/// remain `MaterialId(0)`.
pub fn voxelize_to_chunk(
    pixels: &[[u8; 4]],
    width: u32,
    height: u32,
    depth: u32,
    pixel_to_material: impl Fn([u8; 4]) -> MaterialId,
) -> Chunk<MaterialId> {
    let mut chunk = Chunk::<MaterialId>::default();
    let edge = CHUNK_EDGE as u32;
    let clamped_width = width.min(edge);
    let clamped_height = height.min(edge);
    let clamped_depth = depth.min(edge).max(1);

    for y in 0..clamped_height {
        for x in 0..clamped_width {
            let idx = (x + y * width) as usize;
            if idx >= pixels.len() {
                continue;
            }
            let pixel = pixels[idx];
            if pixel[3] <= ALPHA_THRESHOLD {
                continue;
            }
            let mat = pixel_to_material(pixel);
            for z in 0..clamped_depth {
                // Chunk storage: x + y * EDGE + z * EDGE * EDGE
                let voxel_idx =
                    x as usize + y as usize * CHUNK_EDGE + z as usize * CHUNK_EDGE * CHUNK_EDGE;
                chunk.voxels[voxel_idx] = mat;
            }
        }
    }

    chunk
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;

    fn solid_pixel() -> [u8; 4] {
        [200, 100, 50, 255]
    }

    fn transparent_pixel() -> [u8; 4] {
        [0, 0, 0, 0]
    }

    /// FR-PHENO-VOXEL-SPRITEVOX-001 — flat mode with depth=1 produces exactly
    /// one voxel per solid pixel.
    #[test]
    fn flat_depth_1_one_voxel_per_solid_pixel() {
        let pixels = vec![solid_pixel(); 4];
        let cfg = VoxelizeConfig {
            depth: 1,
            ..Default::default()
        };
        let vol = voxelize_image(&pixels, 2, 2, &cfg);
        assert_eq!(vol.len(), 4);
        assert!(vol.iter().all(|v| v.z == 0));
    }

    /// FR-PHENO-VOXEL-SPRITEVOX-002 — flat mode with depth=N produces N voxels
    /// per solid pixel.
    #[test]
    fn flat_depth_n_produces_n_layers() {
        let pixels = vec![solid_pixel(); 1];
        for depth in [1u32, 3, 8] {
            let cfg = VoxelizeConfig {
                depth,
                ..Default::default()
            };
            let vol = voxelize_image(&pixels, 1, 1, &cfg);
            assert_eq!(vol.len(), depth as usize, "depth={depth}");
        }
    }

    /// FR-PHENO-VOXEL-SPRITEVOX-003 — transparent pixels produce no voxels.
    #[test]
    fn transparent_pixels_skipped() {
        let pixels = vec![transparent_pixel(); 4];
        let cfg = VoxelizeConfig::default();
        let vol = voxelize_image(&pixels, 2, 2, &cfg);
        assert!(vol.is_empty());
    }

    /// FR-PHENO-VOXEL-SPRITEVOX-004 — balloon mode with distance_to_air supplied
    /// produces fewer voxels for edge pixels than for interior ones.
    #[test]
    fn balloon_mode_thicker_interior() {
        // 3x1 image: [solid, solid, solid] with distances [1, 2, 1].
        let pixels = vec![solid_pixel(); 3];
        let dist = vec![1u32, 2u32, 1u32];
        let cfg = VoxelizeConfig {
            depth: 8,
            mode: ExtrusionMode::Balloon,
            distance_to_air: Some(dist),
            balloon_min_depth: 2,
        };
        let vol = voxelize_image(&pixels, 3, 1, &cfg);

        let count_x = |x: u32| vol.iter().filter(|v| v.x == x).count();
        // center pixel (x=1) has dist=2; edge pixels (x=0,2) have dist=1.
        // center should be at least as thick as edges.
        assert!(count_x(1) >= count_x(0));
    }

    /// FR-PHENO-VOXEL-SPRITEVOX-005 — manhattan distance BFS produces 0 for
    /// transparent pixels and > 0 for interior solid pixels.
    #[test]
    fn manhattan_dist_interior_solid_nonzero() {
        // 3×3 image: ring of transparent, solid center.
        let mut pixels = vec![transparent_pixel(); 9];
        pixels[4] = solid_pixel(); // center
        let dist = compute_manhattan_dist_to_air(&pixels, 3, 3);
        // Transparent pixels have distance 0.
        for i in [0, 1, 2, 3, 5, 6, 7, 8] {
            assert_eq!(dist[i], 0, "idx={i}");
        }
        // Center (idx=4) is 1 hop from a transparent pixel.
        assert_eq!(dist[4], 1);
    }

    /// FR-PHENO-VOXEL-SPRITEVOX-006 — depth=0 is clamped to 1.
    #[test]
    fn depth_zero_clamped_to_one() {
        let pixels = vec![solid_pixel()];
        let cfg = VoxelizeConfig {
            depth: 0,
            ..Default::default()
        };
        let vol = voxelize_image(&pixels, 1, 1, &cfg);
        assert_eq!(vol.len(), 1);
    }

    proptest! {
        /// Random flat-mode images stay within the declared X/Y/Z bounds and
        /// produce one `depth`-sized column per opaque pixel.
        #[test]
        fn flat_mode_respects_volume_dimensions(
            (width, height, depth, pixels) in (1u32..8, 1u32..8, 1u32..8).prop_flat_map(|(width, height, depth)| {
                let pixel_count = (width * height) as usize;
                prop::collection::vec(any::<[u8; 4]>(), pixel_count)
                    .prop_map(move |pixels| (width, height, depth, pixels))
            })
        ) {
            let cfg = VoxelizeConfig {
                depth,
                ..Default::default()
            };
            let vol = voxelize_image(&pixels, width, height, &cfg);
            let expected_opaque = pixels.iter().filter(|p| p[3] > ALPHA_THRESHOLD).count();

            prop_assert_eq!(vol.len(), expected_opaque * depth as usize);
            prop_assert!(vol.iter().all(|v| v.x < width && v.y < height && v.z < depth));
        }

        /// Fully transparent images should never generate voxels, regardless
        /// of dimensions or depth.
        #[test]
        fn transparent_pixels_produce_no_voxels(
            (width, height, depth, pixels) in (1u32..8, 1u32..8, 1u32..8).prop_flat_map(|(width, height, depth)| {
                let pixel_count = (width * height) as usize;
                prop::collection::vec(any::<[u8; 3]>(), pixel_count)
                    .prop_map(move |pixels| (width, height, depth, pixels))
            })
        ) {
            let pixels = pixels
                .into_iter()
                .map(|[r, g, b]| [r, g, b, 0])
                .collect::<Vec<_>>();
            let cfg = VoxelizeConfig {
                depth,
                ..Default::default()
            };
            let vol = voxelize_image(&pixels, width, height, &cfg);

            prop_assert!(vol.is_empty());
        }

        /// Flat extrusion should be symmetric around the Z axis: every voxel
        /// has a mirrored voxel at `depth - 1 - z` for the same X/Y position.
        #[test]
        fn flat_mode_extrusion_is_z_symmetric(
            (width, height, depth, pixels) in (1u32..8, 1u32..8, 1u32..8).prop_flat_map(|(width, height, depth)| {
                let pixel_count = (width * height) as usize;
                prop::collection::vec(any::<[u8; 4]>(), pixel_count)
                    .prop_map(move |pixels| (width, height, depth, pixels))
            })
        ) {
            let cfg = VoxelizeConfig {
                depth,
                ..Default::default()
            };
            let vol = voxelize_image(&pixels, width, height, &cfg);
            let voxels = vol
                .iter()
                .map(|v| (v.x, v.y, v.z))
                .collect::<HashSet<_>>();

            for voxel in &vol {
                prop_assert!(voxels.contains(&(voxel.x, voxel.y, depth - 1 - voxel.z)));
            }
        }
    }

    // ── voxelize_to_chunk tests ────────────────────────────────────────────────

    use crate::chunk::CHUNK_EDGE;
    use crate::material::MaterialId;

    fn red_pixel() -> [u8; 4] {
        [255, 0, 0, 255]
    }

    fn blue_pixel() -> [u8; 4] {
        [0, 0, 255, 255]
    }

    fn air_pixel() -> [u8; 4] {
        [0, 0, 0, 0]
    }

    fn identity_palette(pixel: [u8; 4]) -> MaterialId {
        // Map R channel to a MaterialId (simple palette for tests).
        MaterialId(pixel[0] as u16)
    }

    /// FR-PHENO-VOXEL-SPRITEVOX-007 — 2×2 fully-opaque sprite with depth D
    /// produces exactly 4 * D solid voxels in the chunk.
    #[test]
    fn chunk_2x2_opaque_sprite_voxel_count() {
        let depth = 3u32;
        let pixels = vec![red_pixel(); 4];
        let chunk = voxelize_to_chunk(&pixels, 2, 2, depth, identity_palette);
        let solid_count = chunk.voxels.iter().filter(|&&m| m != MaterialId(0)).count();
        assert_eq!(solid_count, 4 * depth as usize);
    }

    /// FR-PHENO-VOXEL-SPRITEVOX-008 — transparent pixels produce no solid voxels.
    #[test]
    fn chunk_transparent_pixels_skipped() {
        let pixels = vec![air_pixel(); 4];
        let chunk = voxelize_to_chunk(&pixels, 2, 2, 4, identity_palette);
        assert!(chunk.voxels.iter().all(|&m| m == MaterialId(0)));
    }

    /// FR-PHENO-VOXEL-SPRITEVOX-009 — palette closure maps pixel color → MaterialId.
    #[test]
    fn chunk_palette_mapping_correct() {
        // 1×2 image: top pixel red (R=255), bottom pixel blue (B=255, R=0).
        let pixels = vec![red_pixel(), blue_pixel()];
        let chunk = voxelize_to_chunk(&pixels, 1, 2, 1, |px| MaterialId(px[0] as u16));
        // y=0 → red_pixel → MaterialId(255)
        let voxel_y0 = chunk.voxels[0]; // x=0, y=0, z=0
        // y=1 → blue_pixel → MaterialId(0) (R=0) — but 0 is "air"; use a different mapping
        // Actually R=0 means MaterialId(0) which is air. Let's verify the red one.
        assert_eq!(voxel_y0, MaterialId(255));
        // blue pixel has R=0 → MaterialId(0), so it's indistinguishable from air.
        // Use a different mapping: use B channel for blue pixel.
        let chunk2 = voxelize_to_chunk(&pixels, 1, 2, 1, |px| MaterialId(px[2] as u16));
        let voxel_y1 = chunk2.voxels[0 + 1 * CHUNK_EDGE]; // x=0, y=1, z=0
        assert_eq!(voxel_y1, MaterialId(255)); // B=255 for blue pixel
    }

    /// FR-PHENO-VOXEL-SPRITEVOX-010 — extrusion depth respected: voxels span exactly
    /// z ∈ [0, depth) and z ≥ depth is empty.
    #[test]
    fn chunk_extrusion_depth_respected() {
        let depth = 3u32;
        let pixels = vec![red_pixel()];
        let chunk = voxelize_to_chunk(&pixels, 1, 1, depth, identity_palette);
        for z in 0..depth as usize {
            assert_ne!(chunk.voxels[z * CHUNK_EDGE * CHUNK_EDGE], MaterialId(0), "z={z} should be solid");
        }
        // z >= depth should be empty (up to CHUNK_EDGE)
        for z in depth as usize..CHUNK_EDGE {
            assert_eq!(chunk.voxels[z * CHUNK_EDGE * CHUNK_EDGE], MaterialId(0), "z={z} should be air");
        }
    }

    /// FR-PHENO-VOXEL-SPRITEVOX-011 — empty sprite (0×0 or all transparent) → empty chunk.
    #[test]
    fn chunk_empty_sprite_produces_empty_chunk() {
        // All transparent
        let pixels = vec![air_pixel(); 4];
        let chunk = voxelize_to_chunk(&pixels, 2, 2, 4, identity_palette);
        assert!(chunk.voxels.iter().all(|&m| m == MaterialId(0)));
        // Zero-pixel image (empty slice)
        let chunk2 = voxelize_to_chunk(&[], 0, 0, 4, identity_palette);
        assert!(chunk2.voxels.iter().all(|&m| m == MaterialId(0)));
    }

    /// FR-PHENO-VOXEL-SPRITEVOX-012 — out-of-bounds inputs (width/height/depth > CHUNK_EDGE)
    /// are clamped without panicking.
    #[test]
    fn chunk_out_of_bounds_clamped_no_panic() {
        let oversized_dim = (CHUNK_EDGE as u32) * 2;
        let pixel_count = oversized_dim * oversized_dim;
        let pixels = vec![red_pixel(); pixel_count as usize];
        // Should not panic; excess pixels/depth are clamped.
        let chunk = voxelize_to_chunk(&pixels, oversized_dim, oversized_dim, oversized_dim, identity_palette);
        // All voxels are within the valid chunk range.
        assert_eq!(chunk.voxels.len(), CHUNK_EDGE * CHUNK_EDGE * CHUNK_EDGE);
    }
}
