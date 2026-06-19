//! Water system: Gerstner waves, fluid mesh, water LOD, materials, ports.
//!
//! Ported from C# `phenotype-water` (L5-111, 2026-06-18). The C# code is now a
//! thin P/Invoke shim under `unity/water/`; all real logic lives here in the
//! single Rust core per ADR-004.

pub mod error;
pub mod gerstner_wave_bank;
pub mod lod_base;
pub mod ports;
pub mod rendering;
