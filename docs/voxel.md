# Voxel Kernel

The voxel kernel (`src/voxel/`) is the heart of `phenotype-gfx`. It provides
the adaptive voxel substrate used by every Phenotype-org game that needs
voxel-style rendering — Civis (3D extension), WorldSphereMod, and downstream
consumers.

> **Source of truth:** `src/voxel/mod.rs:1`
>
> "Adaptive voxel substrate for Phenotype-org games. Primary representation:
> **sparse voxel octree (SVO) for coarse / far-from-camera space + dense 16³
> leaf chunks for near-camera detail**."

## Architecture

```
src/voxel/
  chunk.rs          16³ dense leaf chunks (CHUNK_EDGE = 16, CHUNK_VOXELS = 4096)
  coord.rs          WorldCoord / ChunkCoord / FIXED_SCALE (10^6)
  octree.rs         Sparse voxel octree for coarse space
  world.rs          VoxelWorld: orchestrator over chunks + SVO
  cubic_mesher.rs   Reference mesher: one cube per solid voxel
  greedy_mesher.rs  Production mesher: merges coplanar faces into quads
  lod.rs            LodLevel, LodPolicy, VoxelScaleMultiplier, select_lod()
  simd.rs           SSE2 / AVX2 / NEON batch helpers (feature-gated)
  delta.rs          DirtyChunkEvent, WriteSeq (deterministic replay)
  mesh.rs           MeshBuffer, MeshVertex, Mesher trait
  material.rs       MaterialId, MaterialPalette, VoxelMaterial
  material_pbr.rs   PBR material extension (L5-110)
  shape_hints.rs    ShapeHint / ShapeHintRegistry (L5-111)
  sprite_voxelizer.rs  Sprite → voxel extrusion
  serial.rs         save_chunk / load_chunk (Zstd feature)
  fixtures.rs       Test fixtures
  adapters/         MeshAdapter, OctreeAdapter, DenseChunkStore, VoxelWorldAdapter
  ports/            Hexagonal ports: Camera, Chunkable, FrameId, RendererPort
  bevy_adapter.rs   (feature-gated) Bevy ECS integration
```

## Determinism contract

The kernel guarantees replay-safe outputs so consumers can record
`.civreplay` / serialized voxel diffs and reproduce identical meshes.

- World coordinates are fixed-point `i64` at `10^6` scale — no `f32`/`f64`
  crosses the public API (`coord.rs:FIXED_SCALE`).
- Dirty events are ordered by `(chunk_id, write_seq)` so iteration of internal
  collections never leaks ordering into the public surface
  (`delta.rs:DirtyChunkEvent`).
- `VoxelScaleMultiplier` is a first-class semantic with a sensible default of
  `8.0` — LOD selection composes with it through provided helpers so consumers
  cannot accidentally desynchronise (WSM3D-lineage invariant, `mod.rs:DEFAULT_VOXEL_SCALE_MULTIPLIER`).

```rust,no_run
use phenotype_gfx::voxel::{VoxelWorld, WorldCoord, FIXED_SCALE};

let mut world = VoxelWorld::<u8>::new(FIXED_SCALE);
world.write(WorldCoord { x: 0, y: 0, z: 0 }, 1);
let v = world.read(WorldCoord { x: 0, y: 0, z: 0 });
assert_eq!(v, 1);
```

## Chunk storage

The hot path is **dense leaf chunks** — always `16` voxels on a side, for a
total of `16^3 = 4096` voxels per chunk. Voxel storage is laid out in
`x + y * EDGE + z * EDGE * EDGE` order so a `ChunkView` can be handed to a
mesher without copying.

```rust,no_run
use phenotype_gfx::voxel::{Chunk, CHUNK_EDGE, CHUNK_VOXELS};

let mut chunk: Chunk<u8> = Chunk::default();
assert_eq!(chunk.voxels.len(), CHUNK_VOXELS); // 4096
```

> **Source:** `src/voxel/chunk.rs:6-9`
>
> ```rust
> pub const CHUNK_EDGE: usize = 16;
> pub const CHUNK_VOXELS: usize = CHUNK_EDGE * CHUNK_EDGE * CHUNK_EDGE;
> ```

### ChunkId

Every chunk is identified by a stable `ChunkId(u64)` (`chunk.rs:14`) that
encodes the chunk-grid coordinates as a single `u64`. This lets you use it as
a deterministic key in `HashMap` / `BTreeMap` without committing to a particular
iteration order — important for replay determinism.

### ChunkView

A `ChunkView<'a, T>` (`chunk.rs:34`) is the borrowed slice handed to meshers.
It carries the chunk's stable `ChunkId` plus a `&'a [T]` of voxel data, so
meshers produce engine-specific mesh buffers without taking ownership of
storage.

### Sparse octree

Coarse / far-from-camera space is stored in a sparse voxel octree
(`octree.rs:VoxelOctree`). The `VoxelWorld` (`world.rs:VoxelWorld`) unifies
the SVO and dense chunks behind a single `read` / `write` API; meshers and
streaming consumers never have to know which representation is backing a
coordinate.

## Greedy meshing

The reference mesher (`CubicMesher`) emits one cube per solid voxel — correct,
but wasteful for large flat regions. The production mesher
(`src/voxel/greedy_mesher.rs`) merges coplanar, same-material faces into
maximal quads, drastically reducing triangle count.

### Algorithm

For each of the 6 axis-aligned face directions:

1. **Sweep** through each slice perpendicular to that axis.
2. **Mask** — build a 2-D mask of visible faces keyed by `MaskCell`
   (material + 4-corner AO signature). A face is visible if the voxel on the
   face side is solid and the voxel on the opposite side is not. Per-vertex AO
   is computed with the same `face_ao` helper used by `CubicMesher`.
3. **Extend** — for each non-empty cell, greedily widen along the primary
   axis until the material **or AO signature** changes, then raise along the
   secondary axis as far as the full width is available with the same key.
4. **Emit** one quad per rectangle, propagating the per-corner AO values.
   Consumed cells are cleared from the mask so they are not emitted twice.

> **Source:** `src/voxel/greedy_mesher.rs:5-32`

### AO-aware merging

The equality key includes the 4-corner AO signature, which means:

- Faces in a flat, unoccluded region share `AO=[3,3,3,3]` → merge freely.
- Faces at an occlusion boundary carry different AO signatures → merging
  stops at the boundary (AO detail preserved).
- A merged quad carries uniform AO; no interpolation artefact can arise.

The resulting mesh has the same *visible surface area* as the cubic mesher
but (for large homogeneous regions) far fewer triangles, with correct
per-vertex AO everywhere.

### Usage

```rust,no_run
use phenotype_gfx::voxel::{GreedyMesher, CubicVoxel, ChunkView, MeshBuffer};

struct Vox(u8);
impl CubicVoxel for Vox {
    fn is_solid(&self) -> bool { self.0 != 0 }
    fn material(&self) -> u16 { self.0 as u16 }
}

fn build(view: ChunkView<Vox>) -> MeshBuffer {
    GreedyMesher::<Vox>::new().build(view)
}
```

## Level of detail

LOD selection lives in `src/voxel/lod.rs`. It encodes the WSM3D lesson that
**LOD thresholds must compose with `VoxelScaleMultiplier`** or actors collapse
into the impostor tier prematurely.

### Types

- **`LodLevel(u8)`** — `0` = highest detail (per-voxel), higher = coarser.
- **`VoxelScaleMultiplier(f32)`** — newtype wrapper so consumers cannot
  accidentally combine it with raw scalars from elsewhere. Default `8.0`,
  matching the WSM3D visible-default lesson (mesh-local 11x5x1 ×
  sprite-scale 0.1 ≈ ~1.1x0.5x0.1 world → invisible; multiplier of 8 brought
  it back to a usable rendered scale).
- **`LodPolicy`** — `near_voxel_edges`, `far_voxel_edges`, `max_level`.
  Defaults: `near=64.0`, `far=512.0`, `max_level=4`. Distances are in
  **voxel-edge-multiples**, so the policy is scale-invariant and composes
  correctly with `VoxelScaleMultiplier`.

### `select_lod`

```rust,no_run
use phenotype_gfx::voxel::lod::{
    select_lod, LodPolicy, VoxelScaleMultiplier,
};

let lod = select_lod(
    200.0,                          // distance_metres
    VoxelScaleMultiplier::default(),// 8.0
    LodPolicy::default(),
);
```

## SIMD acceleration

Hot-path helpers in `src/voxel/simd.rs` provide SIMD-accelerated batch
operations used by the meshing pipeline. All functions are safe to call on any
platform; the best available path is selected at runtime.

| Helper                            | Description                                                          |
| --------------------------------- | -------------------------------------------------------------------- |
| `simd_normals_batch`              | Batch normalise `[f32;3]` vectors                                    |
| `simd_aabb_center_batch`          | Compute AABB centres from `[min_xyz; max_xyz]` (length-6 slices)     |
| `simd_dot_batch`                  | Batch dot product of pairs of `[f32;3]` vectors                      |
| `simd_conditional_mix_batch`      | Blend vectors based on per-element mask                              |
| `dispatch_*`                      | Runtime-dispatched wrappers that pick the best path per CPU          |
| `get_simd_level`                  | Returns the detected `SimdLevel`                                     |

### SIMD levels

```rust,no_run
use phenotype_gfx::voxel::simd::SimdLevel;

pub enum SimdLevel {
    Scalar, // no SIMD
    SSE2,   // baseline on x86_64
    AVX2,   // widest x86 SIMD available
    NEON,   // aarch64 baseline
}
```

Selection priority at runtime: **AVX2 > SSE2 > NEON > scalar**. AVX2 and
NEON code paths are always compiled behind `target_feature` guards so
non-target architectures can still compile the crate (they simply won't be
called).

### Batch widths

- SSE2: 4 lanes per batch
- AVX2: 8 lanes per batch
- NEON: 4 lanes per batch

### Cargo feature

The `simd` feature (`Cargo.toml:simd`) enables the best available path at
runtime. Without it, only scalar code is compiled — useful for CI lint,
`wasm-bindgen`, or constrained environments.

```toml
[dependencies]
phenotype-gfx = { version = "0.2", features = ["simd"] }
```

### Safety

`src/voxel/simd.rs` is the one place in the voxel module where
`#![allow(unsafe_code)]` is set — x86_64 / aarch64 SIMD intrinsics require
unsafe blocks. Every public helper is still safe to call.

## PBR materials

Material slots live in a `MaterialPalette` (`material.rs:MaterialPalette`).
PBR material information (`material_pbr.rs`) extends the base palette with
UUID v4 ids (L5-110, 2026-06-18) so that consumers can build a stable
material registry without colliding with neighbouring palettes.

## Sprite voxelizer

`src/voxel/sprite_voxelizer.rs` provides three extrusion modes for converting
sprite art into voxel data — `OrganicBlob`, `Lathe`, and `PerTexel`. The
default extrusion depth is `8` (matches WSM3D's `SpriteVoxelizer`).

```rust,no_run
use phenotype_gfx::voxel::sprite_voxelizer::{
    voxelize_image, voxelize_to_chunk, ExtrusionMode, VoxelizeConfig, DEFAULT_DEPTH,
};

let config = VoxelizeConfig {
    extrusion: ExtrusionMode::PerTexel,
    depth: DEFAULT_DEPTH,
    alpha_threshold: 16,
};
```

## Dirty events

Every write to `VoxelWorld` produces a `DirtyChunkEvent` ordered by
`(chunk_id, write_seq)` (`delta.rs:DirtyChunkEvent`). Consumers (Civis,
WorldSphereMod) subscribe to rebuild meshes in a replay-safe order; no
internal-collection iteration order leaks into the public surface.

## Persistence

`src/voxel/serial.rs` provides `save_chunk` / `load_chunk` over a binary
format. With the `zstd-storage` Cargo feature, chunks are Zstd-compressed for
disk persistence.
