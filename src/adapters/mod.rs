//! Concrete adapter implementations of the port traits.
//!
//! These adapters implement the domain contracts defined in [`crate::ports`]
//! and provide the concrete storage + meshing logic. Future engine-specific
//! renderers (Bevy, Godot, Unreal) will live in their own crates and implement
//! the same port traits.

pub mod chunk;
pub mod mesh;
pub mod octree;
pub mod renderer;
pub mod storage;

pub use chunk::DenseChunkStore;
pub use mesh::{CubicMesher, GreedyMesher, MeshAdapter};
pub use octree::OctreeAdapter;
pub use renderer::FrameCountingRenderer;
pub use storage::VoxelWorldAdapter;
