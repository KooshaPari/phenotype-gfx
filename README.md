# Phenotype GFX SDK

**Work-state:** scaffolding · `▓▓░░░░░░░░` 2/10

Polyglot graphics SDK for Phenotype-org games. One repo, three engines'-worth of
substrate, unified branding / versioning / interop.

| Module | Path | Lang | Role | Source |
|--------|------|------|------|--------|
| voxel | `rust/voxel/` | Rust | Adaptive voxel substrate (SVO + dense leaf chunks, dirty queue, BRP streaming) | phenotype-voxel |
| terrain | `unity/terrain/` | C# | Terrain meshing / heightfield | phenotype-terrain |
| water | `unity/water/` | C# | Water simulation / surface | phenotype-water |

History from the three source repos is preserved via `git subtree`. Source repos
remain the upstreams until folded; this monorepo doubles as the umbrella
(see `VERSION.toml`, `spec/interop.md`).

## Layout
```
rust/voxel/      Rust crate (Cargo)
unity/terrain/   C# library
unity/water/     C# library
spec/interop.md  shared data-format contract between modules
VERSION.toml     umbrella version manifest pinning each module
```

## Why polyglot
Voxel is a Rust substrate for Bevy/BRP games; terrain & water are C# for the
Unity/WorldBox side. They unify at the **data layer** (chunk + heightfield +
surface formats in `spec/interop.md`), not the language layer.
