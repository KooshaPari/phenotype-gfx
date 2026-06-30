# Threat Model — phenotype-gfx

_Last updated: 2026-06-30. Covers commit range origin/main as of that date._

## 1. System description

`phenotype-gfx` is a **headless Rust graphics kernel**: a pure-computation library
(voxel meshing, LOD planning, streaming policy, terrain/water/postfx simulation).
It has **no network stack, no persistent storage, no UI surface, and no runtime
service**. It is consumed by:

- **C# game clients** via C-ABI FFI (`cdylib`) — planned, not yet realized.
- **WASM/TS web consumers** via `wasm-bindgen` — planned, not yet realized.
- **Rust binary integrators** that link the `rlib` directly.

## 2. Assets

| Asset | Sensitivity | Owner |
|---|---|---|
| Caller-supplied voxel/mesh data | Low — game geometry, not PII | Consumer integrator |
| FFI output buffers (pointers) | Medium — memory safety boundary | This crate |
| CI/CD secrets (registry token, tag signing) | High | GitHub Actions encrypted secrets |
| Source code integrity | High | Git history + SHA-pinned CI |

## 3. Attack surface

### 3.1 FFI boundary (highest risk — currently deferred)

The `cdylib` output will expose a C-ABI. Until `pub mod c_api` is realized
(see `src/lib.rs:44`), the boundary is a `pub use voxel::*` Rust API — safe-only.

When the C-ABI lands:

- **Threat**: Caller passes out-of-bounds slice length → buffer over-read / UB.
  - **Mitigation (planned)**: every FFI entry point validates slice lengths before
    constructing `ChunkView`. `#[forbid(unsafe_code)]` in the pure Rust core;
    unsafe is confined to the thin `c_api` shim only.
- **Threat**: Null pointer passed as slice.
  - **Mitigation (planned)**: explicit null checks + `NonNull` wrappers in shim.
- **Threat**: Type confusion — C# sends wrong voxel representation.
  - **Mitigation (planned)**: `bytemuck::Pod` derive enforces repr compatibility;
    cbindgen-generated C header is the single source of truth.

### 3.2 Malformed input data (medium risk — active)

Any public function that takes caller-supplied data can receive adversarial input:

- `ChunkView::voxels` length mismatch: caught by `BadChunkSize` error (both
  `mesh_cubic` and `mesh_greedy` validate `voxels.len() == CHUNK_EDGE³`).
- Extreme `distance_metres` / `vy_weight` values: `ring_distance` is pure integer
  arithmetic; `plan_chunk_render` clamps via `select_lod`. No panics on extreme
  floats (NaN/inf propagates through `f32` arithmetic — callers should pre-validate).
- **Threat**: NaN `distance_metres` passed to `plan_chunk_render`.
  - **Mitigation**: document that callers must pre-validate floats. Future: add
    explicit NaN guard + structured error return.

### 3.3 Supply-chain (medium risk)

Dependencies (`serde`, `glam`, `tracing`, `metrics`, `bytemuck`, `thiserror`,
`uuid`) are all mature, widely-audited crates. The `bevy` optional dep is pinned
to a specific minor. `criterion`/`proptest` are dev-only.

- **Mitigation**: `Cargo.lock` is committed; SHA-pinned GitHub Actions
  (see `.github/workflows/`). `cargo audit` is planned for the CI pipeline.
- **Threat**: dependency confusion / typosquat.
  - **Mitigation**: all deps use crates.io (no git/local path deps outside
    `crates/phenotype-voxel` workspace member which is first-party).

### 3.4 Build and CI pipeline

- **Threat**: compromised GitHub Actions runner injects malicious artifacts.
  - **Mitigation**: all `uses:` steps are SHA-pinned (see
    `.github/workflows/release.yml`). Minimal permission set: `contents: write`
    only on the `release` job; `check-and-test` uses `contents: read`.
- **Threat**: unsigned crate publish to crates.io leaks pre-release code.
  - **Mitigation**: publish only triggered on `v*` tags; registry token scoped
    to this crate only (operator responsibility).

### 3.5 Memory safety

The Rust core uses `#[forbid(unsafe_code)]` in all pure modules. The `cdylib`
shim (when realized) is the sole `unsafe` zone. `bytemuck` transmutes are
gated by its `Pod`/`Zeroable` derives — no hand-rolled pointer casts.

- **Threat**: integer overflow in voxel indexing.
  - **Mitigation**: `CHUNK_EDGE` is `usize`; arithmetic uses `unsigned_abs()` and
    `u32` arithmetic in `ring_distance` — no signed overflow possible.

## 4. Security posture

| Control | Status |
|---|---|
| `#[forbid(unsafe_code)]` in pure modules | Active |
| Input length validation (mesh builders) | Active |
| `Cargo.lock` committed | Active |
| SHA-pinned CI actions | Active (release.yml) |
| `cargo audit` in CI | Planned |
| FFI null/bounds checks | Planned (when `c_api` lands) |
| NaN guard on float inputs | Planned |
| SBOM generation | Not started |

## 5. Out of scope

- **AuthN / AuthZ**: this is a library — there is no network listener, no user
  session, and no access-control surface. Consumers implement their own authZ.
- **Tenancy / data privacy**: no persistent storage; geometry data lives only in
  the caller's memory for the duration of a mesh build call.
- **Rate limiting**: no I/O; resource limiting is the caller's responsibility
  (e.g. limiting concurrent mesh build threads in a game engine).
- **DDoS**: no network surface.

## 6. Revision history

| Date | Author | Change |
|---|---|---|
| 2026-06-30 | phenotype-gfx overhaul | Initial threat model (L20 audit zero) |
