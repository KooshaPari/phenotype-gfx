// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! T27 stub: URP 17 RecordRenderGraph adapter contract.
//!
//! URP 17 (Unity 6) replaces the `OnRenderImage` callback with the
//! `ScriptableRenderPass.RecordRenderGraph` API.  This file is the
//! specification of the adapter port trait that the 7 existing passes
//! must implement when migrating from BRP (`OnRenderImage`) to URP 17.

use crate::postfx::ports::post_fx_pass::{PassQuality, PostFxPass};

/// Hexagonal port: a post-fx pass that can record itself into a URP 17
/// RenderGraph. Each existing BRP pass will gain an adapter implementing
/// this port without changing the BRP-side implementation.
pub trait PostFxUrpPass: Send + Sync {
    /// Stable name of the pass, used for profiling and ordering.
    fn name(&self) -> &str;
    /// Whether the pass should run in the current frame.
    fn is_enabled(&self) -> bool;
    /// Records the pass into the supplied RenderGraph.
    fn record_render_graph(&self, ctx: &PostFxUrpContext);
}

/// Per-camera context for the URP 17 adapter.
#[derive(Debug, Clone)]
pub struct PostFxUrpContext {
    /// Camera being rendered (opaque handle).
    pub camera: u64,
    /// Per-frame quality level.
    pub quality: PassQuality,
}

impl Default for PostFxUrpContext {
    fn default() -> Self {
        Self { camera: 0, quality: PassQuality::Off }
    }
}

/// Adapter that bridges an existing [`PostFxPass`] (BRP-side) to the URP 17
/// `RecordRenderGraph` API.
pub struct BrpToUrpAdapter<P: PostFxPass> {
    brp_pass: P,
}

impl<P: PostFxPass> BrpToUrpAdapter<P> {
    /// Wrap a BRP pass so it can be added to a URP 17 RenderGraph.
    pub fn new(brp_pass: P) -> Self {
        Self { brp_pass }
    }
}

impl<P: PostFxPass> PostFxUrpPass for BrpToUrpAdapter<P> {
    fn name(&self) -> &str {
        self.brp_pass.name()
    }

    fn is_enabled(&self) -> bool {
        self.brp_pass.is_enabled()
    }

    fn record_render_graph(&self, _ctx: &PostFxUrpContext) {
        // The actual URP 17 implementation requires Unity 6 + URP 17 packages.
        // This stub documents the API; the migration work (T27) will fill it
        // in once the project upgrades to Unity 6 in a follow-up.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postfx::ports::post_fx_pass::{PassEffect, PostFxContext};
    use crate::postfx::error::{PostFxError, PostFxResult};

    struct Stub;
    impl PostFxPass for Stub {
        fn name(&self) -> &str { "Stub" }
        fn effect(&self) -> PassEffect { PassEffect::Bloom }
        fn cost(&self) -> f32 { 0.1 }
        fn is_enabled(&self) -> bool { true }
        fn set_enabled(&mut self, _: bool) {}
        fn on_setup(&mut self, _: &PostFxContext) -> PostFxResult<()> { Ok(()) }
        fn on_render(&mut self, _: &PostFxContext) -> PostFxResult<()> { Ok(()) }
        fn on_dispose(&mut self) {}
        fn validate_variants(&self, _: &dyn crate::postfx::ports::shader_availability::PostFxShaderAvailability) -> Result<(), PostFxError> { Ok(()) }
    }

    #[test]
    fn brp_to_urp_adapter_delegates() {
        let adapter = BrpToUrpAdapter::new(Stub);
        assert_eq!(adapter.name(), "Stub");
        assert!(adapter.is_enabled());
        adapter.record_render_graph(&PostFxUrpContext::default());
        // default impl is a no-op
    }

    #[test]
    fn post_fx_urp_context_default() {
        let ctx = PostFxUrpContext::default();
        assert_eq!(ctx.camera, 0);
        assert_eq!(ctx.quality, PassQuality::Off);
    }
}
