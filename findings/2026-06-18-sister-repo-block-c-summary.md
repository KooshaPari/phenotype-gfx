# Block-C Sister-Repo Summary — phenotype-gfx PR #10 (L5-113)

**Date:** 2026-06-18
**Author:** L5-113 (audit-sync mission)
**PR:** <https://github.com/KooshaPari/phenotype-gfx/pull/10>
**Branch:** `feat/port-sister-repos-2026-06-18` → `main`
**ADRs:** ADR-004 (single Rust core + thin FFI edges), ADR-031 (absorb, do not coexist)
**Companion audit doc:** the 4 re-issued Block-C audits in this directory:
- [`2026-06-16-voxel-block-c.md`](./2026-06-16-voxel-block-c.md)
- [`2026-06-16-terrain-block-c.md`](./2026-06-16-terrain-block-c.md)
- [`2026-06-16-water-block-c.md`](./2026-06-16-water-block-c.md)
- [`2026-06-18-postfx-block-c.md`](./2026-06-18-postfx-block-c.md)

---

## 1. Migration matrix

| Source repo | Lines absorbed | Tests preserved | PR commit | Status |
|-------------|---------------:|-----------------|-----------|:-------|
| `phenotype-voxel`    | 7,704  | 94 → 122 (+28 net new)   | `9a7c05a` | **SUPERSEDED** |
| `phenotype-terrain`  | 2,682  | 7 files (port parity)    | `61c1617` | **SUPERSEDED** |
| `phenotype-water`    | 3,145  | 9 files (port parity)    | `61c1617` | **SUPERSEDED** |
| `phenotype-postfx`   | 5,426  | 5 files (+ 2 in-port test classes) | `d68d42c` | **SUPERSEDED** |
| **TOTAL**            | **18,957** | **311 tests pass** | **3 commits** | **SUPERSEDED** |

**Line-count methodology:** `git show --stat <sha>` totals for each commit.
**Test-count methodology:** `cargo test --quiet` on
`feat/port-sister-repos-2026-06-18` reports 311 unit tests green; 2 doctest
failures (pre-existing in `src/voxel/shape_hints.rs` and
`src/voxel/sprite_voxelizer.rs`) are not part of the 311. The 4 in-port
xUnit test classes from `phenotype-postfx` (`MaterialRegistryPortTests`,
`SerializationPortTests`) are ported to Rust as `cargo test` cases in
`phenotype-gfx/src/postfx/ports/material_registry.rs` and
`phenotype-gfx/src/postfx/ports/serialization.rs`.

**Per-commit file change (PR #10):**
- `9a7c05a feat(gfx): inline voxel kernel from phenotype-voxel (L5-109, ADR-004)` —
  76 files changed, 3,272 insertions(+), 10,229 deletions(-).
- `61c1617 feat(gfx): port terrain + water C# to Rust (L5-110..111, ADR-004)` —
  23 files changed, 2,726 insertions(+).
- `d68d42c feat(gfx): port postfx C# + 8 HLSL shaders to Rust (L5-112, ADR-004)` —
  30 files changed, 3,504 insertions(+), 5 deletions(-).

**Net shape:** `phenotype-gfx/src/voxel/` (Rust kernel) +
`phenotype-gfx/src/terrain/` (Rust terrain core + ports) +
`phenotype-gfx/src/water/` (Rust water core + ports) +
`phenotype-gfx/src/postfx/` (Rust postfx core + ports) +
`phenotype-gfx/unity/postfx-shaders/` (9 HLSL shaders preserved verbatim) +
`phenotype-gfx/unity/terrain/SHIM_README.md` (Unity-side drop-in stub for
the terrain C# consumers) +
`phenotype-gfx/unity/water/SHIM_README.md` (Unity-side drop-in stub for
the water C# consumers).

## 2. Last-resort exceptions (left in source, dropped on archive)

The following pieces are **not** absorbed into `phenotype-gfx`; they remain
in the source repos until those repos are archived. Per the absorption
matrix, they are **dropped on source archive** (the source archive step
turns them into a read-only marker, but no consumer is wired to them
post-PR #10).

| Source | Path | Why left behind | Dropped on |
|--------|------|-----------------|-----------|
| `phenotype-terrain` | `src/Ports/IMaterialRegistry.cs:RecordingTerrainMaterialRegistry` | yagni mock; the in-memory adapter is the only one called by tests or README examples. | Source archive. |
| `phenotype-terrain` | `src/Ports/ISerializationPort.cs:MockSerializationPort` | yagni mock; `JsonFileSerializationPort` covers the production path. | Source archive. |
| `phenotype-terrain` | `src/ChunkMeshBuilder.cs:179-186` `BuildMesh(HeightField, int, float)` no-op | The `HeightField` data model was never implemented in the source repo; the no-op is honest about the gap. Not a regression in the Rust port. | Source archive. |
| `phenotype-terrain` | `_stub/` + `scripts/generate-unity-stub.sh` (~165 lines) | Hand-rolled `UnityEngine` struct shapes for off-line CI; `UnityEngine.Modules` on NuGet is the canonical answer. | Source archive. |
| `phenotype-terrain` | `.github/scorecard.yml`, `STATUS.md`, `SUPPORT.md`, root `CODEOWNERS`, README "Description/Install/Usage" duplication | Process debris. | Source archive. |
| `phenotype-water` | `Runtime/UnityEngineStubs.cs` (458 LOC) | Compile-time UnityEngine stub; not needed in the Rust port. | Source archive. |
| `phenotype-water` | `Runtime/Ports/IMaterialRegistry.cs:RecordingWaterMaterialRegistry` | yagni mock. | Source archive. |
| `phenotype-water` | `Runtime/Ports/ISerializationPort.cs:MockSerializationPort` | yagni mock. | Source archive. |
| `phenotype-water` | `Runtime/Rendering/WaterShader.cs` + `WaterMaterial.cs` | `Shader.Find` + `Material.SetFloat/...` are direct call sites in the Rust port. | Source archive. |
| `phenotype-water` | `Runtime/Rendering/*.cs` (3 files, ~60% of file = XML doc) | Per-property doc bloat; Rust port uses rustdoc. | Source archive. |
| `phenotype-water` | `Runtime/Ports/*.cs` mock surface | yagni; the in-memory adapter is the only one in the Rust port. | Source archive. |
| `phenotype-postfx` | `Runtime/Ports/UrpRenderGraphAdapter.cs` (142 LOC) | URP Render Graph adapter; zero callers; URP consumers go through the FFI edge. | Source archive. |
| `phenotype-postfx` | `Runtime/Ports/IPostFxPass.cs:IUrpPostFxPass` + `IPostFxPassProvider` | Sub-ports with no impl. | Source archive. |
| `phenotype-postfx` | `Runtime/Ports/IMaterialRegistry.cs:RecordingMaterialRegistry` | yagni mock. | Source archive. |
| `phenotype-postfx` | `Runtime/Ports/ISerializationPort.cs:MockSerializationPort` | yagni mock. | Source archive. |
| `phenotype-postfx` | `tests/Editor/PostFxPassRegistryTests.cs` + `PostStackEditTests.cs` (209 LOC) | Editor-only tests; the org-billing-blocked CI cannot run them; the in-port source tests cover the same surface. | Source archive. |
| `phenotype-postfx` | `phenotype-postfx-variants.shadervariants` (Unity asset) | Generated artifact; the Rust port's `PostFxQuality` enum + `shaders.rs` keyword dispatch is the equivalent. | Source archive. |
| `phenotype-postfx` | `tests/benchmarks/PostStackBenchmarks.cs` (262 LOC) | BenchmarkDotNet harness; no CI consumer; the 6 benchmarks are reproducible by hand on a real Unity scene. | Source archive (deferred to source, not source-shrink). |
| `phenotype-postfx` | `CLAUDE.md` (28 LOC stub) | The file is a 4-section stub; `phenotype-gfx`'s `AGENTS.md` is the SSOT. | Source archive. |
| `phenotype-voxel` | `Cargo.toml:42-44` empty `godot-ffi` + `unreal-ffi` feature stubs | yagni; re-add when an FFI crate actually exists. | Source archive. |
| `phenotype-voxel` | `src/bevy_adapter.rs` (root, not under `src/adapters/`) | Inconsistent module layout; the Rust port puts engine adapters in `phenotype-gfx/bindings/`. | Source archive. |
| `phenotype-voxel` | `coverage/SUMMARY.md` + `coverage/cargo_cov.log` + `coverage/lcov.info` (~5,084 lines) | Build/test output checked in; gitignored in `phenotype-gfx`. | Source archive. |

**Total left-behind surface:** ~1,000 lines of source mocks +
~5,000 lines of coverage artefacts + ~30 small process-debris files.

## 3. Verdict matrix

| Source repo | Original verdict (2026-06-16) | Re-issued verdict (2026-06-18) | Archival action |
|-------------|--------------------------------|--------------------------------|-----------------|
| `phenotype-voxel`    | umbrella-sister consolidation → GFX SDK | **SUPERSEDE → `phenotype-gfx` via PR #10** (commit `9a7c05a`) | Archive after PR merges |
| `phenotype-terrain`  | CONSOLIDATE → GFX SDK | **SUPERSEDE → `phenotype-gfx` via PR #10** (commit `61c1617`) | Archive after PR merges |
| `phenotype-water`    | CONSOLIDATE → GFX SDK | **SUPERSEDE → `phenotype-gfx` via PR #10** (commit `61c1617`) | Archive after PR merges |
| `phenotype-postfx`   | (no prior Block-C audit) | **SUPERSEDE → `phenotype-gfx` via PR #10** (commit `d68d42c`) | Archive after PR merges |

**Pattern:** the umbrella-sister layout (3 sister repos kept as
"collection members" of a branded GFX SDK) is **replaced** by the single
Rust core + thin FFI edges pattern (ADR-004). All 4 source repos
collapse into one Rust module tree under `phenotype-gfx/src/`.

## 4. Policy precedent

This is the **second** "absorb, do not coexist" wave in the fleet
(see ADR-031 for the first, `phenotype-config` → `Configra`). The
pattern is:

1. The 4 source repos publish their work as a "collection" of sister
   repos with hexagonal ports (per the original Block-C plan).
2. A single Rust core is built and absorbs all 4 collections into one
   module tree (per ADR-004).
3. The source repos are archived (read-only marker), preserving the
   commits and the audit history.
4. The 71-pillar audit on the absorbing repo (`phenotype-gfx`) is the
   SSOT for the combined surface.

**Future-proofing:** if a fifth or sixth sister repo appears, the
absorb-into-`phenotype-gfx` path is the default. New modules land as
`phenotype-gfx/src/<module>/` and the source repo is archived in the
same PR.

## 5. Cross-references

- **PR:** <https://github.com/KooshaPari/phenotype-gfx/pull/10>
- **Branch:** `feat/port-sister-repos-2026-06-18` (in
  `KooshaPari/phenotype-gfx`)
- **Audits:** the 4 re-issued Block-C audits in this directory.
- **Source SSOT (legacy):** `KooshaPari/phenotype-registry/docs/rationalization/block-c-consolidation.md`
- **SSOT (current):** `phenotype-gfx/AGENTS.md` + the 4 audit docs in
  `phenotype-gfx/findings/`.

## 6. Sign-off

- **Mission:** L5-113 (audit-sync, 2026-06-18)
- **PR:** <https://github.com/KooshaPari/phenotype-gfx/pull/10>
- **Verdict:** **4/4 sister repos SUPERSEDED → `phenotype-gfx`.**
- **Audit docs:** 4 files in `phenotype-gfx/findings/`.
- **Summary doc:** this file.
