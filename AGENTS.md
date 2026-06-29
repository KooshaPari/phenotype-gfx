# AGENTS.md — phenotype-gfx

This file governs work inside the `phenotype-gfx` umbrella repository.

## Identity

`phenotype-gfx` is a polyglot graphics SDK: Rust voxel substrate (`src/`) plus C# terrain, water, and postfx (`unity/`). Unified branding, versioning, and interop. The single Rust core holds all algorithm logic per ADR-004; Unity subpackages are folded but retain their own local governance.

Do not apply parent shelf instructions unless explicitly referenced. Work from this directory and treat paths as local to `phenotype-gfx`.

## Quick Links

- **Architecture:** `docs/adr/ADR-004-single-core-ffi-edges.md`
- **Interop contract:** `spec/interop.md`
- **Version manifest:** `VERSION.toml`
- **Unity sub-packages:**
  - `unity/terrain/AGENTS.md`
  - `unity/water/AGENTS.md`
  - `unity/postfx/` (upstream: KooshaPari/phenotype-postfx)

## Commands

```bash
# Check compilation (all targets)
cargo check --all-targets

# Run all tests
cargo test --all-targets

# Format check
cargo fmt --check --all

# Lint
cargo clippy --all-targets

# Run benchmarks
cargo bench

# Build with optional bevy feature
cargo check --features bevy
```

## Working Conventions

- **Branch naming:** `<type>/<topic>` in kebab-case, conventional commits.
- **PR expectations:** Link relevant spec/adr/finding when non-trivial. Each PR includes a short rationale and notes any consumer-side impact on the `unity/` subpackages.
- **Quality gates:** `cargo check --all-targets` and `cargo test --all-targets` must pass before merge.
- **Stack:** Rust 2021 edition; C# / .NET for Unity subpackages. Changes to the Rust core should not break the `bevy` feature (`cargo check --features bevy`).
- **Traceability:** Substantive work links FR IDs, ADR references, or audit finding IDs (e.g. `L5-109`).
- **Security disclosures:** Follow `unity/*/SECURITY.md` in the affected subpackage; never open public issues for security findings.

## Architecture

```
src/
  lib.rs           crate root — re-exports all modules
  voxel/           adaptive voxel substrate (SVO + dense leaf chunks)
  lod/             LOD system (frustum culling, scale-budget primitives)
  streaming/       ring-based chunk lifecycle, eviction ordering
  postfx/          SSAO, SSGI, Bloom, ACES, LUT, vignette, CA
  water/           Gerstner waves, fluid mesh generation, water LOD
  voxelizer/       sprite voxelizer (OrganicBlob, Lathe, PerTexel)
  terrain/         height field, chunk mesh builder, terrain LOD, materials

tests/
  mesher_triangle_regression.rs   mesher output regression guards
  perf_regression_guards.rs       non-timing complexity/behavior regressions

benches/
  voxelizer_bench.rs
  mesher_compare.rs
  perf_suite.rs
  post_stack_bench.rs

unity/
  terrain/         C# terrain (folded from phenotype-terrain)
  water/           C# water (folded from phenotype-water)
  postfx/          Unity package (folded from phenotype-postfx)

docs/
  adr/             Architecture Decision Records
  disposition/     WIP/Fleet manifest documents

spec/
  interop.md       shared data-format contract between modules
```

## Do / Don't

- **Do** keep the Rust core as the single source of truth for algorithm logic — no duplication across languages.
- **Do** use `VERSION.toml` as the SSOT for module versions.
- **Do** add regression guards when fixing bugs or adding new behavior (see `tests/perf_regression_guards.rs`).
- **Do** run `cargo fmt` before committing.
- **Don't** add duplicated logic in the `unity/` subpackages — fold it into `src/` and expose via FFI.
- **Don't** commit binary artifacts, `.meta` blobs outside `unity/`, or large generated files.
- **Don't** remove existing regression tests without a compelling reason documented in the PR.

## Status

This AGENTS.md is living governance for `phenotype-gfx`. Update it when conventions change, and link any new tooling or process notes here.
