// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! Post-processing pipeline: SSAO, SSGI, Bloom, ACES, Vignette, CA, LUT.
//!
//! Ported from C# + HLSL `phenotype-postfx` (L5-112, 2026-06-18). The C# code
//! is now a thin P/Invoke shim under `unity/postfx/`; the HLSL shaders remain
//! in `unity/postfx-shaders/` for the C# edge. All real logic lives here in
//! the single Rust core per ADR-004.
//!
//! ## Sub-modules
//!
//! - [`error`] — typed errors (thiserror).
//! - [`ports`] — hexagonal port traits + adapters (material registry,
//!   serialization, shader availability, LUT pipeline, URP render-graph).
//! - [`bloom_pass`] — bloom config + kernel parameters.
//! - [`ssao_pass`] — SSAO config + radius + sample kernel generation.
//! - [`post_fx_pass_registry`] — registry of enabled passes.
//! - [`post_stack`] — engine-agnostic config + availability + quality;
//!   `describe_passes()` returns `Vec<PassDescriptor>`.
//! - [`rendering`] — engine-agnostic render-target + material handle types
//!   (used by the C# edge; `PostFxMaterial` is a `#[deprecated]` pass-through).
//! - [`shaders`] — HLSL shader constants (preserved verbatim from upstream
//!   `phenotype-postfx/Runtime/Shaders/*.shader`).
//!
//! ## Reference
//!
//! Upstream: <https://github.com/KooshaPari/phenotype-postfx>.

pub mod bloom_pass;
pub mod error;
pub mod ports;
pub mod post_fx_pass_registry;
pub mod post_stack;
pub mod rendering;
pub mod shaders;
pub mod ssao_pass;

pub use bloom_pass::BloomConfig;
pub use error::{PostFxError, PostFxResult};
pub use ports::{
    lut_pipeline::{LutData, LutFormat},
    material_registry::{
        InMemoryPostFxMaterialRegistry, PostFxMaterialInfo, PostFxMaterialKind,
        PostFxMaterialRegistry, RecordingPostFxMaterialRegistry,
    },
    post_fx_pass::{PassDescriptor, PassEffect, PassQuality, PostFxContext, PostFxPass},
    serialization::{JsonFilePostFxSerialization, PostFxSerializationPort, PostFxStackSnapshot},
    shader_availability::{DefaultPostFxShaderAvailability, PostFxShaderAvailability},
    urp_render_graph::{BrpToUrpAdapter, PostFxUrpContext, PostFxUrpPass},
};
pub use post_fx_pass_registry::{BlitPassDescriptor, PostFxPassDescriptor, PostFxPassRegistry};
pub use post_stack::{PostStack, PostStackConfig, DEFAULT_POSTFX_STACK};
pub use rendering::{PostFxMaterial, PostFxRenderer, PostFxShader, RenderTarget};
pub use ssao_pass::SsaoConfig;
