//! Renderer port: engine-neutral GPU frame submission contract.
//!
//! The domain depends **only** on the [`RendererPort`] trait. Adapters in
//! engine crates (Bevy, Godot, Unreal, …) implement this trait; tests and
//! headless contexts use a mock implementation. The hexagon boundary is:
//!
//! ```text
//!   ┌───────────────────────────────┐
//!   │ domain (uses trait)           │
//!   └─────────────┬─────────────────┘
//!                 ▼
//!           RendererPort             ◀── port (this file)
//!                 ▲
//!   ┌─────────────┴─────────────────┐
//!   │ adapters: bevy / godot / mock │
//!   └───────────────────────────────┘
//! ```

use thiserror::Error;

use crate::voxel::mesh::MeshBuffer;

// ────────────────────────────────────────────────────────────────────────────
// Camera view
// ────────────────────────────────────────────────────────────────────────────

/// Engine-neutral camera view passed to [`RendererPort::begin_frame`].
///
/// Captures only the inputs a renderer needs to build a view-projection
/// matrix; engine-specific extensions (FOV curves, TAA jitter, exposure,
/// lens-shift, …) live in the adapter layer and are not visible at the
/// domain boundary.
///
/// Coordinates are `f32` world-space units: this type lives at the
/// rendering boundary where the substrate's fixed-point `i64` world
/// coordinates have already been converted to floats by the caller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// World-space camera position.
    pub position: [f32; 3],
    /// World-space point the camera is aimed at.
    pub target: [f32; 3],
    /// World-space up vector (conventionally `[0, 1, 0]`).
    pub up: [f32; 3],
    /// Vertical field of view, in radians.
    pub fov_y: f32,
    /// Viewport aspect ratio (`width / height`).
    pub aspect: f32,
    /// Distance to the near clip plane. Must be `> 0`.
    pub near: f32,
    /// Distance to the far clip plane. Must be `> near`.
    pub far: f32,
}

impl Default for Camera {
    /// Reasonable defaults: origin looking at origin, 60° vertical FOV,
    /// 16:9 viewport, near 0.1, far 1000.
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 5.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov_y: std::f32::consts::FRAC_PI_3,
            aspect: 16.0 / 9.0,
            near: 0.1,
            far: 1000.0,
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Frame handle
// ────────────────────────────────────────────────────────────────────────────

/// Opaque handle to a frame in flight on the [`RendererPort`].
///
/// Returned by [`RendererPort::begin_frame`] and consumed by
/// [`RendererPort::submit_chunk`] / [`RendererPort::end_frame`]. The
/// inner representation is adapter-defined (monotonic counter, fence
/// token, ring-buffer index, …) — the domain treats it as opaque and
/// only compares it for identity.
///
/// Direct construction is meaningful only for tests and mock adapters
/// that need to fabricate handles; real adapters should mint ids from
/// [`RendererPort::begin_frame`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameId(pub u64);

// ────────────────────────────────────────────────────────────────────────────
// Errors
// ────────────────────────────────────────────────────────────────────────────

/// Errors that can be raised by a [`RendererPort`] adapter.
///
/// Adapters translate their underlying failure (surface lost, device
/// removed, GPU OOM, shader compile error, recycled handle, …) into one
/// of these variants so domain code can pattern-match without depending
/// on engine-specific error types.
#[derive(Debug, Error)]
pub enum RenderError {
    /// The GPU / surface / device context backing the renderer was lost
    /// (surface resize, device removal, TDR, …). Adapters should
    /// translate the platform-specific signal into this variant so
    /// domain code can trigger a context rebuild without depending on
    /// engine internals.
    #[error("render context was lost")]
    LostContext,

    /// The renderer ran out of GPU or staging memory. Domain code may
    /// respond by lowering LOD, dropping pending submissions, or
    /// throttling the next frame.
    #[error("render out of memory")]
    OutOfMemory,

    /// A shader (vertex, fragment, compute, …) failed to compile or
    /// link. The contained string is the adapter-reported diagnostic
    /// message suitable for logging.
    #[error("shader compilation failed: {0}")]
    ShaderCompilationFailed(String),

    /// A frame, chunk, or resource handle was invalid: stale, recycled,
    /// or issued by a different renderer instance. Indicates a contract
    /// violation at the adapter boundary (typically a domain bug).
    #[error("invalid handle")]
    InvalidHandle,
}

/// Result alias for renderer port operations.
pub type RenderResult<T> = Result<T, RenderError>;

// ────────────────────────────────────────────────────────────────────────────
// Port trait
// ────────────────────────────────────────────────────────────────────────────

/// Hexagonal port: engine-neutral GPU frame submission.
///
/// Models a per-frame render lifecycle:
///
/// 1. [`begin_frame`](Self::begin_frame) opens a new frame addressed by
///    the supplied [`Camera`] view and returns an opaque [`FrameId`].
/// 2. [`submit_chunk`](Self::submit_chunk) queues one [`MeshBuffer`]'s
///    worth of geometry into the open frame. Implementations translate
///    the engine-neutral mesh into their engine's native draw resources
///    (vertex buffer, draw command, …).
/// 3. [`end_frame`](Self::end_frame) finalises the frame and hands it
///    off to the GPU / surface.
///
/// Splitting the lifecycle into three calls lets headless tests assert
/// the *sequence* of submissions (begin → N submits → end) without
/// needing a real GPU.
pub trait RendererPort {
    /// Open a new frame for the supplied view. Returns a [`FrameId`]
    /// the caller uses to address subsequent
    /// [`submit_chunk`](Self::submit_chunk) and
    /// [`end_frame`](Self::end_frame) calls.
    fn begin_frame(&mut self, view: &Camera) -> RenderResult<FrameId>;

    /// Queue `chunk` for rendering into the open `frame`.
    ///
    /// Implementations should translate the engine-neutral mesh into
    /// their engine's native draw resources. Must not be called with a
    /// `frame` that was not returned by the most recent
    /// [`begin_frame`](Self::begin_frame) on the same `self`; doing so
    /// must return [`RenderError::InvalidHandle`].
    fn submit_chunk(&mut self, frame: FrameId, chunk: &MeshBuffer) -> RenderResult<()>;

    /// Finalise `frame` and hand it off to the GPU / surface.
    ///
    /// After `end_frame` returns, the `frame` handle is invalid and
    /// must not be reused; passing it again must return
    /// [`RenderError::InvalidHandle`].
    fn end_frame(&mut self, frame: FrameId) -> RenderResult<()>;
}
