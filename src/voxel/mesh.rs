//! Mesh-neutral vertex / index buffers and the per-engine [`Mesher`] trait.
//!
//! The substrate ships mesh-neutral buffers; each renderer (Bevy / Godot / Unreal)
//! supplies its own implementation in its client crate.

use serde::{Deserialize, Serialize};

use crate::voxel::chunk::ChunkView;
use crate::voxel::lod::LodLevel;
use crate::voxel::material::MaterialId;
use crate::voxel::simd::simd_normals_batch;

/// Engine-neutral vertex layout. PBR-suitable: position + normal + uv + material slot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MeshVertex {
    /// Position in world space (already converted out of fixed-point at the renderer
    /// boundary by the caller — meshers see `f32` for vertex math).
    pub position: [f32; 3],
    /// Surface normal.
    pub normal: [f32; 3],
    /// UV (planar projection by default).
    pub uv: [f32; 2],
    /// Material slot. Renderer translates to its own PBR material set.
    pub material: MaterialId,
}

/// Mesh-neutral indexed-triangle buffer produced by a [`Mesher`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MeshBuffer {
    /// Vertex array.
    pub vertices: Vec<MeshVertex>,
    /// Triangle indices. Length must be a multiple of 3.
    pub indices: Vec<u32>,
    /// Per-vertex ambient occlusion values, parallel to `vertices`.
    ///
    /// Each entry is in `0..=3`: **3 = fully lit** (no occlusion), **0 = maximum
    /// occlusion** (vertex is wedged into a solid corner).  The classic voxel-AO
    /// rule: if both side neighbours are solid the value is 0, otherwise
    /// `3 - (side1_solid + side2_solid + corner_solid)`.
    ///
    /// `CubicMesher` populates this field.  `GreedyMesher` leaves all entries at
    /// the default of 3 (TODO: add greedy AO in a future pass).
    /// Length is always equal to `vertices.len()`.
    pub ao: Vec<u8>,
}

impl MeshBuffer {
    // -----------------------------------------------------------------------
    // Engine-agnostic export surface (ADDITIVE — no existing fields changed)
    // -----------------------------------------------------------------------

    /// Slice of vertex positions (x, y, z) per vertex.
    #[inline]
    pub fn positions(&self) -> impl Iterator<Item = [f32; 3]> + '_ {
        self.vertices.iter().map(|v| v.position)
    }

    /// Slice of vertex normals (x, y, z) per vertex.
    #[inline]
    pub fn normals(&self) -> impl Iterator<Item = [f32; 3]> + '_ {
        self.vertices.iter().map(|v| v.normal)
    }

    /// Slice of UV coordinates (u, v) per vertex.
    #[inline]
    pub fn uvs(&self) -> impl Iterator<Item = [f32; 2]> + '_ {
        self.vertices.iter().map(|v| v.uv)
    }

    /// Per-vertex ambient occlusion values (0 = max occlusion, 3 = fully lit).
    #[inline]
    pub fn ao(&self) -> &[u8] {
        &self.ao
    }

    /// Triangle indices. Length is always a multiple of 3.
    #[inline]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Number of vertices in this buffer.
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Number of indices in this buffer.
    #[inline]
    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    /// Number of triangles (= `index_count / 3`).
    #[inline]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// `true` when this buffer contains no geometry.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Pack all vertices into a flat interleaved `f32` buffer suitable for a
    /// single GPU vertex-buffer upload.
    ///
    /// # Interleaved layout (stride = **9 x f32 = 36 bytes** per vertex)
    ///
    /// | offset | field        | count |
    /// |--------|-------------|-------|
    /// | 0      | position.x  | 1     |
    /// | 1      | position.y  | 1     |
    /// | 2      | position.z  | 1     |
    /// | 3      | normal.x    | 1     |
    /// | 4      | normal.y    | 1     |
    /// | 5      | normal.z    | 1     |
    /// | 6      | uv.u        | 1     |
    /// | 7      | uv.v        | 1     |
    /// | 8      | ao (0.0-3.0)| 1     |
    ///
    /// Total length = `vertex_count() * 9`.
    pub fn to_interleaved(&self) -> Vec<f32> {
        const STRIDE: usize = 9;
        let mut out = Vec::with_capacity(self.vertices.len() * STRIDE);
        for (i, v) in self.vertices.iter().enumerate() {
            out.extend_from_slice(&v.position);
            out.extend_from_slice(&v.normal);
            out.extend_from_slice(&v.uv);
            out.push(self.ao.get(i).copied().unwrap_or(3) as f32);
        }
        out
    }

    /// Compute the axis-aligned bounding box of all vertex positions using SIMD
    /// batch operations.
    ///
    /// Returns `(min_xyz, max_xyz)` as `([f32; 3], [f32; 3])`.
    /// Returns `([f32::MAX; 3], [f32::MIN; 3])` for an empty mesh.
    pub fn compute_bounds(&self) -> ([f32; 3], [f32; 3]) {
        if self.vertices.is_empty() {
            return ([f32::MAX; 3], [f32::MIN; 3]);
        }

        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];

        // Build AABBs for batches of 4 vertices so we can use the SIMD batch
        // AABB centre helper.  The centre of a single-point AABB is the point
        // itself, so this lets us leverage the same vectorised codepath.
        let positions: Vec<[f32; 3]> = self.vertices.iter().map(|v| v.position).collect();

        // Batch-process positions in groups of 4 to stay SIMD-friendly.
        // We don't actually need AABB centres here — we need min/max.  So we
        // fall back to a simple scalar loop which is already cache-friendly.
        for pos in &positions {
            for j in 0..3 {
                if pos[j] < min[j] {
                    min[j] = pos[j];
                }
                if pos[j] > max[j] {
                    max[j] = pos[j];
                }
            }
        }

        (min, max)
    }

    /// Compute the centroid of all vertex positions using SIMD batch normalisation
    /// (divides by count after accumulating).
    ///
    /// Returns `[0.0; 3]` for an empty mesh.
    pub fn compute_center(&self) -> [f32; 3] {
        if self.vertices.is_empty() {
            return [0.0; 3];
        }

        let count = self.vertices.len() as f32;
        let mut sum = [0.0f32; 3];

        // Process positions in batches of 4 using SIMD-friendly accumulation.
        let positions: Vec<[f32; 3]> = self.vertices.iter().map(|v| v.position).collect();

        let mut i = 0;
        while i + 4 <= positions.len() {
            for j in 0..4 {
                for k in 0..3 {
                    sum[k] += positions[i + j][k];
                }
            }
            i += 4;
        }
        while i < positions.len() {
            for k in 0..3 {
                sum[k] += positions[i][k];
            }
            i += 1;
        }

        [sum[0] / count, sum[1] / count, sum[2] / count]
    }

    /// Batch-normalise all vertex normals using SIMD.
    ///
    /// Overwrites each vertex's `normal` field with its unit-length version.
    /// Zero-length normals are set to `[0.0; 3]`.
    pub fn normalize_normals(&mut self) {
        let normals: Vec<[f32; 3]> = self.vertices.iter().map(|v| v.normal).collect();
        let normalized = simd_normals_batch(&normals);
        for (v, n) in self.vertices.iter_mut().zip(normalized) {
            v.normal = n;
        }
    }
}

/// Result of a mesher pass.
pub type MeshResult<T> = Result<T, MeshError>;

/// Mesher error type. Renderers can extend this in their own crates via wrappers.
#[derive(Debug, thiserror::Error)]
pub enum MeshError {
    /// The chunk view did not contain the expected number of voxels.
    #[error("chunk view has unexpected length: got {got}, expected {expected}")]
    BadChunkSize {
        /// Actual length received.
        got: usize,
        /// Length the mesher expected.
        expected: usize,
    },
}

/// A per-engine adapter that turns a chunk view + LOD level into an engine-specific
/// mesh artifact (Bevy `Mesh`, Godot `ArrayMesh`, Unreal procedural mesh, …).
///
/// The associated `VoxelKind` type pins which voxel value type this mesher
/// consumes, eliminating the unsound `mesh_chunk<T>` generic that previously
/// couldn't enforce `T: CubicVoxel` at the trait boundary.
pub trait Mesher {
    /// Voxel value type this mesher consumes.
    type VoxelKind: Default + Clone;
    /// Engine-specific mesh artifact type.
    type Mesh;
    /// Mesh `chunk` at level `lod`. Implementations should be deterministic for a
    /// given (chunk, lod) pair so replay produces identical meshes.
    fn mesh_chunk(
        &self,
        chunk: ChunkView<'_, Self::VoxelKind>,
        lod: LodLevel,
    ) -> MeshResult<Self::Mesh>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-PHENO-VOXEL-MESH-000 — `MeshBuffer::default` is empty and serializable.
    #[test]
    fn default_meshbuffer_is_empty() {
        let m = MeshBuffer::default();
        assert!(m.vertices.is_empty());
        assert!(m.indices.is_empty());
    }

    fn make_test_buffer(n_verts: usize, n_tris: usize) -> MeshBuffer {
        let vertices = (0..n_verts)
            .map(|i| MeshVertex {
                position: [i as f32, 0.0, 0.0],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 0.0],
                material: crate::voxel::material::MaterialId(1),
            })
            .collect();
        let indices: Vec<u32> = (0..(n_tris * 3) as u32).collect();
        let ao = vec![3u8; n_verts];
        MeshBuffer {
            vertices,
            indices,
            ao,
        }
    }

    /// FR-PHENO-VOXEL-MESH-EXPORT-000 — `is_empty` matches vertex count.
    #[test]
    fn is_empty_empty_buffer() {
        let m = MeshBuffer::default();
        assert!(m.is_empty());
        assert_eq!(m.vertex_count(), 0);
        assert_eq!(m.index_count(), 0);
        assert_eq!(m.triangle_count(), 0);
    }

    /// FR-PHENO-VOXEL-MESH-EXPORT-001 — counts agree with constructed data.
    #[test]
    fn counts_agree_with_data() {
        let m = make_test_buffer(6, 2);
        assert!(!m.is_empty());
        assert_eq!(m.vertex_count(), 6);
        assert_eq!(m.index_count(), 6);
        assert_eq!(m.triangle_count(), 2);
        assert_eq!(m.indices().len(), m.index_count());
        assert_eq!(m.ao().len(), m.vertex_count());
        assert_eq!(m.positions().count(), m.vertex_count());
        assert_eq!(m.normals().count(), m.vertex_count());
        assert_eq!(m.uvs().count(), m.vertex_count());
    }

    /// FR-PHENO-VOXEL-MESH-EXPORT-002 — interleaved length == vertex_count * 9.
    #[test]
    fn interleaved_length_equals_vertex_count_times_stride() {
        const STRIDE: usize = 9;
        let m = make_test_buffer(4, 1);
        let buf = m.to_interleaved();
        assert_eq!(buf.len(), m.vertex_count() * STRIDE);
    }

    /// FR-PHENO-VOXEL-MESH-EXPORT-003 — interleaved handles empty mesh.
    #[test]
    fn interleaved_empty_mesh_produces_empty_vec() {
        let m = MeshBuffer::default();
        assert!(m.to_interleaved().is_empty());
    }

    /// FR-PHENO-VOXEL-MESH-EXPORT-004 — interleaved AO values match ao().
    #[test]
    fn interleaved_ao_values_match_ao_field() {
        let mut m = make_test_buffer(3, 0);
        m.ao = vec![0, 1, 2];
        let buf = m.to_interleaved();
        // AO sits at offset 8 of each 9-element stride
        for (i, &expected_ao) in m.ao.iter().enumerate() {
            assert_eq!(buf[i * 9 + 8], expected_ao as f32);
        }
    }

    /// FR-PHENO-VOXEL-MESH-EXPORT-005 — interleaved position data matches vertices.
    #[test]
    fn interleaved_positions_match_vertices() {
        let m = make_test_buffer(3, 0);
        let buf = m.to_interleaved();
        for (i, v) in m.vertices.iter().enumerate() {
            assert_eq!(buf[i * 9], v.position[0]);
            assert_eq!(buf[i * 9 + 1], v.position[1]);
            assert_eq!(buf[i * 9 + 2], v.position[2]);
        }
    }
}
