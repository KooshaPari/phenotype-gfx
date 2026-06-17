# ADR-003: Scaffold phenotype-gfx as a Root Cargo Crate with Module Stubs

**Date:** 2026-06-17
**Status:** Accepted
**Superset-Merge:** Additive only — no source files removed

---

## Context

The phenotype-gfx repo was DOCS-ONLY at the time of ADR-001/002. Subsequent subtree merges landed:
- `rust/voxel/` — phenotype-voxel full Rust crate (hexagonal ports, SVO + dense leaf chunks)
- `unity/terrain/` — phenotype-terrain C# Unity crate
- `unity/water/` — phenotype-water C# Unity crate

The MATRIX.md audit (ADR-002) identified 6 subsystems missing from the consolidation picture:
lighting/sky, C# LOD, GPU instancing, foliage/wind, procgen, and Civis Rust streaming modules.

No root Cargo.toml or workspace existed, making phenotype-gfx invisible to Rust tooling
and unable to act as a consolidation target for Rust microlibs.

## Decision

Add a root-level `Cargo.toml` + `src/lib.rs` to establish phenotype-gfx as a real Rust crate.
The crate declares module stubs for all 10 subsystems, acting as a landing zone for:
1. Future re-export facades over `rust/voxel/` (Rust-primary)
2. Civis LOD/streaming fold (`streaming` module, ADR-004)
3. phenotype-postfx fold (`postfx` module, feat/postfx-fold-pilot)
4. C# subsystem documentation stubs (`terrain`, `water`, `lighting`, `lod`, `rendering`, `foliage`, `procgen`)

## Rationale

**Structure decision — Rust-primary + C# sidecars:**
- `voxel` and `streaming` are Rust-native and fold cleanly
- `terrain`, `water`, `lighting`, `lod`, `rendering`, `foliage`, `procgen` are C# Unity modules;
  they cannot live inside a Rust crate body and will graduate to a `phenotype-gfx-csharp`
  sister project when that scope is ready
- The stubs serve as module-intent markers and prevent naming collisions

**No workspace yet:**
- `rust/voxel/` is a subtree crate with its own `Cargo.toml`; merging it into a workspace
  would require touching the subtree (violates superset-merge + anti-wipe gate)
- Root crate + stubs is the minimum footprint; workspace consolidation is ADR-005 scope

**Anti-wipe gate:** This commit adds only:
- `Cargo.toml` (new)
- `src/lib.rs` (new)
- `docs/adr/ADR-003-crate-scaffold.md` (new)

No existing files modified or deleted.

## Consequences

- phenotype-gfx is now a valid `cargo check`-able Rust crate (no deps, no build required)
- Module stubs are foldable placeholders; actual code lands via superset-merge in ADR-004+
- C# subsystem stubs are documentation intent only; they do not compile to anything
- Consuming crates can now add `phenotype-gfx = { path = "..." }` once modules are populated

## Alternatives Rejected

- **Workspace-first:** Would require modifying `rust/voxel/Cargo.toml` (subtree violation)
- **No root crate, just docs:** Leaves Rust tooling blind; no landing zone for streaming fold
- **Fold everything now:** PostFX has hardcoded paths blocker; WSM3D C# has abstraction gap
