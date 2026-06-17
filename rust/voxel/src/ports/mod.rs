//! Trait-driven ports (contracts) for the voxel substrate.
//!
//! Every adapter — mesher, renderer, serializer, material registry — implements
//! one of these traits.  The domain code depends **only** on these traits,
//! never on concrete adapters.

pub mod chunk;
pub mod material;
pub mod mesh;
pub mod octree;
pub mod renderer;
pub mod serialization;
pub mod storage;

pub use chunk::{ChunkId, ChunkView, Chunkable};
pub use material::{
    InMemoryMaterialRegistry, MaterialError, MaterialRegistry, MaterialResult, MockCall,
    MockMaterialRegistry,
};
pub use mesh::{MeshBuffer, MeshError, MeshResult, MeshVertex, Mesher};
pub use octree::{OctreeQueryable, OctreeStorage};
pub use renderer::{Camera, FrameId, RenderError, RenderResult, RendererPort};
pub use serialization::{
    ChunkSerializer, MockChunkSerializer, PvoxRleSerializer, SerializationError,
    SerializationResult,
};
pub use storage::{MockStoreCall, MockWorldStore, StorageError, StorageResult, WorldStore};
