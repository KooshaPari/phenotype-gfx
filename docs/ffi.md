# Foreign Function Interface

The C-ABI layer (`bindings/c_api.rs`) is the only seam between the Rust core
and the Unity C# edge. Per ADR-004, this is a **thin wrapper** — no logic
lives here; all algorithms stay in the single Rust core.

> **Source of truth:** `bindings/c_api.rs:1`
>
> "C-ABI wrapper for phenotype-gfx. cbindgen will generate C headers from this
> module. C# P/Invoke → calls this → calls Rust core algorithms. No logic here;
> thin wrapper only."

## Architecture

```
phenotype-gfx (Rust core)
    │
    │  bindings/c_api.rs        ← #[no_mangle] extern "C" functions
    ▼
cdylib: libphenotype_gfx.so / .dylib / .dll
    │
    │  cbindgen                 ← generates C / C++ headers
    ▼
include/phenotype_gfx.h        ← C-compatible header
include/phenotype_gfx.hpp      ← C++ convenience header
    │
    │  Unity P/Invoke           ← C# [DllImport("phenotype_gfx", ...)]
    ▼
unity/PhenotypeGfx.cs          ← typed C# wrapper
```

## C-ABI exports

The crate exposes **24 `#[no_mangle]` C-ABI functions** organised into four
modules. All functions are unsafe where they accept raw pointers; safe
functions only ever return handles or produce side effects on the global
observability counter.

### Voxel API — 10 functions

| C symbol                                       | Returns   | Purpose                                       |
| ---------------------------------------------- | --------- | --------------------------------------------- |
| `phenotype_gfx_voxel_create(voxel_span)`       | `*mut VoxelWorldHandle` | Create an empty voxel world           |
| `phenotype_gfx_voxel_destroy(handle)`          | `void`    | Destroy a voxel world                          |
| `phenotype_gfx_voxel_set(handle, x, y, z, v)`  | `void`    | Write a voxel value at world coord            |
| `phenotype_gfx_voxel_get(handle, x, y, z)`     | `u8`      | Read a voxel value (0 if unmapped)            |
| `phenotype_gfx_voxel_chunk_count(handle)`      | `u32`     | Number of allocated dense chunks              |
| `phenotype_gfx_voxel_mesh_build(h, cx, cy, cz)`| `*mut MeshBufferHandle` | Build greedy mesh for chunk grid coord |
| `phenotype_gfx_voxel_mesh_destroy(handle)`     | `void`    | Destroy a mesh buffer                         |
| `phenotype_gfx_voxel_vertices(handle)`         | `*const MeshVertex` | Pointer to vertex data (32 B each) |
| `phenotype_gfx_voxel_indices(handle)`          | `*const u32` | Pointer to triangle-index buffer           |
| `phenotype_gfx_voxel_vertex_count(handle)`     | `u32`     | Number of vertices in mesh buffer             |
| `phenotype_gfx_voxel_index_count(handle)`      | `u32`     | Number of indices in mesh buffer              |

> The mesh build path is intentionally a **lightweight cubic mesher** for the
> FFI layer; consumers that need greedy/AO meshing should use the Rust API
> directly via `phenotype_gfx::voxel::GreedyMesher`.

### Material API — 5 functions

| C symbol                                       | Returns   | Purpose                              |
| ---------------------------------------------- | --------- | ------------------------------------ |
| `phenotype_gfx_material_create()`              | `*mut MaterialPaletteHandle` | Create empty material palette |
| `phenotype_gfx_material_destroy(handle)`       | `void`    | Destroy a material palette           |
| `phenotype_gfx_material_set_property(h, name, hardness)` | `u16` | Add a material, returns id |
| `phenotype_gfx_material_get_property(h, id, out_hardness)` | `i32` | Look up material; 1 = ok, 0 = missing |
| `phenotype_gfx_material_count(handle)`         | `u32`     | Number of materials in palette       |

### Streaming API — 6 functions

| C symbol                                       | Returns   | Purpose                              |
| ---------------------------------------------- | --------- | ------------------------------------ |
| `phenotype_gfx_streaming_create()`             | `*mut StreamingManagerHandle` | Create a streaming manager |
| `phenotype_gfx_streaming_destroy(handle)`      | `void`    | Destroy a streaming manager          |
| `phenotype_gfx_streaming_load(h, cx, cy, cz)` | `u32`     | Load chunk; > 0 load-id, 0 = already loaded |
| `phenotype_gfx_streaming_unload(h, cx, cy, cz)`| `i32`     | Unload chunk; 1 = removed, 0 = absent |
| `phenotype_gfx_streaming_loaded_count(handle)` | `u32`     | Number of currently loaded chunks    |
| `phenotype_gfx_streaming_evict_oldest(handle)` | `i32`     | Evict LRU chunk; 1 = evicted, 0 = empty |

### Observability API — 3 functions

| C symbol                                       | Returns   | Purpose                              |
| ---------------------------------------------- | --------- | ------------------------------------ |
| `phenotype_gfx_obs_init()`                     | `void`    | Initialise observability (idempotent) |
| `phenotype_gfx_obs_counter_inc(delta)`         | `void`    | Atomic increment of global counter   |
| `phenotype_gfx_obs_gauge_set(value)`           | `void`    | Atomic gauge set                     |

## Handle model

The four handle types (`VoxelWorldHandle`, `MeshBufferHandle`,
`MaterialPaletteHandle`, `StreamingManagerHandle`) wrap a Rust value in a
heap-allocated `Box<T>` and return the raw pointer to C consumers.

```rust,ignore
#[no_mangle]
pub extern "C" fn phenotype_gfx_voxel_create(voxel_span: i64) -> *mut VoxelWorldHandle {
    let handle = Box::new(VoxelWorldHandle {
        world: VoxelWorld::new(voxel_span),
    });
    Box::into_raw(handle)
}

#[no_mangle]
pub unsafe extern "C" fn phenotype_gfx_voxel_destroy(handle: *mut VoxelWorldHandle) {
    if !handle.is_null() {
        unsafe { let _ = Box::from_raw(handle); }
    }
}
```

### Ownership rules

- `*mut T` parameters are **borrowed for the duration of the call** for
  read-only operations (`get`, `count`, `vertex_count`).
- `*mut T` parameters are **mutably borrowed** for write operations (`set`,
  `unload`, `evict_oldest`).
- Functions ending in `_create` return a freshly boxed handle that **the
  caller owns** and must release with the matching `_destroy`.
- `_destroy` is null-safe (null is a no-op).
- **Do not** call `_destroy` on a handle that has already been destroyed —
  this is a double-free.

### `MeshVertex` ABI layout

`MeshVertex` is `#[repr(C)]` and packed. Each vertex is **32 bytes**:

| Offset | Type     | Field      |
| ------ | -------- | ---------- |
| 0      | `[f32;3]`| `position` |
| 12     | `[f32;3]`| `normal`   |
| 24     | `[f32;2]`| `uv`       |
| 32     | `u16`    | `material` |

The `cbindgen.toml` configuration pins this layout, and the Unity C#
`MeshVertex` struct mirrors it with `[StructLayout(LayoutKind.Sequential,
Pack = 1)]`.

## Safety contract

Every unsafe function documents its `# Safety` invariants:

- All `*mut` / `*const` parameters must be **valid, non-null** pointers
  obtained from the matching `_create` function.
- C strings (`*const c_char`) must be **null-terminated** and remain valid
  for the duration of the call.
- Out-pointers (`*mut f32`) must be **non-null and writable**.
- Callers must **respect the ownership model** documented per function.
- Callers must **synchronise concurrent access** themselves — the FFI layer
  holds a single mutable reference at a time but does not provide internal
  locking.

## cbindgen headers

`cbindgen.toml` drives the cbindgen tool to emit two headers:

- **`include/phenotype_gfx.h`** — C-compatible header. 12 KB. Used by
  any C / C++ consumer.
- **`include/phenotype_gfx.hpp`** — C++ convenience header with
  `extern "C"` guards and RAII-style handle helpers.

```toml
# cbindgen.toml (excerpt)
[parse]
parse_deps = true
include = ["phenotype_gfx"]

[export]
include = ["VoxelWorldHandle", "MeshBufferHandle", ...]
trailer = "// On the safe side: ..."
```

### Building the cdylib

```bash
cargo build --release
# → target/release/libphenotype_gfx.so   (Linux)
# → target/release/libphenotype_gfx.dylib (macOS)
# → target/release/phenotype_gfx.dll     (Windows)
```

The `[lib]` section in `Cargo.toml` declares `crate-type = ["cdylib", "rlib"]`
so the same crate can be consumed as both a dynamic library (Unity) and a
Rust library (CLI / Bevy / wasm-bindgen).

## Unity C# wrapper

`unity/PhenotypeGfx.cs` is a **~750-line** P/Invoke wrapper that turns the 24
raw C-ABI exports into a typed, IDisposable C# surface. It ships as a
single drop-in file you can place in any Unity project's
`Assets/Plugins/PhenotypeGfx/` folder.

### Raw bindings

```csharp
public static class Native
{
    private const string Lib = "phenotype_gfx";

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr phenotype_gfx_voxel_create(long voxelSpan);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void phenotype_gfx_voxel_destroy(IntPtr handle);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void phenotype_gfx_voxel_set(
        IntPtr handle, long x, long y, long z, byte value);

    // ... 20 more declarations ...
}
```

### Blittable types

`MeshVertex` mirrors the Rust layout with
`[StructLayout(LayoutKind.Sequential, Pack = 1)]`:

```csharp
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct MeshVertex
{
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 3)]
    public float[] Position;
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 3)]
    public float[] Normal;
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 2)]
    public float[] Uv;
    public ushort Material;

    public Vector3 UnityPosition => new Vector3(Position[0], Position[1], Position[2]);
    public Vector3 UnityNormal   => new Vector3(Normal[0],   Normal[1],   Normal[2]);
    public Vector2 UnityUv       => new Vector2(Uv[0],       Uv[1]);
}
```

### Constants

```csharp
public static class Constants
{
    public const int   SchemaVersion             = 1;
    public const float DefaultVoxelScaleMultiplier = 8.0f;
    public const int   ChunkEdge                = 16;
    public const int   ChunkVoxels              = 4096;
    public const long  FixedScale               = 1_000_000;
    public const byte  AlphaThreshold           = 16;
    public const int   DefaultDepth             = 8;
}
```

### Typed handle wrappers

`PhenotypeGfx.cs` provides four `IDisposable` wrappers that enforce
create/destroy lifetime semantics, preventing double-free and use-after-free
in C#:

```csharp
public sealed class VoxelWorld : IDisposable
{
    private IntPtr _handle;
    public IntPtr Handle => _handle;

    public VoxelWorld(long voxelSpan)
    {
        _handle = Native.phenotype_gfx_voxel_create(voxelSpan);
        if (_handle == IntPtr.Zero) throw new InvalidOperationException("...");
    }

    public void Set(long x, long y, long z, byte value)
        => Native.phenotype_gfx_voxel_set(_handle, x, y, z, value);

    public byte Get(long x, long y, long z)
        => Native.phenotype_gfx_voxel_get(_handle, x, y, z);

    public uint ChunkCount => Native.phenotype_gfx_voxel_chunk_count(_handle);

    public MeshBuffer BuildMesh(int cx, int cy, int cz)
        => new MeshBuffer(Native.phenotype_gfx_voxel_mesh_build(_handle, cx, cy, cz));

    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            Native.phenotype_gfx_voxel_destroy(_handle);
            _handle = IntPtr.Zero;
        }
    }
}
```

`MaterialPalette`, `MeshBuffer`, and `StreamingManager` follow the same
pattern. See [`unity.md`](./unity.md) for full lifecycle details.

## Build configuration

The C# wrapper targets the native library `phenotype_gfx`. On Windows, drop
`phenotype_gfx.dll` next to the Unity player's executable. On Linux/macOS,
drop `libphenotype_gfx.so` / `.dylib` respectively. Unity's plugin importer
should mark the library for the appropriate target platforms (Standalone,
Editor, etc.).
