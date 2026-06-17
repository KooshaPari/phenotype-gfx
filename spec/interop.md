# GFX SDK interop contract

The three modules are different languages; they unify at the **data layer**.
This file is the contract. Changing a format here is a breaking change and bumps
`VERSION.toml`.

## Shared coordinate frame
- Right-handed, Y-up. 1 unit = 1 metre. Chunk origin = min corner.

## Chunk grid (voxel ↔ terrain)
- Chunk = 32³ cells (power-of-two; voxel dirty queue assumes this).
- `chunk_coord = floor(world_pos / 32)`.
- Heightfield (terrain) indexes the same XZ chunk grid so terrain tiles align to voxel chunk columns.

## Heightfield (terrain → water)
- Per-chunk `f32` heightmap, row-major XZ, `(32+1)²` samples (shared edges).
- Water samples terrain height to derive shoreline / depth.

## Surface exchange (water → renderers)
- Water emits a per-chunk surface level `f32` + flow vector `[f32;2]`.

## TODO (fill when module audits land)
- voxel chunk serialization format (Rust side) — pending phenotype-voxel audit
- terrain/water mesh export format (C# side) — pending audits

<!-- ponytail: spec is the minimum that lets the three modules agree on grid+units.
     Expand only when a real cross-module call needs a field that isn't here. -->
