//! phenotype-gfx: Unified graphics kernel + subsystems consolidation
//!
//! Layout:
//!   rust/voxel/   — phenotype-voxel subtree (full crate, hexagonal ports)
//!   unity/terrain/ — phenotype-terrain subtree (C# Unity crate)
//!   unity/water/   — phenotype-water subtree (C# Unity crate)
//!
//! This root crate is the Rust consolidation target.
//! C# subsystems (terrain, water, LOD, lighting, foliage, procgen) live under
//! unity/ and will eventually feed a phenotype-gfx-csharp sister project.

// Microlib 1: Voxel (from phenotype-voxel, already landed at rust/voxel/)
pub mod voxel {
    // Stub: backed by rust/voxel/ subtree; re-export facade lands with ADR-005
}

// Microlib 2: Terrain (from phenotype-terrain C# subtree)
pub mod terrain {
    // Stub: C# module lives at unity/terrain/; future phenotype-gfx-csharp sister project
}

// Microlib 3: Water (from phenotype-water C# subtree)
pub mod water {
    // Stub: C# module lives at unity/water/; future phenotype-gfx-csharp sister project
}

// Microlib 4: PostFX (from phenotype-postfx)
pub mod postfx {
    // Stub: will be populated by phenotype-postfx fold (see feat/postfx-fold-pilot)
}

// Subsystem 5: Lighting/Sky (from WSM3D)
pub mod lighting {
    // Stub: C# subsystem from WSM3D; future phenotype-gfx-csharp sister project
}

// Subsystem 6: C# LOD System (from WSM3D)
pub mod lod {
    // Stub: C# module from WSM3D; future phenotype-gfx-csharp sister project
}

// Subsystem 7: GPU Instancing (from WSM3D)
pub mod rendering {
    // Stub: GPU instancing + BRG variant from WSM3D; future phenotype-gfx-csharp sister project
}

// Subsystem 8: Foliage/Wind (from WSM3D)
pub mod foliage {
    // Stub: C# module from WSM3D; future phenotype-gfx-csharp sister project
}

// Subsystem 9: ProcGen (from WSM3D)
pub mod procgen {
    // Stub: C# module from WSM3D; future phenotype-gfx-csharp sister project
}

// Subsystem 10: Civis Streaming/LOD (Rust modules)
pub mod streaming {
    // Stub: will be populated by Civis lod.rs/material_pbr.rs/scale_budget.rs fold (ADR-004)
}
