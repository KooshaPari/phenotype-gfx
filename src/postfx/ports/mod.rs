// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! Hexagonal port traits for the post-processing stack.
//!
//! Each adapter implements one of these traits. The [`PostStack`] driver
//! queries them in a fixed order without knowing the concrete type.
//!
//! Reference: `phenotype-voxel/src/ports/*` (T2 SSOT pattern), `phenotype-infra/REUSE.toml` (T20).

pub mod lut_pipeline;
pub mod material_registry;
pub mod post_fx_pass;
pub mod serialization;
pub mod shader_availability;
pub mod urp_render_graph;
