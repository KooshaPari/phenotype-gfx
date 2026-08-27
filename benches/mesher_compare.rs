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
use phenotype_gfx::voxel::{
    chunk::{Chunk, ChunkId, ChunkView, CHUNK_EDGE, CHUNK_VOXELS},
    cubic_mesher::CubicMesher,
    greedy_mesher::GreedyMesher,
    lod::LodLevel,
    material::MaterialId,
    simd::{
        simd_aabb_center_batch, simd_conditional_mix_batch, simd_dot_batch, simd_normals_batch,
    },
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

criterion_group!(benches, bench_cubic, bench_greedy, bench_simd_vs_scalar);
criterion_main!(benches);

// ---------------------------------------------------------------------------
// SIMD vs Scalar benchmark comparisons (4 benchmarks)
// ---------------------------------------------------------------------------
//
// Each benchmark compares a scalar implementation (inline loop) against the
// SIMD-accelerated batch function from `phenotype_gfx::voxel::simd`.  When
// compiled with `--features simd` on x86_64, the SIMD path will use AVX2
// (8-wide batches) when the CPU supports it, falling back to SSE2.

fn bench_simd_vs_scalar(c: &mut Criterion) {
    const N: usize = 1024;

    // -------------------------------------------------------------------
    // 1. Normalise batch: scalar vs SIMD (`normalize8`/`normalize4`)
    // -------------------------------------------------------------------
    {
        let normals: Vec<[f32; 3]> = (0..N)
            .map(|i| {
                let f = i as f32 + 1.0;
                [f, f * 2.0, f * 3.0]
            })
            .collect();

        let mut group = c.benchmark_group("simd_vs_scalar_normalize");
        group.throughput(Throughput::Elements(N as u64));

        group.bench_function("scalar", |b| {
            b.iter(|| {
                let out: Vec<[f32; 3]> = normals
                    .iter()
                    .map(|&v| {
                        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                        if len <= f32::EPSILON {
                            [0.0; 3]
                        } else {
                            let inv = 1.0 / len;
                            [v[0] * inv, v[1] * inv, v[2] * inv]
                        }
                    })
                    .collect();
                black_box(out)
            })
        });

        group.bench_function("simd", |b| {
            b.iter(|| black_box(simd_normals_batch(black_box(&normals))))
        });

        group.finish();
    }

    // -------------------------------------------------------------------
    // 2. AABB centre batch: scalar vs SIMD (`aabb_center8`/`aabb_center4`)
    // -------------------------------------------------------------------
    {
        let bounds: Vec<[f32; 6]> = (0..N)
            .map(|i| {
                let f = i as f32;
                [f, f + 1.0, f + 2.0, f + 10.0, f + 11.0, f + 12.0]
            })
            .collect();

        let mut group = c.benchmark_group("simd_vs_scalar_aabb_center");
        group.throughput(Throughput::Elements(N as u64));

        group.bench_function("scalar", |b| {
            b.iter(|| {
                let out: Vec<[f32; 3]> = bounds
                    .iter()
                    .map(|bb| {
                        [
                            (bb[0] + bb[3]) * 0.5,
                            (bb[1] + bb[4]) * 0.5,
                            (bb[2] + bb[5]) * 0.5,
                        ]
                    })
                    .collect();
                black_box(out)
            })
        });

        group.bench_function("simd", |b| {
            b.iter(|| black_box(simd_aabb_center_batch(black_box(&bounds))))
        });

        group.finish();
    }

    // -------------------------------------------------------------------
    // 3. Dot product batch: scalar vs SIMD (`dot8`/`dot4`)
    // -------------------------------------------------------------------
    {
        let a: Vec<[f32; 3]> = (0..N)
            .map(|i| {
                let f = i as f32 + 1.0;
                [f, f * 0.5, f * 0.25]
            })
            .collect();
        let bv: Vec<[f32; 3]> = (0..N)
            .map(|i| {
                let f = i as f32 + 1.0;
                [f * 0.3, f * 0.7, f * 1.1]
            })
            .collect();

        let mut group = c.benchmark_group("simd_vs_scalar_dot");
        group.throughput(Throughput::Elements(N as u64));

        group.bench_function("scalar", |b| {
            b.iter(|| {
                let out: Vec<f32> = a
                    .iter()
                    .zip(bv.iter())
                    .map(|(av, bvv)| av[0] * bvv[0] + av[1] * bvv[1] + av[2] * bvv[2])
                    .collect();
                black_box(out)
            })
        });

        group.bench_function("simd", |b| {
            b.iter(|| black_box(simd_dot_batch(black_box(&a), black_box(&bv))))
        });

        group.finish();
    }

    // -------------------------------------------------------------------
    // 4. Conditional mix batch: scalar vs SIMD (`conditional_mix8`)
    // -------------------------------------------------------------------
    {
        let a: Vec<[f32; 3]> = (0..N)
            .map(|i| {
                let f = i as f32;
                [f, f + 1.0, f + 2.0]
            })
            .collect();
        let bv: Vec<[f32; 3]> = (0..N)
            .map(|i| {
                let f = i as f32;
                [f * 2.0, f * 3.0, f * 4.0]
            })
            .collect();
        let mask: Vec<[f32; 3]> = (0..N)
            .map(|i| {
                let m = (i as f32) / (N as f32);
                [m, m, m]
            })
            .collect();

        let mut group = c.benchmark_group("simd_vs_scalar_conditional_mix");
        group.throughput(Throughput::Elements(N as u64));

        group.bench_function("scalar", |b| {
            b.iter(|| {
                let out: Vec<[f32; 3]> = a
                    .iter()
                    .zip(bv.iter())
                    .zip(mask.iter())
                    .map(|((&av, &bvv), &mv)| {
                        [
                            av[0] * (1.0 - mv[0]) + bvv[0] * mv[0],
                            av[1] * (1.0 - mv[1]) + bvv[1] * mv[1],
                            av[2] * (1.0 - mv[2]) + bvv[2] * mv[2],
                        ]
                    })
                    .collect();
                black_box(out)
            })
        });

        group.bench_function("simd", |b| {
            b.iter(|| {
                black_box(simd_conditional_mix_batch(
                    black_box(&a),
                    black_box(&bv),
                    black_box(&mask),
                ))
            })
        });

        group.finish();
    }
}
