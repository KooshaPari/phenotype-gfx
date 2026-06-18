# Phenotype GFX SDK

**Work-state:** scaffolding · `▓▓░░░░░░░░` 2/10

Polyglot graphics SDK for Phenotype-org games. One repo, four graphics
substrates, unified branding / versioning / interop.

| Module | Path | Lang | Role | Source |
|--------|------|------|------|--------|
| voxel | `rust/voxel/` | Rust | Adaptive voxel substrate (SVO + dense leaf chunks, dirty queue, BRP streaming) | phenotype-voxel |
| terrain | `unity/terrain/` | C# | Terrain meshing / heightfield | phenotype-terrain |
| water | `unity/water/` | C# | Water simulation / surface | phenotype-water |
| postfx | `unity/postfx/` | Unity package | Post-processing runtime, shaders, docs, tests | phenotype-postfx |

History from the source repos is preserved in the folded module directories.
The standalone source repos are superseded by this umbrella once consumer
references point here (see `VERSION.toml`, `spec/interop.md`).

## Layout
```
rust/voxel/      Rust crate (Cargo)
unity/terrain/   C# library
unity/water/     C# library
unity/postfx/    Unity post-processing package
spec/interop.md  shared data-format contract between modules
VERSION.toml     umbrella version manifest pinning each module
```

## Why polyglot
Voxel is a Rust substrate for Bevy/BRP games; terrain, water, and postfx serve
the Unity/WorldBox side. They unify at the **data layer** (chunk + heightfield +
surface + render-input formats in `spec/interop.md`), not the language layer.
