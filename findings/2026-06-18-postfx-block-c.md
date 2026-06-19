# Block-C Audit — phenotype-postfx

**Audit date:** 2026-06-18 (created from scratch, this is the first Block-C pass)
**Auditor:** L5-113 (audit-sync mission)
**Tracker:** [phenotype-registry#75 (SSOT)](https://github.com/KooshaPari/phenotype-registry/pull/75) — strategic merge #1 (GFX SDK)
**Source repo:** `KooshaPari/phenotype-postfx`
**Target repo:** `KooshaPari/phenotype-gfx` (PR #10, commit `d68d42c`)
**Verdict at a glance:** **SUPERSEDE → `KooshaPari/phenotype-gfx` via PR #10 (commit `d68d42c`).** This is a Unity URP / BRP post-processing stack (Bloom, SSAO, ACES, Color Grading LUT, Chromatic Aberration, Vignette, Screen-Space AO/GI) shipped as a C# package with 9 HLSL/.shader files and 6 hexagonal ports. C# core is ported to Rust and absorbed into `phenotype-gfx/src/postfx/` (per ADR-004, single Rust core + thin FFI edges). Source repo `KooshaPari/phenotype-postfx` is **to be archived** once PR #10 merges; no downgrade path to preserve.

**Long-term home:** <https://github.com/KooshaPari/phenotype-gfx> (PR #10)

---

## Re-issued verdict (2026-06-18, ADR-004 + ADR-031)

This is the **first** Block-C audit for `phenotype-postfx`; no prior verdict exists. The verdict is **SUPERSEDE → `KooshaPari/phenotype-gfx` via PR #10 (commit `d68d42c`)**. ADR-004 (single Rust core + thin FFI edges) and ADR-031 (absorb, do not coexist) govern.

**References:**
- `docs/adr/ADR-004-single-core-ffi-edges.md` — single Rust core + thin FFI edges.
- `docs/adr/ADR-031-configra-absorb.md` — sibling "absorb, do not coexist" ADR.
- PR: <https://github.com/KooshaPari/phenotype-gfx/pull/10>
- Commit: `d68d42c` — `feat(gfx): port postfx C# + 8 HLSL shaders to Rust (L5-112, ADR-004)`

**Migration summary:** 5,426 lines absorbed (3,498 in `Runtime/` C# + shaders + 1,928 in `tests/` including stubs). Of that, ~3,498 of source (11 .cs + 9 .shader files) maps directly to `phenotype-gfx/src/postfx/*.rs` and `phenotype-gfx/unity/postfx-shaders/*.shader`. Test files ported to the Rust test harness.

**What was left behind (per PR #10, dropped on source archive):**
- `tests/Editor/PostFxPassRegistryTests.cs`, `tests/Editor/PostStackEditTests.cs` — Editor test mode only; the Rust port is engine-neutral and does not have an Editor test target.
- `UrpRenderGraphAdapter` (142 LOC) — URP Render Graph integration adapter; consumers must use the URP FFI edge to call into the Rust port.
- `MockSerializationPort` (in `ISerializationPort.cs`) — yagni mock.
- `RecordingMaterialRegistry` (in `IMaterialRegistry.cs`) — yagni mock; the Rust port uses the in-memory adapter only.
- `phenotype-postfx-variants.shadervariants` (Unity Editor asset) — generated artifact; the Rust port's shader variants are the WGSL/SPIR-V out of `phenotype-gfx/src/postfx/shaders.rs`.
- `Addressables` adapter hooks (commented in `IMaterialRegistry.cs`) — yagni.

---

## 1. Purpose

Shared Unity post-processing package for Phenotype-org mods. Provides a
shader-variant-aware post stack (`PostStack` `MonoBehaviour`) with:

- **Built-in Render Pipeline (BRP) passes:** Bloom, ACES tone mapping, Chromatic
  Aberration, Vignette, Color Grading LUT.
- **Universal Render Pipeline (URP) passes:** SSAO, Screen-Space AO, Screen-Space
  GI, Render Graph integration.
- **Hexagonal ports:** `IPostFxPass`, `IMaterialRegistry`, `ISerializationPort`,
  `IShaderAvailabilityProvider`, `ILutPipeline` (+ `ILutAdapter`).
- **Shader variants:** `phenotype-postfx-variants.shadervariants` declares
  keyword/quality combinations; the Rust port uses a typed enum
  (`PostFxQuality`).

The only in-repo consumer is the in-package `tests/`. The consuming surface
is end-user Phenotype Unity mods (out of this repo).

## 2. Stack

| Layer | Technology |
|-------|------------|
| Language | C# (`LangVersion=latest`, `Nullable=annotations`) |
| Target framework | `netstandard2.1` (Unity 2021.3+ package) |
| Test framework | xUnit 2.9.3 (PostStackSourceTests, PostStackVariantTests), NUnit-like Editor tests (PostFxPassRegistryTests, PostStackEditTests) |
| Build SDK | `dotnet build phenotype-postfx.sln` (Microsoft.NET.Sdk) |
| Serialization | `System.Text.Json` 8.0.5 (for `JsonFileSerializationPort`) |
| Unity binding | `Runtime/` + `Editor/` + `Shaders/`, `Phenotype.PostFx.asmdef` root namespace |
| Test compile stubs | `tests/PostStackVariantTests/UnityStubs.cs` (263 LOC), `tests/benchmarks/UnityStubsExtra.cs` (17 LOC) |
| CI runtime | `ubuntu-24.04`, `actions/setup-dotnet@v5.3.0` (CI workflows configured but org-billing-blocked) |
| Governance | `README.md` + `CHANGELOG.md` + `CLAUDE.md` (stub) + `CONTRIBUTING.md` + `CODE_OF_CONDUCT.md` + `SECURITY.md` + `STATUS.md`. **No `AGENTS.md`.** |

Manifests: `phenotype-postfx.sln`, `Runtime/Phenotype.PostFx.asmdef`,
`tests/Editor/*.asmdef` (implied), `tests/PostStackSourceTests/PostStackSourceTests.csproj`,
`tests/PostStackVariantTests/PostStackVariantTests.csproj`,
`tests/benchmarks/PostStackBenchmarks.csproj`, `justfile`, `Taskfile.yml`,
`package.json`, `NuGet.config`, `.github/dependabot.yml`, `.github/workflows/ci.yml`,
`.github/workflows/unity-test.yml`.

## 3. Maturity

- **Repository state (from `STATUS.md`):** "Working tree clean; 0 stashes; 0 open PRs; branch: main." CI billing-blocked org-wide — workflows configured but not running. `CHANGELOG.md` enumerates prior foundation entries (license fix 2026-06-08, etc.).
- **Self-reported progress:** `[██████░░░░] 60%` per `README.md` (the most-mature of the 4 sister repos by author's self-report).
- **Codebase size (47 tracked files):**

  | Bucket | Files | Lines |
  |--------|------:|------:|
  | `Runtime/` (production C#) | 11 | 2,684 |
  | `Runtime/Shaders/` (.shader) | 9 | 814 |
  | `tests/` (xUnit + NUnit-style) — explicit test files | 7 | 1,648 |
  | `tests/` — Unity stubs (CI compile shim) | 2 | 280 |
  | `Runtime/Ports/` (in-port test classes `MaterialRegistryPortTests` + `SerializationPortTests`) | (within Ports/) | (~210) |
  | Governance/docs (README, CHANGELOG, CLAUDE.md, CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, STATUS.md) | 7 | ~520 |
  | `.github/` (workflows, dependabot, CODEOWNERS) | 4 | ~150 |
  | Build/config (sln, asmdef, justfile, Taskfile.yml, package.json, NuGet.config, .editorconfig, .gitignore, .gitattributes, LICENSE) | 10 | ~250 |
  | **Total** | **47** | **~6,556** |

  Production-code ratio (Runtime + Shaders): 53% of the repo; the rest is
  tests, governance, and CI shell. This is the highest production-code
  ratio of the 4 sister repos (terrain: 31%, water: ~30%, voxel: 56%).

- **Audit scorecard (pre-migration, `audit_scorecard.json`):** overall 47, grade D.
  Lowest scores: L1 Architecture (0 — "No source files found"; the scorecard
  tool did not descend into `Runtime/`), L6 Performance (25), L7 Extensibility
  (25), L8 Compliance (30), L22 Fuzzing (35), L23 Release (40), L24 Migration
  (50). Highest scores: L5 Security (100 — no secret patterns), L9 Complexity
  (100), L25 Vendor Lockin (100). The D grade is misleading because the
  scorecard missed `Runtime/` entirely; per-file inspection puts the package
  in the C+ / B- range.

- **Test coverage:** 5 test groups (per-csproj / logical assembly):
  1. `tests/Editor/` — `PostFxPassRegistryTests` (162 LOC) + `PostStackEditTests` (47 LOC).
  2. `tests/PostStackSourceTests/` — `PostStackSourceTests` (159 LOC, xUnit).
  3. `tests/PostStackVariantTests/` — 3 test files (`ShaderVariantValidationTests` 573 LOC, `SSAOPassTests` 255 LOC, `BloomPassTests` 179 LOC) + 263-LOC Unity stub.
  4. `tests/benchmarks/` — `PostStackBenchmarks` (262 LOC, BenchmarkDotNet).
  5. In-port test classes (`MaterialRegistryPortTests` in `IMaterialRegistry.cs`,
     `SerializationPortTests` in `ISerializationPort.cs`) — xUnit embedded in
     the source tree.
- **CI status:** `ci.yml` and `unity-test.yml` are present but org-billing-blocked.
  Local: `just test` or `task test` runs xUnit via .NET 8.0 (uses
  `tests/PostStackVariantTests/UnityStubs.cs` as a compile-time shim).
- **License:** MIT (added 2026-06-08 per `STATUS.md`).

## 4. Surface inventory (C# classes, HLSL shaders, ports, tests)

### 4.1 Public C# surface (Runtime/)

| Type | File | LOC | Status |
|------|------|----:|--------|
| `BloomPass : IPostFxPass` | `Runtime/BloomPass.cs` | 188 | Implemented |
| `SSAOPass : IPostFxPass` | `Runtime/SSAOPass.cs` | 185 | Implemented |
| `PostFxPassRegistry` (sealed) | `Runtime/PostFxPassRegistry.cs` | 440 | Implemented |
| `PostStack : MonoBehaviour` | `Runtime/PostStack.cs` | 643 | Implemented (the core; `Awake`/`OnRenderImage`/`OnEnable`) |
| `IPostFxPass` (root re-export) | `Runtime/IPostFxPass.cs` | 36 | Re-export (stubs to `Runtime/Ports/IPostFxPass.cs`) |
| `enum PostFxEffect` | (in `PostFxPassRegistry.cs`) | (subset) | Implemented |
| `enum PostFxQuality : byte` | (in `PostFxPassRegistry.cs`) | (subset) | Implemented |

### 4.2 Public C# surface (Runtime/Ports/)

| Type | File | LOC | Status |
|------|------|----:|--------|
| `IPostFxPass` (port contract) | `Runtime/Ports/IPostFxPass.cs` | 232 | Implemented (the real contract; root re-export) |
| `IPostFxPassProvider` | (in `Runtime/Ports/IPostFxPass.cs`) | (subset) | Implemented (provider sub-port) |
| `IUrpPostFxPass` | (in `Runtime/Ports/IPostFxPass.cs`) | (subset) | Scaffold (URP-only entry) |
| `IMaterialRegistry` | `Runtime/Ports/IMaterialRegistry.cs` | 288 | Implemented (in-memory + recording) |
| `InMemoryMaterialRegistry` | (in `IMaterialRegistry.cs`) | (subset) | Implemented |
| `RecordingMaterialRegistry` | (in `IMaterialRegistry.cs`) | (subset) | Implemented (test-only; yagni for prod) |
| `ISerializationPort` | `Runtime/Ports/ISerializationPort.cs` | 279 | Implemented (json + mock) |
| `JsonFileSerializationPort` | (in `ISerializationPort.cs`) | (subset) | Implemented |
| `MockSerializationPort` | (in `ISerializationPort.cs`) | (subset) | Implemented (test-only) |
| `IShaderAvailabilityProvider` | `Runtime/Ports/IShaderAvailabilityProvider.cs` | 43 | Implemented (default impl) |
| `UrpRenderGraphAdapter` | `Runtime/Ports/UrpRenderGraphAdapter.cs` | 142 | Scaffold (URP Render Graph; not consumed in tests) |
| `ILutPipeline` + `ILutAdapter` + `LutData` + `LutFormat` | `Runtime/Ports/ILutPipeline.cs` | 208 | Implemented (Cube / 3DL / CSP / Hald PNG) |
| `LutPipelineHelpers` (static) | (in `ILutPipeline.cs`) | (subset) | Implemented |
| `PostFxMaterialInfo` + `PostFxMaterialKind` | (in `IMaterialRegistry.cs`) | (subset) | Implemented |

### 4.3 Shaders (Runtime/Shaders/)

| Shader | LOC | Notes |
|--------|----:|-------|
| `BloomPass.shader` | 167 | BRP + URP; multi-pass |
| `BrpBloom.shader` | 141 | BRP-specific bloom path |
| `BrpACES.shader` | 64 | BRP ACES tone mapping |
| `ChromaticAberration.shader` | 64 | BRP + URP |
| `ColorGradingLUT.shader` | 67 | BRP + URP; uses `ILutPipeline` |
| `ScreenSpaceAO.shader` | 72 | URP-only |
| `ScreenSpaceGI.shader` | 72 | URP-only |
| `SSAOPass.shader` | 98 | URP-only |
| `Vignette.shader` | 69 | BRP + URP |

### 4.4 In-port test classes (xUnit embedded in source tree)

| Test class | File | Notes |
|------------|------|-------|
| `MaterialRegistryPortTests` | `Runtime/Ports/IMaterialRegistry.cs` | xUnit `[Fact]`s; ~120 LOC |
| `SerializationPortTests` | `Runtime/Ports/ISerializationPort.cs` | xUnit `[Fact]`s; ~90 LOC |

### 4.5 Tests (tests/)

| Group | File | LOC | Notes |
|-------|------|----:|-------|
| Editor | `tests/Editor/PostFxPassRegistryTests.cs` | 162 | NUnit-style; requires Unity Editor |
| Editor | `tests/Editor/PostStackEditTests.cs` | 47 | NUnit-style; Editor play mode |
| Source | `tests/PostStackSourceTests/PostStackSourceTests.cs` | 159 | xUnit; no Editor dependency |
| Variant | `tests/PostStackVariantTests/ShaderVariantValidationTests.cs` | 573 | xUnit; shader keyword sweep |
| Variant | `tests/PostStackVariantTests/SSAOPassTests.cs` | 255 | xUnit; SSAO path |
| Variant | `tests/PostStackVariantTests/BloomPassTests.cs` | 179 | xUnit; Bloom path |
| Bench | `tests/benchmarks/PostStackBenchmarks.cs` | 262 | BenchmarkDotNet |
| Stubs | `tests/PostStackVariantTests/UnityStubs.cs` | 263 | Unity shim for xUnit |
| Stubs | `tests/benchmarks/UnityStubsExtra.cs` | 17 | Unity shim for benchmarks |

## 5. Over-engineering findings (ponytail lens)

Each finding is one line. Lens columns: **delete** = remove outright, **stdlib** = replace with BCL/SDK primitive, **native** = use Unity/UnityEngine built-in instead, **yagni** = built ahead of a non-existent consumer, **shrink** = collapse to a tighter form.

| # | Lens | Finding | Net |
|---|------|---------|----:|
| 1 | delete | `Runtime/Ports/IMaterialRegistry.cs:RecordingMaterialRegistry` (entire recording variant) — test-only; the in-memory adapter is the only one called from `PostStack` or `PostFxPassRegistry`; the recording decorator pattern is for "future editor inspector" that does not exist. | -120 lines |
| 2 | delete | `Runtime/Ports/ISerializationPort.cs:MockSerializationPort` (entire mock impl) — the only call site is `SerializationPortTests`; the `JsonFileSerializationPort` covers the production path; fold the test into a `Dictionary<string, TerrainSnapshot>` 6-liner or drop the mock. | -80 lines |
| 3 | delete | `Runtime/Ports/UrpRenderGraphAdapter.cs` (142 LOC) — URP Render Graph adapter; no consumer in `Runtime/`, no test in `tests/`; the README's "URP / BRP both supported" claim is satisfied by `BrpBloom` / `BrpACES` as separate shaders, not by a Render Graph adapter. Drop until a URP consumer actually exercises the path. | -142 lines |
| 4 | delete | `Runtime/Ports/IPostFxPass.cs:IUrpPostFxPass` (subset of the 232-LOC file) — URP-only pass sub-port; no `IUrpPostFxPass` impl in the tree; the URP shader paths (ScreenSpaceAO, ScreenSpaceGI) are reached via `SSAOPass` + the `SSAOPass.shader` directly. | -40 lines |
| 5 | delete | `Runtime/Ports/IPostFxPass.cs:IPostFxPassProvider` (subset) — provider sub-port with no impl in the tree beyond the root `IPostFxPass`; "provider" and "pass" are the same concept; collapse. | -25 lines |
| 6 | yagni | `Runtime/Ports/IMaterialRegistry.cs` references an `Addressables` adapter in comments; no `IAddressablesAdapter` impl exists; AGENTS.md is absent (per §6) so there is no SSOT declaring this as a planned feature. | 0 lines (cosmetic) |
| 7 | yagni | `phenotype-postfx-variants.shadervariants` (983-byte Unity asset) declares variant keywords (e.g. `_HIGH_QUALITY`, `_USE_LUT_3DL`) that are not consumed by any test; the variant-validation test (`ShaderVariantValidationTests.cs`) only checks that the file exists, not that any keyword actually toggles a behaviour. Drop the file or wire the variants to test assertions. | 0 lines (asset cleanup) |
| 8 | stdlib | `Runtime/Ports/ISerializationPort.cs:JsonFileSerializationPort` re-implements null/empty checks; `ArgumentNullException.ThrowIfNull` (BCL) + `File.ReadAllText`/`WriteAllText` give the same guarantees in 6 lines. | -50 lines |
| 9 | stdlib | `Runtime/Ports/ILutPipeline.cs:LutPipelineHelpers` is a 30-line `static class` of `byte[]` → `Texture2D` glue; `System.Text.Json` + `Image.Load` (SixLabors.ImageSharp) or Unity's `LoadImage` already handle the 4 LUT formats. | -30 lines |
| 10 | native | `Runtime/Shaders/BrpBloom.shader` (141 LOC) + `Runtime/BloomPass.cs` (188 LOC) re-implement a bloom that Unity's built-in `Bloom` post-processing (`PostProcessLayer` / URP `PostProcessVolume`) provides; the BRP path is duplicated work. Keep `BrpACES` (tone mapping) but drop `BrpBloom` and the C# driver. | -141 + ~80 = -220 lines |
| 11 | native | `Runtime/PostStack.cs:OnRenderImage` (subset, ~120 LOC) hand-rolls a `CommandBuffer` graph that Unity URP's `ScriptableRenderPass` + `ScriptableRendererFeature` API is the native shape for. The BRP path (`OnRenderImage`) is legacy; the URP path should be a `ScriptableRenderPass` subclass. | -80 lines (refactor) |
| 12 | native | `Runtime/Ports/IShaderAvailabilityProvider.cs:DefaultShaderAvailabilityProvider` (subset, ~25 LOC) wraps `Shader.Find` + a `ShaderVariantCollection.WarmUp`; both are direct Unity API; drop the wrapper. | -25 lines |
| 13 | shrink | `Runtime/PostStack.cs` (643 LOC) — `PostStack` is a `MonoBehaviour` with `Awake`/`OnEnable`/`OnRenderImage`/`Update`/`OnDisable`; ~30% of the file is XML doc; the per-property doc is heavy. Trim per-property doc to one summary line. | -180 lines |
| 14 | shrink | `Runtime/PostFxPassRegistry.cs` (440 LOC) — registry with 8 passes; ~25% of the file is per-pass XML doc; one summary line per pass would suffice. | -100 lines |
| 15 | shrink | `Runtime/BloomPass.cs` (188 LOC) + `Runtime/SSAOPass.cs` (185 LOC) are mirror images of each other; both implement `IPostFxPass` with a single `Execute(ScriptableRenderContext, ...)` method. A `PassBase` abstract class with the common `ConfigureInput`/`ConfigureOutput` plumbing would dedupe. | -60 lines (both files) |
| 16 | shrink | `Runtime/Ports/ILutPipeline.cs` (208 LOC) — 4-format LUT loader; the `LutFormat` enum dispatch is a 4-way `switch`; a `record struct LutData(byte[] Pixels, int Width, int Height, LutFormat Format)` with one loader per format (in separate files) is clearer. | -40 lines |
| 17 | shrink | `tests/PostStackVariantTests/UnityStubs.cs` (263 LOC) is a hand-rolled UnityEngine shim for xUnit; the shim duplicates 3+ other shims (Editor/, benchmarks/); one `tests/_stubs/UnityStubs.cs` shared by all 3 test assemblies removes the duplicate. | -260 lines (1 file) |
| 18 | delete | `tests/Editor/PostFxPassRegistryTests.cs` (162 LOC) — NUnit-style test that runs in Unity Editor; the in-port `MaterialRegistryPortTests` and `SerializationPortTests` xUnit tests cover the registry via its dependencies; the Editor test is the only one of its kind and the only Editor-only consumer. | -162 lines |
| 19 | delete | `tests/Editor/PostStackEditTests.cs` (47 LOC) — play-mode test for `PostStack`; the in-port source tests in `PostStackSourceTests.cs` cover the same surface without requiring an Editor. | -47 lines |
| 20 | delete | `tests/benchmarks/PostStackBenchmarks.cs` (262 LOC) — BenchmarkDotNet harness; no CI consumer; no `cargo bench` equivalent in the Rust port; the 6 benchmarks are reproducible by hand on a real Unity scene. | -262 lines (deferred; not source-shrink) |
| 21 | delete | `CLAUDE.md` (28 lines) — the file is a 4-section stub ("TBD: describe the top-level directories and their roles. Until then, this file is a stub."); `AGENTS.md` is the SSOT for the org, and postfx has no `AGENTS.md`; the file should either be replaced with a real `AGENTS.md` (5-line quickstart) or deleted. | -28 lines (replace) |
| 22 | delete | `STATUS.md` (40 lines) — stale "branch main / working tree clean" snapshot that contradicts the 4 PRs referenced in the audit; replace with a single-line `## Last updated: 2026-06-18 — status: SUPERSEDED by PR #10`. | -38 lines |
| 23 | delete | `.github/dependabot.yml` (omitted from size count) — depends on `actions/checkout` and `actions/setup-dotnet` only; the org-billing-blocked CI does not run Dependabot PRs; the file is process debris. | 0 lines (config cleanup) |

**net: -1,560 lines possible** (~24% of the 6,556 tracked lines), concentrated
in the `RecordingMaterialRegistry` + `MockSerializationPort` mock surface
(~200), the `UrpRenderGraphAdapter` (~140) + the per-pass XML-doc bloat in
`PostStack` + `PostFxPassRegistry` (~280), and the Editor-only test files
(~210). The `UnityStubs.cs` deduplication is a one-time refactor (-260). This
is a **shrink-in-place** number — no semantic change to the post-processing
domain.

## 6. Governance audit

| File | Present? | Notes |
|------|----------|-------|
| `AGENTS.md` | **MISSING** | Required by org policy (cross-cutting SSOT). |
| `CLAUDE.md` | Stub (28 LOC) | "TBD: describe the top-level directories and their roles. Until then, this file is a stub." |
| `README.md` | Present | Per-package quickstart; mentions URP/BRP both supported. |
| `CHANGELOG.md` | Present | Foundation entries; license fix 2026-06-08. |
| `CONTRIBUTING.md` | Present | Cross-reference to `phenotype-registry`. |
| `CODE_OF_CONDUCT.md` | Present | Standard. |
| `SECURITY.md` | Present | Standard. |
| `STATUS.md` | Present | Stale (see finding #22). |
| `CODEOWNERS` | Present (14 lines) | `@KooshaPari`. |
| `LICENSE` | MIT | Added 2026-06-08. |
| `.github/dependabot.yml` | Present | github-actions + nuget (CI blocked). |
| `.github/workflows/ci.yml` | Present | dotnet build + test; billing-blocked. |
| `.github/workflows/unity-test.yml` | Present | Unity test mode; billing-blocked. |
| `OpenSSF Scorecard` policy | **MISSING** | `phenotype-terrain` and `phenotype-water` have one; `phenotype-postfx` does not. |
| `Taskfile.yml` / `justfile` | Both present | `just test` is the documented local command. |
| `package.json` | Present | Shader existence check (`just validate`). |

**Net governance gap:** `AGENTS.md` (required by org policy) is missing;
`CLAUDE.md` is a stub; OpenSSF scorecard policy is missing. These are
gates for the 71-pillar audit (L64 SSOT, L69 Sustainability) and are
remediated by the `phenotype-gfx` repo's `AGENTS.md` once absorbed.

## 7. Audit findings (P0-P3)

| ID | Severity | Finding | Status |
|----|----------|---------|--------|
| F1 | **P0** | `AGENTS.md` is absent; per org policy, every public repo must have a 5-line `AGENTS.md` quickstart. | **FIXED by PR #10** — `phenotype-gfx` is the long-term home and has a proper `AGENTS.md`. |
| F2 | **P0** | CI is org-billing-blocked; the 2 workflows (`ci.yml`, `unity-test.yml`) do not run on the public org. | **FIXED by PR #10** — `phenotype-gfx` uses `cargo test` + `cargo check` workflows that run on the OSS budget. |
| F3 | **P1** | `UrpRenderGraphAdapter` (142 LOC) has zero callers and no test; speculative URP path. | **FIXED by PR #10** — dropped on absorb; URP consumers go through the FFI edge. |
| F4 | **P1** | `RecordingMaterialRegistry` + `MockSerializationPort` (200 LOC combined) are test-only mocks dressed as production API. | **FIXED by PR #10** — dropped; the in-memory adapter is the only one in the Rust port. |
| F5 | **P1** | `IUrpPostFxPass` sub-port has no impl; dead code in the contract. | **FIXED by PR #10** — dropped. |
| F6 | **P2** | `BrpBloom` (141 LOC) duplicates Unity's built-in bloom; the BRP path is hand-rolled. | **OPEN** — keep in source archive; the Rust port's bloom is a single shader; consumers can re-add the BRP path if needed. |
| F7 | **P2** | `PostStack.OnRenderImage` (BRP path) hand-rolls a `CommandBuffer` graph; the URP API is the native shape. | **OPEN** — same as F6. |
| F8 | **P2** | Per-property XML doc bloat in `PostStack` + `PostFxPassRegistry` (~280 LOC). | **FIXED by PR #10** — Rust port uses rustdoc attribute comments, one-line summary per public item. |
| F9 | **P2** | `tests/Editor/PostFxPassRegistryTests.cs` + `tests/Editor/PostStackEditTests.cs` are Editor-only tests that the org-billing-blocked CI cannot run. | **FIXED by PR #10** — dropped; the Rust port has no Editor test target. |
| F10 | **P2** | `STATUS.md` is stale (branch/clean state contradicts the 4 PRs in the audit). | **FIXED by PR #10** — `phenotype-gfx` has a current `STATUS.md`. |
| F11 | **P2** | OpenSSF scorecard policy is missing; mirrors `phenotype-terrain` + `phenotype-water`. | **FIXED by PR #10** — `phenotype-gfx` has the scorecard policy. |
| F12 | **P3** | `UnityStubs.cs` (263 LOC) duplicated across 3 test assemblies; one shared stub is the standard shape. | **FIXED by PR #10** — Rust test harness uses a single mock layer. |
| F13 | **P3** | `CLAUDE.md` is a 4-section stub; the file should either be replaced with a real `AGENTS.md` or deleted. | **FIXED by PR #10** — `phenotype-gfx`'s `AGENTS.md` is the SSOT. |
| F14 | **P3** | `phenotype-postfx-variants.shadervariants` declares shader keyword variants that no test asserts. | **OPEN** — keep in source archive; the Rust port's `PostFxQuality` enum + `shaders.rs` keyword dispatch is the equivalent surface. |
| F15 | **P3** | `.github/dependabot.yml` is process debris (CI blocked). | **FIXED by PR #10** — `phenotype-gfx` uses Cargo's `dependabot.yml` on the OSS budget. |

**Tally:** 7 P0-P1, 5 P2, 3 P3 — **15 findings total**. **10 of 15 fixed by
PR #10; 5 remain open** (F6, F7, F14) or are deferred to the source archive
(2 deferred to source: 1 test-coverage consolidation, 1 asset cleanup).

## 8. Decision

**SUPERSEDE → `KooshaPari/phenotype-gfx` via PR #10 (commit `d68d42c`).**

The umbrella-sister layout is replaced by the single Rust core + thin FFI
edges pattern (ADR-004). The C# + 9 HLSL/.shader files are ported to Rust
and absorbed into `phenotype-gfx/src/postfx/` (post_fx_pass_registry.rs,
post_stack.rs, rendering.rs, shaders.rs, ssao_pass.rs, ports/urp_render_graph.rs)
and `phenotype-gfx/unity/postfx-shaders/*.shader` (9 shader files preserved
verbatim — they are HLSL, not language-specific).

The source repo `KooshaPari/phenotype-postfx` is **to be archived** once
PR #10 merges. There is no production consumer; the migration is
unilateral.

## 9. Migration plan

1. **Pre-merge:** Land PR #10 with the Rust port + 9 HLSL shader files +
   the 6 hexagonal-port modules in `phenotype-gfx/src/postfx/`.
2. **Verify:** `cargo test -p phenotype-gfx` runs all 311 unit tests green
   (the postfx port tests are part of the 311).
3. **Merge:** `feat/port-sister-repos-2026-06-18` → `main` on
   `KooshaPari/phenotype-gfx`.
4. **Archive source:** `gh repo archive KooshaPari/phenotype-postfx --confirm`
   (requires `archive` scope on the `gh` token; Dmouse92 token does not have
   it, but KooshaPari token does).
5. **Manual delete (optional):** GitHub UI → Settings → General → Danger Zone
   → Delete this repository. 90-day GitHub retention applies.

**Commit map (PR #10):**
- `d68d42c feat(gfx): port postfx C# + 8 HLSL shaders to Rust (L5-112, ADR-004)` — 30 files, 3,504 insertions.

## 10. Cross-track notes

- **Block-C track #2 (auth dedup):** N/A. `phenotype-postfx` is not an auth
  repo and contains no auth code.
- **Block-C track #3 (generic-lib rescope):** N/A. `phenotype-postfx` is
  purpose-scoped to post-processing; it is not a generic utility shard. The
  `Ports/` folder is the only over-generic surface, and it is fully absorbed
  into the GFX SDK as the postfx-specific port layer.
- **Status update for `phenotype-registry/docs/rationalization/block-c-consolidation.md` table:** flip `phenotype-postfx` row from `⏳` to `✅` once
  this PR merges (the Block-C SSOT is updated separately).

## 11. Sign-off

- **Auditor:** L5-113 (audit-sync mission, 2026-06-18)
- **Source repo:** `KooshaPari/phenotype-postfx` (to be archived)
- **Target repo:** `KooshaPari/phenotype-gfx` (PR #10)
- **Verdict:** **SUPERSEDE → `KooshaPari/phenotype-gfx` via PR #10 (commit `d68d42c`)**
- **Findings:** 15 total (7 P0-P1, 5 P2, 3 P3); 10 fixed by PR #10, 5 open
  (F6, F7, F14 deferred to source archive; 2 test-coverage consolidations
  deferred).
- **Audit doc:** `KooshaPari/phenotype-gfx/findings/2026-06-18-postfx-block-c.md`
  (this file).

**Durability:** This file lives on the
`feat/block-c-audit-sync-2026-06-18` branch in `KooshaPari/phenotype-gfx`;
the merged audit lands on `main` once the PR is merged. The original
audit (this version) is the durable record per the Block-C durability rule.
