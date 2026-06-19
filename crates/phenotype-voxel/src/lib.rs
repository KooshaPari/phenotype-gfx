//! Compatibility crate for fleet consumers migrating off the archived
//! `KooshaPari/phenotype-voxel` repo.
//!
//! Canonical implementation lives in [`phenotype_gfx::voxel`] per ADR-004.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use phenotype_gfx::voxel::*;
