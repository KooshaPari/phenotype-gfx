//! Re-export of the abstract `LodBase` trait from the terrain module.
//!
//! In C# the water module inherited from a `LodBase` class that lived in the
//! terrain repo. In the single Rust core the abstract shape lives in
//! [`crate::terrain::lod`] and is shared by both terrain and water consumers.

pub use crate::terrain::lod::{LodBase, LodTier};
