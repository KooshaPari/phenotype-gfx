//! Compatibility crate for fleet consumers migrating off the archived
//! `KooshaPari/phenotype-voxel` repo.
//!
//! Canonical implementation lives in [`phenotype_gfx::voxel`] per ADR-004.

#![warn(missing_docs)]
#![allow(unsafe_code)] // SIMD intrinsics require unsafe blocks

pub mod simd;
pub use phenotype_gfx::voxel::*;
