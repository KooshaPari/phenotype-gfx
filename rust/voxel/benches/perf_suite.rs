//! PLAN-VOXEL-003 → NFR-VOXEL-006 performance benchmark suite.
//!
//! Covers perf-critical paths NOT already in mesher_compare or voxelizer_bench:
//!   - Chunk RLE serialize / deserialize throughput (empty / sparse / dense / checkerboard)
//!   - SVO compaction throughput (8-leaf and 64-leaf pyramid)
//!   - AO computation cost: CubicMesher with vs without AO pass (dense solid chunk)
//!   - Chunk fill (world write) + dirty-tracking drain

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use phenotype_voxel::{
    chunk::{Chunk, ChunkId, ChunkView, CHUNK_EDGE, CHUNK_VOXELS},
    coord::ChunkCoord,
    cubic_mesher::CubicMesher,
    lod::LodLevel,
    material::MaterialId,
    octree::VoxelOctree,
    serial::{load_chunk, save_chunk},
    world::VoxelWorld,
    WorldCoord, FIXED_SCALE,
};

// ---------------------------------------------------------------------------
// Shared chunk fixtures (same 4 shapes as mesher_compare for consistency)
// ---------------------------------------------------------------------------

// --- MaterialId chunks (for mesher / AO / world benches) ---

fn dense_solid_mat_chunk() -> Chunk<MaterialId> {
    Chunk {
        voxels: vec![MaterialId(1); CHUNK_VOXELS],
    }
}

fn checkerboard_mat_chunk() -> Chunk<MaterialId> {
    let mut c = Chunk::<MaterialId>::default();
    for z in 0..CHUNK_EDGE {
        for y in 0..CHUNK_EDGE {
            for x in 0..CHUNK_EDGE {
                if (x + y + z) % 2 == 0 {
                    c.voxels[x + y * CHUNK_EDGE + z * CHUNK_EDGE * CHUNK_EDGE] = MaterialId(1);
                }
            }
        }
    }
    c
}

// --- u8 chunks (for serial benches — u8: Pod + Eq + Default + Clone) ---

fn empty_u8_chunk() -> Chunk<u8> {
    Chunk::<u8>::default()
}

fn sparse_u8_chunk() -> Chunk<u8> {
    let mut c = Chunk::<u8>::default();
    let step = 3usize;
    let mut count = 0usize;
    'outer: for z in (0..CHUNK_EDGE).step_by(step) {
        for y in (0..CHUNK_EDGE).step_by(step) {
            for x in (0..CHUNK_EDGE).step_by(step) {
                c.voxels[x + y * CHUNK_EDGE + z * CHUNK_EDGE * CHUNK_EDGE] = 1u8;
                count += 1;
                if count >= 64 {
                    break 'outer;
                }
            }
        }
    }
    c
}

/// Alternating 0/1 pattern — no two adjacent voxels equal, forces CHUNK_VOXELS RLE runs.
/// Named "dense" because it represents the dense/worst-case serialization path.
fn dense_u8_chunk() -> Chunk<u8> {
    let voxels: Vec<u8> = (0..CHUNK_VOXELS).map(|i| (i % 2) as u8).collect();
    Chunk { voxels }
}

fn checkerboard_u8_chunk() -> Chunk<u8> {
    let mut c = Chunk::<u8>::default();
    for z in 0..CHUNK_EDGE {
        for y in 0..CHUNK_EDGE {
            for x in 0..CHUNK_EDGE {
                if (x + y + z) % 2 == 0 {
                    c.voxels[x + y * CHUNK_EDGE + z * CHUNK_EDGE * CHUNK_EDGE] = 1u8;
                }
            }
        }
    }
    c
}

/// Pre-serialize a u8 chunk to bytes so deserialization bench doesn't include encode time.
fn serialize_u8_to_vec(chunk: &Chunk<u8>) -> Vec<u8> {
    let mut buf = Vec::new();
    save_chunk(chunk, &mut buf).expect("save_chunk");
    buf
}

// ---------------------------------------------------------------------------
// 1. RLE serialize / deserialize throughput (u8 voxels — Pod-satisfying)
// ---------------------------------------------------------------------------

fn bench_serial_save(c: &mut Criterion) {
    let shapes: &[(&str, Chunk<u8>)] = &[
        ("empty", empty_u8_chunk()),
        ("sparse", sparse_u8_chunk()),
        ("dense_solid", dense_u8_chunk()),
        ("checkerboard", checkerboard_u8_chunk()),
    ];

    let mut group = c.benchmark_group("serial_save");
    group.throughput(Throughput::Elements(CHUNK_VOXELS as u64));

    for (name, chunk) in shapes {
        group.bench_with_input(BenchmarkId::from_parameter(name), name, |b, _| {
            b.iter(|| {
                let mut buf = Vec::<u8>::with_capacity(64);
                save_chunk(black_box(chunk), &mut buf).unwrap();
                black_box(buf)
            })
        });
    }
    group.finish();
}

fn bench_serial_load(c: &mut Criterion) {
    let shapes: &[(&str, Vec<u8>)] = &[
        ("empty", serialize_u8_to_vec(&empty_u8_chunk())),
        ("sparse", serialize_u8_to_vec(&sparse_u8_chunk())),
        ("dense_solid", serialize_u8_to_vec(&dense_u8_chunk())),
        (
            "checkerboard",
            serialize_u8_to_vec(&checkerboard_u8_chunk()),
        ),
    ];

    let mut group = c.benchmark_group("serial_load");
    group.throughput(Throughput::Elements(CHUNK_VOXELS as u64));

    for (name, bytes) in shapes {
        group.bench_with_input(BenchmarkId::from_parameter(name), name, |b, _| {
            b.iter(|| {
                let result: Chunk<u8> = load_chunk(&mut black_box(bytes.as_slice())).unwrap();
                black_box(result)
            })
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// 2. SVO compaction throughput
// ---------------------------------------------------------------------------

/// Build an octree with `n_leaves` uniform siblings (all same value).
/// n_leaves must be a multiple of 8 so at least one compaction fires.
fn uniform_octree(n_leaves: usize) -> VoxelOctree<MaterialId> {
    let mut tree = VoxelOctree::<MaterialId>::default();
    for i in 0..(n_leaves as i32) {
        // Space siblings at even coordinates so parent keys align.
        let cx = (i % 2) * 2;
        let cy = ((i / 2) % 2) * 2;
        let cz = ((i / 4) % 2) * 2;
        // Group index offsets each 8-sibling group by 4 in each axis.
        let g = i / 8;
        let gx = (g % 4) * 4;
        let gy = ((g / 4) % 4) * 4;
        let gz = (g / 16) * 4;
        tree.insert_uniform(
            ChunkCoord {
                cx: cx + gx,
                cy: cy + gy,
                cz: cz + gz,
            },
            MaterialId(1),
        );
    }
    tree
}

fn bench_svo_compact(c: &mut Criterion) {
    let mut group = c.benchmark_group("svo_compact");

    // 8-leaf: single group collapses in one pass.
    group.bench_function("8_leaf", |b| {
        b.iter_batched(
            || uniform_octree(8),
            |mut tree| black_box(tree.compact()),
            criterion::BatchSize::SmallInput,
        )
    });

    // 64-leaf: 8 groups of 8, first pass collapses to 8 parents, second collapses
    // to 1 — exercises multi-level fixpoint.
    group.bench_function("64_leaf_pyramid", |b| {
        b.iter_batched(
            || uniform_octree(64),
            |mut tree| black_box(tree.compact()),
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 3. AO cost: CubicMesher dense solid with vs without AO
//    (CubicMesher always computes AO; this bench tracks the all-in cost)
// ---------------------------------------------------------------------------

fn bench_cubic_ao(c: &mut Criterion) {
    let dense = dense_solid_mat_chunk();

    let mut group = c.benchmark_group("cubic_ao_cost");
    group.throughput(Throughput::Elements(CHUNK_VOXELS as u64));

    // "with_ao" — the standard path that populates MeshBuffer.ao.
    group.bench_function("dense_solid_with_ao", |b| {
        b.iter(|| {
            let view = ChunkView {
                id: ChunkId(0),
                voxels: black_box(&dense.voxels),
            };
            black_box(CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)))
        })
    });

    // Checkerboard — worst-case AO neighbour lookups (every voxel fully exposed,
    // each has maximum number of solid diagonal neighbours).
    let checker = checkerboard_mat_chunk();
    group.bench_function("checkerboard_with_ao", |b| {
        b.iter(|| {
            let view = ChunkView {
                id: ChunkId(0),
                voxels: black_box(&checker.voxels),
            };
            black_box(CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)))
        })
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 4. Chunk fill + dirty-tracking (world-level write + drain)
// ---------------------------------------------------------------------------

fn bench_world_fill_and_drain(c: &mut Criterion) {
    let span = FIXED_SCALE; // 1 m voxel span

    let mut group = c.benchmark_group("world_fill_drain");
    group.throughput(Throughput::Elements(CHUNK_VOXELS as u64));

    // Fill all CHUNK_VOXELS slots in a single chunk, then drain dirty events.
    group.bench_function("fill_one_chunk_then_drain", |b| {
        b.iter(|| {
            let mut world = VoxelWorld::<MaterialId>::new(span);
            for z in 0..CHUNK_EDGE as i64 {
                for y in 0..CHUNK_EDGE as i64 {
                    for x in 0..CHUNK_EDGE as i64 {
                        world.write(
                            WorldCoord {
                                x: x * span,
                                y: y * span,
                                z: z * span,
                            },
                            MaterialId(1),
                        );
                    }
                }
            }
            black_box(world.drain_dirty())
        })
    });

    // Idempotent writes: write the same value twice — should emit 0 extra events.
    group.bench_function("idempotent_writes_no_dirty", |b| {
        b.iter(|| {
            let mut world = VoxelWorld::<MaterialId>::new(span);
            let pos = WorldCoord { x: 0, y: 0, z: 0 };
            world.write(pos, MaterialId(1));
            let _ = world.drain_dirty();
            // Second write is idempotent.
            for _ in 0..CHUNK_VOXELS {
                world.write(pos, MaterialId(1));
            }
            black_box(world.drain_dirty())
        })
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Criterion entry-point
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_serial_save,
    bench_serial_load,
    bench_svo_compact,
    bench_cubic_ao,
    bench_world_fill_and_drain,
);
criterion_main!(benches);
