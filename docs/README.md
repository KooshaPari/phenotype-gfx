# Introduction

**Phenotype GFX** is a polyglot graphics SDK that provides the rendering kernels
used by Phenotype-org games. One Rust core, four graphics substrates, unified
branding / versioning / interop — and a thin C# / Unity edge that consumes the
core via P/Invoke.

## At a glance

| Module    | Path              | Language | Role                                                           |
| --------- | ----------------- | -------- | -------------------------------------------------------------- |
| voxel     | `src/voxel/`      | Rust     | Adaptive voxel substrate (SVO + dense leaf chunks, BRP stream) |
| lod       | `src/lod.rs`      | Rust     | Frustum culling, scale-budget LOD primitives                   |
| streaming | `src/streaming.rs`| Rust     | Ring-based chunk lifecycle, eviction ordering                  |
| postfx    | `src/postfx/`     | Rust     | SSAO, Bloom, ACES, SSGI, Vignette, Chromatic, LUT              |
| water     | `src/water/`      | Rust     | Gerstner waves, fluid mesh generation                          |
| terrain   | `src/terrain/`    | Rust     | Height field, chunk mesh builder                               |
| unity     | `unity/`          | C#       | P/Invoke wrapper, MonoBehaviour example, NUnit tests           |

## What you'll find in this book

- **[Voxel Kernel](./voxel.md)** — chunk storage, greedy meshing, level-of-detail
  selection, and the SSE2/AVX2/NEON SIMD hot paths.
- **[Post-Processing](./postfx.md)** — the seven post-fx passes that drive the
  engine-agnostic render stack.
- **[Foreign Function Interface](./ffi.md)** — the 24 `#[no_mangle]` exports,
  cbindgen headers, and the Unity C# wrapper.
- **[Unity Integration](./unity.md)** — `PhenotypeGfx.cs` lifecycle, the
  MonoBehaviour example, and the NUnit test suite.
- **[Development](./development.md)** — build, test, run, Cargo features.

## Links

- **[README.md](https://github.com/KooshaPari/phenotype-gfx)** — top-level
  project overview, status, and quick links.
- **[AGENTS.md](https://github.com/KooshaPari/phenotype-gfx/blob/main/AGENTS.md)** —
  working conventions, branch naming, quality gates, do/don't rules.
- **[DESIGN.md](https://github.com/KooshaPari/phenotype-gfx/blob/main/DESIGN.md)** —
  architecture, data flow, non-goals.
- **[spec/interop.md](https://github.com/KooshaPari/phenotype-gfx/blob/main/spec/interop.md)** —
  shared data-format contract between Rust core and Unity C# edge.
- **[VERSION.toml](https://github.com/KooshaPari/phenotype-gfx/blob/main/VERSION.toml)** —
  umbrella version manifest pinning each module.

## Status

Work-state: scaffolding · `▓▓░░░░░░░░` 2/10. Single Rust core holds all
algorithm logic per ADR-004; Unity subpackages are folded but retain their own
local governance.

## License

Dual-licensed under **MIT** or **Apache-2.0** at your option. See `LICENSE-MIT`
and `LICENSE-APACHE` in the repository root.
