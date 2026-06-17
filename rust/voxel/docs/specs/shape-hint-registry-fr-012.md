# FR-VOXEL-012 — Shape-Hint Registry Formalization

## Summary

`phenotype-voxel` already ships a deterministic `ShapeHintRegistry` in
`src/shape_hints.rs`. It maps asset-name prefixes to voxelization hints and
comes with WSM3D-compatible defaults, but the FR/NFR catalog still calls this
surface out as a planned gap. This spec formalizes the API that already exists
so downstream consumers can rely on it as a documented contract.

## Proposed Increment

Formalize the shape-hint registry as `FR-VOXEL-012` with the following contract:

- `ShapeHintRegistry::new()` returns an empty registry.
- `ShapeHintRegistry::with_wsm3d_defaults()` loads the 47 prefix mappings from
  WSM3D in deterministic order.
- `register(prefix, hint)` appends a prefix rule; first match wins.
- `get(asset_id)` performs case-insensitive prefix matching and returns
  `ShapeHint::Auto` when no rule matches.
- `clear()`, `len()`, and `is_empty()` remain part of the public ergonomics
  surface.

## Why This Is High-Value

- It turns an already-implemented helper into a documented compatibility
  contract, which reduces consumer guesswork.
- It matches the repo's API-first direction: engine-agnostic, deterministic,
  and small enough to keep stable.
- It gives voxelization callers a low-friction way to align asset naming with
  shape selection without touching the core voxel substrate.

## Acceptance Criteria

- `docs/requirements/phenotype-voxel-frnfr.md` includes a catalog entry for
  `FR-VOXEL-012` that points at the existing `src/shape_hints.rs` tests.
- The doc states the matching and precedence rules explicitly.
- The doc preserves the deterministic / first-match behavior already enforced
  by tests.

## Non-Goals

- No new voxelization algorithm.
- No change to the default prefix table.
- No new engine dependency.

## Traceability

- Planned gap: `PLAN-VOXEL-004`
- Existing implementation and tests: `src/shape_hints.rs`

