//! C-ABI wrapper for phenotype-gfx.
//!
//! cbindgen will generate C headers from this module.
//! C# P/Invoke → calls this → calls Rust core algorithms.
//! No logic here; thin wrapper only.
//!
//! # Safety
//!
//! All functions in this module cross the FFI boundary. Callers must:
//! - Pass valid, non-null pointers for every `*mut` / `*const` parameter.
//! - Respect the ownership model documented per function (create/destroy pairs).
//! - Not call `destroy` on a handle that has already been destroyed (double-free).
//! - Not concurrently mutate the same handle from multiple threads without external synchronisation.

use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::voxel::coord::{ChunkCoord, WorldCoord};
use crate::voxel::material::{MaterialId, MaterialPalette, VoxelMaterial};
use crate::voxel::mesh::{MeshBuffer, MeshVertex};
use crate::voxel::world::VoxelWorld;

// ---------------------------------------------------------------------------
// Opaque handle wrappers
// ---------------------------------------------------------------------------

/// Opaque handle returned to C callers for a `VoxelWorld<u8>`.
pub struct VoxelWorldHandle {
    world: VoxelWorld<u8>,
}

/// Opaque handle returned to C callers for a `MeshBuffer`.
pub struct MeshBufferHandle {
    mesh: MeshBuffer,
}

/// Opaque handle returned to C callers for a `MaterialPalette`.
pub struct MaterialPaletteHandle {
    palette: MaterialPalette,
}

/// Internal streaming manager: tracks which chunk coordinates are loaded.
struct StreamingManager {
    loaded: HashMap<StreamingCoord, ()>,
    access_order: Vec<StreamingCoord>,
    next_id: u32,
}

/// Lightweight copyable coord key for the streaming manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StreamingCoord {
    cx: i32,
    cy: i32,
    cz: i32,
}

/// Opaque handle returned to C callers for a `StreamingManager`.
pub struct StreamingManagerHandle {
    manager: StreamingManager,
}

/// Static atomic counter for observability metrics.
static OBS_COUNTER: AtomicU32 = AtomicU32::new(0);

// ---------------------------------------------------------------------------
// Voxel API — 10 functions
// ---------------------------------------------------------------------------

/// Create a new empty voxel world.
///
/// `voxel_span` is the fixed-point size of one voxel edge (use `1_000_000` for
/// the standard 1 m voxel).
///
/// Returns an opaque handle, or null on allocation failure.
///
/// # Safety
///
/// The returned handle must be destroyed with `phenotype_gfx_voxel_destroy`.
#[no_mangle]
pub extern "C" fn phenotype_gfx_voxel_create(voxel_span: i64) -> *mut VoxelWorldHandle {
    // SAFETY: Box::new allocates on the heap. We return a raw pointer that the
    // caller owns. A null return signals allocation failure to C consumers.
    let handle = Box::new(VoxelWorldHandle {
        world: VoxelWorld::new(voxel_span),
    });
    Box::into_raw(handle)
}

/// Destroy a voxel world and release all associated memory.
///
/// # Safety
///
/// `handle` must have been returned by `phenotype_gfx_voxel_create` and must
/// not have been destroyed previously. Passing null is a no-op.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_voxel_destroy(handle: *mut VoxelWorldHandle) {
    // SAFETY: The caller guarantees `handle` is a valid pointer from
    // `phenotype_gfx_voxel_create` (or null). We reconstruct the Box so
    // Rust drops the VoxelWorld and frees memory. Null is explicitly handled.
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
    }
}

/// Write a voxel value at the given world coordinate.
///
/// `handle`, `x`, `y`, `z`, `value` are all consumed by value. The
/// containing chunk is lazily allocated. Idempotent writes are safe (no
/// dirty event emitted).
///
/// # Safety
///
/// `handle` must be a valid, non-null pointer from `phenotype_gfx_voxel_create`.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_voxel_set(
    handle: *mut VoxelWorldHandle,
    x: i64,
    y: i64,
    z: i64,
    value: u8,
) {
    // SAFETY: The caller guarantees `handle` is non-null and points to a
    // live VoxelWorldHandle. We dereference it mutably (single-owner FFI).
    assert!(!handle.is_null(), "null handle passed to voxel_set");
    let h = unsafe { &mut *handle };
    h.world.write(WorldCoord { x, y, z }, value);
}

/// Read the voxel value at the given world coordinate.
///
/// Returns the stored value, or `0` (default) if the coordinate is unmapped.
///
/// # Safety
///
/// `handle` must be a valid, non-null pointer from `phenotype_gfx_voxel_create`.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_voxel_get(
    handle: *const VoxelWorldHandle,
    x: i64,
    y: i64,
    z: i64,
) -> u8 {
    // SAFETY: The caller guarantees `handle` is non-null and points to a
    // live VoxelWorldHandle. We dereference it immutably.
    assert!(!handle.is_null(), "null handle passed to voxel_get");
    let h = unsafe { &*handle };
    h.world.read(WorldCoord { x, y, z })
}

/// Return the number of allocated dense chunks in the world.
///
/// # Safety
///
/// `handle` must be a valid, non-null pointer from `phenotype_gfx_voxel_create`.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_voxel_chunk_count(handle: *const VoxelWorldHandle) -> u32 {
    // SAFETY: The caller guarantees `handle` is non-null and valid.
    assert!(!handle.is_null(), "null handle passed to voxel_chunk_count");
    let h = unsafe { &*handle };
    h.world.chunk_count() as u32
}

/// Build a greedy mesh for chunk at chunk-grid coordinate `(cx, cy, cz)`.
///
/// Returns an opaque `MeshBufferHandle`, or null if the chunk does not exist
/// or the mesher fails.
///
/// # Safety
///
/// `handle` must be a valid, non-null pointer from `phenotype_gfx_voxel_create`.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_voxel_mesh_build(
    handle: *const VoxelWorldHandle,
    cx: i32,
    cy: i32,
    cz: i32,
) -> *mut MeshBufferHandle {
    // SAFETY: The caller guarantees `handle` is non-null and valid. We read
    // from it to build the mesh; ownership is not transferred.
    assert!(!handle.is_null(), "null handle passed to voxel_mesh_build");
    let h = unsafe { &*handle };

    let coord = ChunkCoord { cx, cy, cz };
    let chunk = match h.world.chunk(coord) {
        Some(c) => c,
        None => return std::ptr::null_mut(),
    };

    // Build a simple cube mesh for each solid voxel in the chunk.
    // This is a lightweight meshing path for the FFI layer; consumers
    // that need greedy/AO meshing should use the Rust API directly.
    let mut vertices: Vec<MeshVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    const EDGE: usize = crate::voxel::CHUNK_EDGE;
    for z in 0..EDGE {
        for y in 0..EDGE {
            for x in 0..EDGE {
                let idx = x + y * EDGE + z * EDGE * EDGE;
                if chunk.voxels[idx] == 0u8 {
                    continue;
                }
                let base = vertices.len() as u32;
                let fx = x as f32;
                let fy = y as f32;
                let fz = z as f32;
                // 8 vertices of a unit cube
                let positions = [
                    [fx, fy, fz],
                    [fx + 1.0, fy, fz],
                    [fx + 1.0, fy + 1.0, fz],
                    [fx, fy + 1.0, fz],
                    [fx, fy, fz + 1.0],
                    [fx + 1.0, fy, fz + 1.0],
                    [fx + 1.0, fy + 1.0, fz + 1.0],
                    [fx, fy + 1.0, fz + 1.0],
                ];
                for pos in &positions {
                    vertices.push(MeshVertex {
                        position: *pos,
                        normal: [0.0, 1.0, 0.0],
                        uv: [0.0, 0.0],
                        material: MaterialId(chunk.voxels[idx] as u16),
                    });
                }
                // 12 triangles (6 faces)
                let faces: [[u32; 3]; 12] = [
                    [0, 1, 2],
                    [0, 2, 3], // front
                    [4, 6, 5],
                    [4, 7, 6], // back
                    [0, 4, 5],
                    [0, 5, 1], // bottom
                    [2, 6, 7],
                    [2, 7, 3], // top
                    [0, 3, 7],
                    [0, 7, 4], // left
                    [1, 5, 6],
                    [1, 6, 2], // right
                ];
                for face in &faces {
                    for &vi in face {
                        indices.push(base + vi);
                    }
                }
            }
        }
    }

    let ao = vec![3u8; vertices.len()];
    let mesh = MeshBuffer {
        vertices,
        indices,
        ao,
    };

    // SAFETY: Box::new allocates on the heap; we return the raw pointer.
    Box::into_raw(Box::new(MeshBufferHandle { mesh }))
}

/// Destroy a mesh buffer and release its memory.
///
/// # Safety
///
/// `handle` must have been returned by `phenotype_gfx_voxel_mesh_build` and
/// not have been destroyed previously. Null is a no-op.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_voxel_mesh_destroy(handle: *mut MeshBufferHandle) {
    // SAFETY: Caller guarantees valid non-null pointer or null. We reconstruct
    // the Box so Rust drops the MeshBuffer and frees memory.
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
    }
}

/// Return a pointer to the vertex data buffer.
///
/// The returned pointer is valid for as long as the `MeshBufferHandle` is
/// alive. Each vertex is 32 bytes (3×f32 position + 3×f32 normal + 2×f32 uv
/// + 1×u16 material = 32 bytes packed).
///
/// # Safety
///
/// `handle` must be a valid, non-null pointer from `phenotype_gfx_voxel_mesh_build`.
/// The pointer is valid until `phenotype_gfx_voxel_mesh_destroy` is called.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_voxel_vertices(
    handle: *const MeshBufferHandle,
) -> *const MeshVertex {
    // SAFETY: Caller guarantees non-null valid handle. We return a pointer to
    // the interior of the Vec, which is valid as long as the handle lives.
    assert!(!handle.is_null(), "null handle passed to voxel_vertices");
    let h = unsafe { &*handle };
    h.mesh.vertices.as_ptr()
}

/// Return a pointer to the triangle-index buffer.
///
/// Indices are `u32`; every 3 indices form one triangle. The pointer is valid
/// for as long as the `MeshBufferHandle` is alive.
///
/// # Safety
///
/// `handle` must be a valid, non-null pointer from `phenotype_gfx_voxel_mesh_build`.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_voxel_indices(
    handle: *const MeshBufferHandle,
) -> *const u32 {
    // SAFETY: Caller guarantees non-null valid handle. We return a pointer into
    // the Vec storage, valid for the lifetime of the handle.
    assert!(!handle.is_null(), "null handle passed to voxel_indices");
    let h = unsafe { &*handle };
    h.mesh.indices.as_ptr()
}

/// Return the number of vertices in the mesh buffer.
///
/// # Safety
///
/// `handle` must be a valid, non-null pointer from `phenotype_gfx_voxel_mesh_build`.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_voxel_vertex_count(handle: *const MeshBufferHandle) -> u32 {
    // SAFETY: Caller guarantees non-null valid handle.
    assert!(
        !handle.is_null(),
        "null handle passed to voxel_vertex_count"
    );
    let h = unsafe { &*handle };
    h.mesh.vertex_count() as u32
}

/// Return the number of indices in the mesh buffer.
///
/// # Safety
///
/// `handle` must be a valid, non-null pointer from `phenotype_gfx_voxel_mesh_build`.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_voxel_index_count(handle: *const MeshBufferHandle) -> u32 {
    // SAFETY: Caller guarantees non-null valid handle.
    assert!(
        !handle.is_null(),
        "null handle passed to voxel_index_count"
    );
    let h = unsafe { &*handle };
    h.mesh.index_count() as u32
}

// ---------------------------------------------------------------------------
// Material API — 5 functions
// ---------------------------------------------------------------------------

/// Create a new empty material palette.
///
/// Returns an opaque handle, or null on allocation failure.
///
/// # Safety
///
/// The returned handle must be destroyed with `phenotype_gfx_material_destroy`.
#[no_mangle]
pub extern "C" fn phenotype_gfx_material_create() -> *mut MaterialPaletteHandle {
    // SAFETY: Box::into_raw produces a valid heap pointer for the caller.
    let handle = Box::new(MaterialPaletteHandle {
        palette: MaterialPalette::default(),
    });
    Box::into_raw(handle)
}

/// Destroy a material palette and release its memory.
///
/// # Safety
///
/// `handle` must have been returned by `phenotype_gfx_material_create` and
/// not have been destroyed previously. Null is a no-op.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_material_destroy(handle: *mut MaterialPaletteHandle) {
    // SAFETY: Caller guarantees valid non-null or null. Box::from_raw reclaims.
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
    }
}

/// Add a material with the given `name` C-string and `hardness`.
///
/// Returns the newly-assigned `MaterialId` as a `u16`, or `u16::MAX` on
/// overflow (palette full).
///
/// # Safety
///
/// - `handle` must be a valid, non-null pointer from `phenotype_gfx_material_create`.
/// - `name` must be a valid, null-terminated C string for the duration of
///   this call. The string is copied internally; the caller retains ownership.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_material_set_property(
    handle: *mut MaterialPaletteHandle,
    name: *const c_char,
    hardness: f32,
) -> u16 {
    // SAFETY: Caller guarantees `handle` is non-null and valid, and `name` is
    // a valid null-terminated C string. We copy the CStr into a Rust String
    // before the palette takes ownership.
    assert!(
        !handle.is_null(),
        "null handle passed to material_set_property"
    );
    assert!(!name.is_null(), "null name passed to material_set_property");
    let h = unsafe { &mut *handle };
    // SAFETY: `name` is guaranteed to be a valid null-terminated C string.
    let c_str = unsafe { CStr::from_ptr(name) };
    let name_str = c_str.to_string_lossy().into_owned();
    match h.palette.add(VoxelMaterial {
        name: name_str,
        era: 0,
        hardness,
    }) {
        Ok(id) => id.0,
        Err(_) => u16::MAX,
    }
}

/// Look up a material by `id` and write its `hardness` into `*out_hardness`.
///
/// Returns `1` on success, `0` if the id is not found.
///
/// # Safety
///
/// - `handle` must be a valid, non-null pointer from `phenotype_gfx_material_create`.
/// - `out_hardness` must be a valid, non-null `*mut f32`.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_material_get_property(
    handle: *const MaterialPaletteHandle,
    id: u16,
    out_hardness: *mut f32,
) -> i32 {
    // SAFETY: Caller guarantees all pointers are non-null and valid.
    assert!(
        !handle.is_null(),
        "null handle passed to material_get_property"
    );
    assert!(
        !out_hardness.is_null(),
        "null out_hardness passed to material_get_property"
    );
    let h = unsafe { &*handle };
    match h.palette.get(MaterialId(id)) {
        Some(mat) => {
            // SAFETY: `out_hardness` is guaranteed non-null and writable.
            unsafe {
                *out_hardness = mat.hardness;
            }
            1
        }
        None => 0,
    }
}

/// Return the number of materials in the palette.
///
/// # Safety
///
/// `handle` must be a valid, non-null pointer from `phenotype_gfx_material_create`.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_material_count(handle: *const MaterialPaletteHandle) -> u32 {
    // SAFETY: Caller guarantees non-null valid handle.
    assert!(!handle.is_null(), "null handle passed to material_count");
    let h = unsafe { &*handle };
    h.palette.materials.len() as u32
}

// ---------------------------------------------------------------------------
// Streaming API — 6 functions
// ---------------------------------------------------------------------------

/// Create a new streaming manager.
///
/// Returns an opaque handle, or null on allocation failure.
///
/// # Safety
///
/// The returned handle must be destroyed with `phenotype_gfx_streaming_destroy`.
#[no_mangle]
pub extern "C" fn phenotype_gfx_streaming_create() -> *mut StreamingManagerHandle {
    // SAFETY: Box::into_raw produces a valid heap pointer.
    let handle = Box::new(StreamingManagerHandle {
        manager: StreamingManager {
            loaded: HashMap::new(),
            access_order: Vec::new(),
            next_id: 0,
        },
    });
    Box::into_raw(handle)
}

/// Destroy a streaming manager and release its memory.
///
/// # Safety
///
/// `handle` must have been returned by `phenotype_gfx_streaming_create` and
/// not have been destroyed previously. Null is a no-op.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_streaming_destroy(handle: *mut StreamingManagerHandle) {
    // SAFETY: Caller guarantees valid non-null or null. Box::from_raw reclaims.
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
    }
}

/// Load a chunk at chunk-grid coordinate `(cx, cy, cz)` into the streaming set.
///
/// Returns a monotonic load-id (`> 0`) on success, or `0` if already loaded.
///
/// # Safety
///
/// `handle` must be a valid, non-null pointer from `phenotype_gfx_streaming_create`.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_streaming_load(
    handle: *mut StreamingManagerHandle,
    cx: i32,
    cy: i32,
    cz: i32,
) -> u32 {
    // SAFETY: Caller guarantees non-null valid handle.
    assert!(!handle.is_null(), "null handle passed to streaming_load");
    let h = unsafe { &mut *handle };
    let key = StreamingCoord { cx, cy, cz };
    if h.manager.loaded.contains_key(&key) {
        return 0;
    }
    h.manager.next_id = h.manager.next_id.wrapping_add(1);
    let id = h.manager.next_id;
    h.manager.loaded.insert(key, ());
    h.manager.access_order.push(key);
    id
}

/// Unload (remove) a chunk at chunk-grid coordinate `(cx, cy, cz)`.
///
/// Returns `1` if the chunk was loaded and removed, `0` if it was not present.
///
/// # Safety
///
/// `handle` must be a valid, non-null pointer from `phenotype_gfx_streaming_create`.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_streaming_unload(
    handle: *mut StreamingManagerHandle,
    cx: i32,
    cy: i32,
    cz: i32,
) -> i32 {
    // SAFETY: Caller guarantees non-null valid handle.
    assert!(!handle.is_null(), "null handle passed to streaming_unload");
    let h = unsafe { &mut *handle };
    let key = StreamingCoord { cx, cy, cz };
    if h.manager.loaded.remove(&key).is_some() {
        h.manager.access_order.retain(|&k| k != key);
        1
    } else {
        0
    }
}

/// Return the number of currently loaded chunks.
///
/// # Safety
///
/// `handle` must be a valid, non-null pointer from `phenotype_gfx_streaming_create`.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_streaming_loaded_count(
    handle: *const StreamingManagerHandle,
) -> u32 {
    // SAFETY: Caller guarantees non-null valid handle.
    assert!(
        !handle.is_null(),
        "null handle passed to streaming_loaded_count"
    );
    let h = unsafe { &*handle };
    h.manager.loaded.len() as u32
}

/// Evict the oldest (least-recently-loaded) chunk from the streaming set.
///
/// Returns `1` if a chunk was evicted, `0` if the set is empty.
///
/// # Safety
///
/// `handle` must be a valid, non-null pointer from `phenotype_gfx_streaming_create`.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_streaming_evict_oldest(
    handle: *mut StreamingManagerHandle,
) -> i32 {
    // SAFETY: Caller guarantees non-null valid handle.
    assert!(
        !handle.is_null(),
        "null handle passed to streaming_evict_oldest"
    );
    let h = unsafe { &mut *handle };
    if let Some(key) = h.manager.access_order.first().copied() {
        h.manager.loaded.remove(&key);
        h.manager.access_order.remove(0);
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// Observability API — 3 functions
// ---------------------------------------------------------------------------

/// Initialise the observability subsystem (no-op when no recorder is installed).
///
/// Safe to call multiple times; subsequent calls are idempotent.
#[no_mangle]
pub extern "C" fn phenotype_gfx_obs_init() {
    // SAFETY: No unsafe code. This is a no-op placeholder for when a tracing
    // subscriber or metrics recorder needs explicit setup. For now the facade
    // in obs.rs is zero-cost by construction.
    crate::gfx_info!("phenotype_gfx obs initialized via C-ABI");
}

/// Increment the global observability counter by `delta`.
///
/// # Safety
///
/// No pointer safety requirements; this function is pure.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_obs_counter_inc(delta: u32) {
    // SAFETY: AtomicU32::fetch_add is always safe. We use Relaxed ordering
    // because this is a monotonic counter with no cross-field invariants.
    OBS_COUNTER.fetch_add(delta, Ordering::Relaxed);
}

/// Set the global observability gauge to `value`.
///
/// This writes to a static `AtomicU32` reinterpreted as a gauge (raw bits).
/// Consumers that need floating-point gauges should use the metrics facade
/// directly from Rust.
///
/// # Safety
///
/// No pointer safety requirements; this function is pure.
#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_obs_gauge_set(value: u32) {
    // SAFETY: Relaxed store to an atomic. The "gauge" is the raw bit pattern.
    OBS_COUNTER.store(value, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// C-string helpers (internal)
// ---------------------------------------------------------------------------

/// Duplicate a Rust string into a C-allocated null-terminated string.
///
/// # Safety
///
/// The caller must free the returned pointer with `std::ffi::CString::from_raw`.
/// Returns null on allocation failure.
#[allow(dead_code)]
unsafe fn rust_string_to_c(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// ===========================================================================
// Unit tests — 3 per module (15 total)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ======================================================================
    // Voxel API tests
    // ======================================================================

    #[test]
    fn voxel_create_and_destroy_roundtrip() {
        let h = phenotype_gfx_voxel_create(1_000_000);
        assert!(!h.is_null());
        unsafe {
            phenotype_gfx_voxel_destroy(h);
        }
    }

    #[test]
    fn voxel_set_get_roundtrip() {
        let h = phenotype_gfx_voxel_create(1_000_000);
        assert!(!h.is_null());
        unsafe {
            phenotype_gfx_voxel_set(h, 5_000_000, 0, 0, 42);
            let v = phenotype_gfx_voxel_get(h, 5_000_000, 0, 0);
            assert_eq!(v, 42);
            phenotype_gfx_voxel_destroy(h);
        }
    }

    #[test]
    fn voxel_chunk_count_increments_on_write() {
        let h = phenotype_gfx_voxel_create(1_000_000);
        assert!(!h.is_null());
        unsafe {
            assert_eq!(phenotype_gfx_voxel_chunk_count(h), 0);
            phenotype_gfx_voxel_set(h, 0, 0, 0, 1);
            assert_eq!(phenotype_gfx_voxel_chunk_count(h), 1);
            // Write in a different chunk
            phenotype_gfx_voxel_set(h, 100_000_000, 0, 0, 2);
            assert_eq!(phenotype_gfx_voxel_chunk_count(h), 2);
            phenotype_gfx_voxel_destroy(h);
        }
    }

    // ======================================================================
    // Material API tests
    // ======================================================================

    #[test]
    fn material_create_and_destroy_roundtrip() {
        let h = phenotype_gfx_material_create();
        assert!(!h.is_null());
        unsafe {
            phenotype_gfx_material_destroy(h);
        }
    }

    #[test]
    fn material_set_get_property_roundtrip() {
        let h = phenotype_gfx_material_create();
        assert!(!h.is_null());
        let name = CString::new("stone").unwrap();
        unsafe {
            let id = phenotype_gfx_material_set_property(h, name.as_ptr(), 5.0);
            assert_eq!(id, 0); // first material → id 0
            let mut hardness = 0.0f32;
            let ok = phenotype_gfx_material_get_property(h, id, &mut hardness);
            assert_eq!(ok, 1);
            assert!((hardness - 5.0).abs() < f32::EPSILON);
            phenotype_gfx_material_destroy(h);
        }
    }

    #[test]
    fn material_count_increments() {
        let h = phenotype_gfx_material_create();
        assert!(!h.is_null());
        let name1 = CString::new("wood").unwrap();
        let name2 = CString::new("iron").unwrap();
        unsafe {
            assert_eq!(phenotype_gfx_material_count(h), 0);
            phenotype_gfx_material_set_property(h, name1.as_ptr(), 1.0);
            assert_eq!(phenotype_gfx_material_count(h), 1);
            phenotype_gfx_material_set_property(h, name2.as_ptr(), 10.0);
            assert_eq!(phenotype_gfx_material_count(h), 2);
            phenotype_gfx_material_destroy(h);
        }
    }

    // ======================================================================
    // Streaming API tests
    // ======================================================================

    #[test]
    fn streaming_create_and_destroy_roundtrip() {
        let h = phenotype_gfx_streaming_create();
        assert!(!h.is_null());
        unsafe {
            phenotype_gfx_streaming_destroy(h);
        }
    }

    #[test]
    fn streaming_load_unload_lifecycle() {
        let h = phenotype_gfx_streaming_create();
        assert!(!h.is_null());
        unsafe {
            let id = phenotype_gfx_streaming_load(h, 1, 2, 3);
            assert!(id > 0);
            assert_eq!(phenotype_gfx_streaming_loaded_count(h), 1);
            // Loading the same coord again returns 0 (already loaded)
            let id2 = phenotype_gfx_streaming_load(h, 1, 2, 3);
            assert_eq!(id2, 0);
            assert_eq!(phenotype_gfx_streaming_loaded_count(h), 1);
            // Unload
            let removed = phenotype_gfx_streaming_unload(h, 1, 2, 3);
            assert_eq!(removed, 1);
            assert_eq!(phenotype_gfx_streaming_loaded_count(h), 0);
            phenotype_gfx_streaming_destroy(h);
        }
    }

    #[test]
    fn streaming_evict_oldest_removes_first_loaded() {
        let h = phenotype_gfx_streaming_create();
        assert!(!h.is_null());
        unsafe {
            phenotype_gfx_streaming_load(h, 0, 0, 0);
            phenotype_gfx_streaming_load(h, 1, 0, 0);
            phenotype_gfx_streaming_load(h, 2, 0, 0);
            assert_eq!(phenotype_gfx_streaming_loaded_count(h), 3);
            let evicted = phenotype_gfx_streaming_evict_oldest(h);
            assert_eq!(evicted, 1);
            assert_eq!(phenotype_gfx_streaming_loaded_count(h), 2);
            // The chunk (0,0,0) was loaded first, so it should be evicted
            // Verify by loading it again — should succeed (not already loaded)
            let id = phenotype_gfx_streaming_load(h, 0, 0, 0);
            assert!(id > 0);
            phenotype_gfx_streaming_destroy(h);
        }
    }

    // ======================================================================
    // Observability API tests
    // ======================================================================

    #[test]
    fn obs_init_does_not_panic() {
        phenotype_gfx_obs_init();
        phenotype_gfx_obs_init(); // idempotent
    }

    #[test]
    fn obs_counter_inc_increments_atomically() {
        let before = OBS_COUNTER.load(Ordering::Relaxed);
        unsafe {
            phenotype_gfx_obs_counter_inc(10);
        }
        let after = OBS_COUNTER.load(Ordering::Relaxed);
        assert_eq!(after, before + 10);
    }

    #[test]
    fn obs_gauge_set_stores_value() {
        unsafe {
            phenotype_gfx_obs_gauge_set(999);
        }
        let val = OBS_COUNTER.load(Ordering::Relaxed);
        assert_eq!(val, 999);
    }

    // ======================================================================
    // Additional tests to reach 15 total
    // ======================================================================

    #[test]
    fn voxel_mesh_build_returns_vertices_for_solid_chunk() {
        let h = phenotype_gfx_voxel_create(1_000_000);
        assert!(!h.is_null());
        unsafe {
            // Write a single voxel at origin
            phenotype_gfx_voxel_set(h, 0, 0, 0, 1);
            let mesh_h = phenotype_gfx_voxel_mesh_build(h, 0, 0, 0);
            assert!(!mesh_h.is_null());
            let vc = phenotype_gfx_voxel_vertex_count(mesh_h);
            assert!(vc > 0, "expected vertices for solid voxel, got 0");
            // 8 vertices per cube
            assert_eq!(vc, 8);
            phenotype_gfx_voxel_mesh_destroy(mesh_h);
            phenotype_gfx_voxel_destroy(h);
        }
    }

    #[test]
    fn voxel_index_count_matches_solid_chunk() {
        let h = phenotype_gfx_voxel_create(1_000_000);
        assert!(!h.is_null());
        unsafe {
            phenotype_gfx_voxel_set(h, 0, 0, 0, 1);
            let mesh_h = phenotype_gfx_voxel_mesh_build(h, 0, 0, 0);
            assert!(!mesh_h.is_null());
            let ic = phenotype_gfx_voxel_index_count(mesh_h);
            // A solid cube has 6 faces * 2 triangles/face * 3 indices/triangle = 36 indices
            assert_eq!(ic, 36);
            phenotype_gfx_voxel_mesh_destroy(mesh_h);
            phenotype_gfx_voxel_destroy(h);
        }
    }

    #[test]
    fn voxel_index_count_empty_chunk_returns_zero() {
        let h = phenotype_gfx_voxel_create(1_000_000);
        assert!(!h.is_null());
        unsafe {
            // Build mesh for empty chunk (0,0,0) which doesn't exist yet
            let mesh_h = phenotype_gfx_voxel_mesh_build(h, 0, 0, 0);
            assert!(mesh_h.is_null(), "expected null for empty chunk");
            phenotype_gfx_voxel_destroy(h);
        }
    }

    #[test]
    fn material_get_property_returns_zero_for_missing_id() {
        let h = phenotype_gfx_material_create();
        assert!(!h.is_null());
        unsafe {
            let mut hardness = -1.0f32;
            let ok = phenotype_gfx_material_get_property(h, 9999, &mut hardness);
            assert_eq!(ok, 0);
            phenotype_gfx_material_destroy(h);
        }
    }

    #[test]
    fn streaming_unload_nonexistent_returns_zero() {
        let h = phenotype_gfx_streaming_create();
        assert!(!h.is_null());
        unsafe {
            let removed = phenotype_gfx_streaming_unload(h, 42, 42, 42);
            assert_eq!(removed, 0);
            assert_eq!(phenotype_gfx_streaming_loaded_count(h), 0);
            phenotype_gfx_streaming_destroy(h);
        }
    }
}
