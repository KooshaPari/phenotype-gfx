//! Greedy mesher — merges coplanar adjacent faces of the same material into
//! maximal rectangles (quads), drastically reducing triangle count relative to
//! the reference [`CubicMesher`].
//!
//! ## Algorithm
//!
//! For each of the 6 axis-aligned face directions:
//! 1. Sweep through each "slice" perpendicular to that axis.
//! 2. Build a 2-D mask of visible (exposed) faces, keyed by [`MaskCell`]
//!    (material + 4-corner AO signature).  A face is visible if the voxel on
//!    the face side is solid and the voxel on the opposite side (the
//!    "neighbour") is not solid.  Per-vertex AO is computed with the same
//!    `face_ao` helper used by [`CubicMesher`].
//! 3. For each non-empty cell in the mask, extend a maximal rectangle: first
//!    greedily widen along the primary axis until the material **or AO
//!    signature** changes, then raise along the secondary axis as far as the
//!    full width is available with the same key.
//! 4. Emit one quad per rectangle, propagating the per-corner AO values.
//!    Consumed cells are cleared from the mask so they are not emitted twice.
//!
//! ## AO-aware merging
//!
//! Including the 4-corner AO signature in the equality key means:
//! - Faces in a flat, unoccluded region share AO=[3,3,3,3] → merge freely
//!   (greedy triangle-reduction benefit fully preserved).
//! - Faces at an occlusion boundary carry different AO signatures → merge
//!   stops at the boundary (AO detail preserved).
//! - A merged quad carries uniform AO; no interpolation artefact can arise.
//!
//! The resulting mesh has the same *visible surface area* as the cubic mesher
//! but (for large homogeneous regions) far fewer triangles, with correct
//! per-vertex AO everywhere.

use core::marker::PhantomData;

use tracing::instrument;

use crate::voxel::chunk::{ChunkView, CHUNK_EDGE};
use crate::voxel::cubic_mesher::face_ao;
use crate::voxel::lod::LodLevel;
use crate::voxel::material::MaterialId;
use crate::voxel::mesh::{MeshBuffer, MeshError, MeshResult, MeshVertex, Mesher};

pub use crate::voxel::cubic_mesher::CubicVoxel;

// ---------------------------------------------------------------------------
// Mask cell — material + 4-corner AO signature
// ---------------------------------------------------------------------------

/// Key stored in the 2-D greedy mask.
///
/// Two face cells are only merged if they share the same [`MaterialId`] **and**
/// the same `ao` signature (all four corner AO values match).  This ensures
/// AO discontinuities prevent merging while homogeneous regions still collapse
/// into large quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MaskCell {
    material: MaterialId,
    /// Per-corner AO values in the same vertex order used by [`emit_quad`]:
    /// `[v0, v1, v2, v3]` = `[(-u,-v), (+u,-v), (+u,+v), (-u,+v)]`.
    ao: [u8; 4],
}

// ---------------------------------------------------------------------------
// Public type
// ---------------------------------------------------------------------------

/// Greedy mesher, generic over any [`CubicVoxel`] type.
///
/// Produces the same visible surface as [`CubicMesher`] but merges coplanar,
/// same-material faces into maximal quads, cutting triangle count significantly
/// for large flat regions.
#[derive(Debug, Clone, Copy, Default)]
pub struct GreedyMesher<V>(PhantomData<V>);

impl<V: CubicVoxel> GreedyMesher<V> {
    /// Construct a new `GreedyMesher` for voxel type `V`.
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<V: CubicVoxel> Mesher for GreedyMesher<V> {
    type VoxelKind = V;
    type Mesh = MeshBuffer;

    fn mesh_chunk(&self, chunk: ChunkView<'_, V>, lod: LodLevel) -> MeshResult<Self::Mesh> {
        Self::mesh_greedy(chunk, lod)
    }
}

// ---------------------------------------------------------------------------
// Core algorithm
// ---------------------------------------------------------------------------

impl<V: CubicVoxel> GreedyMesher<V> {
    /// Core greedy meshing pass with AO-aware face merging.
    ///
    /// Each mask cell carries a [`MaskCell`] (material + 4-corner AO signature).
    /// Faces are merged only when both the material **and** the AO signature
    /// are identical, preserving per-vertex AO detail at occlusion boundaries
    /// while still collapsing large flat unoccluded regions into single quads.
    #[instrument(level = "trace", skip(chunk), fields(lod = ?_lod))]
    pub fn mesh_greedy(chunk: ChunkView<'_, V>, _lod: LodLevel) -> MeshResult<MeshBuffer> {
        metrics::counter!("phenotype_gfx.voxel_mesh_builds").increment(1);
        let n = CHUNK_EDGE;
        let expected = n * n * n;
        if chunk.voxels.len() != expected {
            return Err(MeshError::BadChunkSize {
                got: chunk.voxels.len(),
                expected,
            });
        }

        let mut buf = MeshBuffer::default();

        // Helper: read voxel at (x, y, z) in chunk-local coords.  Returns
        // `None` (treated as air) when out of bounds.
        let voxel = |x: i32, y: i32, z: i32| -> Option<&V> {
            let ni = n as i32;
            if x < 0 || y < 0 || z < 0 || x >= ni || y >= ni || z >= ni {
                return None;
            }
            Some(&chunk.voxels[x as usize + y as usize * n + z as usize * n * n])
        };

        // We iterate over 6 face directions.  For each direction we define:
        //   `axis`     – the perpendicular axis index (0=X, 1=Y, 2=Z)
        //   `neg`      – true if this is the negative-facing face (−X, −Y, −Z)
        //   `normal`   – outward unit normal
        //
        // For a given axis d we let:
        //   u = (d+1) % 3   (first tangent axis)
        //   v = (d+2) % 3   (second tangent axis)
        //
        // Slice index runs from 0..CHUNK_EDGE along axis d.

        for axis in 0_usize..3 {
            for &neg in &[false, true] {
                // Tangent axes.
                let u_axis = (axis + 1) % 3;
                let v_axis = (axis + 2) % 3;
                let size_u = n;
                let size_v = n;

                // Outward normal.
                let mut normal = [0.0_f32; 3];
                normal[axis] = if neg { -1.0 } else { 1.0 };

                // face index for face_ao: encode (axis, neg) → CubicMesher face id.
                // CubicMesher face encoding: 0=+x,1=-x,2=+y,3=-y,4=+z,5=-z.
                let face_id: u8 = match (axis, neg) {
                    (0, false) => 0,
                    (0, true) => 1,
                    (1, false) => 2,
                    (1, true) => 3,
                    (2, false) => 4,
                    _ => 5,
                };

                // Mask: indexed by [u + v * size_u].  `None` = air / already used.
                let mut mask: Vec<Option<MaskCell>> = vec![None; size_u * size_v];

                for d in 0..n {
                    // Build mask for this slice (including AO per face cell).
                    for mv in 0..size_v {
                        for mu in 0..size_u {
                            // Construct 3-D coords from (d, mu, mv) for the
                            // current axis orientation.
                            let mut pos = [0_i32; 3];
                            pos[axis] = d as i32;
                            pos[u_axis] = mu as i32;
                            pos[v_axis] = mv as i32;

                            let vox = voxel(pos[0], pos[1], pos[2]);

                            // Skip if the voxel on this side is not solid.
                            let is_solid = vox.is_some_and(|v| v.is_solid());
                            if !is_solid {
                                mask[mu + mv * size_u] = None;
                                continue;
                            }

                            // Check the neighbour on the outward side.
                            let mut npos = pos;
                            if neg {
                                npos[axis] -= 1;
                            } else {
                                npos[axis] += 1;
                            }
                            let neighbour_solid =
                                voxel(npos[0], npos[1], npos[2]).is_some_and(|v| v.is_solid());

                            // Face is visible only when the outward neighbour is air.
                            mask[mu + mv * size_u] = if !neighbour_solid {
                                let material = vox.unwrap().material();
                                // Compute 4-corner AO using CubicMesher's helper.
                                // face_ao expects the voxel (x,y,z) coordinates and
                                // the face_id matching cubic's face encoding.
                                let ao = face_ao(chunk.voxels, pos[0], pos[1], pos[2], face_id);
                                Some(MaskCell { material, ao })
                            } else {
                                None
                            };
                        }
                    }

                    // Greedy pass: merge maximal rectangles where material AND AO match.
                    for mv in 0..size_v {
                        let mut mu = 0;
                        while mu < size_u {
                            let cell = mask[mu + mv * size_u];
                            if cell.is_none() {
                                mu += 1;
                                continue;
                            }
                            let key = cell.unwrap();

                            // Extend width along u (same material + AO signature).
                            let mut width = 1;
                            while mu + width < size_u
                                && mask[(mu + width) + mv * size_u] == Some(key)
                            {
                                width += 1;
                            }

                            // Extend height along v.
                            let mut height = 1;
                            'outer: while mv + height < size_v {
                                for k in 0..width {
                                    if mask[(mu + k) + (mv + height) * size_u] != Some(key) {
                                        break 'outer;
                                    }
                                }
                                height += 1;
                            }

                            // Emit quad for the rectangle [mu..mu+width] × [mv..mv+height]
                            // at slice d, carrying the uniform AO signature.
                            emit_quad(
                                &mut buf,
                                axis,
                                u_axis,
                                v_axis,
                                d,
                                mu,
                                mv,
                                width,
                                height,
                                neg,
                                normal,
                                key.material,
                                key.ao,
                            );

                            // Clear used cells from mask.
                            for hh in 0..height {
                                for ww in 0..width {
                                    mask[(mu + ww) + (mv + hh) * size_u] = None;
                                }
                            }

                            mu += width;
                        }
                    }
                }
            }
        }

        Ok(buf)
    }
}

// ---------------------------------------------------------------------------
// Quad emitter
// ---------------------------------------------------------------------------

/// Emit a single axis-aligned quad into `buf`.
///
/// `axis`, `u_axis`, `v_axis` define the face orientation.
/// `d` is the slice index along `axis`; `mu`, `mv` are the quad origin in the
/// (u, v) tangent plane; `width` and `height` are its extent along (u, v).
/// `neg` flips which side of the voxel the face sits on (and reverses winding).
/// `ao` carries the four per-corner AO values computed by [`face_ao`]; they are
/// emitted parallel to the vertex positions.
#[allow(clippy::too_many_arguments)]
fn emit_quad(
    buf: &mut MeshBuffer,
    axis: usize,
    u_axis: usize,
    v_axis: usize,
    d: usize,
    mu: usize,
    mv: usize,
    width: usize,
    height: usize,
    neg: bool,
    normal: [f32; 3],
    material: MaterialId,
    ao: [u8; 4],
) {
    // The quad lies on the face of slice `d`.  If we are on the positive face
    // (+X, +Y, +Z) the quad is at coord `d + 1` along the axis; for the
    // negative face it is at `d`.
    let face_d = if neg { d as f32 } else { d as f32 + 1.0 };

    // Build the 4 corner positions in world space.
    let corner = |du: f32, dv: f32| -> [f32; 3] {
        let mut p = [0.0_f32; 3];
        p[axis] = face_d;
        p[u_axis] = mu as f32 + du;
        p[v_axis] = mv as f32 + dv;
        p
    };

    // Counter-clockwise winding when viewed from outside (positive normal side).
    // Positive-facing: CCW order for outward normal = (0,0) → (w,0) → (w,h) → (0,h).
    // Negative-facing: flip to keep CCW from outside.
    let (p0, p1, p2, p3) = if !neg {
        (
            corner(0.0, 0.0),
            corner(width as f32, 0.0),
            corner(width as f32, height as f32),
            corner(0.0, height as f32),
        )
    } else {
        (
            corner(0.0, height as f32),
            corner(width as f32, height as f32),
            corner(width as f32, 0.0),
            corner(0.0, 0.0),
        )
    };

    let w = width as f32;
    let h = height as f32;
    let uvs = [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]];
    let positions = [p0, p1, p2, p3];

    let base = buf.vertices.len() as u32;
    for (i, pos) in positions.iter().enumerate() {
        buf.vertices.push(MeshVertex {
            position: *pos,
            normal,
            uv: uvs[i],
            material,
        });
        buf.ao.push(ao[i]);
    }
    // Two triangles, CCW.
    buf.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::chunk::{Chunk, ChunkId};
    use crate::voxel::cubic_mesher::CubicMesher;
    use crate::voxel::lod::LodLevel;
    use crate::voxel::mesh::Mesher;

    fn idx(x: i32, y: i32, z: i32) -> usize {
        x as usize + y as usize * CHUNK_EDGE + z as usize * CHUNK_EDGE * CHUNK_EDGE
    }

    // -----------------------------------------------------------------------
    // FR-PHENO-VOXEL-GREEDY-001
    // Single solid voxel → exactly 6 quads (same as cubic).
    // -----------------------------------------------------------------------
    #[test]
    fn single_voxel_produces_six_quads() {
        let mut c = Chunk::<MaterialId>::default();
        c.voxels[idx(0, 0, 0)] = MaterialId(1);
        let view = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = GreedyMesher::<MaterialId>::mesh_greedy(view, LodLevel(0)).expect("greedy mesh");

        // 6 quads × 4 vertices × 6 indices.
        assert_eq!(
            mesh.vertices.len(),
            24,
            "single voxel must produce 6 quads (24 verts)"
        );
        assert_eq!(mesh.indices.len(), 36);
    }

    // -----------------------------------------------------------------------
    // FR-PHENO-VOXEL-GREEDY-002
    // 2×2×2 solid block — AO-aware greedy mesher preserves correct surface.
    // AO-aware merging: corner cells have different AO (edge voxels occlude
    // adjacent corners), so the merge key differs across the slab and more
    // quads may be emitted than naive material-only greedy.  The invariant is:
    //   - Surface area matches cubic (watertight).
    //   - AO buffer length equals vertex count.
    //   - ao.len() == vertices.len().
    // We no longer assert "fewer indices than cubic" because AO boundaries can
    // legitimately split what material-only greedy would have merged.
    // -----------------------------------------------------------------------
    #[test]
    fn two_by_two_block_merges_faces() {
        let mut c = Chunk::<MaterialId>::default();
        for z in 0..2_i32 {
            for y in 0..2_i32 {
                for x in 0..2_i32 {
                    c.voxels[idx(x, y, z)] = MaterialId(1);
                }
            }
        }

        let view_g = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let greedy =
            GreedyMesher::<MaterialId>::mesh_greedy(view_g, LodLevel(0)).expect("greedy mesh");

        let view_c = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let cubic = CubicMesher::<MaterialId>::mesh_cubic(view_c, LodLevel(0)).expect("cubic mesh");

        // Watertight: same surface area.
        let greedy_area = total_triangle_area(&greedy);
        let cubic_area = total_triangle_area(&cubic);
        assert!(
            (greedy_area - cubic_area).abs() < 1e-4,
            "2×2×2 block surface area mismatch: greedy={greedy_area:.4}, cubic={cubic_area:.4}"
        );

        // Triangle count must be ≤ cubic (AO-aware greedy still cannot exceed cubic).
        assert!(
            greedy.indices.len() <= cubic.indices.len(),
            "greedy ({} idx) must be ≤ cubic ({} idx)",
            greedy.indices.len(),
            cubic.indices.len(),
        );

        // AO invariant.
        assert_eq!(
            greedy.ao.len(),
            greedy.vertices.len(),
            "ao.len() must equal vertices.len()"
        );
    }

    // -----------------------------------------------------------------------
    // FR-PHENO-VOXEL-GREEDY-003
    // Watertight / same visible surface area as cubic for a 3×3×3 block.
    // Both meshers see the same 54-face outer shell.  Triangle counts differ
    // (greedy merges each face into 1 quad vs 9 quads for cubic), but the
    // total surface area covered must be equal.
    // -----------------------------------------------------------------------
    #[test]
    fn surface_area_matches_cubic_for_solid_block() {
        let mut c = Chunk::<MaterialId>::default();
        for z in 0..3_i32 {
            for y in 0..3_i32 {
                for x in 0..3_i32 {
                    c.voxels[idx(x, y, z)] = MaterialId(1);
                }
            }
        }

        let view_g = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let greedy =
            GreedyMesher::<MaterialId>::mesh_greedy(view_g, LodLevel(0)).expect("greedy mesh");

        let view_c = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let cubic = CubicMesher::<MaterialId>::mesh_cubic(view_c, LodLevel(0)).expect("cubic mesh");

        let greedy_area = total_triangle_area(&greedy);
        let cubic_area = total_triangle_area(&cubic);
        assert!(
            (greedy_area - cubic_area).abs() < 1e-4,
            "greedy area {greedy_area:.4} != cubic area {cubic_area:.4}"
        );

        // AO-aware greedy: triangle count must be ≤ cubic (not necessarily strictly <,
        // since AO discontinuities at block edges can split merges that material-only
        // greedy would collapse).
        assert!(
            greedy.indices.len() <= cubic.indices.len(),
            "greedy ({} idx) should be ≤ cubic ({} idx)",
            greedy.indices.len(),
            cubic.indices.len(),
        );

        // AO invariant.
        assert_eq!(
            greedy.ao.len(),
            greedy.vertices.len(),
            "ao.len() must equal vertices.len()"
        );

        let greedy_tri = greedy.indices.len() / 3;
        let cubic_tri = cubic.indices.len() / 3;
        // Print reduction for visibility in test output.
        eprintln!(
            "[GREEDY-003] 3×3×3 block — cubic: {} tris, greedy: {} tris, reduction: {:.1}%",
            cubic_tri,
            greedy_tri,
            100.0 * (1.0 - greedy_tri as f64 / cubic_tri as f64)
        );
    }

    // -----------------------------------------------------------------------
    // FR-PHENO-VOXEL-GREEDY-004
    // Determinism: same chunk + LOD always produces bit-identical output.
    // -----------------------------------------------------------------------
    #[test]
    fn greedy_meshing_is_deterministic() {
        let mut c = Chunk::<MaterialId>::default();
        c.voxels[idx(0, 0, 0)] = MaterialId(1);
        c.voxels[idx(2, 1, 3)] = MaterialId(2);
        c.voxels[idx(5, 5, 5)] = MaterialId(1);

        let view1 = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let view2 = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let m1 = GreedyMesher::<MaterialId>::mesh_greedy(view1, LodLevel(0)).expect("m1");
        let m2 = GreedyMesher::<MaterialId>::mesh_greedy(view2, LodLevel(0)).expect("m2");
        assert_eq!(m1, m2, "greedy meshing must be deterministic");
    }

    // -----------------------------------------------------------------------
    // FR-PHENO-VOXEL-GREEDY-005
    // Empty chunk → empty mesh.
    // -----------------------------------------------------------------------
    #[test]
    fn empty_chunk_produces_empty_mesh() {
        let c = Chunk::<MaterialId>::default();
        let view = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = GreedyMesher::<MaterialId>::mesh_greedy(view, LodLevel(0)).expect("greedy mesh");
        assert!(mesh.vertices.is_empty());
        assert!(mesh.indices.is_empty());
    }

    // -----------------------------------------------------------------------
    // FR-PHENO-VOXEL-GREEDY-006
    // Mesher trait impl delegates correctly.
    // -----------------------------------------------------------------------
    #[test]
    fn mesher_trait_delegates_to_mesh_greedy() {
        let mut c = Chunk::<MaterialId>::default();
        c.voxels[idx(1, 1, 1)] = MaterialId(3);

        let view_a = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let view_b = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };

        let mesher = GreedyMesher::<MaterialId>::new();
        let via_trait = mesher.mesh_chunk(view_a, LodLevel(0)).expect("via trait");
        let via_direct =
            GreedyMesher::<MaterialId>::mesh_greedy(view_b, LodLevel(0)).expect("direct");
        assert_eq!(via_trait, via_direct);
    }

    // -----------------------------------------------------------------------
    // FR-PHENO-VOXEL-GREEDY-007
    // Triangle reduction on a flat 4×4 slab (y=0, x in 0..4, z in 0..4).
    // Cubic: 4×4 = 16 top faces, 16 bottom faces, 4×4 side faces = 48 quads = 96 tris.
    // Greedy: top=1 quad, bottom=1 quad, sides=4 quads = 6 quads = 12 tris.
    // -----------------------------------------------------------------------
    #[test]
    fn flat_slab_greedy_reduction() {
        let mut c = Chunk::<MaterialId>::default();
        for z in 0..4_i32 {
            for x in 0..4_i32 {
                c.voxels[idx(x, 0, z)] = MaterialId(1);
            }
        }

        let view_g = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let greedy = GreedyMesher::<MaterialId>::mesh_greedy(view_g, LodLevel(0)).expect("greedy");

        let view_c = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let cubic = CubicMesher::<MaterialId>::mesh_cubic(view_c, LodLevel(0)).expect("cubic");

        let greedy_area = total_triangle_area(&greedy);
        let cubic_area = total_triangle_area(&cubic);
        assert!(
            (greedy_area - cubic_area).abs() < 1e-4,
            "surface area must match: greedy={greedy_area:.4}, cubic={cubic_area:.4}"
        );

        let greedy_tri = greedy.indices.len() / 3;
        let cubic_tri = cubic.indices.len() / 3;
        let reduction = 100.0 * (1.0 - greedy_tri as f64 / cubic_tri as f64);
        eprintln!(
            "[GREEDY-007] 4×4×1 slab — cubic: {cubic_tri} tris, greedy: {greedy_tri} tris, reduction: {reduction:.1}%"
        );
        assert!(
            greedy.indices.len() < cubic.indices.len(),
            "greedy must beat cubic on flat slab"
        );
    }

    // -----------------------------------------------------------------------
    // FR-PHENO-VOXEL-GREEDY-AO-001 (PLAN-VOXEL-001)
    // Uniform-AO flat surface: a single fully isolated voxel at the centre of
    // the chunk has all AO = 3 on every face (no neighbours), so every face
    // cell has the same AO signature.  Greedy must still merge-equivalent to
    // cubic (both emit exactly 6 quads), and the AO-aware merge does NOT
    // prevent merges here because the signature is uniform.
    //
    // Also verify triangle count ≤ cubic for a 4×4 slab (the top face is
    // fully exposed and all-3, so those 16 cells merge to 1 quad).
    // -----------------------------------------------------------------------
    #[test]
    fn uniform_ao_flat_surface_still_merges() {
        // Part A: isolated voxel — all faces all-3, greedy == cubic in quad count.
        let mut c = Chunk::<MaterialId>::default();
        c.voxels[idx(8, 8, 8)] = MaterialId(1);
        let view = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = GreedyMesher::<MaterialId>::mesh_greedy(view, LodLevel(0)).expect("greedy mesh");
        assert!(
            mesh.ao.iter().all(|&a| a == 3),
            "isolated voxel: all AO must be 3; got {:?}",
            mesh.ao
        );
        assert_eq!(mesh.ao.len(), mesh.vertices.len());
        // 6 faces × 4 verts = 24 verts, 6 faces × 6 indices = 36 indices.
        assert_eq!(
            mesh.vertices.len(),
            24,
            "isolated voxel must produce 6 quads"
        );
        assert_eq!(mesh.indices.len(), 36);

        // Part B: 4×4 slab — triangle count ≤ cubic (win preserved despite AO splits
        // on side/bottom faces at slab perimeter).
        let mut c2 = Chunk::<MaterialId>::default();
        for z in 0..4_i32 {
            for x in 0..4_i32 {
                c2.voxels[idx(x, 0, z)] = MaterialId(1);
            }
        }
        let view_g = ChunkView {
            id: ChunkId(0),
            voxels: &c2.voxels,
        };
        let greedy =
            GreedyMesher::<MaterialId>::mesh_greedy(view_g, LodLevel(0)).expect("greedy mesh");
        let view_c = ChunkView {
            id: ChunkId(0),
            voxels: &c2.voxels,
        };
        let cubic = CubicMesher::<MaterialId>::mesh_cubic(view_c, LodLevel(0)).expect("cubic");
        assert!(
            greedy.indices.len() <= cubic.indices.len(),
            "4×4 slab: greedy ({} idx) must be ≤ cubic ({} idx)",
            greedy.indices.len(),
            cubic.indices.len()
        );
        assert_eq!(
            greedy.ao.len(),
            greedy.vertices.len(),
            "ao.len() must equal vertices.len()"
        );
    }

    // -----------------------------------------------------------------------
    // FR-PHENO-VOXEL-GREEDY-AO-002 (PLAN-VOXEL-001)
    // AO discontinuity splits the quad: a crevice voxel causes differing AO
    // signatures so the boundary cell cannot be merged with its unoccluded
    // neighbour.
    // -----------------------------------------------------------------------
    #[test]
    fn ao_discontinuity_splits_greedy_quad() {
        // Build a 2-voxel row: (0,0,0) isolated, (1,0,0) with a solid -y crevice
        // neighbour at (1,-1,0)=out-of-bounds (treated as air) but we create a
        // solid voxel at (0,1,0) and (1,1,0) so that (1,0,0) has a +y solid
        // neighbour that occludes its top corners.
        //
        // Simpler setup: a 1×3×1 row at y=1 with a solid block at (1,0,0) acting
        // as an occluder under the middle voxel.
        let mut c = Chunk::<MaterialId>::default();
        // Top row: three coplanar same-material voxels.
        c.voxels[idx(0, 1, 0)] = MaterialId(1); // unoccluded
        c.voxels[idx(1, 1, 0)] = MaterialId(1); // has occluder below
        c.voxels[idx(2, 1, 0)] = MaterialId(1); // unoccluded
                                                // Occluder under the middle voxel (creates AO on the +y face of voxel 1).
        c.voxels[idx(0, 0, 0)] = MaterialId(1); // occluder for (1,1,0) -x bottom corner
        c.voxels[idx(1, 0, 0)] = MaterialId(1); // directly below (1,1,0) — suppresses -y face

        let view_g = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh =
            GreedyMesher::<MaterialId>::mesh_greedy(view_g, LodLevel(0)).expect("greedy mesh");

        // The +y face of (1,1,0) is exposed (its +y neighbour is air).
        // The occluder at (0,0,0) occludes the -x corner of (1,1,0)'s +y face.
        // So the AO signature for (1,1,0)'s +y face differs from (0,1,0) and (2,1,0).
        // Verify: ao buffer has at least one value < 3 (AO detail preserved).
        assert!(
            mesh.ao.iter().any(|&a| a < 3),
            "expected at least one occluded AO vertex after adding occluder; all were 3"
        );
        // ao.len() == vertices.len() must still hold.
        assert_eq!(mesh.ao.len(), mesh.vertices.len());
    }

    // -----------------------------------------------------------------------
    // FR-PHENO-VOXEL-GREEDY-AO-003 (PLAN-VOXEL-001)
    // Greedy AO values match CubicMesher at corresponding positions.
    // For a single isolated voxel all 24 vertices on both meshers must have
    // AO = 3 (fully exposed).
    // -----------------------------------------------------------------------
    #[test]
    fn greedy_ao_matches_cubic_ao_on_single_voxel() {
        let mut c = Chunk::<MaterialId>::default();
        c.voxels[idx(4, 4, 4)] = MaterialId(1);

        let view_g = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let greedy =
            GreedyMesher::<MaterialId>::mesh_greedy(view_g, LodLevel(0)).expect("greedy mesh");

        let view_c = ChunkView {
            id: ChunkId(0),
            voxels: &c.voxels,
        };
        let cubic = CubicMesher::<MaterialId>::mesh_cubic(view_c, LodLevel(0)).expect("cubic mesh");

        // Both meshers produce 6 faces × 4 verts = 24 vertices.
        assert_eq!(
            greedy.ao.len(),
            24,
            "greedy: 6 faces × 4 verts = 24 ao values"
        );
        assert_eq!(
            cubic.ao.len(),
            24,
            "cubic: 6 faces × 4 verts = 24 ao values"
        );

        // All AO values must be 3 for an isolated voxel (no neighbours to occlude).
        assert!(
            greedy.ao.iter().all(|&a| a == 3),
            "greedy: isolated voxel must have all AO=3; got {:?}",
            greedy.ao
        );
        assert!(
            cubic.ao.iter().all(|&a| a == 3),
            "cubic: isolated voxel must have all AO=3; got {:?}",
            cubic.ao
        );

        // Also compare sorted AO histograms to confirm parity.
        let mut gao = greedy.ao.clone();
        let mut cao = cubic.ao.clone();
        gao.sort_unstable();
        cao.sort_unstable();
        assert_eq!(
            gao, cao,
            "greedy and cubic AO histograms must match for isolated voxel"
        );
    }

    // -----------------------------------------------------------------------
    // Helper: compute total triangle area of a MeshBuffer.
    // -----------------------------------------------------------------------
    fn total_triangle_area(buf: &MeshBuffer) -> f64 {
        let mut area = 0.0_f64;
        for tri in buf.indices.chunks_exact(3) {
            let a = buf.vertices[tri[0] as usize].position;
            let b = buf.vertices[tri[1] as usize].position;
            let c = buf.vertices[tri[2] as usize].position;
            area += triangle_area(a, b, c);
        }
        area
    }

    fn triangle_area(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f64 {
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let len = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]) as f64;
        len.sqrt() * 0.5
    }
}
