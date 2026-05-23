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

/// Engine-neutral reference cubic mesher.
#[derive(Debug, Clone, Copy, Default)]
pub struct CubicMesher;

impl Mesher for CubicMesher {
    type Mesh = MeshBuffer;

    fn mesh_chunk<T: Default + Clone>(
        &self,
        _chunk: ChunkView<'_, T>,
        _lod: LodLevel,
    ) -> MeshResult<Self::Mesh> {
        // The trait can't statically know that T: CubicVoxel, so use the
        // typed variant `mesh_cubic` below. We return BadChunkSize{got:0,expected:0}
        // to signal "wrong entry point" — callers should prefer `mesh_cubic`.
        Err(MeshError::BadChunkSize {
            got: 0,
            expected: 0,
        })
    }
}

impl CubicMesher {
    /// Typed entry point that requires the voxel value type to implement
    /// [`CubicVoxel`]. Use this instead of the trait method when meshing a
    /// concrete voxel world.
    ///
    /// LOD currently affects nothing for the cubic mesher (every level emits the
    /// same geometry). Future LOD-aware meshers will collapse far chunks into
    /// merged-face geometry.
    pub fn mesh_cubic<T: CubicVoxel>(
        chunk: ChunkView<'_, T>,
        _lod: LodLevel,
    ) -> MeshResult<MeshBuffer> {
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
                            emit_face(&mut buf, x, y, z, face, material);
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

/// Append the four vertices + two triangles for a single exposed face.
///
/// Winding is counter-clockwise when viewed from outside the voxel, which matches
/// Bevy / Godot / Unreal default front-face conventions.
fn emit_face(buf: &mut MeshBuffer, x: i32, y: i32, z: i32, face: u8, material: MaterialId) {
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

    let base = buf.vertices.len() as u32;
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for (i, p) in verts.iter().enumerate() {
        buf.vertices.push(MeshVertex {
            position: *p,
            normal,
            uv: uvs[i],
            material,
        });
    }
    // Two triangles per quad, CCW from outside.
    buf.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::Chunk;

    fn single_voxel_chunk_at_origin() -> Chunk<MaterialId> {
        let mut c = Chunk::<MaterialId>::default();
        c.voxels[idx(0, 0, 0)] = MaterialId(1);
        c
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
        let mesh = CubicMesher::mesh_cubic(view, LodLevel(0)).expect("mesh");
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
        let mesh = CubicMesher::mesh_cubic(view, LodLevel(0)).expect("mesh");
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
        let mesh = CubicMesher::mesh_cubic(view, LodLevel(0)).expect("mesh");
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
        let res = CubicMesher::mesh_cubic(view, LodLevel(0));
        assert!(matches!(res, Err(MeshError::BadChunkSize { .. })));
    }
}
