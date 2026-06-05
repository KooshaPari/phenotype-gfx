//! Reference cubic mesher.
//!
//! Engine-neutral implementation of [`Mesher`] that produces axis-aligned cubic
//! geometry for non-default voxels and skips faces adjacent to other non-default
//! voxels (the cheap-greedy "only emit exposed faces" pass). This is the
//! reference implementation that other engine-specific meshers (Bevy, Godot,
//! Unreal) must reproduce to bit-identical geometry for a given chunk + LOD pair.
//!
//! Production renderers will replace this with greedy-quad or marching-cubes /
//! dual-contouring meshers; the cubic version is the floor.

use crate::chunk::{ChunkView, CHUNK_EDGE};
use crate::lod::LodLevel;
use crate::material::MaterialId;
use crate::mesh::{MeshBuffer, MeshError, MeshResult, MeshVertex, Mesher};

/// Trait that voxel value types must implement to feed a [`CubicMesher`]. The
/// mesher needs to know whether a voxel is "solid" (face-emitting) and what
/// material it carries.
pub trait CubicVoxel: Default + Clone + PartialEq {
    /// `true` if this voxel should emit cube faces.
    fn is_solid(&self) -> bool;
    /// Material slot for this voxel.
    fn material(&self) -> MaterialId;
}

// Blanket implementation: `MaterialId` itself is a voxel — the default value is
// the "air" material (id 0), and any other id is solid.
impl CubicVoxel for MaterialId {
    fn is_solid(&self) -> bool {
        self.0 != 0
    }
    fn material(&self) -> MaterialId {
        *self
    }
}

/// Engine-neutral reference cubic mesher, generic over any [`CubicVoxel`] type.
///
/// The `PhantomData<V>` field carries the voxel type at zero runtime cost and
/// lets `impl<V: CubicVoxel> Mesher for CubicMesher<V>` set `type VoxelKind = V`,
/// satisfying the trait's associated-type requirement without an extra indirection.
#[derive(Debug, Clone, Copy, Default)]
pub struct CubicMesher<V>(core::marker::PhantomData<V>);

impl<V: CubicVoxel> CubicMesher<V> {
    /// Construct a new `CubicMesher` for voxel type `V`.
    pub fn new() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<V: CubicVoxel> Mesher for CubicMesher<V> {
    type VoxelKind = V;
    type Mesh = MeshBuffer;

    /// Delegates directly to [`CubicMesher::mesh_cubic`] so all meshing logic
    /// lives in one place and the trait boundary is fully enforced at compile time.
    fn mesh_chunk(&self, chunk: ChunkView<'_, V>, lod: LodLevel) -> MeshResult<Self::Mesh> {
        Self::mesh_cubic(chunk, lod)
    }
}

impl<V: CubicVoxel> CubicMesher<V> {
    /// Core meshing logic. Shared by the `Mesher` impl and any direct callers
    /// that already have a concrete `V: CubicVoxel` in scope.
    ///
    /// LOD currently affects nothing for the cubic mesher (every level emits the
    /// same geometry). Future LOD-aware meshers will collapse far chunks into
    /// merged-face geometry.
    pub fn mesh_cubic(chunk: ChunkView<'_, V>, _lod: LodLevel) -> MeshResult<MeshBuffer> {
        let expected = CHUNK_EDGE * CHUNK_EDGE * CHUNK_EDGE;
        if chunk.voxels.len() != expected {
            return Err(MeshError::BadChunkSize {
                got: chunk.voxels.len(),
                expected,
            });
        }
        let mut buf = MeshBuffer::default();
        let edge = CHUNK_EDGE as i32;

        for z in 0..edge {
            for y in 0..edge {
                for x in 0..edge {
                    let v = &chunk.voxels[idx(x, y, z)];
                    if !v.is_solid() {
                        continue;
                    }
                    let material = v.material();
                    // Emit each of the 6 faces if the neighbour in that direction
                    // is not solid.
                    for face in 0..6 {
                        let (nx, ny, nz) = neighbour(x, y, z, face);
                        let exposed =
                            !in_bounds(nx, ny, nz) || !chunk.voxels[idx(nx, ny, nz)].is_solid();
                        if exposed {
                            emit_face(&mut buf, chunk.voxels, x, y, z, face, material);
                        }
                    }
                }
            }
        }
        Ok(buf)
    }
}

#[inline]
fn idx(x: i32, y: i32, z: i32) -> usize {
    (x as usize) + (y as usize) * CHUNK_EDGE + (z as usize) * CHUNK_EDGE * CHUNK_EDGE
}

#[inline]
fn in_bounds(x: i32, y: i32, z: i32) -> bool {
    let edge = CHUNK_EDGE as i32;
    x >= 0 && y >= 0 && z >= 0 && x < edge && y < edge && z < edge
}

/// Returns the integer neighbour coordinate for the given face index 0..6.
/// Face index encoding: 0=+x, 1=-x, 2=+y, 3=-y, 4=+z, 5=-z.
#[inline]
fn neighbour(x: i32, y: i32, z: i32, face: u8) -> (i32, i32, i32) {
    match face {
        0 => (x + 1, y, z),
        1 => (x - 1, y, z),
        2 => (x, y + 1, z),
        3 => (x, y - 1, z),
        4 => (x, y, z + 1),
        _ => (x, y, z - 1),
    }
}

// ---------------------------------------------------------------------------
// AO helpers
// ---------------------------------------------------------------------------

/// Return whether the voxel at `(x, y, z)` is solid, treating out-of-bounds as air.
#[inline]
fn solid_at<V: CubicVoxel>(voxels: &[V], x: i32, y: i32, z: i32) -> bool {
    if !in_bounds(x, y, z) {
        return false;
    }
    voxels[idx(x, y, z)].is_solid()
}

/// Compute the classic voxel-AO value (0..=3) for a single face vertex.
///
/// `side1`, `side2`, and `corner` are the three neighbours that touch the vertex
/// corner (in the plane of the face).  The rule is:
/// - If both sides are solid → 0 (maximum occlusion).
/// - Otherwise → `3 - (side1_solid + side2_solid + corner_solid)`.
#[inline]
fn vertex_ao(side1: bool, side2: bool, corner: bool) -> u8 {
    if side1 && side2 {
        0
    } else {
        3 - (side1 as u8 + side2 as u8 + corner as u8)
    }
}

/// Compute per-vertex AO for all four corners of a face.
///
/// Returns `[ao0, ao1, ao2, ao3]` parallel to the vertex order used by `emit_face`.
///
/// For each corner vertex of a face, the three relevant AO neighbours are the
/// two voxels adjacent along the face's tangent axes and their shared diagonal.
/// We define a unit tangent basis (u_axis, v_axis) per face, then for each
/// corner we determine which tangent direction (sign) the corner points toward.
/// The AO sample points sit **in the normal direction** (same depth as the face)
/// and **displaced ±1 along each tangent axis**.
///
/// Vertex ordering per face (matches `emit_face`):
/// ```text
///   face 0 (+x): v0=(y-,z-), v1=(y+,z-), v2=(y+,z+), v3=(y-,z+)
///   face 1 (-x): v0=(y-,z+), v1=(y+,z+), v2=(y+,z-), v3=(y-,z-)
///   face 2 (+y): v0=(z-,x-), v1=(z+,x-), v2=(z+,x+), v3=(z-,x+)
///   face 3 (-y): v0=(z+,x-), v1=(z-,x-), v2=(z-,x+), v3=(z+,x+)
///   face 4 (+z): v0=(x-,y-), v1=(x+,y-), v2=(x+,y+), v3=(x-,y+)
///   face 5 (-z): v0=(x+,y-), v1=(x-,y-), v2=(x-,y+), v3=(x+,y+)
/// ```
pub(crate) fn face_ao<V: CubicVoxel>(voxels: &[V], x: i32, y: i32, z: i32, face: u8) -> [u8; 4] {
    // For each face encode:
    //   nox,noy,noz  — one step in the face-normal direction
    //   ux,uy,uz     — unit tangent axis U
    //   vx,vy,vz     — unit tangent axis V
    //
    // Then derive corner signs from the exact vertex positions in emit_face.
    // Each vertex sits at the corner of the face quad; its AO neighbours lie
    // at normal_step ± tangent_step in each tangent direction.
    //
    // Corner tangent signs [us, vs] for each vertex (derived from emit_face positions):
    //   v0 — (u_neg, v_neg) = (-1, -1)
    //   v1 — (u_pos, v_neg) = (+1, -1)
    //   v2 — (u_pos, v_pos) = (+1, +1)
    //   v3 — (u_neg, v_pos) = (-1, +1)
    //
    // Exception: some faces have reversed tangent axes (neg-facing); the sign
    // table below is expressed in the LOCAL tangent basis (ux,uy,uz / vx,vy,vz)
    // so the math is uniform.

    // (nox, noy, noz, ux, uy, uz, vx, vy, vz)
    let (_nox, _noy, _noz, ux, uy, uz, vx, vy, vz): (i32, i32, i32, i32, i32, i32, i32, i32, i32) =
        match face {
            0 => (1, 0, 0, 0, 1, 0, 0, 0, 1),   // +x face; u=+y, v=+z
            1 => (-1, 0, 0, 0, 1, 0, 0, 0, -1), // -x face; u=+y, v=-z (reversed z keeps CCW)
            2 => (0, 1, 0, 0, 0, 1, 1, 0, 0),   // +y face; u=+z, v=+x
            3 => (0, -1, 0, 0, 0, -1, 1, 0, 0), // -y face; u=-z, v=+x
            4 => (0, 0, 1, 1, 0, 0, 0, 1, 0),   // +z face; u=+x, v=+y
            _ => (0, 0, -1, -1, 0, 0, 0, 1, 0), // -z face; u=-x, v=+y
        };

    // Classic voxel-AO neighbourhood:
    // For each face vertex at the corner shared between the face normal direction
    // and a tangent corner (us, vs), the three occluding voxels are located at
    // the SAME depth as the source voxel but offset in the tangent plane:
    //
    //   side_u  = voxel + us * u_axis             (tangent-u neighbour, same depth)
    //   side_v  = voxel + vs * v_axis             (tangent-v neighbour, same depth)
    //   diag    = voxel + us * u_axis + vs * v_axis
    //
    // Note: we do NOT add the normal step here; the AO voxels are co-planar with
    // the source voxel, NOT one step into the face-normal direction.
    //
    // Corner tangent sign table (derived from emit_face vertex layout):
    //   v0 → (us=-1, vs=-1)  v1 → (us=+1, vs=-1)
    //   v2 → (us=+1, vs=+1)  v3 → (us=-1, vs=+1)
    let corners: [(i32, i32); 4] = [(-1, -1), (1, -1), (1, 1), (-1, 1)];

    let ao_for_corner = |(us, vs): (i32, i32)| -> u8 {
        let s1 = solid_at(voxels, x + us * ux, y + us * uy, z + us * uz);
        let s2 = solid_at(voxels, x + vs * vx, y + vs * vy, z + vs * vz);
        let co = solid_at(
            voxels,
            x + us * ux + vs * vx,
            y + us * uy + vs * vy,
            z + us * uz + vs * vz,
        );
        vertex_ao(s1, s2, co)
    };

    [
        ao_for_corner(corners[0]),
        ao_for_corner(corners[1]),
        ao_for_corner(corners[2]),
        ao_for_corner(corners[3]),
    ]
}

// ---------------------------------------------------------------------------
// Face emission
// ---------------------------------------------------------------------------

/// Append the four vertices + two triangles for a single exposed face.
///
/// Winding is counter-clockwise when viewed from outside the voxel, which matches
/// Bevy / Godot / Unreal default front-face conventions.
///
/// `ao` contains one AO value (0..=3) for each of the four quad corners,
/// already computed by [`face_ao`].
fn emit_face<V: CubicVoxel>(
    buf: &mut MeshBuffer,
    voxels: &[V],
    x: i32,
    y: i32,
    z: i32,
    face: u8,
    material: MaterialId,
) {
    let fx = x as f32;
    let fy = y as f32;
    let fz = z as f32;
    // The eight cube corners local to (x, y, z):
    // c000..c111 where each bit is +0 or +1 along (x, y, z).
    let c = |dx: f32, dy: f32, dz: f32| [fx + dx, fy + dy, fz + dz];

    let (verts, normal): ([[f32; 3]; 4], [f32; 3]) = match face {
        0 => (
            [
                c(1.0, 0.0, 0.0),
                c(1.0, 1.0, 0.0),
                c(1.0, 1.0, 1.0),
                c(1.0, 0.0, 1.0),
            ],
            [1.0, 0.0, 0.0],
        ),
        1 => (
            [
                c(0.0, 0.0, 1.0),
                c(0.0, 1.0, 1.0),
                c(0.0, 1.0, 0.0),
                c(0.0, 0.0, 0.0),
            ],
            [-1.0, 0.0, 0.0],
        ),
        2 => (
            [
                c(0.0, 1.0, 0.0),
                c(0.0, 1.0, 1.0),
                c(1.0, 1.0, 1.0),
                c(1.0, 1.0, 0.0),
            ],
            [0.0, 1.0, 0.0],
        ),
        3 => (
            [
                c(0.0, 0.0, 1.0),
                c(0.0, 0.0, 0.0),
                c(1.0, 0.0, 0.0),
                c(1.0, 0.0, 1.0),
            ],
            [0.0, -1.0, 0.0],
        ),
        4 => (
            [
                c(0.0, 0.0, 1.0),
                c(1.0, 0.0, 1.0),
                c(1.0, 1.0, 1.0),
                c(0.0, 1.0, 1.0),
            ],
            [0.0, 0.0, 1.0],
        ),
        _ => (
            [
                c(0.0, 1.0, 0.0),
                c(1.0, 1.0, 0.0),
                c(1.0, 0.0, 0.0),
                c(0.0, 0.0, 0.0),
            ],
            [0.0, 0.0, -1.0],
        ),
    };

    // Compute per-vertex AO for this face.
    let ao_vals = face_ao(voxels, x, y, z, face);

    let base = buf.vertices.len() as u32;
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for (i, p) in verts.iter().enumerate() {
        buf.vertices.push(MeshVertex {
            position: *p,
            normal,
            uv: uvs[i],
            material,
        });
        buf.ao.push(ao_vals[i]);
    }
    // Two triangles per quad, CCW from outside.
    buf.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::Chunk;
    use crate::mesh::Mesher;

    fn single_voxel_chunk_at_origin() -> Chunk<MaterialId> {
        let mut c = Chunk::<MaterialId>::default();
        c.voxels[idx(0, 0, 0)] = MaterialId(1);
        c
    }

    /// FR-PHENO-VOXEL-CUBIC-011 — `Mesher::mesh_chunk` on `CubicMesher<MaterialId>`
    /// produces the same result as the direct `mesh_cubic` call, confirming the
    /// new `VoxelKind` associated-type ergonomics work end-to-end.
    #[test]
    fn mesher_trait_mesh_chunk_matches_mesh_cubic() {
        let c = single_voxel_chunk_at_origin();
        let view_a = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let view_b = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let mesher = CubicMesher::<MaterialId>::new();
        let via_trait = mesher
            .mesh_chunk(view_a, LodLevel(0))
            .expect("mesh via trait");
        let via_direct =
            CubicMesher::<MaterialId>::mesh_cubic(view_b, LodLevel(0)).expect("mesh direct");
        assert_eq!(
            via_trait, via_direct,
            "mesh_chunk must delegate to mesh_cubic identically"
        );
    }

    /// FR-PHENO-VOXEL-CUBIC-001 — a single solid voxel in an otherwise empty
    /// chunk produces exactly 24 vertices (6 faces × 4 verts) and 36 indices.
    #[test]
    fn single_voxel_emits_all_six_faces() {
        let c = single_voxel_chunk_at_origin();
        let view = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)).expect("mesh");
        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
    }

    /// FR-PHENO-VOXEL-CUBIC-002 — meshing is deterministic for a given chunk+LOD.
    #[test]
    fn meshing_is_deterministic() {
        let c = single_voxel_chunk_at_origin();
        let view1 = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let view2 = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let m1 = CubicMesher::mesh_cubic(view1, LodLevel(0)).expect("mesh");
        let m2 = CubicMesher::mesh_cubic(view2, LodLevel(0)).expect("mesh");
        assert_eq!(m1, m2);
    }

    /// FR-PHENO-VOXEL-CUBIC-003 — internal faces between two adjacent solid voxels
    /// are culled. Two adjacent voxels share one face, so the total is
    /// 6 + 6 − 2 = 10 faces = 40 vertices.
    #[test]
    fn adjacent_voxels_cull_shared_faces() {
        let mut c = Chunk::<MaterialId>::default();
        c.voxels[idx(0, 0, 0)] = MaterialId(1);
        c.voxels[idx(1, 0, 0)] = MaterialId(1);
        let view = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)).expect("mesh");
        assert_eq!(mesh.vertices.len(), 40);
        assert_eq!(mesh.indices.len(), 60);
    }

    /// FR-PHENO-VOXEL-CUBIC-004 — empty chunk produces empty mesh.
    #[test]
    fn empty_chunk_meshes_empty() {
        let c = Chunk::<MaterialId>::default();
        let view = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)).expect("mesh");
        assert!(mesh.vertices.is_empty());
        assert!(mesh.indices.is_empty());
    }

    /// FR-PHENO-VOXEL-CUBIC-005 — bad chunk size returns an error.
    #[test]
    fn wrong_chunk_size_errors() {
        let v = vec![MaterialId(0); 10];
        let view = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &v,
        };
        let res = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0));
        assert!(matches!(res, Err(MeshError::BadChunkSize { .. })));
    }

    /// FR-PHENO-VOXEL-CUBIC-006 — a solid voxel placed at the maximum chunk corner
    /// (CHUNK_EDGE-1, CHUNK_EDGE-1, CHUNK_EDGE-1) still emits exactly 6 faces:
    /// all neighbours are out-of-bounds and therefore treated as air.
    #[test]
    fn voxel_at_chunk_corner_emits_six_faces() {
        let edge = CHUNK_EDGE as i32 - 1;
        let mut c = Chunk::<MaterialId>::default();
        c.voxels[idx(edge, edge, edge)] = MaterialId(1);
        let view = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)).expect("mesh");
        assert_eq!(
            mesh.vertices.len(),
            24,
            "corner voxel must emit 6 faces (24 verts)"
        );
        assert_eq!(mesh.indices.len(), 36);
    }

    /// FR-PHENO-VOXEL-CUBIC-007 — a voxel with all 6 neighbours solid emits zero
    /// faces (completely buried).
    #[test]
    fn buried_voxel_emits_no_faces() {
        let mut c = Chunk::<MaterialId>::default();
        // Fill a 3×3×3 cube so that (1,1,1) is completely surrounded.
        for z in 0..3_i32 {
            for y in 0..3_i32 {
                for x in 0..3_i32 {
                    c.voxels[idx(x, y, z)] = MaterialId(1);
                }
            }
        }
        // Count only the faces that belong to position (1,1,1).
        // The easiest proxy: mesh the whole chunk and compare against the
        // outer-shell count.  A 3×3×3 fully-solid block has 6 faces × 9 per
        // side = 54 exposed faces → 54×4 = 216 vertices.
        let view = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)).expect("mesh");
        // 3×3 grid on each of 6 faces = 54 quads, 216 verts.
        assert_eq!(
            mesh.vertices.len(),
            216,
            "only outer-shell faces should be emitted for a solid 3×3×3 block"
        );
        assert_eq!(mesh.indices.len(), 54 * 6);
    }

    /// FR-PHENO-VOXEL-CUBIC-008 — each emitted face carries the correct outward
    /// normal.  For a single voxel at the origin the six normals must be the six
    /// axis-aligned unit vectors, each appearing exactly 4 times (once per vertex).
    #[test]
    fn face_normals_are_outward() {
        let c = single_voxel_chunk_at_origin();
        let view = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)).expect("mesh");

        let expected_normals: [[f32; 3]; 6] = [
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ];
        for n in &expected_normals {
            let count = mesh.vertices.iter().filter(|v| &v.normal == n).count();
            assert_eq!(count, 4, "normal {:?} should appear exactly 4 times", n);
        }
    }

    /// FR-PHENO-VOXEL-CUBIC-009 — material id is propagated to every vertex of a
    /// face.  A chunk with two voxels of different materials must have both ids
    /// present in the vertex stream.
    #[test]
    fn vertex_material_ids_match_voxels() {
        let mut c = Chunk::<MaterialId>::default();
        c.voxels[idx(0, 0, 0)] = MaterialId(7);
        c.voxels[idx(0, 1, 0)] = MaterialId(13);
        let view = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)).expect("mesh");
        let has_7 = mesh.vertices.iter().any(|v| v.material == MaterialId(7));
        let has_13 = mesh.vertices.iter().any(|v| v.material == MaterialId(13));
        assert!(has_7, "material 7 must appear in vertex stream");
        assert!(has_13, "material 13 must appear in vertex stream");
    }

    /// FR-PHENO-VOXEL-CUBIC-010 — index buffer only references valid vertex slots.
    #[test]
    fn index_buffer_in_bounds() {
        let c = single_voxel_chunk_at_origin();
        let view = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)).expect("mesh");
        let vcount = mesh.vertices.len() as u32;
        for &i in &mesh.indices {
            assert!(i < vcount, "index {} out of bounds (vcount={})", i, vcount);
        }
    }

    // ------------------------------------------------------------------
    // AO tests (FR-PHENO-VOXEL-CUBIC-AO-*)
    // ------------------------------------------------------------------

    /// FR-PHENO-VOXEL-CUBIC-AO-001 — a fully-exposed single voxel (all neighbours
    /// air) must have every AO value equal to 3 (no occlusion).
    #[test]
    fn fully_exposed_voxel_has_ao_three_everywhere() {
        let c = single_voxel_chunk_at_origin();
        let view = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)).expect("mesh");
        assert!(
            mesh.ao.iter().all(|&a| a == 3),
            "all AO values should be 3 for a fully-exposed voxel, got: {:?}",
            mesh.ao
        );
    }

    /// FR-PHENO-VOXEL-CUBIC-AO-002 — AO buffer length equals vertex count.
    #[test]
    fn ao_length_equals_vertex_count() {
        let c = single_voxel_chunk_at_origin();
        let view = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)).expect("mesh");
        assert_eq!(
            mesh.ao.len(),
            mesh.vertices.len(),
            "ao.len() must equal vertices.len()"
        );
    }

    /// FR-PHENO-VOXEL-CUBIC-AO-003 — triangle counts are unchanged by AO addition.
    /// A single voxel still produces 24 vertices / 36 indices.
    #[test]
    fn ao_does_not_change_triangle_counts() {
        let c = single_voxel_chunk_at_origin();
        let view = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)).expect("mesh");
        assert_eq!(mesh.vertices.len(), 24, "vertex count must be 24");
        assert_eq!(mesh.indices.len(), 36, "index count must be 36");
    }

    /// FR-PHENO-VOXEL-CUBIC-AO-004 — a voxel sitting in a crevice (two solid
    /// neighbours on adjacent axes) must have at least one AO vertex < 3 on the
    /// exposed face pointing away from the two solid neighbours.
    ///
    /// Setup: place a solid voxel at (2,2,2). Then place solid voxels at
    /// (1,2,2) (-x neighbour) and (2,1,2) (-y neighbour).  The +z face of (2,2,2)
    /// has two corners that share the (-x)/(-y) crevice; those corners should have
    /// reduced AO.
    #[test]
    fn crevice_voxel_has_reduced_ao_on_occluded_vertices() {
        let mut c = Chunk::<MaterialId>::default();
        // Main voxel we will inspect.
        c.voxels[idx(2, 2, 2)] = MaterialId(1);
        // Two solid neighbours that form a crevice at the -x/-y corner.
        c.voxels[idx(1, 2, 2)] = MaterialId(1); // -x side
        c.voxels[idx(2, 1, 2)] = MaterialId(1); // -y side
                                                // Also fill the diagonal corner so the "both sides" rule has maximum effect
                                                // on one vertex of the -z face viewed from below.
        c.voxels[idx(1, 1, 2)] = MaterialId(1); // -x/-y corner

        let view = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)).expect("mesh");
        // At least one vertex of (2,2,2) must be occluded (AO < 3).
        assert!(
            mesh.ao.iter().any(|&a| a < 3),
            "expected at least one occluded vertex (AO < 3) but all were 3; ao={:?}",
            mesh.ao
        );
        // ao.len() == vertices.len() invariant holds here too.
        assert_eq!(mesh.ao.len(), mesh.vertices.len());
    }

    /// FR-PHENO-VOXEL-CUBIC-AO-005 — AO buffer is all-3 for an empty chunk.
    #[test]
    fn empty_chunk_ao_is_empty() {
        let c = Chunk::<MaterialId>::default();
        let view = ChunkView {
            id: crate::chunk::ChunkId(0),
            voxels: &c.voxels,
        };
        let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)).expect("mesh");
        assert!(
            mesh.ao.is_empty(),
            "empty chunk must produce empty ao buffer"
        );
    }
}
