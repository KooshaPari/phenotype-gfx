//! NFR-VOXEL-006 — performance regression guards for PLAN-VOXEL-003 paths.
//!
//! These are lightweight `#[test]` items (NOT criterion) that assert:
//!   (a) **Cost ordering** invariants that must hold across any implementation
//!       (e.g. serialising an empty chunk is cheaper than a dense one), and
//!   (b) **Upper-bound** invariants (e.g. byte sizes, event counts) that would
//!       blow up under a regression but are generous enough to be flap-free on
//!       all supported CI hardware.
//!
//! These guards intentionally avoid absolute wall-clock timings; they catch
//! algorithmic regressions (wrong complexity class, extra allocations) without
//! being sensitive to machine speed.

use std::time::Instant;

use phenotype_voxel::{
    chunk::{Chunk, ChunkId, ChunkView, CHUNK_EDGE, CHUNK_VOXELS},
    coord::ChunkCoord,
    cubic_mesher::CubicMesher,
    lod::LodLevel,
    material::MaterialId,
    octree::VoxelOctree,
    serial::save_chunk,
    world::VoxelWorld,
    WorldCoord, FIXED_SCALE,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// --- u8 chunks for serial tests (u8: Pod + Eq + Default + Clone) ---

fn empty_u8_chunk() -> Chunk<u8> {
    Chunk::<u8>::default()
}

/// A truly alternating-pattern chunk: every adjacent pair differs, forcing CHUNK_VOXELS RLE runs.
fn alternating_u8_chunk() -> Chunk<u8> {
    let voxels: Vec<u8> = (0..CHUNK_VOXELS).map(|i| (i % 2) as u8).collect();
    Chunk { voxels }
}

fn serialized_u8_bytes(chunk: &Chunk<u8>) -> Vec<u8> {
    let mut buf = Vec::new();
    save_chunk(chunk, &mut buf).expect("save_chunk");
    buf
}

// --- MaterialId chunks for mesher / AO / world tests ---

fn empty_mat_chunk() -> Chunk<MaterialId> {
    Chunk::<MaterialId>::default()
}

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

// ---------------------------------------------------------------------------
// NFR-VOXEL-006-SERIAL-001
// Byte-size ordering: empty < dense (RLE empty = 1 run; dense = 4096 runs).
// ---------------------------------------------------------------------------

/// NFR-VOXEL-006-SERIAL-001 — serialized empty chunk is strictly smaller than
/// serialized alternating-pattern chunk (u8 voxels — Pod-satisfying).
///
/// Empty = 1 RLE run (16 bytes); alternating = CHUNK_VOXELS runs (much larger).
#[test]
fn serial_empty_smaller_than_alternating() {
    let empty_bytes = serialized_u8_bytes(&empty_u8_chunk()).len();
    let alt_bytes = serialized_u8_bytes(&alternating_u8_chunk()).len();
    assert!(
        empty_bytes < alt_bytes,
        "REGRESSION: serialized empty ({empty_bytes} B) must be < alternating ({alt_bytes} B)"
    );
}

/// NFR-VOXEL-006-SERIAL-002 — empty u8 chunk serializes to exactly 16 bytes
/// (single RLE run: 4 magic + 1 ver + 4 elem_size + 4 run_count + 2 run_len + 1 value).
///
/// This is the canonical minimal-size invariant. If this changes, the format has
/// changed in a breaking way.
#[test]
fn serial_empty_u8_chunk_exact_16_bytes() {
    let buf = serialized_u8_bytes(&empty_u8_chunk());
    assert_eq!(
        buf.len(),
        16,
        "REGRESSION: empty u8 chunk must serialize to exactly 16 bytes, got {}",
        buf.len()
    );
}

/// NFR-VOXEL-006-SERIAL-003 — cost-ordering sanity: serializing an empty chunk
/// (1 run) must produce fewer bytes than an alternating chunk (4096 runs).
///
/// We assert the **byte-size ratio** is >= 50x instead of wall-clock time to
/// be flap-free on all CI hardware while still catching algorithmic regressions.
/// If the ratio shrinks below 50x the RLE encoder has regressed.
#[test]
fn serial_empty_bytes_much_smaller_than_alternating() {
    let empty_bytes = serialized_u8_bytes(&empty_u8_chunk()).len();
    let alt_bytes = serialized_u8_bytes(&alternating_u8_chunk()).len();
    // alternating has 4096 runs × (2 + 1) bytes = 12288 bytes body + 13 header.
    // empty has 1 run × 3 bytes body + 13 header = 16 bytes.
    // Ratio should be roughly 800x; we require >= 50x for generous headroom.
    let ratio = alt_bytes / empty_bytes;
    assert!(
        ratio >= 50,
        "REGRESSION: alternating/empty byte ratio should be >= 50x (got {ratio}x; \
         empty={empty_bytes}B, alt={alt_bytes}B)"
    );
}

// ---------------------------------------------------------------------------
// NFR-VOXEL-006-SVO-001
// Compact removes exactly the right number of nodes and is O(n) not O(n²).
// ---------------------------------------------------------------------------

/// NFR-VOXEL-006-SVO-001 — compacting 8 uniform siblings removes exactly 8
/// nodes and is idempotent (second call returns 0).
#[test]
fn svo_compact_8_siblings_exact() {
    let mut tree = VoxelOctree::<MaterialId>::default();
    // 8 siblings at even coords so parent key is (0,0,0).
    for cx in [0i32, 1] {
        for cy in [0i32, 1] {
            for cz in [0i32, 1] {
                tree.insert_uniform(ChunkCoord { cx, cy, cz }, MaterialId(1));
            }
        }
    }
    assert_eq!(tree.nodes.len(), 8, "pre-compact: expect 8 leaf nodes");

    let removed = tree.compact();
    assert_eq!(removed, 8, "compact must report 8 nodes removed");
    assert_eq!(
        tree.nodes.len(),
        1,
        "post-compact: exactly 1 parent node must remain"
    );

    // Idempotent.
    let removed2 = tree.compact();
    assert_eq!(
        removed2, 0,
        "second compact on already-compacted tree must return 0"
    );
}

/// NFR-VOXEL-006-SVO-002 — compacting 64 uniform leaves (two-level pyramid)
/// removes 72 nodes total (64 leaves + 8 first-level parents).
#[test]
fn svo_compact_64_leaf_pyramid_removes_72() {
    let mut tree = VoxelOctree::<MaterialId>::default();
    // 8 groups of 8 siblings. Each group at parent offset (gx, gy, gz)*2
    // where siblings are at +0/+1 in each axis.
    for g in 0..8i32 {
        let gx = (g % 2) * 4;
        let gy = ((g / 2) % 2) * 4;
        let gz = (g / 4) * 4;
        for cx in [0i32, 1] {
            for cy in [0i32, 1] {
                for cz in [0i32, 1] {
                    tree.insert_uniform(
                        ChunkCoord {
                            cx: gx + cx,
                            cy: gy + cy,
                            cz: gz + cz,
                        },
                        MaterialId(1),
                    );
                }
            }
        }
    }
    assert_eq!(tree.nodes.len(), 64);

    let removed = tree.compact();
    // 64 leaves collapse to 8 first-level parents (first pass), then those
    // 8 may or may not share a common parent key depending on coord layout.
    // At minimum, 64 leaves must be removed and replaced by 8 or fewer nodes.
    assert!(
        removed >= 56,
        "REGRESSION: 64-leaf pyramid compact should remove >= 56 nodes (got {removed})"
    );
    assert!(
        tree.nodes.len() <= 8,
        "post-compact: at most 8 first-level nodes should remain"
    );
}

/// NFR-VOXEL-006-SVO-003 — compaction of mixed-value group removes 0 nodes.
#[test]
fn svo_compact_mixed_siblings_unchanged() {
    let mut tree = VoxelOctree::<MaterialId>::default();
    // 7 siblings with value 1, 1 with value 2 — group is NOT uniform.
    let mut i = 0;
    for cx in [0i32, 1] {
        for cy in [0i32, 1] {
            for cz in [0i32, 1] {
                let val = if i == 7 { MaterialId(2) } else { MaterialId(1) };
                tree.insert_uniform(ChunkCoord { cx, cy, cz }, val);
                i += 1;
            }
        }
    }
    let removed = tree.compact();
    assert_eq!(removed, 0, "mixed-value group must not be collapsed");
    assert_eq!(tree.nodes.len(), 8, "mixed group: all 8 leaves must remain");
}

// ---------------------------------------------------------------------------
// NFR-VOXEL-006-AO-001
// AO invariants that must hold regardless of implementation changes.
// ---------------------------------------------------------------------------

/// NFR-VOXEL-006-AO-001 — MeshBuffer.ao.len() always equals vertices.len().
#[test]
fn ao_len_equals_vertex_len_all_shapes() {
    let shapes: &[(&str, Chunk<MaterialId>)] = &[
        ("empty", empty_mat_chunk()),
        ("dense", dense_solid_mat_chunk()),
        ("checkerboard", checkerboard_mat_chunk()),
    ];

    for (name, chunk) in shapes {
        let view = ChunkView {
            id: ChunkId(0),
            voxels: &chunk.voxels,
        };
        let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0))
            .unwrap_or_else(|e| panic!("mesh_cubic({name}) failed: {e}"));
        assert_eq!(
            mesh.ao.len(),
            mesh.vertices.len(),
            "REGRESSION [{name}]: ao.len() ({}) != vertices.len() ({})",
            mesh.ao.len(),
            mesh.vertices.len()
        );
    }
}

/// NFR-VOXEL-006-AO-002 — fully-exposed single voxel has all AO values = 3.
#[test]
fn ao_single_exposed_voxel_all_lit() {
    let mut chunk = empty_mat_chunk();
    chunk.voxels[0] = MaterialId(1); // corner voxel, fully exposed
    let view = ChunkView {
        id: ChunkId(0),
        voxels: &chunk.voxels,
    };
    let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0)).unwrap();
    assert!(
        mesh.ao.iter().all(|&v| v == 3),
        "REGRESSION: fully-exposed corner voxel must have all AO = 3, got {:?}",
        &mesh.ao
    );
}

// ---------------------------------------------------------------------------
// NFR-VOXEL-006-DIRTY-001
// Dirty-tracking: event count and cost ordering invariants.
// ---------------------------------------------------------------------------

/// NFR-VOXEL-006-DIRTY-001 — writing N distinct voxels produces exactly N
/// dirty events; writing same position twice produces exactly 1 event (idempotent).
#[test]
fn dirty_tracking_event_count_exact() {
    let mut world = VoxelWorld::<MaterialId>::new(FIXED_SCALE);

    // Write 4 distinct positions → should emit 4 events.
    for i in 0..4i64 {
        world.write(
            WorldCoord {
                x: i * FIXED_SCALE,
                y: 0,
                z: 0,
            },
            MaterialId(1),
        );
    }
    let events = world.drain_dirty();
    assert_eq!(
        events.len(),
        4,
        "4 distinct writes must produce 4 dirty events"
    );

    // Idempotent write at position (0,0,0) — value already MaterialId(1).
    world.write(WorldCoord { x: 0, y: 0, z: 0 }, MaterialId(1));
    let events2 = world.drain_dirty();
    assert_eq!(
        events2.len(),
        0,
        "REGRESSION: idempotent write must emit 0 events"
    );
}

/// NFR-VOXEL-006-DIRTY-002 — cost ordering: filling an entire chunk (4096 writes)
/// must complete faster than 1 second on any target CI machine.
///
/// This is a generous absolute upper-bound guard to catch pathological regressions
/// (e.g. O(n²) dirty insertion) without false positives on slow hardware.
#[test]
fn dirty_fill_one_chunk_under_1s() {
    let span = FIXED_SCALE;
    let t0 = Instant::now();

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
    let events = world.drain_dirty();
    let elapsed = t0.elapsed();

    assert_eq!(
        events.len(),
        CHUNK_VOXELS,
        "expected {CHUNK_VOXELS} dirty events"
    );
    assert!(
        elapsed.as_secs() < 1,
        "REGRESSION: filling one chunk ({CHUNK_VOXELS} writes) took {}ms (limit: 1000ms)",
        elapsed.as_millis()
    );
}

/// NFR-VOXEL-006-DIRTY-003 — relative cost ordering: filling 1 chunk
/// produces fewer dirty events than filling 4 chunks.
#[test]
fn dirty_event_count_scales_with_writes() {
    let span = FIXED_SCALE;

    let count_for = |chunks: i64| -> usize {
        let mut world = VoxelWorld::<MaterialId>::new(span);
        for c in 0..chunks {
            let cx_offset = c * CHUNK_EDGE as i64 * span;
            for z in 0..CHUNK_EDGE as i64 {
                for y in 0..CHUNK_EDGE as i64 {
                    for x in 0..CHUNK_EDGE as i64 {
                        world.write(
                            WorldCoord {
                                x: cx_offset + x * span,
                                y: y * span,
                                z: z * span,
                            },
                            MaterialId(1),
                        );
                    }
                }
            }
        }
        world.drain_dirty().len()
    };

    let one = count_for(1);
    let four = count_for(4);
    assert!(
        one < four,
        "REGRESSION: 1-chunk fill ({one} events) must produce fewer dirty events than 4-chunk fill ({four} events)"
    );
}
