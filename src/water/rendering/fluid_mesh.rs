//! Procedural water grid mesh.
//!
//! Ported from C# `Rendering/FluidMesh.cs`. The grid is centred at the origin
//! in the XZ plane; each vertex is displaced by a [`GerstnerWaveBank`] at
//! the given time.

use crate::water::error::WaterError;
use crate::water::gerstner_wave_bank::GerstnerWaveBank;

/// Snapshot of a generated water grid mesh at a single point in time.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshData {
    /// Displaced world-space vertex positions (length = `(resolution+1)^2`).
    pub vertices: Vec<glam::Vec3>,
    /// Analytic unit normals per vertex.
    pub normals: Vec<glam::Vec3>,
    /// Triangle index list (length = `resolution^2 * 6`).
    pub indices: Vec<u32>,
    /// UV coordinates in `[0, 1]` matching each vertex.
    pub uvs: Vec<glam::Vec2>,
}

impl MeshData {
    /// Whether this mesh has any triangles.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }
    /// Number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
    /// Number of triangles (every 3 indices = 1 triangle).
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Generate a tiling water grid mesh displaced by `bank`.
///
/// `resolution` is the number of quad columns (and rows) — must be `>= 1`.
/// `size` is the world-space side length of the square grid; must be `> 0`.
pub fn build(
    bank: &GerstnerWaveBank,
    resolution: u32,
    size: f32,
    time: f32,
) -> Result<MeshData, WaterError> {
    if resolution < 1 {
        return Err(WaterError::OutOfBounds {
            msg: "resolution must be >= 1".to_string(),
        });
    }
    if size <= 0.0 {
        return Err(WaterError::OutOfBounds {
            msg: "size must be > 0".to_string(),
        });
    }

    let res = resolution as usize;
    let verts = (res + 1) * (res + 1);
    let tris = res * res * 6;

    let mut vertices = Vec::with_capacity(verts);
    let mut normals = Vec::with_capacity(verts);
    let mut uvs = Vec::with_capacity(verts);
    let mut indices = Vec::with_capacity(tris);

    let step = size / resolution as f32;
    let half = size * 0.5;

    // Build vertices and normals
    for row in 0..=res {
        for col in 0..=res {
            let x = col as f32 * step - half;
            let z = row as f32 * step - half;
            let xz = glam::Vec2::new(x, z);
            let disp = bank.sample_displacement(xz, time);
            vertices.push(glam::Vec3::new(x + disp.x, disp.y, z + disp.z));
            normals.push(bank.sample_normal(xz, time));
            uvs.push(glam::Vec2::new(
                col as f32 / resolution as f32,
                row as f32 / resolution as f32,
            ));
        }
    }

    // Build triangle indices (two CCW triangles per quad)
    for row in 0..res {
        for col in 0..res {
            let bl = (row * (res + 1) + col) as u32;
            let br = bl + 1;
            let tl = ((row + 1) * (res + 1) + col) as u32;
            let tr = tl + 1;
            // Triangle 1: bl, tl, tr
            indices.push(bl);
            indices.push(tl);
            indices.push(tr);
            // Triangle 2: bl, tr, br
            indices.push(bl);
            indices.push(tr);
            indices.push(br);
        }
    }

    Ok(MeshData {
        vertices,
        normals,
        indices,
        uvs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn vertex_count_matches_resolution() {
        for res in [1u32, 4, 8, 16] {
            let bank = GerstnerWaveBank::new();
            let mesh = build(&bank, res, 10.0, 0.0).unwrap();
            let expected = ((res + 1) * (res + 1)) as usize;
            assert_eq!(mesh.vertices.len(), expected);
            assert_eq!(mesh.normals.len(), expected);
            assert_eq!(mesh.uvs.len(), expected);
        }
    }

    #[test]
    fn index_count_matches_resolution() {
        for res in [1u32, 4, 8, 16] {
            let bank = GerstnerWaveBank::new();
            let mesh = build(&bank, res, 10.0, 0.0).unwrap();
            let expected = (res * res * 6) as usize;
            assert_eq!(mesh.indices.len(), expected);
        }
    }

    #[test]
    fn empty_bank_normals_are_unit_length() {
        let bank = GerstnerWaveBank::new();
        let mesh = build(&bank, 4, 10.0, 0.0).unwrap();
        for n in &mesh.normals {
            assert!(approx(n.length(), 1.0, 1e-4));
        }
    }

    #[test]
    fn ocean_preset_normals_are_unit_length() {
        let bank = GerstnerWaveBank::create_ocean_preset();
        let mesh = build(&bank, 8, 50.0, 3.3).unwrap();
        for n in &mesh.normals {
            assert!(approx(n.length(), 1.0, 1e-4));
        }
    }

    #[test]
    fn empty_bank_flat_grid_y_zero() {
        let bank = GerstnerWaveBank::new();
        let mesh = build(&bank, 4, 10.0, 0.0).unwrap();
        for v in &mesh.vertices {
            assert!(approx(v.y, 0.0, 1e-4));
        }
    }

    #[test]
    fn uvs_in_zero_one_range() {
        let bank = GerstnerWaveBank::new();
        let mesh = build(&bank, 4, 10.0, 0.0).unwrap();
        for uv in &mesh.uvs {
            assert!((0.0..=1.0).contains(&uv.x));
            assert!((0.0..=1.0).contains(&uv.y));
        }
    }

    #[test]
    fn no_degenerate_triangles_empty_bank() {
        let bank = GerstnerWaveBank::new();
        let mesh = build(&bank, 8, 20.0, 0.0).unwrap();
        for tri in mesh.indices.chunks(3) {
            let a = tri[0];
            let b = tri[1];
            let c = tri[2];
            assert_ne!(a, b);
            assert_ne!(b, c);
            assert_ne!(a, c);
            let v0 = mesh.vertices[a as usize];
            let v1 = mesh.vertices[b as usize];
            let v2 = mesh.vertices[c as usize];
            let cross = (v1 - v0).cross(v2 - v0);
            assert!(cross.length() > 1e-8, "degenerate tri {a} {b} {c}");
        }
    }

    #[test]
    fn vertices_match_wave_bank_displacement() {
        let bank = GerstnerWaveBank::create_lake_preset();
        let res = 4u32;
        let size = 8.0f32;
        let time = 2.1f32;
        let mesh = build(&bank, res, size, time).unwrap();
        let step = size / res as f32;
        let half = size * 0.5;
        for row in 0..=res {
            for col in 0..=res {
                let x = col as f32 * step - half;
                let z = row as f32 * step - half;
                let disp = bank.sample_displacement(glam::Vec2::new(x, z), time);
                let idx = (row * (res + 1) + col) as usize;
                let v = mesh.vertices[idx];
                assert!(approx(v.x, x + disp.x, 1e-4));
                assert!(approx(v.y, disp.y, 1e-4));
                assert!(approx(v.z, z + disp.z, 1e-4));
            }
        }
    }

    #[test]
    fn zero_resolution_raises() {
        let bank = GerstnerWaveBank::new();
        assert!(build(&bank, 0, 10.0, 0.0).is_err());
    }

    #[test]
    fn negative_size_raises() {
        let bank = GerstnerWaveBank::new();
        assert!(build(&bank, 4, -1.0, 0.0).is_err());
    }
}
