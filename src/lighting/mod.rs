// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! Lighting system: SSAO, directional light, point light, and shadow mapping.
//!
//! Shader types live in [`shaders`]. The Rust core is engine-agnostic (ADR-004);
//! consumers pass the source string to the engine-side shader compilation
//! pipeline.

pub mod shaders;
