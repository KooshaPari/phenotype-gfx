//! single-core home; real algorithms fold in here (see ADR-0001).
//!
//! 5-pass postfx pipeline: SSAO → SSGI → Bloom → ACES tonemapping → LUT grade.
//! WSM3D's C# postfx logic (5-pass) is PORTED HERE; C# becomes a thin P/Invoke shim.
//! Do not copy C# as-is — port the algorithm to Rust, expose via cbindgen C-ABI.
