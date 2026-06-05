//! Criterion benchmark: GreedyMesher vs CubicMesher throughput across representative
//! chunk shapes.
//!
//! Shapes:
//!   empty         – all air; both meshers should return in near-zero time.
//!   sparse        – 64 isolated solid voxels scattered across the 16³ grid.
//!   dense_solid   – full 16³ solid block (worst case for cubic, best for greedy).
//!   checkerboard  – alternating air/solid 3-D checkerboard (greedy's hardest case;
//!                   no faces can be merged across material boundaries).

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use phenotype_voxel::{
    chunk::{Chunk, ChunkId, ChunkView, CHUNK_EDGE, CHUNK_VOXELS},
    cubic_mesher::CubicMesher,
    greedy_mesher::GreedyMesher,
    lod::LodLevel,
    material::MaterialId,
};

// ---------------------------------------------------------------------------
// Chunk factories
// ---------------------------------------------------------------------------

fn empty_chunk() -> Chunk<MaterialId> {
    Chunk::<MaterialId>::default()
}

fn sparse_chunk() -> Chunk<MaterialId> {
    let mut c = Chunk::<MaterialId>::default();
    // 64 isolated voxels placed so that no two are adjacent (step of 3 in each
    // axis keeps them separated by at least one air voxel).
    let step = 3;
    let mut count = 0;
    'outer: for z in (0..CHUNK_EDGE as i32).step_by(step) {
        for y in (0..CHUNK_EDGE as i32).step_by(step) {
            for x in (0..CHUNK_EDGE as i32).step_by(step) {
                c.voxels
                    [x as usize + y as usize * CHUNK_EDGE + z as usize * CHUNK_EDGE * CHUNK_EDGE] =
                    MaterialId(1);
                count += 1;
                if count >= 64 {
                    break 'outer;
                }
            }
        }
    }
    c
}

fn dense_solid_chunk() -> Chunk<MaterialId> {
    let mut c = Chunk::<MaterialId>::default();
    for v in c.voxels.iter_mut() {
        *v = MaterialId(1);
    }
    c
}

fn checkerboard_chunk() -> Chunk<MaterialId> {
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

// ---------------------------------------------------------------------------
// Benchmark groups
// ---------------------------------------------------------------------------

fn bench_cubic(c: &mut Criterion) {
    let shapes: &[(&str, Chunk<MaterialId>)] = &[
        ("empty", empty_chunk()),
        ("sparse", sparse_chunk()),
        ("dense_solid", dense_solid_chunk()),
        ("checkerboard", checkerboard_chunk()),
    ];

    let mut group = c.benchmark_group("cubic_mesher");
    group.throughput(Throughput::Elements(CHUNK_VOXELS as u64));

    for (name, chunk) in shapes {
        group.bench_with_input(BenchmarkId::from_parameter(name), name, |b, _| {
            b.iter(|| {
                let view = ChunkView {
                    id: ChunkId(0),
                    voxels: black_box(&chunk.voxels),
                };
                black_box(CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)))
            })
        });
    }
    group.finish();
}

fn bench_greedy(c: &mut Criterion) {
    let shapes: &[(&str, Chunk<MaterialId>)] = &[
        ("empty", empty_chunk()),
        ("sparse", sparse_chunk()),
        ("dense_solid", dense_solid_chunk()),
        ("checkerboard", checkerboard_chunk()),
    ];

    let mut group = c.benchmark_group("greedy_mesher");
    group.throughput(Throughput::Elements(CHUNK_VOXELS as u64));

    for (name, chunk) in shapes {
        group.bench_with_input(BenchmarkId::from_parameter(name), name, |b, _| {
            b.iter(|| {
                let view = ChunkView {
                    id: ChunkId(0),
                    voxels: black_box(&chunk.voxels),
                };
                black_box(GreedyMesher::<MaterialId>::mesh_greedy(view, LodLevel(0)))
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_cubic, bench_greedy);
criterion_main!(benches);
