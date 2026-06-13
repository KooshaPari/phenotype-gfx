//! Renderer adapter: [`FrameCountingRenderer`] wraps a [`RendererPort`] and
//! exposes observability counters (begin frames, chunk submissions,
//! end frames) for benchmarks, dashboards, and replay harnesses.
//!
//! The adapter forwards every port call to an inner [`RendererPort`] and
//! increments lightweight counters. It is engine-neutral — the wrapped
//! renderer can be Bevy, Godot, headless, …; the counters work for all of
//! them.

use crate::mesh::MeshBuffer;
use crate::ports::renderer::{Camera, FrameId, RenderResult, RendererPort};

/// Renderer decorator that tracks how many `begin_frame`,
/// `submit_chunk`, and `end_frame` calls have been forwarded to the inner
/// renderer.
///
/// Use this in benchmarks and tests to verify that a domain code path
/// produced the expected number of frames (e.g. "one frame per dirty-event
/// drain" or "zero frames when the world is idle").
#[derive(Debug, Clone)]
pub struct FrameCountingRenderer<R: RendererPort> {
    inner: R,
    /// Number of `begin_frame` calls forwarded.
    begun_frames: u64,
    /// Number of `submit_chunk` calls forwarded.
    chunk_submissions: u64,
    /// Number of `end_frame` calls forwarded.
    ended_frames: u64,
}

impl<R: RendererPort> FrameCountingRenderer<R> {
    /// Wrap an existing [`RendererPort`].
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            begun_frames: 0,
            chunk_submissions: 0,
            ended_frames: 0,
        }
    }

    /// Borrow the inner renderer.
    pub fn inner(&self) -> &R {
        &self.inner
    }

    /// Mutably borrow the inner renderer.
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consume the adapter and return the inner renderer.
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Total `begin_frame` calls forwarded so far.
    pub fn begun_frames(&self) -> u64 {
        self.begun_frames
    }

    /// Total `submit_chunk` calls forwarded so far.
    pub fn chunk_submissions(&self) -> u64 {
        self.chunk_submissions
    }

    /// Total `end_frame` calls forwarded so far.
    pub fn ended_frames(&self) -> u64 {
        self.ended_frames
    }

    /// Number of frames currently in progress (i.e.
    /// `begun - ended`).
    pub fn frames_in_flight(&self) -> u64 {
        self.begun_frames.saturating_sub(self.ended_frames)
    }

    /// Reset all counters. The inner renderer's own state is untouched.
    pub fn reset_counters(&mut self) {
        self.begun_frames = 0;
        self.chunk_submissions = 0;
        self.ended_frames = 0;
    }
}

impl<R: RendererPort> RendererPort for FrameCountingRenderer<R> {
    fn begin_frame(&mut self, view: &Camera) -> RenderResult<FrameId> {
        // Bump the counter on every forwarded call. The inner renderer is
        // responsible for any frame-lifecycle validation (e.g. "no frame
        // already in flight"); we record the call the same way a real
        // benchmark harness would.
        self.begun_frames = self.begun_frames.saturating_add(1);
        self.inner.begin_frame(view)
    }

    fn submit_chunk(&mut self, frame: FrameId, chunk: &MeshBuffer) -> RenderResult<()> {
        // Increment only on a successfully forwarded call — a rejected
        // submit (e.g. invalid `frame` handle) must not bump the counter,
        // so counters always reflect calls that actually reached the inner
        // renderer.
        self.inner.submit_chunk(frame, chunk)?;
        self.chunk_submissions = self.chunk_submissions.saturating_add(1);
        Ok(())
    }

    fn end_frame(&mut self, frame: FrameId) -> RenderResult<()> {
        // Bump the counter on every forwarded call (matches
        // `begin_frame` semantics).
        self.inner.end_frame(frame)?;
        self.ended_frames = self.ended_frames.saturating_add(1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::renderer::RenderError;

    /// Recording renderer used to verify adapter forwarding semantics
    /// without depending on the production `HeadlessRenderer` (which is
    /// intentionally strict about frame-lifecycle state).
    #[derive(Debug, Default)]
    struct RecordingRenderer {
        last_camera: Option<Camera>,
        last_chunk_vertex_count: Option<usize>,
        last_end_frame: Option<FrameId>,
    }

    impl RendererPort for RecordingRenderer {
        fn begin_frame(&mut self, view: &Camera) -> RenderResult<FrameId> {
            self.last_camera = Some(*view);
            Ok(FrameId(1))
        }

        fn submit_chunk(&mut self, _frame: FrameId, chunk: &MeshBuffer) -> RenderResult<()> {
            self.last_chunk_vertex_count = Some(chunk.vertex_count());
            Ok(())
        }

        fn end_frame(&mut self, frame: FrameId) -> RenderResult<()> {
            self.last_end_frame = Some(frame);
            Ok(())
        }
    }

    /// Recording renderer that rejects `submit_chunk` with
    /// `InvalidHandle` (to verify the counter does not increment on
    /// rejection).
    #[derive(Debug, Default)]
    struct RejectingRenderer {
        begun: u64,
    }

    impl RendererPort for RejectingRenderer {
        fn begin_frame(&mut self, _view: &Camera) -> RenderResult<FrameId> {
            self.begun = self.begun.saturating_add(1);
            Ok(FrameId(1))
        }

        fn submit_chunk(&mut self, _frame: FrameId, _chunk: &MeshBuffer) -> RenderResult<()> {
            Err(RenderError::InvalidHandle)
        }

        fn end_frame(&mut self, _frame: FrameId) -> RenderResult<()> {
            Ok(())
        }
    }

    /// FR-PHENO-VOXEL-PORT-RENDERER-ADAPTER-000 — counters increment on
    /// forwarded calls and `frames_in_flight` reflects in-progress frames.
    #[test]
    fn counters_increment_on_forwarded_calls() {
        let mut r = FrameCountingRenderer::new(RecordingRenderer::default());
        assert_eq!(r.begun_frames(), 0);
        assert_eq!(r.ended_frames(), 0);
        assert_eq!(r.frames_in_flight(), 0);

        let frame = r.begin_frame(&Camera::default()).expect("begin");
        assert_eq!(r.begun_frames(), 1);
        assert_eq!(r.frames_in_flight(), 1);

        let empty = MeshBuffer {
            vertices: Vec::new(),
            indices: Vec::new(),
            ao: Vec::new(),
        };
        r.submit_chunk(frame, &empty).expect("submit a");
        r.submit_chunk(frame, &empty).expect("submit b");
        assert_eq!(r.chunk_submissions(), 2);

        r.end_frame(frame).expect("end");
        assert_eq!(r.ended_frames(), 1);
        assert_eq!(r.frames_in_flight(), 0);
    }

    /// FR-PHENO-VOXEL-PORT-RENDERER-ADAPTER-001 — invalid-handle errors
    /// from the inner renderer bubble up unchanged and do not bump the
    /// `chunk_submissions` counter.
    #[test]
    fn invalid_state_propagates() {
        let mut r = FrameCountingRenderer::new(RejectingRenderer::default());
        let frame = r.begin_frame(&Camera::default()).expect("begin");
        let empty = MeshBuffer {
            vertices: Vec::new(),
            indices: Vec::new(),
            ao: Vec::new(),
        };
        let err = r.submit_chunk(frame, &empty).unwrap_err();
        assert!(matches!(err, RenderError::InvalidHandle));
        assert_eq!(r.chunk_submissions(), 0);
    }

    /// FR-PHENO-VOXEL-PORT-RENDERER-ADAPTER-002 — `reset_counters` zeroes
    /// the counters without disturbing the inner renderer's state.
    #[test]
    fn reset_counters_preserves_inner_state() {
        let mut r = FrameCountingRenderer::new(RecordingRenderer::default());
        let frame = r.begin_frame(&Camera::default()).expect("begin");
        let empty = MeshBuffer {
            vertices: Vec::new(),
            indices: Vec::new(),
            ao: Vec::new(),
        };
        r.submit_chunk(frame, &empty).expect("submit");
        r.end_frame(frame).expect("end");
        assert!(r.chunk_submissions() > 0);
        assert_eq!(r.begun_frames(), 1);

        r.reset_counters();
        assert_eq!(r.chunk_submissions(), 0);
        assert_eq!(r.begun_frames(), 0);
        assert_eq!(r.ended_frames(), 0);
        // Inner state is preserved — the recording renderer still holds
        // the last camera and last end-frame handle.
        assert!(r.inner().last_camera.is_some());
        assert_eq!(r.inner().last_end_frame, Some(frame));
    }
}
