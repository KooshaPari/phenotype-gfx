//! Round-trip integration test for the voxel meshers.
//!
//! Validates that [`CubicMesher`] and [`GreedyMesher`] both:
//! - Produce a non-empty, well-formed [`MeshBuffer`].
//! - Carry a bounding box that matches the chunk extents.
//! - Emit a triangle/face count that is plausible for the input pattern.
//! - Can be **inverted**: walking the triangle stream and decoding each
//!   quad's outward normal + extents recovers exactly the original exposed
//!   voxel set.
//!
//! The test pattern is deliberately chosen so every solid voxel is
//! **exposed** (no buried voxels):
//!
//! - Solid top layer: `z = 15`, all `(x, y)` ∈ [0, 16)².
//! - Pillar at center: `(x = 8, y = 8, z)` ∈ [0, 16).
//!
//! The pillar's top voxel `(8, 8, 15)` is shared with the top layer; the
//! pillar itself has no horizontal neighbours, and the chunk boundary
//! exposes the pillar bottom.

use std::collections::BTreeSet;

use phenotype_gfx::voxel::chunk::{Chunk, ChunkId, ChunkView, CHUNK_EDGE};
use phenotype_gfx::voxel::coord::{ChunkCoord, FIXED_SCALE, WorldCoord};
use phenotype_gfx::voxel::cubic_mesher::{CubicMesher, CubicVoxel};
use phenotype_gfx::voxel::greedy_mesher::GreedyMesher;
use phenotype_gfx::voxel::lod::LodLevel;
use phenotype_gfx::voxel::material::MaterialId;
use phenotype_gfx::voxel::mesh::MeshBuffer;
use phenotype_gfx::voxel::world::VoxelWorld;

const EDGE_I: i32 = CHUNK_EDGE as i32; // 16
const VOXEL_SPAN: i64 = FIXED_SCALE;

// ---------------------------------------------------------------------------
// u8-backed voxel wrapper.
//
// `u8` itself doesn't implement [`CubicVoxel`], so we wrap it. The wrapper
// is what the mesher sees; the world stores it as `VoxelWorld<U8Vox>` and we
// read/write through `u8` semantics: `0` = air, non-zero = solid + material.
// ---------------------------------------------------------------------------

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct U8Vox(u8);

impl CubicVoxel for U8Vox {
    fn is_solid(&self) -> bool {
        self.0 != 0
    }
    fn material(&self) -> MaterialId {
        MaterialId(self.0 as u16)
    }
}

#[inline]
fn chunk_idx(x: i32, y: i32, z: i32) -> usize {
    (x as usize) + (y as usize) * CHUNK_EDGE + (z as usize) * CHUNK_EDGE * CHUNK_EDGE
}

/// Compute the canonical set of solid voxels for the round-trip pattern.
fn expected_pattern_set() -> BTreeSet<(i32, i32, i32)> {
    let mut set: BTreeSet<(i32, i32, i32)> = BTreeSet::new();
    // Solid top layer: z = 15, all (x, y) ∈ [0, 16)².
    for x in 0..EDGE_I {
        for y in 0..EDGE_I {
            set.insert((x, y, EDGE_I - 1));
        }
    }
    // Pillar at center: (8, 8, z) for z ∈ [0, 16). The (8, 8, 15) entry
    // is already in the top-layer set; the BTreeSet dedupes it for us.
    let cx = EDGE_I / 2;
    let cy = EDGE_I / 2;
    for z in 0..EDGE_I {
        set.insert((cx, cy, z));
    }
    set
}

// ---------------------------------------------------------------------------
// Round-trip reconstruction.
//
// Both meshers emit quads (two CCW triangles sharing two vertices). For each
// quad we read the four unique vertex positions and the outward normal.
// - The normal axis is constant across the four vertices; its sign tells us
//   whether this is a +n or -n face.
// - For a +n face at integer plane `N`, the owning voxel sits at `N - 1`
//   along that axis (the face is the cube's outer wall at `voxel + 1`).
// - For a -n face at integer plane `N`, the owning voxel sits at `N`.
// - The other two axes span `[min, max)` in vertex space; every integer in
//   that half-open range is a voxel whose exposed face is part of this quad.
//
// Cubic emits one quad per exposed voxel face; greedy emits merged quads
// whose tangent extents cover multiple voxels. Both decode identically.
// ---------------------------------------------------------------------------

fn reconstruct_voxels(buf: &MeshBuffer) -> BTreeSet<(i32, i32, i32)> {
    let mut voxels: BTreeSet<(i32, i32, i32)> = BTreeSet::new();

    for tri_pair in buf.indices.chunks_exact(6) {
        let v0 = buf.vertices[tri_pair[0] as usize].position;
        let v1 = buf.vertices[tri_pair[1] as usize].position;
        let v2 = buf.vertices[tri_pair[2] as usize].position;
        let v3 = buf.vertices[tri_pair[4] as usize].position; // shared with 2nd tri

        let min = [
            v0[0].min(v1[0]).min(v2[0]).min(v3[0]),
            v0[1].min(v1[1]).min(v2[1]).min(v3[1]),
            v0[2].min(v1[2]).min(v2[2]).min(v3[2]),
        ];
        let max = [
            v0[0].max(v1[0]).max(v2[0]).max(v3[0]),
            v0[1].max(v1[1]).max(v2[1]).max(v3[1]),
            v0[2].max(v1[2]).max(v2[2]).max(v3[2]),
        ];

        // Axis-aligned outward normal: pick v0 (all four vertices share it).
        let normal = buf.vertices[tri_pair[0] as usize].normal;
        let (axis, sign) = if normal[0].abs() > 0.5 {
            (0_usize, normal[0].signum() as i32)
        } else if normal[1].abs() > 0.5 {
            (1_usize, normal[1].signum() as i32)
        } else {
            (2_usize, normal[2].signum() as i32)
        };
        debug_assert!(sign != 0, "axis-aligned normal must be ±1, got {:?}", normal);

        // Face-plane integer coordinate on the normal axis. Both meshers emit
        // vertices at exact integer coordinates (face_d is `d as f32` or
        // `d as f32 + 1.0` for greedy; cubic emits `fx ± {0,1}`), so
        // `min[axis]` is already the integer face plane.
        let face_coord = min[axis].round() as i32;
        // Voxel coord: +face → voxel at face_coord - 1; -face → voxel at face_coord.
        let voxel_axis = if sign > 0 { face_coord - 1 } else { face_coord };

        // Tangent axes.
        let (u_axis, v_axis) = match axis {
            0 => (1_usize, 2_usize),
            1 => (2_usize, 0_usize),
            _ => (0_usize, 1_usize),
        };
        let u_min = min[u_axis].round() as i32;
        let u_max_inclusive = max[u_axis].round() as i32 - 1;
        let v_min = min[v_axis].round() as i32;
        let v_max_inclusive = max[v_axis].round() as i32 - 1;

        for u in u_min..=u_max_inclusive {
            for v in v_min..=v_max_inclusive {
                let mut voxel = [0_i32; 3];
                voxel[axis] = voxel_axis;
                voxel[u_axis] = u;
                voxel[v_axis] = v;
                voxels.insert((voxel[0], voxel[1], voxel[2]));
            }
        }
    }

    voxels
}

/// Run both meshers against the same chunk and return (cubic_mesh, greedy_mesh).
fn mesh_both<V: CubicVoxel + Copy>(voxels: &[V]) -> (MeshBuffer, MeshBuffer) {
    let view = ChunkView {
        id: ChunkId(0),
        voxels,
    };
    let cubic = CubicMesher::<V>::mesh_cubic(view, LodLevel(0)).expect("cubic mesh");
    let greedy = GreedyMesher::<V>::mesh_greedy(view, LodLevel(0)).expect("greedy mesh");
    (cubic, greedy)
}

// ---------------------------------------------------------------------------
// Test 1 — pattern round-trip on the canonical test pattern.
// ---------------------------------------------------------------------------

#[test]
fn voxel_mesher_round_trip_pattern() {
    // a) Create a VoxelWorld<u8> (U8Vox is the u8 wrapper that carries the
    //    CubicVoxel impl; the world stores u8-equivalent values).
    let mut world = VoxelWorld::<U8Vox>::new(VOXEL_SPAN);

    // b) Fill a 16×16×16 chunk with the known pattern via world writes.
    for x in 0..EDGE_I {
        for y in 0..EDGE_I {
            world.write(
                WorldCoord {
                    x: x as i64 * VOXEL_SPAN,
                    y: y as i64 * VOXEL_SPAN,
                    z: (EDGE_I - 1) as i64 * VOXEL_SPAN,
                },
                U8Vox(1),
            );
        }
    }
    for z in 0..EDGE_I {
        world.write(
            WorldCoord {
                x: (EDGE_I / 2) as i64 * VOXEL_SPAN,
                y: (EDGE_I / 2) as i64 * VOXEL_SPAN,
                z: z as i64 * VOXEL_SPAN,
            },
            U8Vox(1),
        );
    }

    // Drain dirty events and sanity-check that the world actually wrote voxels.
    let dirty = world.drain_dirty();
    assert!(!dirty.is_empty(), "writes should produce dirty events");
    assert_eq!(world.chunk_count(), 1, "all writes land in the origin chunk");

    let coord = ChunkCoord {
        cx: 0,
        cy: 0,
        cz: 0,
    };
    let chunk = world.chunk(coord).expect("chunk must exist after writes");
    assert_eq!(chunk.voxels.len(), CHUNK_EDGE * CHUNK_EDGE * CHUNK_EDGE);

    let expected = expected_pattern_set();
    // Sanity-check: total solid voxels = 256 top + 16 pillar, but the
    // (8, 8, 15) entry is shared → 271 distinct set entries.
    assert_eq!(
        expected.len(),
        256 + 16 - 1,
        "expected pattern set size mismatch: {} entries",
        expected.len()
    );

    let (cubic_mesh, greedy_mesh) = mesh_both(&chunk.voxels);

    // d) Basic sanity: > 0 verts and > 0 indices.
    assert!(!cubic_mesh.vertices.is_empty(), "cubic mesh must have vertices");
    assert!(!cubic_mesh.indices.is_empty(), "cubic mesh must have indices");
    assert!(!greedy_mesh.vertices.is_empty(), "greedy mesh must have vertices");
    assert!(!greedy_mesh.indices.is_empty(), "greedy mesh must have indices");

    // All indices reference valid vertex slots (well-formed).
    for m in [&cubic_mesh, &greedy_mesh] {
        let vc = m.vertices.len() as u32;
        assert!(m.indices.iter().all(|&i| i < vc), "index out of bounds");
        assert_eq!(m.ao.len(), m.vertices.len(), "ao.len() must equal vertices.len()");
        assert_eq!(m.indices.len() % 3, 0, "indices must be multiple of 3");
    }

    // e) Bounding box matches voxel extents [0, 16] on every axis.
    for m in [&cubic_mesh, &greedy_mesh] {
        let (bmin, bmax) = m.compute_bounds();
        assert_eq!(bmin, [0.0, 0.0, 0.0], "bbox min must be at chunk origin");
        assert_eq!(
            bmax,
            [EDGE_I as f32, EDGE_I as f32, EDGE_I as f32],
            "bbox max must be at chunk far corner"
        );
    }

    // f) Face count plausibility. Each quad = 1 face = 6 indices. The
    //    cubic mesher emits one quad per exposed voxel face; for our
    //    pattern that's:
    //      - top plane +z: 256 quads
    //      - top plane -z: 255 quads (the pillar voxel (8, 8, 15) has -z
    //        buried against (8, 8, 14))
    //      - top plane perimeter (±x, ±y): 4 × 16 = 64 quads
    //      - pillar z = 0..14 (15 voxels × 5 exposed faces): 75 quads
    //    → 650 quads total. We assert a tight range to catch silent
    //    regressions in the cubic mesher.
    //
    //    The greedy mesher aggressively merges coplanar same-AO-signature
    //    faces. For this pattern the AO signatures are highly uniform in
    //    the interior of the top plane (most vertices have AO=0 because
    //    all tangent neighbours are solid top-plane voxels) but split
    //    around the pillar hole and at chunk-boundary edges. Greedy ends
    //    up with ~30–100 quads — an order of magnitude fewer than cubic.
    let cubic_quads = cubic_mesh.indices.len() / 6;
    let greedy_quads = greedy_mesh.indices.len() / 6;
    assert!(
        (600..=700).contains(&cubic_quads),
        "cubic quad count {cubic_quads} outside plausible range [600, 700]"
    );
    assert!(
        (10..=300).contains(&greedy_quads),
        "greedy quad count {greedy_quads} outside plausible range [10, 300]"
    );
    // Greedy must collapse significantly for this pattern.
    assert!(
        greedy_quads * 5 < cubic_quads,
        "greedy ({greedy_quads}) should be < 1/5 of cubic ({cubic_quads})"
    );

    // g/h) Round-trip: walk each mesh and reconstruct the voxel set.
    let reconstructed_cubic = reconstruct_voxels(&cubic_mesh);
    let reconstructed_greedy = reconstruct_voxels(&greedy_mesh);

    assert_eq!(
        reconstructed_cubic, expected,
        "cubic round-trip must reconstruct the exact original voxel set"
    );
    assert_eq!(
        reconstructed_greedy, expected,
        "greedy round-trip must reconstruct the exact original voxel set"
    );

    // i) Repeat the round-trip assertion for greedy (already done above);
    //    explicitly re-state it for the task's enumerated step ordering.
    assert_eq!(reconstructed_cubic, reconstructed_greedy,
        "both meshers must reconstruct the same voxel set");

    // j) Greedy vertex count must be ≤ cubic (greedy merges coplanar faces).
    assert!(
        greedy_mesh.vertices.len() <= cubic_mesh.vertices.len(),
        "greedy ({}) must have <= vertices than cubic ({})",
        greedy_mesh.vertices.len(),
        cubic_mesh.vertices.len()
    );
    assert!(
        greedy_mesh.indices.len() <= cubic_mesh.indices.len(),
        "greedy ({}) must have <= indices than cubic ({})",
        greedy_mesh.indices.len(),
        cubic_mesh.indices.len()
    );

    eprintln!(
        "[ROUND-TRIP] pattern quads: cubic={cubic_quads}, greedy={greedy_quads} ({:.1}% reduction); \
         verts: cubic={}, greedy={}; round-trip voxels reconstructed: {}",
        100.0 * (1.0 - (greedy_mesh.indices.len() as f64) / (cubic_mesh.indices.len() as f64)),
        cubic_mesh.vertices.len(),
        greedy_mesh.vertices.len(),
        reconstructed_cubic.len()
    );
}

// ---------------------------------------------------------------------------
// Test 2 — round-trip on a degenerate-but-meaningful pattern: a 4×4 solid
// slab at the chunk floor (y = 0). All 64 voxels are exposed (no buried
// voxels because the slab is exactly one cell thick and the chunk boundary
// provides the bottom), and greedy must collapse the slab into a thin set
// of large quads.
// ---------------------------------------------------------------------------

#[test]
fn voxel_mesher_round_trip_slab() {
    let mut chunk = Chunk::<U8Vox>::default();
    let mut expected: BTreeSet<(i32, i32, i32)> = BTreeSet::new();
    for x in 0..4_i32 {
        for z in 0..4_i32 {
            chunk.voxels[chunk_idx(x, 0, z)] = U8Vox(1);
            expected.insert((x, 0, z));
        }
    }

    let (cubic_mesh, greedy_mesh) = mesh_both(&chunk.voxels);

    assert_eq!(reconstruct_voxels(&cubic_mesh), expected);
    assert_eq!(reconstruct_voxels(&greedy_mesh), expected);

    // 4×4 slab: cubic emits 16 (top) + 16 (bottom, OOB) + 16 (perimeter) =
    // 48 quads = 192 verts. Greedy should collapse to far fewer.
    assert!(
        greedy_mesh.vertices.len() < cubic_mesh.vertices.len(),
        "greedy ({} verts) must be strictly smaller than cubic ({} verts) on flat slab",
        greedy_mesh.vertices.len(),
        cubic_mesh.vertices.len()
    );

    let cubic_quads = cubic_mesh.indices.len() / 6;
    let greedy_quads = greedy_mesh.indices.len() / 6;
    eprintln!(
        "[ROUND-TRIP-SLAB] 4×4×1 slab — cubic: {cubic_quads} quads, greedy: {greedy_quads} quads"
    );
}

// ---------------------------------------------------------------------------
// Test 3 — round-trip on a single isolated voxel. Both meshers must agree
// on the recovered set (single voxel) and on the surface area.
// ---------------------------------------------------------------------------

#[test]
fn voxel_mesher_round_trip_single_voxel() {
    let mut chunk = Chunk::<U8Vox>::default();
    chunk.voxels[chunk_idx(3, 4, 5)] = U8Vox(1);
    let mut expected = BTreeSet::new();
    expected.insert((3, 4, 5));

    let (cubic_mesh, greedy_mesh) = mesh_both(&chunk.voxels);

    assert_eq!(reconstruct_voxels(&cubic_mesh), expected);
    assert_eq!(reconstruct_voxels(&greedy_mesh), expected);

    // Single voxel: 6 faces × 4 verts = 24 verts, 36 indices. Greedy
    // must match (no merging possible without coplanar neighbours).
    assert_eq!(cubic_mesh.vertices.len(), 24);
    assert_eq!(cubic_mesh.indices.len(), 36);
    assert_eq!(greedy_mesh.vertices.len(), 24);
    assert_eq!(greedy_mesh.indices.len(), 36);

    // Bounding box covers just that voxel: [3,4] × [4,5] × [5,6].
    let (bmin, bmax) = cubic_mesh.compute_bounds();
    assert_eq!(bmin, [3.0, 4.0, 5.0]);
    assert_eq!(bmax, [4.0, 5.0, 6.0]);
}
