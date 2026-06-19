//! Terrain system: height field, chunk mesh builder, terrain LOD, materials, ports.
//!
//! Ported from C# `phenotype-terrain` (L5-110, 2026-06-18). The C# code is now
//! a thin P/Invoke shim under `unity/terrain/`; all real logic lives here in the
//! single Rust core per ADR-004.
//!
//! Sub-modules:
//! - [`error`] — typed errors (thiserror).
//! - [`height_field`] — 2D elevation grid (`HeightField`).
//! - [`chunk_mesh_builder`] — flat grid + height-mapped mesh builder.
//! - [`lod`] — `LodTier` enum + `LodBase` trait (re-used by `water`).
//! - [`terrain_lod`] — `TerrainLod` concrete impl.
//! - [`materials`] — `TerrainMaterial`, `TerrainMaterialProperty` (+ type enum).
//! - [`ports`] — `IMaterialRegistry` + `ISerializationPort` traits + adapters.

pub mod chunk_mesh_builder;
pub mod error;
pub mod height_field;
pub mod lod;
pub mod materials;
pub mod ports;
pub mod terrain_lod;
