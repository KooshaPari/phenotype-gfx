# ADR-003: phenotype-gfx as Monorepo Root (per-language subpackages)

**Date:** 2026-06-17
**Status:** Accepted
**Supersedes:** ADR-003 (original) — single Rust crate scaffold on `feat/scaffold-crate` (PR #2)

---

## Context

`phenotype-gfx` is the consolidation point for all Phenotype-org graphics microlibs:
- `phenotype-voxel` (Rust) — SVO + dense leaf chunks, dirty queue, per-engine mesher
- `phenotype-terrain` (Rust/C#) — terrain generation
- `phenotype-water` (Rust/C#) — water simulation
- `phenotype-postfx` (C#/TS) — post-processing effects

Consumer patterns span multiple languages:
- **Civis** (Bevy/Rust desktop) — consumes Rust crates directly
- **WSM3D** (Unity 2022.3 BRP, D3D11) — consumes C# packages / DLLs
- In-tree Civis modules (`lod.rs`, `material_pbr.rs`, `scale_budget.rs`) are Rust

A single Rust crate cannot host C# or TypeScript subsystems. The original PR #2 scaffold
was a Rust-only crate — correct for the kernel but insufficient for the full consolidation.

---

## Decision

Scaffold `phenotype-gfx` as a **monorepo root** with per-language subpackages:

```
phenotype-gfx/
├── rust/
│   ├── Cargo.toml          # Cargo workspace root
│   ├── voxel/              # subtree: phenotype-voxel (existing)
│   └── phenotype-gfx-voxel/  # NEW unified voxel+streaming crate
│       └── src/
│           ├── lib.rs      # module stubs
│           ├── voxel.rs    # stub → phenotype-voxel fold
│           └── streaming.rs # stub → Civis lod/material/scale fold
├── csharp/
│   ├── phenotype-gfx-csharp.csproj  # .NET Standard 2.1
│   └── src/                # C# subsystems (postfx, voxelizer, lighting, LOD, …)
├── ts/
│   ├── package.json        # @phenotype/gfx-postfx placeholder
│   └── README.md
├── docs/
│   ├── adr/                # Architecture Decision Records
│   └── migration/          # MATRIX.md, DIVERGENCE.md, etc.
├── spec/                   # existing spec files
├── unity/                  # existing Unity subtree
└── VERSION.toml
```

---

## Per-subsystem fold plan

| Subsystem       | Language | Source                          | Target                              | Status  |
|-----------------|----------|---------------------------------|-------------------------------------|---------|
| voxel kernel    | Rust     | `phenotype-voxel`               | `rust/phenotype-gfx-voxel/src/voxel.rs`     | todo    |
| streaming/LOD   | Rust     | Civis `lod.rs`, `scale_budget.rs` | `rust/phenotype-gfx-voxel/src/streaming.rs` | todo    |
| material PBR    | Rust     | Civis `material_pbr.rs`         | `rust/phenotype-gfx-voxel/`         | todo    |
| lighting        | C#       | WSM3D / postfx                  | `csharp/src/`                       | todo    |
| LOD (C#)        | C#       | WSM3D                           | `csharp/src/`                       | todo    |
| rendering       | C#       | WSM3D                           | `csharp/src/`                       | todo    |
| foliage         | C#       | WSM3D / phenotype-terrain       | `csharp/src/`                       | todo    |
| procgen         | C#       | WSM3D / phenotype-terrain       | `csharp/src/`                       | todo    |
| postfx          | C# / TS  | `phenotype-postfx`              | `csharp/src/` or `ts/`             | todo    |

---

## Rationale

- **Per-language isolation** enables clean CI/CD and independent builds per language toolchain.
- **Matches consumer patterns**: Civis (Rust workspace), WSM3D (Unity .csproj), web (npm).
- **Superset merge** principle: best of all microlibs, no drop-one-pick-other.
- **Cargo workspace** at `rust/Cargo.toml` keeps Rust crates co-versioned.
- **.NET Standard 2.1** in `csharp/` is compatible with Unity 2022.3 BRP (netstandard2.1).

---

## Consequences

- Step 2: Fold Civis Rust modules into `rust/phenotype-gfx-voxel/` (gate: user approval).
- Step 3: Fold `phenotype-voxel` subtree API into `rust/phenotype-gfx-voxel/src/voxel.rs`.
- Step 4: Port C# subsystems from WSM3D / phenotype-postfx into `csharp/src/`.
- PR #2 (`feat/scaffold-crate`) is superseded by this PR and should be closed.
