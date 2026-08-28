# phenotype-gfx — Unity P/Invoke Wrapper

C# bindings for the `phenotype-gfx` native library. Wraps all 24 FFI
exports with safe `IDisposable` handle classes and a `MonoBehaviour` example.

## Quick start

1. **Build the native library** (from the repo root):

   ```bash
   # Windows (MSVC)
   cargo build --release --lib
   # → target/release/phenotype_gfx.dll

   # macOS
   cargo build --release --lib
   # → target/release/libphenotype_gfx.dylib

   # Linux
   cargo build --release --lib
   # → target/release/libphenotype_gfx.so
   ```

2. **Copy into your Unity project:**

   ```
   Assets/
   └── Plugins/
       ├── phenotype_gfx.dll          ← (or .so / .dylib)
       └── PhenotypeGfx/
           ├── PhenotypeGfx.cs
           └── PhenotypeGfx.Example.cs
   ```

   For **macOS**, the `.dylib` must sit alongside `Assets/` or inside
   `Assets/Plugins/x86_64/`. For **Linux**, place the `.so` in
   `Assets/Plugins/x86_64/`.

3. **Try the example:** Attach `PhenotypeGfxExample` to any GameObject
   and press Play.

## Architecture

```
Unity C#  ──P/Invoke──▶  phenotype_gfx.dll (Rust)
              │
              ├── VoxelWorld       (create / destroy / set / get / mesh)
              ├── MeshBuffer       (vertices / indices / FillUnityMesh)
              ├── MaterialPalette  (add / lookup / count)
              ├── StreamingManager (load / unload / evict)
              └── Observability    (init / counter / gauge)
```

Every handle wrapper implements `IDisposable` with a finalizer safety net.
Always prefer `using` blocks or explicit `Dispose()` over relying on
the GC finalizer.

## API overview

| Class | Purpose |
|---|---|
| `VoxelWorld` | Create, read, write, and mesh a voxel octree. |
| `MeshBuffer` | Read vertex/index data; populate a Unity `Mesh`. |
| `MaterialPalette` | Registry of named materials with hardness values. |
| `StreamingManager` | LRU chunk loading/unloading for streaming worlds. |
| `Observability` | Global counters and gauges (stateless helpers). |
| `Constants` | Schema version, chunk edge, fixed-scale, etc. |
| `MeshVertex` | Blittable struct matching the Rust vertex layout. |
| `Native` | Raw `[DllImport]` declarations (advanced use only). |

## Requirements

- Unity 2021.3 LTS or newer (uses `Marshal.PtrToStructure<T>`).
- .NET Standard 2.1 / .NET Framework 4.x.
- The native library compiled for the target platform.

## Platform notes

- **Windows x64:** Place `phenotype_gfx.dll` in `Assets/Plugins/`.
- **macOS:** Place `libphenotype_gfx.dylib` in `Assets/Plugins/` and
  set the import file path in the `.meta` if Unity cannot find it.
- **Linux:** Place `libphenotype_gfx.so` in `Assets/Plugins/x86_64/`.
- **Android/iOS:** Cross-compile the Rust crate with the appropriate
  target triple and place the `.so`/`.dylib` in the platform plugin folder.
