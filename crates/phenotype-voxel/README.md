# phenotype-voxel

Compatibility crate for fleet consumers migrating off the archived [`KooshaPari/phenotype-voxel`](https://github.com/KooshaPari/phenotype-voxel) repository.

The canonical implementation now lives in [`phenotype_gfx::voxel`] within the parent project, following [ADR-004](./docs/ADR-004-voxel-migration.md).

## Usage

```rust
// Add to Cargo.toml
// phenotype-voxel = { path = "crates/phenotype-voxel" }

// Use in your code
use phenotype_voxel::voxel::chunk::Chunk;
use phenotype_voxel::voxel::lod::LodLevel;
```

## Purpose

This crate provides a 100% compatible re-export of the voxel kernel, allowing downstream projects to update their dependencies incrementally without breaking changes.

## License

MIT OR Apache-2.0
