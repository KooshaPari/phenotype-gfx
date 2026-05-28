//! Triangle-count regression guards: GreedyMesher must never produce more
//! triangles than CubicMesher for any canonical chunk shape.
//!
//! For the *dense-solid* case, the guard is strict: greedy must produce
//! *fewer* triangles (not just equal), locking in the key optimisation so any
//! future regression is caught immediately.
//!
//! These are regular `#[test]` items — not benchmarks — so `cargo test` catches
//! regressions in CI without needing a timing baseline.

use phenotype_voxel::{
    chunk::{Chunk, ChunkId, ChunkView, CHUNK_EDGE, CHUNK_VOXELS},
    cubic_mesher::CubicMesher,
    greedy_mesher::GreedyMesher,
    lod::LodLevel,
    material::MaterialId,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_view(chunk: &Chunk<MaterialId>) -> ChunkView<'_, MaterialId> {
    ChunkView { id: ChunkId(0), voxels: &chunk.voxels }
}

fn cubic_tris(chunk: &Chunk<MaterialId>) -> usize {
    CubicMesher::<MaterialId>::mesh_cubic(make_view(chunk), LodLevel(0))
        .expect("cubic mesh")
        .indices
        .len()
        / 3
}

fn greedy_tris(chunk: &Chunk<MaterialId>) -> usize {
    GreedyMesher::<MaterialId>::mesh_greedy(make_view(chunk), LodLevel(0))
        .expect("greedy mesh")
        .indices
        .len()
        / 3
}

fn empty_chunk() -> Chunk<MaterialId> {
    Chunk::<MaterialId>::default()
}

fn sparse_chunk() -> Chunk<MaterialId> {
    let mut c = Chunk::<MaterialId>::default();
    let step = 3_usize;
    let mut count = 0;
    'outer: for z in (0..CHUNK_EDGE).step_by(step) {
        for y in (0..CHUNK_EDGE).step_by(step) {
            for x in (0..CHUNK_EDGE).step_by(step) {
                c.voxels[x + y * CHUNK_EDGE + z * CHUNK_EDGE * CHUNK_EDGE] = MaterialId(1);
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
    Chunk {
        voxels: vec![MaterialId(1); CHUNK_VOXELS],
    }
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
// Regression guards
// ---------------------------------------------------------------------------

/// FR-REGRESS-MESHER-001: empty chunk — both meshers produce zero triangles.
#[test]
fn empty_chunk_zero_triangles() {
    let c = empty_chunk();
    let cubic = cubic_tris(&c);
    let greedy = greedy_tris(&c);
    assert_eq!(cubic, 0, "cubic: empty chunk must yield 0 triangles");
    assert_eq!(greedy, 0, "greedy: empty chunk must yield 0 triangles");
    // greedy <= cubic trivially holds (0 <= 0).
}

/// FR-REGRESS-MESHER-002: sparse chunk — greedy produces <= cubic triangles.
#[test]
fn sparse_chunk_greedy_le_cubic() {
    let c = sparse_chunk();
    let cubic = cubic_tris(&c);
    let greedy = greedy_tris(&c);
    assert!(
        greedy <= cubic,
        "sparse: greedy ({greedy} tris) must be <= cubic ({cubic} tris)"
    );
    eprintln!("[REGRESS-MESHER-002] sparse — cubic: {cubic} tris, greedy: {greedy} tris");
}

/// FR-REGRESS-MESHER-003: dense-solid full 16³ chunk — greedy produces
/// *strictly fewer* triangles than cubic (the key optimisation must hold).
///
/// Cubic emits one quad per exposed voxel-face: 6 * CHUNK_EDGE² outer faces
/// each at 2 triangles = 12 * CHUNK_EDGE² triangles.
/// Greedy merges the entire face of the cube into a single quad per side:
/// 6 quads * 2 triangles = 12 triangles.
#[test]
fn dense_solid_chunk_greedy_strictly_fewer_triangles() {
    let c = dense_solid_chunk();
    let cubic = cubic_tris(&c);
    let greedy = greedy_tris(&c);

    // Sanity-check the cubic count: 6 sides × CHUNK_EDGE² quads × 2 tris.
    let expected_cubic = 6 * CHUNK_EDGE * CHUNK_EDGE * 2;
    assert_eq!(
        cubic, expected_cubic,
        "cubic dense-solid triangle count sanity check failed: got {cubic}, expected {expected_cubic}"
    );

    // Greedy must merge all co-planar same-material quads into 1 per side → 12 tris.
    let expected_greedy = 6 * 2; // 6 faces, each 1 merged quad = 2 tris
    assert_eq!(
        greedy, expected_greedy,
        "greedy dense-solid must produce {expected_greedy} tris (1 merged quad/side), got {greedy}"
    );

    // Strict inequality: this is the regression lock.
    assert!(
        greedy < cubic,
        "REGRESSION: greedy ({greedy} tris) must be STRICTLY FEWER than cubic ({cubic} tris) for dense-solid chunk"
    );

    let reduction_pct = 100.0 * (1.0 - greedy as f64 / cubic as f64);
    eprintln!(
        "[REGRESS-MESHER-003] dense-solid 16³ — cubic: {cubic} tris, greedy: {greedy} tris, reduction: {reduction_pct:.1}%"
    );
}

/// FR-REGRESS-MESHER-004: checkerboard chunk — greedy produces <= cubic triangles.
///
/// Checkerboard is greedy's hardest case because no two adjacent visible faces
/// share the same solid voxel, so theoretically no merging is possible.  The
/// guard is non-strict (<=) to allow for equal performance without a false failure.
#[test]
fn checkerboard_chunk_greedy_le_cubic() {
    let c = checkerboard_chunk();
    let cubic = cubic_tris(&c);
    let greedy = greedy_tris(&c);
    assert!(
        greedy <= cubic,
        "checkerboard: greedy ({greedy} tris) must be <= cubic ({cubic} tris)"
    );
    eprintln!("[REGRESS-MESHER-004] checkerboard — cubic: {cubic} tris, greedy: {greedy} tris");
}
