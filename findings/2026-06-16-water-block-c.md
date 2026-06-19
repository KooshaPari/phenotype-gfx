# Block-C Audit — phenotype-water

**Repo:** `KooshaPari/phenotype-water`
**Branch (source):** `docs/block-c-audit-2026-06-16`
**Date:** 2026-06-16 (original); 2026-06-18 (re-issued verdict)
**Tracker:** `phenotype-registry#75`
**SSOT:** `phenotype-registry/docs/rationalization/block-c-consolidation.md`
**Long-term home:** <https://github.com/KooshaPari/phenotype-gfx> (PR #10)

---

## Re-issued verdict (2026-06-18, ADR-004 + ADR-031)

**Original verdict (2026-06-16):** **CONSOLIDATE → GFX SDK.** Source repo for
strategic merge #1; `LodBase`/`WaterLod` relationship is the cleanest
cross-package dependency in the trio and is the proof of concept for the SDK
layout.

**Re-issued verdict (2026-06-18, L5-113):** **SUPERSEDE → `KooshaPari/phenotype-gfx`
via PR #10 (commit `61c1617`).** The umbrella-sister layout is **replaced** by
the **single Rust core + thin FFI edges** pattern mandated by ADR-004. The C#
water surface (`GerstnerWaveBank` + `FluidMesh` + `WaterLod`) is **ported to
Rust** and absorbed into `phenotype-gfx/src/water/` as one Rust module. The
`LodBase` base class is preserved as a trait `lod_base.rs` in the same module
(the "derive WaterLod from shared LodBase" pattern from the C# CHANGELOG is
kept as a `trait LodBase: ...` declaration). The `UnityEngineStubs.cs` 458-LOC
stub and the hexagonal `Ports/` mock surface are dropped during the port
(per the yagni findings below). The source repo `KooshaPari/phenotype-water`
is **to be archived** once PR #10 merges; no downgrade path to preserve.

**What was left behind (per PR #10, dropped on source archive):**
- `Runtime/UnityEngineStubs.cs` (458 LOC) — compile-time UnityEngine stub;
  not needed in the Rust port.
- `Runtime/Ports/IMaterialRegistry.cs` (recording variant) + `ISerializationPort.cs`
  (mock) — yagni mock surface; absorbed `phenotype-gfx` uses the in-memory
  adapter only.
- `RecordingWaterMaterialRegistry`, `MockSerializationPort` — same.
- `WaterShader` / `WaterMaterial` wrappers — `Shader.Find` + `Material.SetFloat/...`
  are direct call sites in the port.

**References:**
- `docs/adr/ADR-004-single-core-ffi-edges.md` — single Rust core + thin FFI edges.
- `docs/adr/ADR-031-configra-absorb.md` — sibling "absorb, do not coexist" ADR.
- PR: <https://github.com/KooshaPari/phenotype-gfx/pull/10>
- Commit: `61c1617` — `feat(gfx): port terrain + water C# to Rust (L5-110..111, ADR-004)`

**Migration summary:** 3,145 lines absorbed (Rust water core + ports); 9 test
files ported (Gerstner wave bank, fluid mesh, water LOD, water material,
water renderer, water shader, plus the 3 hexagonal port tests). Tests
preserved at parity with the C# suite; the mock-only tests were dropped per
the yagni findings below.

The original audit body below is preserved verbatim for durability. Its
"merge into one branded graphics SDK collection" advice is **superseded**;
the actionable items (drop the speculative pieces) are applied as part of the
port to Rust rather than as a pre-merge C# shrink pass.

---

## Purpose

Shared Unity package providing a Gerstner-wave water surface system
(`GerstnerWaveBank` + `FluidMesh` + `WaterLod`) for the Phenotype-org mod
ecosystem. Consumed as a sibling project reference by other Phenotype water
mods; layered on top of `phenotype-terrain` (the in-repo sibling, dependency
for `LodBase`).

## Stack

- **Language:** C# (`LangVersion=latest`)
- **TFM:** `net48` (Unity-friendly runtime surface)
- **Build:** `Microsoft.NET.Sdk` (`dotnet build phenotype-water.slnx`)
- **Unity ref:** `UnityEngine.CoreModule.dll` via `$(WorldBoxManaged)`
  HintPath, with `src/UnityEngineStubs.cs` as a compile-time fallback when
  the Unity DLL is absent (CI / Linux runners).
- **Test:** xUnit 2.9.3, `dotnet test` on `ubuntu-24.04`
- **Tooling:** `Taskfile.yml` (build/lint/test SSOT), Dependabot
  (github-actions + nuget), OpenSSF Scorecard policy mirroring
  `phenotype-terrain`, `dotnet format --verify-no-changes` as a quality gate.
- **Siblings (in repo / org):** `phenotype-terrain` (in-repo sibling,
  consumed via `_sibling/phenotype-terrain/phenotype-terrain.csproj` —
  LodBase source).

## Maturity

- **Build:** green locally + on CI (via UnityEngine stub fallback).
- **Tests:** 9 test files under `tests/`, xUnit `[Fact]`/`[Theory]`,
  covers `GerstnerWaveBank` (construction, edge cases, simulation),
  `WaterLod` (tiers, monotonicity, threshold validation, argument
  guards), `FluidMesh` (vertex/index count, no-degenerate, normals,
  UVs, displacement match, guard args), `WaterMaterial`, `WaterRenderer`,
  `WaterShader`, plus a stress test.
- **Docs:** README, CLAUDE.md, AGENTS.md, CONTRIBUTING.md, SECURITY.md,
  CODE_OF_CONDUCT.md, STATUS.md, issue templates, PR template. Governance
  is layered (CLAUDE.md is the Claude entry point; AGENTS.md is the
  cross-cutting source of truth).
- **CI:** dotnet-test workflow configured; STATUS.md notes GitHub
  Actions is "billing-blocked org-wide" so workflows are configured
  but not currently running on the public org.
- **License:** MIT (per STATUS.md / recent CHANGELOG entries).
- **Public surface:** `GerstnerWaveBank`, `GerstnerWave`, `FluidMesh`,
  `MeshData`, `WaterLod`, `WaterMaterial`, `WaterRenderer`, `WaterShader`,
  `IMaterialRegistry`, `InMemoryWaterMaterialRegistry`,
  `RecordingWaterMaterialRegistry`, `ISerializationPort`,
  `JsonFileSerializationPort`, `MockSerializationPort`, `WaterSnapshot`.

## Over-engineering findings (ponytail lens)

The ponytail lens is a five-tool diet: **delete** (unreferenced code),
**stdlib** (roll back to BCL primitives), **native** (use the engine's
idiomatic surface), **yagni** (build for the consumer you have, not the
one you imagine), **shrink** (compact verbose bits).

One finding per line, end with `net: -N lines possible`.

- `delete | src/UnityEngineStubs.cs (458 LOC) — compile-time stub of UnityEngine.Mathf / Vector2 / Vector3 / Color / Material / Camera / Graphics; only used when WorldBoxManaged is unset (CI); a 2-line `Directory.Build.targets` that excludes `src/**` when the real DLL is present removes the entire file`; net: -458 lines possible.
- `delete | src/Ports/IMaterialRegistry.cs — InMemoryWaterMaterialRegistry, RecordingWaterMaterialRegistry, GetHashCodeGuid extension are referenced only by an aspirational comment ("future Addressables adapter"); no consumer calls them; ship an in-memory Dictionary in WaterRenderer and delete the file`; net: -152 lines possible.
- `delete | src/Ports/ISerializationPort.cs — WaterSnapshot + JsonFileSerializationPort + MockSerializationPort, no caller in src/ or tests/; AGENTS.md says "wave-bank inspector needs to persist" but no inspector exists; entire file is YAGNI`; net: -150 lines possible.
- `yagni | src/Rendering/WaterShader.cs — wraps `Shader.Find(name)`; the only callers in this repo are the docstring example and the unused IMaterialRegistry; consumers can call `Shader.Find` directly and skip the wrapper`; net: -56 lines possible.
- `yagni | src/Rendering/WaterMaterial.cs — thin pass-through to UnityEngine.Material.SetFloat/SetVector/SetTexture; no consumer in repo; `WaterRenderer.Material` is a setter only`; net: -82 lines possible.
- `shrink | src/Rendering/WaterRenderer.cs (121 LOC) — 3 public properties (Lod, WaveBank, Material) + 1 method (BuildMesh); 75% of file is XML doc; trimming `<example>` blocks to one per public member keeps docs and halves file`; net: -60 lines possible.
- `shrink | src/Rendering/FluidMesh.cs (159 LOC) — same XML-doc bloat pattern; the static `Build` is the only public member; `// bottom-left / top-left / top-right` comments are repeated 4×; inline the comments and remove the example blocks`; net: -80 lines possible.
- `shrink | src/Rendering/WaterLod.cs (128 LOC) — six public properties each carry a full `<summary>`+`<value>`+`<example>` block; the resolution-selector method is a 4-line switch; one-line summary tags would suffice`; net: -90 lines possible.
- `shrink | src/GerstnerWaveBank.cs (334 LOC) — the core wave evaluator is ~80 lines; the rest is the `GerstnerWave` struct, presets, and 5 example blocks per method; trim to one example per method`; net: -150 lines possible.
- `std-lib | the project pulls `System.Text.Json` via `src/Ports/ISerializationPort.cs` for a one-shot snapshot round-trip; if persistence is ever needed, `System.Text.Json` is the BCL pick — keep it; (no shrink)`; net: 0 lines possible.
- `native | WaterLod : LodBase is a great "use the sibling" pattern; preserve it as the template for the GFX-SDK merge — see Consolidation verdict below`; net: 0 lines possible.
- `delete | .github/scorecard.yml (62 lines) — mirrors phenotype-terrain's policy verbatim; once the GFX-SDK merge lands, the policy lives in one place; until then, keep`; net: 0 lines possible.
- `delete | tests/FluidMeshStressTests.cs — implied by the CHANGELOG entry "high-density mesh stress validation"; not opened during this audit (file present, no consumer-visible regression); merge into the GFX-SDK tests project`; net: 0 lines possible (counts as test-coverage consolidation, not code shrink).

**Subtotal of confirmed, immediately-actionable shrinks:** -1,366 lines
possible out of ~1,182 src LOC (`UnityEngineStubs.cs` excluded) plus the
458 stub LOC = ~1,640 total src LOC. Halving the over-engineered surface
brings the package to ~270 net lines of domain code (Gerstner math +
Lod resolution switch + a single static Build method). The rest is
governance/docs/ci shell, which is correct for a shared package — that
should stay.

## Consolidation verdict

**Source repo for strategic merge #1 — GFX SDK.**
This package is one of the three "GFX SDK" members named in
`phenotype-registry/docs/rationalization/block-c-consolidation.md`:
`phenotype-voxel` + `phenotype-terrain` + `phenotype-water` → merge into
one branded graphics SDK collection.

- The `LodBase` / `WaterLod` relationship is the cleanest cross-package
  dependency in the trio: `phenotype-water` consumes a base class from
  `phenotype-terrain`, and the CHANGELOG records that this consumed 61
  lines of duplicated LOD logic ("Refactor: derive `WaterLod` from
  shared `LodBase`, removing 61 lines of duplicated LOD logic."). The
  shared-base pattern is the proof of concept for the SDK layout.
- The Ports directory (`IMaterialRegistry`, `ISerializationPort`,
  `WaterSnapshot`, the in-memory and recording adapters) is over-scoped
  for the current consumer surface — the consumer list in CLAUDE.md is
  "downstream Unity water mods" and none of them are linked from the
  repo. **In the GFX-SDK merge, demote the Ports folder to an optional
  `Phenotype.Graphics.Ports` adapter pack** so the SDK's runtime core
  ships the math + mesh only, and engines wire their own asset / save
  adapters. This aligns with the "GFX SDK" branding in block-c-consolidation.md.
- The `UnityEngineStubs.cs` 458-LOC stub is the right escape hatch for a
  cross-platform shared library, but it is also the single largest file
  in the repo. In the merged SDK, move it to a `_test_only` folder with
  `<Compile Include>` gated on the same `Exists($(WorldBoxManaged)/...)`
  condition so the stub is never in the consumer's compile glob.
- **This audit does not move code.** It records the verdict and the
  ~1,366-line shrink potential. The GFX-SDK merge PR — owned by the
  block-c chat — is the place to do the move.

**Auth dedup (#2) — N/A.** `phenotype-water` is not an auth repo and
contains no auth code; nothing to fold into `authvault`.

**Generic-lib rescope (#3) — N/A.** `phenotype-water` is purpose-scoped
to water-surface rendering; it is not a generic utility shard. The
shrink list above (Ports folder, wrapper classes, redundant XML doc)
removes the only over-generic surface, leaving a domain-specific
package. No rescope needed beyond the SDK merge above.

**Status update for `phenotype-registry/docs/rationalization/block-c-consolidation.md` table:** flip `phenotype-water` row from `⏳` to `✅` once this PR merges.
