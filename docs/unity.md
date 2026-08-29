# Unity integration

This chapter describes how to embed `phenotype-gfx` in a Unity 2022+ project
on Windows, macOS, or Linux. The integration is a thin P/Invoke surface
plus a typed C# wrapper — there is no Unity-managed renderer; rendering
goes through `Graphics.RenderMeshIndirect` or a `Mesh` upload.

> **Source of truth:** `unity/PhenotypeGfx.cs:1`, `unity/PhenotypeGfx.Example.cs:1`,
> `tests/unity/PhenotypeGfx.Tests/`

## Layout

```
phenotype-gfx/
├── include/phenotype_gfx.h        ← cbindgen C header
├── target/release/phenotype_gfx.dll
├── target/release/libphenotype_gfx.so
├── target/release/libphenotype_gfx.dylib
├── unity/PhenotypeGfx.cs          ← drop-in wrapper
├── unity/PhenotypeGfx.Example.cs  ← reference integration
└── tests/unity/PhenotypeGfx.Tests/ ← NUnit test suite
```

After `cargo build --release`, copy the native binary for the host OS into
your Unity project's `Assets/Plugins/PhenotypeGfx/`:

| Unity Platform | Plugin file              | Source location                              |
| -------------- | ------------------------ | -------------------------------------------- |
| Standalone Windows | `phenotype_gfx.dll` | `target/release/phenotype_gfx.dll`           |
| Standalone macOS   | `libphenotype_gfx.dylib` | `target/release/libphenotype_gfx.dylib`   |
| Standalone Linux   | `libphenotype_gfx.so` | `target/release/libphenotype_gfx.so`         |
| Linux Server       | `libphenotype_gfx.so` | `target/release/libphenotype_gfx.so`         |

Unity's plugin importer must be configured for the right CPU/architecture
(x86_64, ARM64) and **IL2CPP / Mono** settings. We target `x86_64-linux-gnu`
glibc 2.31+ for headless server builds.

## `PhenotypeGfx.cs` lifecycle

`unity/PhenotypeGfx.cs` is the only Unity-facing file. It provides:

1. **`PhenotypeGfx.Native`** — raw `[DllImport]` declarations for all 24 C-ABI
   functions.
2. **`PhenotypeGfx.MeshVertex`** — `[StructLayout(Pack = 1)]` blittable struct
   mirroring the Rust layout (`src/voxel/mod.rs`).
3. **`PhenotypeGfx.Constants`** — schema-version, chunk size, scale values.
4. **`PhenotypeGfx.VoxelWorld`**, **`MaterialPalette`**, **`MeshBuffer`**,
   **`StreamingManager`** — four `IDisposable` typed wrappers.
5. **`PhenotypeGfx.Observation`** — observability counter helpers.

### `IDisposable` pattern

Every wrapper holds a single `IntPtr` and releases it on `Dispose()`. The
finalizer is omitted because the `Box<T>` reclaim is deterministic and
double-free would crash Unity. Consumers must `Dispose()` in the same
scope as they would call `_destroy` from C.

```csharp
using var world = new PhenotypeGfx.VoxelWorld(voxelSpan: 1_000_000);
world.Set(0, 0, 0, 1);
byte v = world.Get(0, 0, 0);
Debug.Assert(v == 1);

using var mesh = world.BuildMesh(cx: 0, cy: 0, cz: 0);
uint vc = mesh.VertexCount;
uint ic = mesh.IndexCount;
```

### `MeshBuffer` → Unity `Mesh`

`MeshBuffer` exposes `Vertices` (managed `MeshVertex[]`) and `Indices`
(`uint[]`). To push them into a Unity `Mesh`:

```csharp
var unityMesh = new Mesh
{
    indexFormat = UnityEngine.Rendering.IndexFormat.UInt32,
    vertices = Array.ConvertAll(mesh.Vertices, v => v.UnityPosition),
    normals  = Array.ConvertAll(mesh.Vertices, v => v.UnityNormal),
    uv       = Array.ConvertAll(mesh.Vertices, v => v.UnityUv),
};
unityMesh.SetTriangles(mesh.Indices, 0, calculateBounds: true);
```

`PhenotypeGfx.Example.cs` shows a fuller pattern using
`Mesh.MeshDataArray` and `MeshUpdateFlags.DontRecalculateBounds | DontValidateIndices`
for the upload.

### Materials and palettes

`MaterialPalette` is keyed by `ushort` id. Add materials by name and
hardness; the C# wrapper stores the id internally for fast lookup:

```csharp
using var palette = new PhenotypeGfx.MaterialPalette();
ushort stoneId = palette.Add("stone", hardness: 5.0f);
ushort woodId  = palette.Add("wood",  hardness: 1.0f);

Debug.Assert(palette.Count == 2);

float h;
Debug.Assert(palette.TryGetProperty(stoneId, out h));
Debug.Assert(Mathf.Approximately(h, 5.0f));
```

### Streaming

`StreamingManager` wraps the LRU load/unload set:

```csharp
using var sm = new PhenotypeGfx.StreamingManager();
uint id1 = sm.Load(cx:  0, cy: 0, cz:  0);  // monotonic > 0
uint id2 = sm.Load(cx:  1, cy: 0, cz:  0);
uint id3 = sm.Load(cx:  2, cy: 0, cz:  0);

Debug.Assert(sm.LoadedCount == 3);
Debug.Assert(sm.EvictOldest() == 1);        // evicts id1 (cx=0)
Debug.Assert(sm.LoadedCount == 2);

Debug.Assert(sm.Unload(cx: 1, cy: 0, cz: 0) == 1);
Debug.Assert(sm.LoadedCount == 1);
```

The C# wrapper does not auto-evict; eviction is the host's responsibility.
A Unity coroutine can call `EvictOldest` once per frame, gated by
`LoadedCount > maxResident`.

## NUnit test suite

`tests/unity/PhenotypeGfx.Tests/` contains an Edit-mode NUnit project that
exercises the wrapper end-to-end. The project is a `.csproj` that uses
NUnit 3.13+ and `nunit3-console` for headless CI runs.

```
tests/unity/PhenotypeGfx.Tests/
├── PhenotypeGfx.Tests.csproj
├── Fixtures/
│   ├── NativeLibraryFixture.cs   ← resolves libphenotype_gfx.{so,dylib,dll}
│   └── TempWorldFixture.cs       ← per-test VoxelWorld lifecycle
├── VoxelWorldTests.cs            ← 8 tests
├── MaterialPaletteTests.cs       ← 6 tests
├── StreamingTests.cs             ← 5 tests
├── MeshBuildTests.cs             ← 4 tests
└── ObservabilityTests.cs         ← 4 tests
```

### Native library resolution

`NativeLibraryFixture` searches in this order:

1. `PHENOTYPE_GFX_LIB` environment variable (CI uses this).
2. `Assets/Plugins/PhenotypeGfx/` relative to the Unity project root.
3. `target/release/` relative to the repository root (local dev).
4. OS-default system library search path.

### Running the suite locally

```bash
# 1. Build the native library
cargo build --release

# 2. Copy the binary into the Unity Plugins folder (path varies by project)
cp target/release/libphenotype_gfx.so ~/MyUnityProject/Assets/Plugins/PhenotypeGfx/

# 3. Run NUnit headless
cd tests/unity/PhenotypeGfx.Tests
dotnet test --logger "console;verbosity=detailed"
```

### CI integration

The CI workflow `.github/workflows/ci.yml` runs the NUnit suite as a
separate job on Ubuntu Linux runners. It does **not** depend on a Unity
Editor installation — it builds a stand-alone .NET console runner that
hosts the NUnit driver and P/Invokes into the prebuilt `libphenotype_gfx.so`.

```yaml
# .github/workflows/ci.yml (excerpt)
- name: Run Unity C# tests
  working-directory: tests/unity/PhenotypeGfx.Tests
  run: dotnet test --logger "console;verbosity=normal"
  env:
    PHENOTYPE_GFX_LIB: ${{ github.workspace }}/target/release/libphenotype_gfx.so
```

## Versioning and stability

- The C-ABI uses **schema version 1** (`Constants.SchemaVersion = 1`).
- New functions may be added at the bottom of each module — additive changes
  are not breaking.
- Removing or changing a function signature is a breaking change and
  requires `SchemaVersion` to bump.
- Renaming a function is always a breaking change (Unity recompiles).
- Layout changes to `MeshVertex` are breaking and require `SchemaVersion` bump.

## Performance notes

- **`Set` is amortised O(1)** — chunk allocation happens lazily on the
  first write into a new chunk grid cell.
- **`BuildMesh` is O(chunk)`** for a fully populated chunk; for sparse
  chunks it depends on visible-face count.
- **Vertex / index pointers are stable** for the lifetime of a
  `MeshBuffer`; you may upload them to GPU memory without copying.
- **Dispose explicitly** in `OnDisable` / `OnDestroy` to avoid leaking
  the boxed Rust handle across scene loads.

## Troubleshooting

| Symptom                                  | Likely cause                       | Fix                                  |
| ---------------------------------------- | ---------------------------------- | ------------------------------------ |
| `DllNotFoundException`                   | Native library not on path         | Set `PHENOTYPE_GFX_LIB` or copy DLL  |
| `EntryPointNotFoundException`            | ABI version mismatch               | Rebuild native + update wrapper      |
| Garbage verts / indices                  | `MeshBuffer` disposed early        | Hold reference until upload complete |
| `AccessViolationException`               | `_destroy` called twice            | Ensure `Dispose()` runs once         |
| `Mesh.vertices` count wrong              | `BuildMesh` returned null handle   | Verify chunk grid coord is loaded    |

## Reference

- `unity/PhenotypeGfx.cs` — full C# wrapper (~750 lines).
- `unity/PhenotypeGfx.Example.cs` — concrete integration example.
- `tests/unity/PhenotypeGfx.Tests/` — NUnit suite.
- [`ffi.md`](./ffi.md) — C-ABI surface reference.
