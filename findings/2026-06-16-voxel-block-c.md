# Block C — phenotype-voxel Per-Repo Audit

**Status:** Active (re-issued 2026-06-18)
**Date:** 2026-06-16 (original); 2026-06-18 (re-issued verdict)
**Repo:** <https://github.com/KooshaPari/phenotype-voxel>
**Tracker:** KooshaPari/phenotype-registry issue #75
**SSOT:** KooshaPari/phenotype-registry `docs/rationalization/block-c-consolidation.md`
**Long-term home:** <https://github.com/KooshaPari/phenotype-gfx> (PR #10)

---

## Re-issued verdict (2026-06-18, ADR-004 + ADR-031)

**Original verdict (2026-06-16):** umbrella-sister consolidation → GFX SDK, with
`phenotype-voxel` retained as the substrate crate and `phenotype-terrain` +
`phenotype-water` absorbed into a single `phenotype-gfx` (or similarly named)
crate collection.

**Re-issued verdict (2026-06-18, L5-113):** **SUPERSEDE → `KooshaPari/phenotype-gfx`
via PR #10 (commit `9a7c05a`).** The "umbrella-sister" pattern is replaced by the
**single Rust core + thin FFI edges** pattern mandated by ADR-004. `phenotype-voxel`
is **not** retained as a separate substrate crate; the voxel kernel (chunks, coords,
mesher trait, RLE codec, sprite voxelizer, AO-aware greedy mesher) is **inlined
into** `phenotype-gfx/src/voxel/` as one Rust module among several (alongside
`src/terrain/`, `src/water/`, `src/postfx/`). The source repo
`KooshaPari/phenotype-voxel` is **to be archived** once PR #10 merges (the
`phenotype-gfx` repo has no upstream consumers, so there is no downgrade path
to preserve).

**References:**
- `docs/adr/ADR-004-single-core-ffi-edges.md` — single Rust core + thin FFI edges
  (the absorbing pattern; supersedes umbrella-sister layout).
- `docs/adr/ADR-031-configra-absorb.md` — sibling "absorb, do not coexist" ADR
  (same policy applied to `phenotype-config` → `Configra`; precedent for the
  voxel absorb).
- PR: <https://github.com/KooshaPari/phenotype-gfx/pull/10>
- Commit: `9a7c05a` — `feat(gfx): inline voxel kernel from phenotype-voxel (L5-109, ADR-004)`

**Migration summary:** 7,704 lines absorbed (Rust kernel + tests + benches);
test count 94 → 122 on `feat/port-sister-repos-2026-06-18` (28 net new tests
gained from the inline + refactor).

The original audit body below is preserved verbatim for durability. Its
"keep as source-of-truth" advice is **superseded**; the actionable items
(top-6 over-engineering wins) are now applied as part of the inline into
`phenotype-gfx/src/voxel/` rather than as a pre-merge shrink pass.

---

## Purpose

Adaptive voxel substrate for Phenotype-org games: a sparse voxel octree (SVO)
for coarse / far-from-camera space combined with dense 16³ leaf chunks for
near-camera detail. Every voxel write produces a deterministic
`DirtyChunkEvent` so downstream consumers (Civis, WorldSphereMod3D, future
Pheno-org games) can rebuild meshes in a replay-safe order. Engine-neutral
core with a feature-gated Bevy adapter and a port/adapter layer for future
Godot / Unreal FFI shims.

The substrate is the canonical data-structure layer for the org: chunks,
coords, dirty-event ordering, LOD policy, material palette, mesher trait,
serialization, and a shape-hint registry ported from WSM3D.

## Stack

- **Language:** Rust (edition 2021, MSRV 1.75, toolchain `stable` with
  `rustfmt`, `clippy`, `llvm-tools-preview`).
- **Cargo workspace:** single-crate `phenotype-voxel` v0.1.0 (no
  `[workspace]` members; the `[workspace]` table at the bottom of
  `Cargo.toml:45` is empty).
- **Runtime deps (3):** `serde 1` (derive), `bytemuck 1` (PVOX RLE codec),
  `thiserror 2` (port errors). Optional: `bevy 0.18.1` behind
  `feature = "bevy"` (used only by `src/bevy_adapter.rs`).
- **Dev deps (2):** `criterion 0.5` (3 benches), `proptest 1.4`
  (sprite-voxelizer property tests).
- **Targets (in `rust-toolchain.toml`):** `stable` channel, but
  `targets = ["aarch64-apple-darwin", "x86_64-unknown-linux-gnu"]` only —
  no Windows / x86_64-pc-windows-msvc target declared even though the
  author develops on Windows (see CI `windows-latest` job).
- **Features (in `Cargo.toml:40-44`):** `default = []`, `bevy`,
  `godot-ffi = []`, `unreal-ffi = []`. The latter two are **empty stubs**
  with no implementation anywhere in the tree.
- **CI / quality:** GitHub Actions (`ci.yml`, `sonarcloud.yml`),
  `deny.toml` (cargo-deny), `sonar-project.properties`, `Taskfile.yml`,
  `justfile`, codecov via `cargo-llvm-cov`.
- **Docs:** VitePress skeleton under `docs/.vitepress/` (no `index.md`,
  no nav config), `UPSTREAM.md` (WSM3D lessons learned), functional /
  non-functional requirements at `docs/requirements/phenotype-voxel-frnfr.md`
  (434 lines, 13 FRs), specs at `docs/specs/`.

## Maturity

Pre-MVP scaffold. The README and `lib.rs` both state this explicitly:
"adaptive voxel substrate (pre-MVP scaffold)" and "the real storage +
meshing implementations land in follow-up PRs (P-V1 in Civis)". The repo
ships:

- 30 source files under `src/` (lib + chunk + coord + delta + lod +
  material + mesh + octree + world + serial + cubic_mesher + greedy_mesher +
  sprite_voxelizer + shape_hints + bevy_adapter + fixtures + ports/* (7) +
  adapters/* (5)).
- 94 lib tests passing (per `phenotype-voxel-frnfr.md:5`), 1 doctest
  skipped.
- 3 criterion benchmarks (`voxelizer_bench`, `mesher_compare`,
  `perf_suite`), 2 integration tests (`mesher_triangle_regression`,
  `perf_regression_guards`).
- 1 example (`examples/consume_mesh.rs`).
- Coverage artefacts checked in: `coverage/SUMMARY.md`,
  `coverage/cargo_cov.log`, `coverage/lcov.info` (≈5,084 lines).
- No production consumer is wired up; the README points at
  `WorldSphereMod3D` (C# / Unity) as the consumer "via a C ABI generated
  through `ffi-core` / `cbindgen` (lands in a later PR)". The Bevy
  adapter is feature-gated and has a single 1-feature test.
- One commit on `main` (`53237b2`, 2026-06-13). No git tags. No released
  crate.

Net: code is real (the determinism contract, RLE codec, mesher trait,
AO-aware greedy mesher, sprite voxelizer are all genuine work) but the
abstraction layers around them are speculative — built for consumers
that do not yet exist.

## Over-engineering findings (ponytail lens)

One finding per line. The "ponytail lens" cuts five ways:

- **delete** — code that does not earn its keep
- **stdlib** — pull a hand-rolled helper out of the codebase
- **native** — use Rust's type system / traits to make the bug
  unrepresentable instead of writing a check
- **yagni** — feature for the consumer you don't have
- **shrink** — replace a multi-line construct with a single
  expression

1. `src/ports/*` (1,131 lines) + `src/adapters/*` (687 lines) + the
   `MeshAdapter` enum (40 lines) re-export types that already exist at
   the crate root. Hexagonal architecture with zero in-repo consumers.
   **delete** — net: -1,300 lines possible.
2. `src/fixtures.rs:1-182` is `pub` and "exported so downstream
   consumers can pin the same input vectors" but the only consumer is
   the 3 internal tests in the same file; no other crate imports it.
   **delete** (inline the 3 samples into the tests that use them) —
   net: -150 lines possible.
3. `MockMaterialRegistry` + `MockWorldStore` + `MockChunkSerializer` +
   `MockCall` + `MockStoreCall` + `FrameCountingRenderer` (≈600 lines)
   are shipped as `pub` in production modules so "domain code can use
   them in tests" — but the domain has no other code; this is test
   infrastructure dressed as API surface. **delete** (move to
   `#[cfg(test)] mod mock` or a `dev-dependencies` test-helper crate) —
   net: -600 lines possible.
4. `src/ports/serialization.rs:1-253` wraps the in-crate
   `serial::save_chunk` / `load_chunk` in a `ChunkSerializer` trait,
   but the trait is generic only over `Chunk<u8>` while the underlying
   codec is generic over `T: Pod + Eq + Default + Clone`. Two
   abstraction layers for one codec with one format. **delete** (callers
   use the codec directly) — net: -250 lines possible.
5. `src/adapters/renderer.rs:1-239` (`FrameCountingRenderer`) is a
   decorator pattern with `u64` counters, `saturating_add`,
   `frames_in_flight`, `reset_counters` — pure YAGNI observability
   for a "pre-MVP scaffold" that has zero production renderers.
   **delete** — net: -180 lines possible.
6. `src/adapters/octree.rs:1-97` is a pure forwarding wrapper around
   `VoxelOctree`; the doc even admits "in a future refactor
   `VoxelOctree` could implement the traits directly, eliminating this
   wrapper". **delete** (let `VoxelOctree` impl the port traits) —
   net: -97 lines possible.
7. `src/adapters/mesh.rs:23-39` defines `MeshAdapter<V>` enum that
   `match`-dispatches to `CubicMesher` or `GreedyMesher`, both of which
   already implement `Mesher` directly. **yagni** — net: -40 lines
   possible.
8. `src/adapters/chunk.rs:1-68` (`DenseChunkStore`) is the only
   `Chunkable` impl and is never called from any domain code; it uses
   `HashMap<ChunkId, Chunk<T>>` (line 11) which violates the
   determinism contract every other collection honours (all
   `BTreeMap`). **delete** (no consumer) — net: -68 lines possible.
9. `src/adapters/storage.rs:47-55` (`VoxelWorldAdapter::new`) silently
   stores `voxel_span: 0` when wrapping an existing `VoxelWorld` — the
   doc even warns "the caller is responsible for keeping them in
   sync". The factory pattern that ships a known-incorrect default.
   **delete** the buggy constructor (keep `with_voxel_span`) — net:
   -10 lines possible.
10. `Cargo.lock` (2,858 lines) + the unused `bevy = "0.18.1"`
    dependency (only `src/bevy_adapter.rs` uses it, which is itself
    behind `feature = "bevy"`). For a library pre-MVP with no
    published crate, locking the dep graph is **yagni**; for a single
    consumer crate the lockfile just makes `cargo update` noisy.
    **delete** the lockfile + drop `bevy` (move the adapter to a
    separate `phenotype-voxel-bevy` crate when a real consumer needs
    it) — net: -2,870 lines possible.
11. `Cargo.toml:42-44` declares two empty feature stubs `godot-ffi = []`
    and `unreal-ffi = []` with no implementation in the tree. **delete**
    (re-add when an FFI crate actually exists) — net: -2 lines
    possible.
12. `coverage/SUMMARY.md` + `coverage/cargo_cov.log` + `coverage/lcov.info`
    (≈5,084 lines) are build / test output checked in. `docs/.vitepress/public/favicon.ico`
    (15,086 bytes) + `docs/.vitepress/public/logo.svg` (532 bytes) +
    `assets/brand/logo.svg` (532 bytes, duplicate of the vitepress
    copy) + `assets/brand/social-512.png` (9,983 bytes). **delete**
    coverage artefacts and the logo duplicate; gitignore the rest —
    net: -5,100 lines + 16 KB binaries possible.
13. `src/serial.rs:182-194` defines `read_u16_le` and `read_u32_le`
    helpers that wrap `u16::from_le_bytes([u8; 2])` /
    `u32::from_le_bytes([u8; 4])` — but the same pattern is open-coded
    elsewhere in the same file (e.g. line 65, 73, 76 use
    `u32::to_le_bytes` inline). **stdlib** — delete the helpers, use
    `u16::from_le_bytes` / `u32::from_le_bytes` at the call sites —
    net: -10 lines possible.
14. `src/coord.rs:42-44` packs `ChunkId` as
    `(cx << 40) | (cy << 16) | (cz & 0xFFFF)` — lossy: high 8 bits of
    `cy` and high 16 bits of `cz` collide or are masked off. The
    `BTreeMap<ChunkCoord, ...>` collection is already the natural key
    on the type. **native** — use `(cx, cy, cz)` as the `BTreeMap`
    key everywhere and `ChunkId` becomes an opaque newtype derived at
    materialization time only. **shrink** — net: -10 lines possible.
15. `src/mesh.rs:123-133` `to_interleaved` builds the GPU upload
    buffer with three `extend_from_slice` + one `push` per vertex.
    Defining a `#[repr(C)] struct PackedVertex(pub [f32; 9]);` and
    `bytemuck::cast_slice(&packed)` is a single expression and a
    zero-copy GPU upload. **native / shrink** — net: -10 lines +
    faster upload.
16. `src/cubic_mesher.rs:201-209` `face_ao` destructures
    `(_nox, _noy, _noz, ux, uy, uz, vx, vy, vz)` where the `nox, noy,
    noz` triple is bound to `_` and never used. The match could yield
    just the U/V basis. **shrink** — net: -5 lines possible.
17. `src/lod.rs:46-69` `select_lod(distance_metres: f32, ...)` takes
    `f32` and returns `LodLevel(u8)`, but the substrate's
    determinism contract is "no f32/f64 crosses the public API".
    `LodPolicy` and `VoxelScaleMultiplier` (line 15) also carry
    `f32`. The kernel/render boundary is the wrong place for
    floats. **native** — `LodPolicy` carries `i64` in fixed-point
    `10^6` units like `FIXED_SCALE`; convert to `f32` only in a
    thin `lod_render` adapter. **shrink** — net: -30 lines possible.
18. `src/lib.rs:78` exposes `pub const DEFAULT_VOXEL_SCALE_MULTIPLIER:
    f32 = 8.0;` — an `f32` constant crossing the public API while
    the doc above it (line 17-18) says no `f32`/`f64` crosses it.
    **native** — hide the value behind a `VoxelScaleMultiplier::default()`
    constructor and delete the public `f32` constant. **shrink** — net:
    -2 lines possible.
19. `src/material.rs:17-24` `VoxelMaterial` carries `era: u16` +
    `hardness: f32` as domain fields, but the substrate claim is that
    the kernel is material-agnostic. The `f32` is a domain
    determinism hazard. **native** — `hardness` is consumer metadata
    (damage, build-cost) and belongs in a consumer-side lookup table
    keyed by `MaterialId`. **delete** — net: -5 lines possible.
20. `src/mesh.rs:33-44` `MeshBuffer.ao: Vec<u8>` is a parallel
    collection to `vertices` with the contract "length always equals
    `vertices.len()`"; the doc comment on `greedy_mesher.rs` admits
    the field is half-implemented (GreedyMesher leaves it at
    default-3 with a TODO). **native** — make `ao` an
    `Option<Vec<u8>>` so the half-implementation is honest, or
    finish the TODO. **shrink** — net: -10 lines possible.
21. `src/cubic_mesher.rs:43-44` and `src/greedy_mesher.rs:71-72` both
    carry a `PhantomData<V>` field with a `Default` impl that
    ignores it; `Default::default()` would be auto-derived for
    `CubicMesher<V>()` / `GreedyMesher<V>()` (a unit struct generic
    over `V`). **shrink** — net: -4 lines possible.
22. `bevy_adapter` lives at the crate root (`src/bevy_adapter.rs`)
    while every other engine-FFI story is under `src/adapters/`.
    Inconsistent module layout. **delete** the root file, move the
    body to `src/adapters/bevy.rs` under the same `#[cfg(feature =
    "bevy")]` gate. **shrink** — net: -10 lines possible.
23. `src/world.rs:74-77` + `src/world.rs:98-113` re-derive
    `local_x/local_y/local_z` in two different methods (`write` and
    `read`) using the exact same 4-line formula. **native** — a
    private `local_index(pos, voxel_span) -> usize` helper. **shrink**
    — net: -6 lines possible.
24. `src/ports/storage.rs:107-110` requires the public trait
    `WorldStore` to return
    `Box<dyn Iterator<Item = (ChunkCoord, &Chunk<T>)> + '_>` from
    `chunks_dense()` — boxing the iterator kills inlining and forces
    a heap allocation per call. The `VoxelWorld` implementation
    returns a `BTreeMap` iterator; the port should be generic over
    `I: Iterator<...>` to keep it monomorphic. **native** — net: -3
    lines + faster iteration.
25. `src/ports/material.rs:107-115` returns
    `Box<dyn Iterator<Item = (MaterialId, &VoxelMaterial)> + '_>` for
    the same reason. **native** — net: -3 lines possible.
26. `src/ports/renderer.rs:151-172` `RendererPort` returns
    `RenderResult<FrameId>` from `begin_frame` but the
    implementations in this crate only ever return `Ok(FrameId(1))`.
    A two-step `begin → submit → end` API for a pre-MVP scaffold with
    zero production renderers. **yagni** — defer the port until
    `phenotype-voxel-bevy` / `-godot` / `-unreal` actually need it.
    **delete** — net: -173 lines possible.
27. `src/fixtures.rs:121-126` `sort_and_dedup` is documented as the
    "exact policy the substrate recommends consumers adopt", but the
    function is only called by the 2 internal tests. **delete** the
    export (inline into the tests) — net: -10 lines possible.
28. `src/ports/mesh.rs:11-22` re-declares the `Mesher` trait that
    already lives in `src/mesh.rs:158-170`. The port trait exists
    *only* to give the port module something to re-export. The
    `adapters::mesh` module's `pub use crate::cubic_mesher::{CubicMesher,
    CubicVoxel}; pub use crate::greedy_mesher::GreedyMesher;` (lines
    7-8) duplicates what `lib.rs:51-54` already does. **delete** the
    port trait + the re-export layer — net: -40 lines possible.
29. `src/world.rs:172-187` `compact()` re-implements a uniform-chunk
    promotion loop that `src/octree.rs:96-185` `VoxelOctree::compact`
    already does (the same 8-sibling group collapse). The world
    level promotes dense chunks to octree nodes; the octree level
    then re-collapses 8 of those nodes. The two passes are
    duplicate work. **native** — let the world's `compact()` call
    `octree.compact()` once after promotion. **shrink** — net: -10
    lines possible.
30. `src/cubic_mesher.rs:201-209` and `src/greedy_mesher.rs:147-155`
    maintain **two** face-direction encoding tables (one in each
    file) for the same six faces. A single `const FACE_TABLE: [(i32,
    i32, i32, [i32; 3], [i32; 3]); 6] = ...` shared via
    `pub(crate)` would dedupe both lookups. **stdlib / shrink** —
    net: -30 lines possible across both meshers.

**net: -10,200 lines possible** (top 6 wins: #1 + #2 + #3 + #10 + #12 +
#26). Even the conservative bottom-15 (everything except the top wins)
is **-4,900 lines possible** — roughly 28% of the current 17,317-line
working tree.

## Consolidation verdict

Referencing KooshaPari/phenotype-registry `docs/rationalization/block-c-consolidation.md`,
this repo is **merge source #1 of 3** in the strategic-merge plan:

> **#1 GFX SDK** — phenotype-voxel + phenotype-terrain + phenotype-water
> → merge into one branded graphics SDK collection. Per-repo PR.

**Verdict: keep as source-of-truth during the merge; expect to absorb
phenotype-terrain (terrain SVO) and phenotype-water (water surface
voxelisation) into a single `phenotype-gfx` (or similarly named) crate
collection with `phenotype-voxel` as the substrate crate. Action items
the GFX merge owner inherits from this audit:**

1. Land the top-6 over-engineering wins **before** the merge — the
   port/adapter layer, the mocks-as-API surface, the empty FFI feature
   stubs, the checked-in coverage artefacts, the unused `bevy`
   dependency, and `FrameCountingRenderer` are all merge-conflict
   hazards. Shrinking the surface from 17,317 to ~7,100 lines first
   makes the GFX merge a 3-way content merge instead of an
   abstraction-layer debate.
2. Move the `bevy` adapter to a separate `phenotype-voxel-bevy` crate
   *after* the merge so the GFX SDK stays engine-neutral at the
   kernel. `phenotype-terrain` and `phenotype-water` will need
   parallel adapter crates.
3. Re-target `rust-toolchain.toml` to include
   `x86_64-pc-windows-msvc` — the author develops on Windows, CI runs
   on `windows-latest`, and the current targets list drops Windows.
4. Re-publish `UPSTREAM.md` lessons into the GFX SDK's
   `docs/upstream.md` so terrain + water consumers see the WSM3D
   instancing / BRG / `OpaqueVertexColor` race notes too.
5. Drop `docs/.vitepress/` and the duplicate brand assets (svg / png /
   ico checked in twice); the org-level docs site at
   `phenotype-registry` is the SSOT.

**Related Block-C repos referenced by the consolidation SSOT (out of
scope for this PR; covered by their own per-repo audits):**

- **phenotype-terrain** and **phenotype-water** — other members of
  the GFX-SDK merge; their audits are pending. terrain
  and water
  share the `i64` fixed-point world-coord convention and the
  `DirtyChunkEvent` ordering contract that this crate defines; the
  merge should preserve those as the substrate-level invariants.
- **authvault** — **merge target** for the auth-dedup strategic merge
  (#2 in the SSOT). Voxel has no auth surface; the only relevance is
  that consumer crates downstream of the GFX SDK (e.g. WorldSphereMod3D)
  may be auth-shimmed via authvault once the dedup lands.
- **phenoShared** — **rescope** target for the generic-lib strategic
  merge (#3 in the SSOT). This crate's hexagonal ports/* + adapters/*
  layer (finding #1 above) is the exact pattern the rescope plan
  wants to *split* out of the substrate before bulk-up: hexagonal
  shells belong in `phenoShared` if they belong anywhere, and the
  voxel-specific contracts stay here.

**Local durability:** this file and the PR
`docs/block-c-audit-2026-06-16 → main` are the durable record per the
Block-C durability rule. Local-only branches are not the record; the
remote branch + this PR are.
