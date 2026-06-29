//! `WaterRenderer` — orchestrator that combines LOD, mesh, and material.
//!
//! Deprecated: the renderer was a thin orchestrator in C#. In the single
//! Rust core, callers compose the [`GerstnerWaveBank`], [`WaterLod`], and
//! [`super::fluid_mesh::build`] directly.

use super::fluid_mesh::{self, MeshData};
use super::water_lod::WaterLod;
use super::water_material::WaterMaterial;
use crate::water::error::WaterResult;
use crate::water::gerstner_wave_bank::GerstnerWaveBank;

/// Orchestrator that combines the wave bank, LOD selection, mesh generation,
/// and material application. Deprecated.
#[derive(Debug)]
#[deprecated(
    note = "Use FluidMesh::build + WaterLod directly; this orchestrator is a pass-through."
)]
pub struct WaterRenderer {
    lod: WaterLod,
    wave_bank: Option<GerstnerWaveBank>,
    material: Option<WaterMaterial>,
    patch_size: f32,
}

impl WaterRenderer {
    /// New empty renderer with default LOD and patch size = 100.
    pub fn new() -> Self {
        Self {
            lod: WaterLod::new(),
            wave_bank: None,
            material: None,
            patch_size: 100.0,
        }
    }
    /// Get the LOD controller.
    pub fn lod(&self) -> &WaterLod {
        &self.lod
    }
    /// Get the LOD controller mutably.
    pub fn lod_mut(&mut self) -> &mut WaterLod {
        &mut self.lod
    }
    /// The wave bank driving vertex displacement.
    pub fn wave_bank(&self) -> Option<&GerstnerWaveBank> {
        self.wave_bank.as_ref()
    }
    /// Set the wave bank.
    pub fn set_wave_bank(&mut self, b: GerstnerWaveBank) {
        self.wave_bank = Some(b);
    }
    /// The water material.
    pub fn material(&self) -> Option<&WaterMaterial> {
        self.material.as_ref()
    }
    /// Set the water material.
    pub fn set_material(&mut self, m: WaterMaterial) {
        self.material = Some(m);
    }
    /// The world-space size of the water patch in metres.
    pub fn patch_size(&self) -> f32 {
        self.patch_size
    }
    /// Set the patch size.
    pub fn set_patch_size(&mut self, v: f32) {
        self.patch_size = v;
    }

    /// Generate the water mesh for the given time and camera distance.
    /// Returns an empty mesh when the patch is culled.
    pub fn build_mesh(&self, time: f32, distance: f32) -> WaterResult<MeshData> {
        let bank = self
            .wave_bank
            .as_ref()
            .ok_or(crate::water::error::WaterError::NullWaveBank)?;
        let resolution = self.lod.select_resolution(distance)?;
        if resolution <= 0 {
            return Ok(MeshData {
                vertices: Vec::new(),
                normals: Vec::new(),
                indices: Vec::new(),
                uvs: Vec::new(),
            });
        }
        fluid_mesh::build(bank, resolution as u32, self.patch_size, time)
    }
}

impl Default for WaterRenderer {
    fn default() -> Self {
        Self::new()
    }
}
