//! Voxel kernel: storage, meshing, chunk management.
//!
//! The SVO + dense-leaf storage, dirty queue, and mesher trait live in the
//! shared `phenotype_voxel` kernel (re-exported from the crate root). This
//! module hosts the gfx-side voxel algorithms folded from Civis.

/// PBR material policy substrate (folded from Civis `material_pbr.rs`):
/// CC0 attestation, LOD render mode, material seed manifest, missing-texture
/// policy, channel maps, triplanar splat plan, greedy atlas plan.
pub mod material_pbr;
