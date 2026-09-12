# Development guide

How to build, test, benchmark, and ship `phenotype-gfx` from a clean
checkout. Targets Linux x86_64, Windows x86_64, macOS arm64, and wasm32.

> **Source of truth:** `Cargo.toml:1`, `DESIGN.md:1`, `AGENTS.md:1`,
> `.github/workflows/ci.yml:1`.

## Prerequisites

- **Rust** ≥ 1.75 (uses edition 2021 and 2024 features).
- **Cargo** (bundled with rustup).
- **mdBook** (only for documentation site):
  `cargo install mdbook --locked --version "^0.4"`.
- **cbindgen** (only when regenerating C headers):
  `cargo install cbindgen --locked`.
- **NUnit / dotnet** (only for the Unity C# test suite).

For the `gpu` feature, you also need a Vulkan / Metal / DX12 capable
device and a recent GPU driver.

## Layout

```
phenotype-gfx/
├── src/
│   ├── voxel/         ← chunk, greedy mesher, LOD, SIMD
│   ├── postfx/        ← post_stack, SSAO, Bloom, ACES, …
│   ├── water/         ← water surface + caustics
│   ├── voxelizer/     ← mesh → voxel rasteriser
│   ├── terrain/       ← heightmap + biome registry
│   ├── streaming/     ← chunk LRU manager
│   ├── compute/       ← GPU dispatch (feature-gated)
│   └── obs/           ← tracing + metrics facade
├── bindings/c_api.rs  ← 24 #[no_mangle] C-ABI exports
├── include/           ← cbindgen output (C / C++)
├── unity/             ← C# wrapper + integration example
├── tests/             ← integration tests + Unity NUnit
├── benches/           ← criterion micro-benchmarks
├── examples/          ← seven runnable demos
├── crates/phenotype-voxel/  ← inlined voxel crate
├── docs/              ← this mdBook site
└── .github/workflows/ ← CI pipelines
```

## Building

```bash
# Default build (rlib + cdylib, no optional features)
cargo build --release

# With SIMD meshing
cargo build --release --features simd

# With GPU compute dispatch
cargo build --release --features gpu

# With OTLP + Prometheus observability stack
cargo build --release --features full-obs

# With the Bevy ECS adapter
cargo build --release --features bevy

# With C-ABI surface only (minimal)
cargo build --release --no-default-features

# Run an example
cargo run --release --example terrain_demo
cargo run --release --example full_scene
cargo run --release --example postfx_demo
```

### Build outputs

| Build profile         | Output files                                              |
| --------------------- | --------------------------------------------------------- |
| `cargo build`         | `target/debug/libphenotype_gfx.so`, `phenotype_gfx.dll`   |
| `cargo build --release` | `target/release/libphenotype_gfx.so`, `phenotype_gfx.dll` |

## Testing

```bash
# Default tests (no GPU)
cargo test --release

# With SIMD
cargo test --release --features simd

# With GPU compute dispatch (requires Vulkan/Metal/DX12)
cargo test --release --features gpu

# All features
cargo test --release --all-features

# Just the GPU smoke test
cargo test --release --features gpu --test gpu_compute_smoke

# Run only doc tests
cargo test --release --doc

# Unity C# test suite (after building the native lib)
cd tests/unity/PhenotypeGfx.Tests && PHENOTYPE_GFX_LIB=$(pwd)/../../../target/release/libphenotype_gfx.so dotnet test
```

The Rust test suite is **deterministic** — no network, no real GPU, no
real time. The GPU smoke test uses `pollster::block_on` to drive wgpu
synchronously and runs entirely in-process.

## Linting & formatting

```bash
# Format check
cargo fmt --all --check

# Format apply
cargo fmt --all

# Clippy with all targets
cargo clippy --all-targets --no-deps -- -D warnings

# Clippy with all features
cargo clippy --all-targets --all-features --no-deps -- -D warnings

# Audit for known vulnerabilities
cargo install cargo-audit --locked
cargo audit

# License / advisory deny (CI runs this)
cargo install cargo-deny --locked
cargo deny check
```

## Benchmarking

```bash
# All benchmarks
cargo bench --release

# Single benchmark
cargo bench --release --bench voxelizer_bench
cargo bench --release --bench mesher_compare
cargo bench --release --bench perf_suite
cargo bench --release --bench post_stack_bench
```

Criterion HTML reports land in `target/criterion/`. The `mesher_compare`
benchmark pits the greedy mesher against a naive cubic mesher with
identical inputs so you can see the 5–10× vertex-reduction win.

## Documentation site

The `docs/` directory is an [mdBook](https://rust-lang.github.io/mdBook/)
site:

```bash
# Install once
cargo install mdbook --locked

# Local preview with hot-reload
mdbook serve --open

# Static build for CI
mdbook build
# → book/index.html, book/voxel.html, …
```

### mdBook chapters

| File                  | Topic                                  |
| --------------------- | -------------------------------------- |
| `docs/README.md`      | Project overview and roadmap           |
| `docs/SUMMARY.md`     | Table of contents (mdBook entry point) |
| `docs/voxel.md`       | Chunk storage, mesher, LOD, SIMD        |
| `docs/postfx.md`      | 7-pass post-processing stack           |
| `docs/ffi.md`         | C-ABI exports + C# wrapper             |
| `docs/unity.md`       | Unity integration and NUnit tests      |
| `docs/development.md` | Build, test, run, ship                 |

### CI for docs

`.github/workflows/docs.yml` builds the book on every push to `main`
and to feature branches that touch `docs/` or the workflow itself. The
deploy step is **not** wired — `gh-pages` branch setup requires repo
admin permissions and is left for the project owner to enable.

## Cargo features

| Feature       | Pulls in                                  | Purpose                                  |
| ------------- | ----------------------------------------- | ---------------------------------------- |
| `default`     | *(none)*                                  | Lean build for rlib + cdylib consumers   |
| `simd`        | runtime `cfg`                             | SSE2/AVX2 greedy meshing on x86_64       |
| `gpu`         | `wgpu 22`, `pollster 0.3`                 | GPU compute dispatch (`compute::gpu_mesher`) |
| `otlp`        | `opentelemetry`, `opentelemetry-otlp`     | OTLP gRPC tracing exporter               |
| `prometheus`  | `opentelemetry-prometheus`, `prometheus`  | Prometheus metrics scrape endpoint       |
| `full-obs`    | `otlp` + `prometheus`                     | Both observability stacks simultaneously |
| `bevy`        | `bevy 0.18`                               | Bevy ECS asset / mesh adapter            |
| `c_api`       | runtime `cfg`                             | Compile the 24 `#[no_mangle]` exports    |
| `zstd-storage`| `tokio`                                   | Async disk persistence for streamed chunks |

```bash
# Minimal build (rlib only, no C-ABI)
cargo build --no-default-features

# Unity-ship build (cdylib with C-ABI)
cargo build --release --features c_api,simd

# Headless server build (no GPU)
cargo build --release --features full-obs,zstd-storage

# Full kitchen sink
cargo build --release --all-features
```

## Running the examples

```bash
# Single mesh consumption
cargo run --release --example consume_mesh

# Terrain with biomes
cargo run --release --example terrain_demo

# Animated water surface
cargo run --release --example water_demo

# 7-pass post-processing showcase
cargo run --release --example postfx_demo

# Full scene: terrain + water + postfx + streaming
cargo run --release --example full_scene

# Networking demo (client/server chunk sync)
cargo run --release --example networking_demo

# Bevy ECS integration
cargo run --release --example ecs_integration --features bevy
```

## CI pipelines

Two workflows live in `.github/workflows/`:

| File             | Triggers on                                  | Job                                            |
| ---------------- | -------------------------------------------- | ---------------------------------------------- |
| `ci.yml`         | every push + PR                              | rustfmt, clippy, cargo test (default + features), cargo deny, Unity NUnit |
| `docs.yml`       | push to `main` + any `docs/**` change        | `mdbook build` smoke                           |

## Release checklist

1. `cargo fmt --all`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --release --all-features`
4. `cargo bench --release` (record numbers in CHANGELOG)
5. `cargo deny check`
6. `cargo audit`
7. `mdbook build` (regenerate the site)
8. `cbindgen` (regenerate `include/phenotype_gfx.h`)
9. Bump version in `Cargo.toml`.
10. Tag `vX.Y.Z`; push tag; cut GitHub release with `target/release/`
    artifacts attached.

## Troubleshooting

| Symptom                                  | Likely cause                       | Fix                                  |
| ---------------------------------------- | ---------------------------------- | ------------------------------------ |
| `error: linker not found`                | Missing C toolchain                | Install `build-essential` / VS Build Tools |
| `error: linking with link.exe failed`    | MSVC toolchain missing             | Install Visual Studio Build Tools 2022 |
| `simd` tests fail on ARM                 | SIMD is x86_64-only                | Use scalar fallback                  |
| `wgpu` adapter creation fails            | No Vulkan/Metal/DX12 device        | Use `--features gpu` only on GPU hosts |
| `cbindgen` regenerates empty headers     | Outdated `cbindgen.toml`           | Re-run `cbindgen --config cbindgen.toml` |
| `mdbook` not found                       | `cargo install mdbook` skipped     | Install per Prerequisites section    |
