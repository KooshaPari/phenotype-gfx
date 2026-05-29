# phenotype-voxel — Functional & Non-Functional Requirements

**Scope:** Backfilled catalog for Tracera + AgilePlus ingestion.
**Schema version:** 1 (matches `SCHEMA_VERSION` constant in `src/lib.rs`).
**Test baseline:** 94 lib tests passing (1 doctest skipped — prose comment, not executable).

---

## Functional Requirements

### FR-VOXEL-001 — Chunk Storage

**Title:** Dense 16³ leaf chunk storage parameterised over voxel type.

**Description:** The substrate stores voxels as dense 16³ arrays (`CHUNK_EDGE=16`,
`CHUNK_VOXELS=4096`) wrapped in `Chunk<T: Default + Clone>`. Chunks are indexed by
deterministic `ChunkId(u64)`. A borrowed `ChunkView<'a, T>` provides zero-copy
access to the voxel slice plus the stable `ChunkId` for meshing.

**Acceptance criteria:**
- `Chunk::default()` produces exactly 4096 voxels all set to `T::default()`.
- `CHUNK_VOXELS == CHUNK_EDGE³` is enforced at compile+test time.
- `ChunkView` carries a valid `&[T]` of length `CHUNK_VOXELS`.

**Traceability:**
- PR #1 (format), PR #3 (generic parameterisation)
- In-code: `FR-PHENO-VOXEL-CHUNK-000`, `FR-PHENO-VOXEL-CHUNK-001` (`src/chunk.rs`)

---

### FR-VOXEL-002 — Compact RLE Chunk Serialization / Deserialization

**Title:** Binary PVOX RLE round-trip for scene persistence.

**Description:** `save_chunk<T, W>` / `load_chunk<T, R>` serialize/deserialize a
`Chunk<T: Pod + Eq + Default + Clone>` using a self-describing binary PVOX format
(magic `b"PVOX"`, format version `u8`, element size `u32 LE`, RLE run count `u32 LE`,
then `(run_length: u16 LE, value bytes)*`). A fully uniform chunk serializes to a
single run (16 bytes for `u8`). Type-size mismatches are detected before reading
voxel data.

**Acceptance criteria:**
- Empty (uniform-default) chunk serializes to exactly 16 bytes for `T=u8`.
- Fully dense alternating chunk round-trips losslessly (4096 runs of length 1).
- Bad magic returns `io::ErrorKind::InvalidData`.
- Element-size mismatch detected before voxel data is consumed.
- Run-length sum != `CHUNK_VOXELS` returns an error.

**Traceability:**
- PR #2 (RLE implementation)
- In-code: `FR-PHENO-VOXEL-SERIAL-000..003` (`src/serial.rs`)

---

### FR-VOXEL-003 — Pluggable Mesher Trait

**Title:** Engine-agnostic `Mesher` trait with associated `VoxelKind`.

**Description:** `Mesher` is a Rust trait with two associated types:
`VoxelKind: Default + Clone` (pins the voxel type) and `Mesh` (engine artifact).
The single required method `mesh_chunk(chunk, lod) -> MeshResult<Self::Mesh>` must
be deterministic for a given `(chunk, lod)` pair. The `CubicVoxel` sub-trait
(`is_solid()`, `material()`) enables the reference meshers; blanket impl for
`MaterialId` is provided.

**Acceptance criteria:**
- `Mesher::VoxelKind` prevents mixing voxel types at the trait boundary (compile-time enforcement).
- Any `T: CubicVoxel` can be wired to `CubicMesher<T>` or `GreedyMesher<T>`.
- Determinism: calling `mesh_chunk` twice on equal `(chunk, lod)` inputs returns bit-identical buffers.

**Traceability:**
- PR #3 (VoxelKind associated type + generic CubicMesher)
- In-code: `FR-PHENO-VOXEL-CUBIC-002`, `FR-PHENO-VOXEL-CUBIC-011` (`src/cubic_mesher.rs`)

---

### FR-VOXEL-004 — Cubic Reference Mesher

**Title:** Axis-aligned cubic mesher emitting only exposed faces.

**Description:** `CubicMesher<V: CubicVoxel>` iterates each solid voxel and emits
up to 6 quad faces (2 triangles each). A face is suppressed when the adjacent voxel
is solid. Outward normals are the canonical axis unit vectors. Each face vertex
carries `position`, `normal`, `uv`, and `material` from the voxel's `CubicVoxel`
impl. Index buffer is consistent (no out-of-range references).

**Acceptance criteria:**
- Single solid voxel in empty chunk → exactly 6 faces (12 triangles).
- Two adjacent solid voxels → shared face suppressed (10 faces).
- Fully surrounded voxel (all 6 neighbours solid) → 0 faces.
- Each face normal equals the correct outward axis vector.
- Material ID propagated to all 4 vertices of each face.
- `indices` only reference valid vertex slots.

**Traceability:**
- PR #3 (generic CubicMesher)
- In-code: `FR-PHENO-VOXEL-CUBIC-001..010` (`src/cubic_mesher.rs`)

---

### FR-VOXEL-005 — Greedy (Maximal-Rect) Mesher

**Title:** Greedy quad-merging mesher for reduced triangle count.

**Description:** `GreedyMesher<V: CubicVoxel>` sweeps each of the 6 axis-aligned
face directions, builds a 2-D material mask of visible faces per slice, and merges
coplanar same-material faces into maximal rectangles (greedy width-then-height
extension). One quad (2 triangles) is emitted per rectangle. Implements the same
`Mesher` trait as `CubicMesher`.

**Acceptance criteria:**
- Produces the same visible surface (watertight; no T-junctions on flat regions)
  as `CubicMesher` for identical inputs.
- Triangle count ≤ cubic count for any chunk (regression guard — see NFR-VOXEL-002).
- Same-material coplanar faces are merged; different-material or non-coplanar faces
  are not merged.
- `ao` field of produced `MeshBuffer` is all-3 (fully lit; see PLANNED section).

**Traceability:**
- PR #5 (GreedyMesher), PR #6 (benchmark + regression guard)
- In-code: `FR-PHENO-VOXEL-*` tests in `src/greedy_mesher.rs`

---

### FR-VOXEL-006 — Per-Vertex Ambient Occlusion

**Title:** AO values in `MeshBuffer.ao` for cubic meshing.

**Description:** `CubicMesher` populates `MeshBuffer.ao` (a `Vec<u8>` parallel to
`vertices`) with classic voxel AO per vertex. Value range: 0 (maximum occlusion, both
side neighbours solid) to 3 (fully lit, no neighbours). Formula:
`ao = if side1 && side2 { 0 } else { 3 - (side1 + side2 + corner) }`.
`ao.len()` always equals `vertices.len()`.

**Acceptance criteria:**
- Fully exposed single voxel → all AO values = 3.
- `ao.len() == vertices.len()` for any mesh output.
- Triangle count is unchanged relative to non-AO cubic output.
- Voxel in a right-angle crevice (two perpendicular solid neighbours) → corner
  vertices have AO value < 3.
- Empty chunk → `ao` is empty (all-3 trivially).

**Traceability:**
- PR #7 (per-vertex AO)
- In-code: `FR-PHENO-VOXEL-CUBIC-AO-001..005` (`src/cubic_mesher.rs`)

---

### FR-VOXEL-007 — Engine-Agnostic Mesh Export API

**Title:** `MeshBuffer` export surface: iterators + `to_interleaved()`.

**Description:** `MeshBuffer` exposes zero-copy iterator accessors
(`positions()`, `normals()`, `uvs()`, `ao()`, `indices()`) plus
`vertex_count()`, `index_count()`, `triangle_count()`, `is_empty()`.
`to_interleaved() -> Vec<f32>` packs all vertices into a flat stride-9 buffer
`[px, py, pz, nx, ny, nz, u, v, ao]` (36 bytes/vertex) suitable for a single
GPU VBO upload.

**Acceptance criteria:**
- `to_interleaved().len() == vertex_count() * 9`.
- Interleaved AO values at offset 8 of each stride match `ao()`.
- Interleaved positions match `vertices[i].position`.
- Empty mesh → empty interleaved buffer.
- All accessor counts equal `vertices.len()` / `indices.len()`.

**Traceability:**
- PR #8 (engine-agnostic export API)
- In-code: `FR-PHENO-VOXEL-MESH-EXPORT-000..005` (`src/mesh.rs`)

---

### FR-VOXEL-008 — SVO Node Compaction

**Title:** Greedy upward merging of uniform sibling nodes in `VoxelOctree`.

**Description:** `VoxelOctree<T>` stores `OctreeNode::Uniform(T)` or
`OctreeNode::Dense` per `ChunkCoord` in a deterministic `BTreeMap`.
`compact()` performs a fixpoint greedy upward merge: any complete 8-sibling
group where all members are `Uniform` with the same value collapses to one
parent-level `Uniform` node. Passes repeat until no further collapses occur.
Returns total nodes removed. Operation is idempotent.

**Acceptance criteria:**
- 8 uniform siblings → 8 nodes removed, 1 parent node inserted.
- Mixed-value group → 0 nodes removed.
- Idempotent: second call on compacted tree → 0 removed, state unchanged.
- Multi-level pyramid (64 leaves) → 72 nodes removed, 1 root remains.
- Incomplete group (7/8 siblings) → 0 removed.
- Query semantics preserved: parent node carries merged material.

**Traceability:**
- PR #9 (SVO node compaction)
- In-code: `FR-PHENO-VOXEL-OCTREE-000..015` (`src/octree.rs`)

---

### FR-VOXEL-009 — Deterministic Dirty-Event Queue

**Title:** `DirtyChunkEvent` ordered by `(chunk_id, write_seq)`.

**Description:** Every voxel write on `VoxelWorld` produces a `DirtyChunkEvent`
tagged with a monotonically increasing `WriteSeq`. Consumers drain events via
`drain_dirty()`. Events sort by `(chunk_id, write_seq)` so replay is order-stable
across implementations. Idempotent writes (same value already present) do not emit
events. `DirtyChunkEvent` and `WriteSeq` are `Serialize/Deserialize`.

**Acceptance criteria:**
- `WriteSeq::next()` is strictly monotonic.
- Two events with different `(chunk_id, write_seq)` sort predictably.
- Idempotent write (same value) → no event emitted.
- Replaying the same write sequence on a fresh `VoxelWorld` yields
  bit-identical chunk state.

**Traceability:**
- PR #2 (introduced dirty events), PR #9 (world compaction integration)
- In-code: `FR-PHENO-VOXEL-DELTA-000..001` (`src/delta.rs`),
  `FR-PHENO-VOXEL-WORLD-001..016` (`src/world.rs`)

---

### FR-VOXEL-010 — Optional Bevy Mesh Adapter

**Title:** Feature-gated Bevy adapter from `MeshBuffer` to `bevy::render::mesh::Mesh`.

**Description:** When the non-default `bevy` cargo feature is enabled, the crate
exposes `to_bevy_mesh(&MeshBuffer) -> bevy::render::mesh::Mesh`. The adapter maps
positions, normals, UVs, triangle indices, and a custom per-vertex AO attribute
into Bevy's mesh type without changing the engine-neutral `MeshBuffer` surface.
The core crate stays dep-light by default because the adapter is fully optional.

**Acceptance criteria:**
- `cargo build --features bevy` compiles.
- The produced Bevy mesh contains position, normal, UV0, AO, and index data.
- `cargo test --lib` remains green with no features enabled.

**Traceability:**
- Former PLANNED item: `PLAN-VOXEL-002`
- In-code: `FR-PHENO-VOXEL-BEVY-000` (`src/bevy_adapter.rs`)

---

## Non-Functional Requirements

### NFR-VOXEL-001 — Determinism

**Title:** All public operations are deterministic across runs and platforms.

**Description:** World coordinates are fixed-point `i64` at `10^6` scale; no
`f32/f64` crosses the public API boundary. Internal collections use `BTreeMap`
(deterministic iteration order), never `HashMap`. Mesher output is bit-identical
for equal `(chunk, lod)` inputs. Dirty events are ordered, not set-like.
`#![forbid(unsafe_code)]` enforced crate-wide.

**Evidence:** `src/lib.rs` determinism contract comment; `VoxelOctree` uses
`BTreeMap`; `FR-PHENO-VOXEL-CUBIC-002` determinism test; `FR-PHENO-VOXEL-WORLD-005`
replay bit-identity test.

---

### NFR-VOXEL-002 — Greedy ≤ Cubic Triangle Count (Regression Guard)

**Title:** `GreedyMesher` never produces more triangles than `CubicMesher` for any chunk.

**Description:** Greedy meshing merges coplanar faces; the triangle count for any
input chunk must not exceed the cubic count. A Criterion benchmark (`benches/`) and
a dedicated triangle-count regression test enforce this invariant on every CI run.

**Evidence:** PR #6 (benchmark + regression guard); `benches/` Criterion suite; test
`greedy_triangle_count_le_cubic` (or equivalent) in `src/greedy_mesher.rs`.

---

### NFR-VOXEL-003 — Zero-Dependency Export Surface

**Title:** `MeshBuffer` export API has no engine-specific dependencies.

**Description:** `MeshBuffer` and its export methods (`to_interleaved`, iterators,
counts) depend only on `std` + `serde`. No Bevy, Godot, or Unreal crates appear in
the dependency tree of the core `phenotype-voxel` crate. Engine adapters are
consumer-side.

**Evidence:** `Cargo.toml` deps: `bytemuck`, `serde`, `thiserror`, `log` only; PR #8
confirms no engine deps added.

---

### NFR-VOXEL-004 — Watertight Meshes (No Duplicate / Missing Faces)

**Title:** Both meshers produce watertight, non-degenerate geometry.

**Description:** Exposed faces are emitted exactly once. Internal (solid-to-solid)
faces are never emitted. Normals are unit vectors on the correct outward axis.
Index buffer references only valid vertex slots.

**Evidence:** `FR-PHENO-VOXEL-CUBIC-003` (internal face suppression),
`FR-PHENO-VOXEL-CUBIC-006` (corner voxel), `FR-PHENO-VOXEL-CUBIC-007` (fully
surrounded → 0 faces), `FR-PHENO-VOXEL-CUBIC-008` (outward normals),
`FR-PHENO-VOXEL-CUBIC-010` (index validity).

---

### NFR-VOXEL-005 — Test Suite Coverage (91+ Tests Green)

**Title:** All lib tests pass on every merge to `main`.

**Description:** The test suite must maintain ≥ 91 passing lib tests. Doctest failures
caused by prose comment blocks (not executable code) are excluded. CI enforces
`cargo test --lib` on PR merge.

**Evidence:** `cargo test --lib` output: `94 passed; 0 failed` (2026-05-29).

---

### FR-VOXEL-011 — AO-Aware Greedy Meshing (SHIPPED)

**Title:** Per-vertex AO propagated through greedy face merging.

**Description:** `GreedyMesher` computes a 4-corner AO signature for each
visible face cell (reusing `CubicMesher`'s `face_ao` helper) and includes
that signature in the mask equality key.  Two face cells are only merged when
both their `MaterialId` **and** their `[u8; 4]` AO signature are equal.  This
preserves AO detail at occlusion boundaries while fully collapsing large
homogeneous flat regions (uniform AO = all-3) into single quads.  Each merged
quad carries the uniform per-corner AO values from its mask key.  The `ao`
field of every `MeshBuffer` emitted by `GreedyMesher` is now populated with
real values (previously hardcoded to 3).

**Acceptance criteria:**
- A fully isolated voxel (no neighbours) → all 24 AO values = 3 (unchanged from cubic).
- `ao.len() == vertices.len()` for any greedy mesh output.
- Flat surface with uniform AO=3 still merges to fewer triangles than cubic (greedy win preserved).
- A face at an AO occlusion boundary carries at least one AO value < 3 (AO detail preserved).
- Greedy AO values match CubicMesher AO values for corresponding faces on a single isolated voxel.
- Triangle count ≤ cubic for any chunk (NFR-VOXEL-002 regression guard remains green).

**Traceability:**
- Former PLANNED item: `PLAN-VOXEL-001`
- PR feat/greedy-ao
- In-code: `FR-PHENO-VOXEL-GREEDY-AO-001..003` (`src/greedy_mesher.rs`)

---

### NFR-VOXEL-006 — Extended Performance Benchmark Suite (SHIPPED)

**Title:** Criterion benchmarks + regression guards for all perf-critical paths beyond mesher_compare.

**Description:** A dedicated `benches/perf_suite.rs` Criterion benchmark suite covers:
- RLE chunk serialize (`serial_save`) and deserialize (`serial_load`) throughput across
  empty / sparse / dense / checkerboard shapes (u8 voxels — `Pod`-satisfying).
- SVO compaction throughput: 8-leaf single-group and 64-leaf two-level pyramid fixpoint.
- AO computation cost: `CubicMesher` on dense-solid and checkerboard fixtures.
- World fill + dirty-tracking drain: 4096 writes in one chunk + idempotent-write path.

A complementary integration test file (`tests/perf_regression_guards.rs`) provides
11 lightweight `#[test]` guards that assert cost-ordering and upper-bound invariants
without absolute wall-clock thresholds, making them flap-free on any CI hardware:
- Serial byte-size ordering (empty < alternating), exact 16-byte empty invariant, ratio >= 50x.
- SVO compact: exact node removal counts for 8-leaf, 64-leaf, mixed-value groups.
- AO: `ao.len() == vertices.len()` for all shapes; fully-exposed voxel all-3.
- Dirty: exact event counts, idempotent-write emits 0, fill-one-chunk under 1s, count scales with writes.

**Acceptance criteria:**
- `cargo bench --no-run` compiles cleanly.
- `cargo test --lib --tests` remains green (94 lib + 4 triangle-regression + 11 perf-regression = 109 total).
- All 11 NFR regression guards pass on every CI run.

**Traceability:**
- PLAN-VOXEL-003 (formerly PLANNED) → SHIPPED in PR bench/perf-suite
- `benches/perf_suite.rs`, `tests/perf_regression_guards.rs`
- In-code guard IDs: `NFR-VOXEL-006-SERIAL-001..003`, `NFR-VOXEL-006-SVO-001..003`,
  `NFR-VOXEL-006-AO-001..002`, `NFR-VOXEL-006-DIRTY-001..003`

---

## Test-ID to Catalog Mapping

| In-code test series | Catalog FR/NFR |
|---|---|
| `FR-PHENO-VOXEL-CHUNK-000..001` | FR-VOXEL-001 |
| `FR-PHENO-VOXEL-SERIAL-000..003` | FR-VOXEL-002 |
| `FR-PHENO-VOXEL-CUBIC-001..011` | FR-VOXEL-003, FR-VOXEL-004, NFR-VOXEL-004 |
| `FR-PHENO-VOXEL-CUBIC-AO-001..005` | FR-VOXEL-006 |
| `FR-PHENO-VOXEL-MESH-000`, `MESH-EXPORT-000..005` | FR-VOXEL-007 |
| `FR-PHENO-VOXEL-OCTREE-000..015` | FR-VOXEL-008 |
| `FR-PHENO-VOXEL-DELTA-000..001` | FR-VOXEL-009 |
| `FR-PHENO-VOXEL-WORLD-001..016` | FR-VOXEL-009, NFR-VOXEL-001 |
| `FR-PHENO-VOXEL-BEVY-000` | FR-VOXEL-010 (SHIPPED — `bevy` feature, `src/bevy_adapter.rs`) |
| `FR-PHENO-VOXEL-COORD-000..001` | (coordinate contract, underpins FR-VOXEL-001/009) |
| `FR-PHENO-VOXEL-LOD-000..006` | (LOD selection, underpins FR-VOXEL-004/005) |
| `FR-PHENO-VOXEL-MATERIAL-000` | (palette, underpins FR-VOXEL-004/005) |
| `FR-PHENO-VOXEL-GREEDY-AO-001..003` | FR-VOXEL-011 |
| `FR-PHENO-VOXEL-SHAPEHINT-001..009` | (shape-hint registry, not yet in catalog — see PLANNED; formalize as FR-VOXEL-012) |
| `FR-PHENO-VOXEL-SPRITEVOX-001..006` | (sprite voxelizer, not yet in catalog — see PLANNED; formalize as FR-VOXEL-013) |

---

## Gaps / PLANNED

| ID | Title | Notes |
|---|---|---|
| ~~PLAN-VOXEL-001~~ | ~~Greedy-mesher per-vertex AO~~ | **SHIPPED** as FR-VOXEL-011 — AO-aware mask key in `src/greedy_mesher.rs`; 3 new tests `FR-PHENO-VOXEL-GREEDY-AO-001..003` |
| ~~PLAN-VOXEL-003~~ | ~~Performance under load~~ | **SHIPPED** as NFR-VOXEL-006 — see `benches/perf_suite.rs` + `tests/perf_regression_guards.rs` |
| PLAN-VOXEL-004 | Shape-hint registry FR | `FR-PHENO-VOXEL-SHAPEHINT-*` tests exist but no catalog entry; formalize as FR-VOXEL-011 |
| PLAN-VOXEL-005 | Sprite voxelizer FR | `FR-PHENO-VOXEL-SPRITEVOX-*` tests exist but no catalog entry; formalize as FR-VOXEL-012 |
| PLAN-VOXEL-006 | Full recursive SVO subdivision | `octree.rs` notes "8-way branches reserved for follow-up PR"; current model is flat `BTreeMap` |
| PLAN-VOXEL-007 | Doctest hygiene | 1 doctest fails on Windows (prose comment mistaken for code block in `cubic_mesher.rs`); fix or mark `ignore` |
