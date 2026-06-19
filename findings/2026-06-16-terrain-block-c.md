# Block-C Audit — phenotype-terrain

**Audit date:** 2026-06-16 (original); 2026-06-18 (re-issued verdict)
**Auditor:** Block-C chat (per-repo audit pass)
**Tracker:** [phenotype-registry#75 (SSOT)](https://github.com/KooshaPari/phenotype-registry/pull/75) — strategic merge #1 (GFX SDK)
**Verdict at a glance:** **SUPERSEDE → `KooshaPari/phenotype-gfx` via PR #10 (commit `61c1617`).** This repo is a net-donor of design ideas (hexagonal ports, `net48`/`$(WorldBoxManaged)` contract) but a net-sink of speculative surface area (mock ports, docstring bloat, hand-rolled Unity stub). The C# `terrain` core is ported to Rust and absorbed into `phenotype-gfx/src/terrain/` (per ADR-004, single Rust core + thin FFI edges). The source repo `KooshaPari/phenotype-terrain` is **to be archived** once PR #10 merges; no downgrade path to preserve.
**Long-term home:** <https://github.com/KooshaPari/phenotype-gfx> (PR #10)

---

## Re-issued verdict (2026-06-18, ADR-004 + ADR-031)

**Original verdict (2026-06-16):** **CONSOLIDATE → GFX SDK.** Merge alongside
`phenotype-voxel` + `phenotype-water` into a single `phenotype-gfx` umbrella
crate collection, preserving the C#-only Unity-consumer path documented in
the README.

**Re-issued verdict (2026-06-18, L5-113):** **SUPERSEDE → `KooshaPari/phenotype-gfx`
via PR #10 (commit `61c1617`).** The umbrella-sister layout is **replaced** by
the **single Rust core + thin FFI edges** pattern mandated by ADR-004. The C#
implementation is **ported to Rust** and absorbed into
`phenotype-gfx/src/terrain/` as one Rust module (height-field, chunk mesh
builder, terrain LOD, the hexagonal `IMaterialRegistry` + `ISerializationPort`
ports with their non-mock adapters). The C# build surface (`net48`,
`$(WorldBoxManaged)`, `unityyamlmerge`) is **not** preserved at the SDK level;
the SDK's engine bindings are thin FFI edges per ADR-004, not C# projects.

**What was left behind (per PR #10, dropped on source archive):**
- `RecordingTerrainMaterialRegistry`, `MockSerializationPort` — yagni mock
  surface; absorbed `phenotype-gfx` uses the in-memory adapter only.
- `BuildMesh(HeightField, …)` no-op overload — the HeightField data model
  was never implemented in the source repo; not a regression in the Rust port.
- `_stub/` + `scripts/generate-unity-stub.sh` (~165 lines) — hand-rolled
  `UnityEngine` struct shapes; `phenotype-gfx` does not need them.
- `.github/scorecard.yml`, `STATUS.md`, `SUPPORT.md`, root `CODEOWNERS`,
  README "Description/Install/Usage" duplication — process debris.

**References:**
- `docs/adr/ADR-004-single-core-ffi-edges.md` — single Rust core + thin FFI edges.
- `docs/adr/ADR-031-configra-absorb.md` — sibling "absorb, do not coexist" ADR.
- PR: <https://github.com/KooshaPari/phenotype-gfx/pull/10>
- Commit: `61c1617` — `feat(gfx): port terrain + water C# to Rust (L5-110..111, ADR-004)`

**Migration summary:** 2,682 lines absorbed (Rust terrain core + ports); 7 test
files ported (mesh builder, LOD, material, serialization, plus the 4 hexagonal
port tests). Tests preserved at parity with the C# suite; the mock-only tests
were dropped per the yagni findings below.

The original audit body below is preserved verbatim for durability. Its
"merge into one branded graphics SDK collection" advice is **superseded**;
the actionable items (drop the speculative pieces) are applied as part of the
port to Rust rather than as a pre-merge C# shrink pass.

---

## 1. Purpose

Shared Unity terrain mesh infrastructure for Phenotype-org mods targeting Unity / WorldBox. Three concerns:

- **Height-field storage** — `HeightField` (per-tile elevation + bounds-checked Y queries).
- **Chunk mesh generation** — `ChunkMeshBuilder` (Unity `Mesh`-shaped `MeshData` DTO from a height field or a flat grid).
- **Level-of-detail selection** — `TerrainLod` + abstract `LodBase` (camera-distance → tier → resolution).

Additionally, the package ships two **hexagonal ports** that are nominally engine-agnostic adapters for editor/save/load use cases:

- `IMaterialRegistry` (material lookup; in-memory + recording adapters).
- `ISerializationPort` (terrain snapshot save/load; JSON file + mock adapters).

The only in-repo consumer declared in `AGENTS.md` is the sibling `phenotype-water`; downstream consumers are end-user Phenotype Unity mods (out of this repo).

## 2. Stack

| Layer | Technology |
|-------|------------|
| Language | C# (LangVersion 10) |
| Target framework | `net48` (Unity-friendly runtime surface) |
| Build SDK | Microsoft.NET.Sdk, .NET 8.0 host (LangVersion pin, `Nullable: annotations`) |
| Test framework | xUnit 2.9.3 + `xunit.runner.visualstudio` 2.8.2 + `Microsoft.NET.Test.Sdk` 17.12.0 |
| CI runtime | ubuntu-24.04, `actions/setup-dotnet@v5.3.0`, `mono-complete` for net48 test host |
| Serialization | `System.Text.Json` 8.0.5 |
| Unity binding | Conditional `<Reference>` to `$(WorldBoxManaged)/UnityEngine.CoreModule.dll` (no Editor-only API allowed) |
| Off-line CI | Hand-rolled `UnityEngine.CoreModule` stub built from `_stub/Vector3.cs`/`_stub/Vector2.cs`/`_stub/Color.cs` via `scripts/generate-unity-stub.sh` (PE-format `net8.0` assembly, `AssemblyName=UnityEngine.CoreModule`) |
| Governance | `AGENTS.md` (source of truth) + `CLAUDE.md` (Claude entry point) + `CONTRIBUTING.md` (AgilePlus spec mandate) + `CODE_OF_CONDUCT.md` + `SECURITY.md` + issue/PR templates + OpenSSF Scorecard policy |

Manifests: `phenotype-terrain.csproj` (lib), `tests/phenotype-terrain.tests.csproj`, `NuGet.config`, `Taskfile.yml`, `.editorconfig`, `.gitattributes`, `.github/dependabot.yml`, `.github/scorecard.yml`.

## 3. Maturity

- **Repository state (from `STATUS.md`):** "Working tree clean; 0 stashes; 0 open PRs; branch: master." CI billing-blocked org-wide — workflows configured but not running.
- **Shallow clone tip:** 1 commit (`abdd274 chore(deps): bump actions/setup-dotnet from 4.3.1 to 5.3.0`). `CHANGELOG.md` `[Unreleased]` enumerates ~15 prior foundation entries (license fix, doc hygiene, CI hardening) but none are in the clone's history. The repo is **scaffold-mature** (governance + tests + CI hooks in place) and **implementation-immature** (1 shipped feature path: the `BuildMesh(HeightField,…)` overload is a one-liner that delegates to the flat overload and explicitly notes "For now, HeightField is a stub with no data").
- **Self-reported progress:** `[██░░░░░░░░] 20%` in `README.md`.
- **Codebase size (47 tracked files, 3 698 total lines):**

  | Bucket | Files | Lines |
  |--------|------:|------:|
  | `src/` (production) | 7 | 1 154 |
  | `tests/` (xUnit) | 7 | 1 125 |
  | Governance/docs (README, CHANGELOG, CLAUDE.md, AGENTS.md, CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, SUPPORT.md, STATUS.md) | 9 | 405 |
  | `.github/` (workflows, issue templates, PR template, dependabot, scorecard, CODEOWNERS) | 8 | 282 |
  | `_stub/` (UnityEngine stub) | 4 | 62 |
  | `scripts/` (stub generator) | 1 | 98 |
  | Build/config (csproj, .editorconfig, .gitattributes, .gitignore, NuGet.config, LICENSE) | 6 | 128 |
  | **Total** | **47** | **3 698** |

  Production-code ratio: 31% of the repo is `src/`+`tests/`; the rest is governance, stub infra, and process docs.

## 4. Over-engineering findings (ponytail lens)

Each finding is one line. Lens columns: **delete** = remove outright, **stdlib** = replace with BCL/SDK primitive, **native** = use Unity/UnityEngine built-in instead, **yagni** = built ahead of a non-existent consumer, **shrink** = collapse to a tighter form.

| Lens | Finding |
|------|---------|
| delete | `src/Ports/IMaterialRegistry.cs:100-146` `RecordingTerrainMaterialRegistry` mock is a 47-line near-duplicate of the `InMemory` adapter; the only call site is `TerrainPortsTests.cs:64-75` which is the only thing exercising the recording logic — fold into `InMemory` with an opt-in `bool record` flag, or drop the mock entirely. |
| delete | `src/Ports/ISerializationPort.cs:144-176` `MockSerializationPort` is a 33-line bespoke stub for "stage-load / capture-save" that the BCL test-doubles pattern (or a plain `Dictionary<string,TerrainSnapshot>`) handles in 6 lines; delete. |
| delete | `.github/scorecard.yml` (90 lines) defines policies for a `scorecard-action` workflow that does not exist in `.github/workflows/` (only `dotnet-build.yml` is present) and CI is billing-blocked per `STATUS.md` — dead policy. |
| delete | `STATUS.md` (33 lines) is a stale "branch master / working tree clean" snapshot that contradicts itself ("Open PRs: 0" while 4 PRs exist on the upstream) and points at `phenotype-org-governance/SUPERSEDED.md`, which is itself a superseded path; `AGENTS.md` is the live governance. |
| delete | `SUPPORT.md` (10 lines) — GitHub Discussions / the issue templates are the support surfaces; this is a thin stub duplicating `CONTRIBUTING.md` "Reporting issues" guidance. |
| delete | Root-level `CODEOWNERS` (6 lines) duplicates `.github/CODEOWNERS` (8 lines); the `.github/CODEOWNERS` is the conventional location and is already the one GitHub reads. |
| delete | `README.md:72-88` "Description" / "Install" / "Usage" sections restate `README.md:50-69` "Build" / "Consuming from another mod" verbatim; the last `<!-- ci-refresh -->` HTML comment is also process noise. |
| delete | `src/ChunkMeshBuilder.cs:179-186` `BuildMesh(HeightField, int, float)` is a no-op that delegates to the flat overload and explicitly admits the height field is unused ("For now, HeightField is a stub with no data") — drop the overload, call `BuildMesh(int,float)` from the (single) test directly. |
| stdlib | `src/Materials/TerrainMaterialProperty.cs:50-249` reimplements discriminated-union getters/setters with manual `Type` checks + `InvalidOperationException`; one `record` per kind (`record FloatProp(string Name, float Value)`) cuts ~100 lines and gives `with`-expression ergonomics for free. |
| stdlib | `src/Ports/ISerializationPort.cs:99-138` `JsonFileSerializationPort` re-implements null/empty checks; `ArgumentNullException.ThrowIfNull` (BCL) + `File.ReadAllText` (already there) give the same guarantees in 6 lines. |
| stdlib | `src/LodBase.cs:113-122` `SelectTier` is a 6-line if-ladder for monotonic thresholds; a `static readonly (float, LodTier)[]` lookup or `Math.Clamp(distance / maxDist * tierCount)` collapses it. |
| native | `_stub/Vector3.cs` / `_stub/Vector2.cs` / `_stub/Color.cs` + `scripts/generate-unity-stub.sh` (~165 lines) hand-roll `UnityEngine` struct shapes so the HintPath reference resolves; Unity ships a `UnityEngine` reference-assembly package on NuGet (`UnityEngine.Modules`) that provides exactly this without a custom project — drop the stub dir and the script. |
| native | `src/ChunkMeshBuilder.cs:99-186` builds intermediate parallel `Vector3[]`/`int[]` arrays and returns a `MeshData` DTO; the only consumer is the test that asserts on counts. Unity's own `Mesh.SetVertices`/`SetIndices`/`SetUVs`/`SetNormals` is the native equivalent and is what `phenotype-voxel` already uses per `src/Ports/IMaterialRegistry.cs:12` reference. |
| yagni | The whole `src/Materials/` (3 files, 566 lines: `TerrainMaterial` 281 + `TerrainMaterialProperty` 251 + `TerrainMaterialPropertyType` 34) and `src/Ports/IMaterialRegistry.cs` (147 lines) ship a hexagonal material-registry port with two adapters (in-memory + recording) and an `AddressablesTerrainMaterialRegistry` that the comment marks as "future" — there is no editor inspector, no Unity Addressables adapter, no cloud asset backend wired in, and no consumer test outside the in-repo xUnit suite. |
| yagni | `src/Ports/ISerializationPort.cs` (177 lines) ships a JSON adapter + mock + snapshot DTO for an editor save/load flow that does not exist in this repo — the comment says "Used by the editor inspector and by the cloud-save CLI exporter" but neither lives in `phenotype-terrain`. |
| yagni | `.github/workflows/dotnet-build.yml:57` `dotnet format --verify-no-changes` runs on a billing-blocked workflow per `STATUS.md` — dead config. |
| yagni | `AGENTS.md:12-17` "Quick Links" reference `phenotype-water` as the only in-repo sibling, but there is no `phenotype-water` directory inside this repo (the AGENTS file itself is the only place the name appears); the cross-repo consumer wiring is aspirational. |
| shrink | `src/TerrainLod.cs` is 139 lines, of which ~50 are per-property XML doc + `<example>` blocks for trivial scalar getters; keep the class summary, drop the per-property doc unless the property encodes a non-obvious invariant. |
| shrink | `src/ChunkMeshBuilder.cs:27-55` `MeshData` is four parallel `Vector3[]` / `int[]` / `Vector2[]` / `Vector3[]` with a docstring explaining the parallelism; a `record struct MeshData(Vector3[] Vertices, int[] Indices, Vector2[] UVs, Vector3[] Normals)` is 1 line + the constructor. |
| shrink | `src/LodBase.cs:127-145` `ValidateThresholds` is documented as "Call after modifying distances to ensure consistency" but is never called from `SelectTier` or any constructor — drop or wire it in. |
| shrink | `src/Materials/TerrainMaterialPropertyType.cs:21-32` is a 4-value enum whose `<summary>` + `<remarks>` are pure docstrings (~10 lines, no runtime behaviour). |
| shrink | `src/TerrainLod.cs:127-137` `SelectResolution` `switch` expression ends with a `default => 0` arm that is unreachable because `SelectTier` returns one of the 4 named values or throws — delete the dead arm. |

**net: -400 lines possible** (≈ -11% of the 3 698 tracked lines), concentrated in the hexagonal-port mock surface (≈ -80), the hand-rolled Unity stub (≈ -165 once `UnityEngine.Modules` is adopted), and the per-property XML-doc bloat in `TerrainLod`/`TerrainMaterialProperty`/the no-op `BuildMesh(HeightField,…)` overload (≈ -100), with the remainder from governance-doc trimming and `record struct` collapses. This is a **shrink-in-place** number — no semantic change to the height-field / mesh / LOD domain.

## 5. Consolidation verdict

**Action: CONSOLIDATE → GFX SDK (Block-C strategic merge #1).**

Reference: [`phenotype-registry/docs/rationalization/block-c-consolidation.md`](../phenotype-registry/docs/rationalization/block-c-consolidation.md) → table row 1 (GFX SDK, members: phenotype-voxel + phenotype-terrain + phenotype-water, verdict: "Merge into one branded graphics SDK collection").

`phenotype-terrain` is the **terrain flank** of the GFX SDK trio:

- `phenotype-voxel` — volume data (Rust port referenced at `src/Ports/IMaterialRegistry.cs:12` and `src/Ports/ISerializationPort.cs:12`).
- `phenotype-terrain` — height-field + chunk mesh + LOD (this repo).
- `phenotype-water` — fluid / water surface; declared as the only in-repo sibling consumer of `phenotype-terrain` in `AGENTS.md:7`.

The shared hexagonal-port motif (`IMaterialRegistry`, `ISerializationPort`) is a strong signal that these three repos are one design surface expressed in three language bindings (C# / C# / C# here, with a Rust port in `phenotype-voxel`). Merging preserves the C#-only Unity-consumer path that `phenotype-terrain` already documents in its README.

### What to bring into the GFX SDK

- The **`HeightField` / `ChunkMeshBuilder` / `LodBase` + `TerrainLod`** core: keep verbatim, just relocate to the merged SDK's namespace.
- The **`net48` / `$(WorldBoxManaged)` / `unityyamlmerge`** contract: this is the only C# / Unity binding convention in the trio; promote to GFX SDK build defaults.
- The **hexagonal-port pattern** (`IMaterialRegistry`, `ISerializationPort`): port to the GFX SDK as the cross-cutting material / save-load surface, but use the BCL `record struct`/`OneOf` shape (see §4 stdlib findings) and drop the mock-only adapters that `phenotype-terrain` carries alone.

### What to leave behind

- `RecordingTerrainMaterialRegistry`, `MockSerializationPort` (yagni) — the SDK mock suite should come from a shared test-helpers package, not a per-domain mock.
- `_stub/` + `scripts/generate-unity-stub.sh` (native) — `UnityEngine.Modules` from NuGet is the canonical answer; the GFX SDK should ship one shared stub, not three.
- `.github/scorecard.yml`, `STATUS.md`, `SUPPORT.md`, root `CODEOWNERS`, README "Description/Install/Usage" duplication (delete) — process debris that does not survive the SDK merge.
- The **per-property XML-doc bloat** in `TerrainLod` / `TerrainMaterialProperty` (shrink) — let the SDK enforce one doc-style across members.

### Cross-track notes

- The GFX SDK merge does **not** touch Block-C tracks #2 (auth dedup) or #3 (generic-lib rescope); `phenotype-terrain` has no auth surface and no generic-utility surface.
- The only in-scope other finding is the **note in `AGENTS.md:7`** that lists `phenotype-water` as the sole in-repo consumer. During the GFX merge, this becomes a same-SDK consumer and the AGENTS doc is rewritten against the merged package.

### Final status (mirrors `block-c-consolidation.md` row)

| Repo | Audit landed | Notes |
|------|--------------|-------|
| phenotype-terrain | ✅ this PR | GFX merge source (see Block-C #1) — `docs/audit/BLOCK-C-AUDIT.md` |

---

**Durability:** This file lives on the `docs/block-c-audit-2026-06-16` branch in this repo; the merged audit lands on `main` once the PR is merged. Mirror update to `phenotype-registry/docs/rationalization/block-c-consolidation.md` row 31 (phenotype-terrain) flips from `⏳` to `✅` in the GFX merge PR, not here.
