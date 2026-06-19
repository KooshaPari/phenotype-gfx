use criterion::{black_box, criterion_group, criterion_main, Criterion};
use phenotype_gfx::voxel::{voxelize_image, ExtrusionMode, VoxelizeConfig};

fn random_rgba_image(width: u32, height: u32, seed: u32) -> Vec<[u8; 4]> {
    let mut state = seed;
    let mut pixels = Vec::with_capacity((width * height) as usize);

    for _ in 0..(width * height) {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        pixels.push([
            (state & 0xFF) as u8,
            ((state >> 8) & 0xFF) as u8,
            ((state >> 16) & 0xFF) as u8,
            ((state >> 24) & 0xFF) as u8,
        ]);
    }

    pixels
}

fn bench_voxelize_image(c: &mut Criterion) {
    let pixels = random_rgba_image(32, 32, 0xC0FFEE_u32);
    let cfg = VoxelizeConfig {
        depth: 8,
        mode: ExtrusionMode::Flat,
        ..Default::default()
    };

    c.bench_function("voxelize_image_32x32_flat_depth8", |b| {
        b.iter(|| {
            black_box(voxelize_image(
                black_box(&pixels),
                black_box(32),
                black_box(32),
                black_box(&cfg),
            ))
        })
    });
}

criterion_group!(benches, bench_voxelize_image);
criterion_main!(benches);
