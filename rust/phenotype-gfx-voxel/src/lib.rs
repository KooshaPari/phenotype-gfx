//! phenotype-gfx-voxel: Unified voxel + streaming + subsystems
//!
//! Language: Rust
//! Role: Core voxel kernel + Civis streaming/LOD modules
//! Source folds (pending):
//!   - voxel: from phenotype-voxel (rust/voxel subtree)
//!   - streaming: from Civis (lod.rs, material_pbr.rs, scale_budget.rs)
//!
//! C# subsystems (lighting, LOD, rendering, foliage, procgen) live in csharp/

// Microlib: voxel (from phenotype-voxel)
pub mod voxel {
    // Stub: will be folded from phenotype-voxel (rust/voxel subtree)
}

// Subsystem: Civis streaming/LOD modules
pub mod streaming {
    // Stub: will be folded from Civis (lod.rs, material_pbr.rs, scale_budget.rs)
}
