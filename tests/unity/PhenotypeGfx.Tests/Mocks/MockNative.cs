// MockNative.cs — Pure-managed mock of the phenotype_gfx C ABI.
//
// This file replaces the [DllImport("phenotype_gfx", …)] declarations in
// unity/PhenotypeGfx.cs with in-process implementations that simulate the
// Rust core's behaviour for the 24 exported symbols. It exists so the C#
// wrapper's lifecycle contract (create → use → dispose) can be exercised
// in `dotnet test` without compiling or shipping the real cdylib.
//
// The mock's behaviour mirrors bindings/c_api.rs:
//
//   * voxel_create → allocate a VoxelWorld handle
//   * voxel_set    → lazily allocate a 16×16×16 chunk, store the byte
//   * voxel_get    → return the stored byte (0 for unmapped)
//   * chunk_count  → return number of allocated chunks
//   * mesh_build   → emit one 8-vertex / 36-index cube per solid voxel
//                    (returns null for empty/unmapped chunks)
//   * material_*   → maintain a List<VoxelMaterial>
//   * streaming_*  → LRU set of (cx,cy,cz) keys
//   * obs_*        → static atomic-like counters
//
// The wrapper classes (VoxelWorld, MeshBuffer, MaterialPalette,
// StreamingManager) at the bottom of this file mirror the real C#
// wrapper 1:1 — they call the static mock methods just as the real
// wrappers call into the native library.

using System;
using System.Collections.Generic;

namespace PhenotypeGfx.Tests
{
    /// <summary>
    /// Managed re-implementation of every extern "C" symbol in
    /// <c>bindings/c_api.rs</c>. Used by the wrapper classes below and by
    /// the NUnit tests for direct FFI contract checks.
    /// </summary>
    public static class MockNative
    {
        // ----------------------------------------------------------------
        // Constants (mirror include/phenotype_gfx.h)
        // ----------------------------------------------------------------

        public const long FixedScale = 1_000_000L;
        public const int ChunkEdge = 16;
        public const int ChunkVoxels = ChunkEdge * ChunkEdge * ChunkEdge;

        // ----------------------------------------------------------------
        // Handle types — managed classes that simulate the opaque Rust
        // structs. They are allocated with an integer id, and the mock
        // functions dispatch on that id (mirroring what a real P/Invoke
        // boundary would do via pointer arithmetic).
        // ----------------------------------------------------------------

        private static readonly object _gate = new object();
        private static long _nextHandleId = 0;

        private static long NextId() { lock (_gate) return ++_nextHandleId; }

        // Voxel world state
        private static readonly Dictionary<long, VoxelWorldState> _worlds = new();
        // Mesh buffer state
        private static readonly Dictionary<long, MeshBufferState> _meshes = new();
        // Material palette state
        private static readonly Dictionary<long, MaterialPaletteState> _palettes = new();
        // Streaming manager state
        private static readonly Dictionary<long, StreamingState> _streaming = new();

        // Observability state (process-wide)
        public static uint ObsCounter;
        public static uint ObsGauge;

        // ----------------------------------------------------------------
        // Voxel API (10 functions)
        // ----------------------------------------------------------------

        public static long phenotype_gfx_voxel_create(long voxelSpan)
        {
            var id = NextId();
            _worlds[id] = new VoxelWorldState(voxelSpan);
            return id;
        }

        public static void phenotype_gfx_voxel_destroy(long handle)
        {
            if (handle == 0) return;
            _worlds.Remove(handle);
        }

        public static void phenotype_gfx_voxel_set(long handle, long x, long y, long z, byte value)
        {
            if (!_worlds.TryGetValue(handle, out var w))
                throw new InvalidOperationException($"voxel_set: invalid handle {handle}");

            var key = (cx: (int)(x / FixedScale) / ChunkEdge,
                       cy: (int)(y / FixedScale) / ChunkEdge,
                       cz: (int)(z / FixedScale) / ChunkEdge);
            if (!w.Chunks.TryGetValue(key, out var chunk))
            {
                chunk = new byte[ChunkVoxels];
                w.Chunks[key] = chunk;
            }

            // Translate world (x,y,z) into the chunk-local voxel index.
            int lx = ((int)(x / FixedScale) % ChunkEdge + ChunkEdge) % ChunkEdge;
            int ly = ((int)(y / FixedScale) % ChunkEdge + ChunkEdge) % ChunkEdge;
            int lz = ((int)(z / FixedScale) % ChunkEdge + ChunkEdge) % ChunkEdge;

            int idx = lx + ly * ChunkEdge + lz * ChunkEdge * ChunkEdge;
            chunk[idx] = value;
        }

        public static byte phenotype_gfx_voxel_get(long handle, long x, long y, long z)
        {
            if (!_worlds.TryGetValue(handle, out var w))
                throw new InvalidOperationException($"voxel_get: invalid handle {handle}");

            var key = (cx: (int)(x / FixedScale) / ChunkEdge,
                       cy: (int)(y / FixedScale) / ChunkEdge,
                       cz: (int)(z / FixedScale) / ChunkEdge);

            if (!w.Chunks.TryGetValue(key, out var chunk))
                return 0;

            int lx = ((int)(x / FixedScale) % ChunkEdge + ChunkEdge) % ChunkEdge;
            int ly = ((int)(y / FixedScale) % ChunkEdge + ChunkEdge) % ChunkEdge;
            int lz = ((int)(z / FixedScale) % ChunkEdge + ChunkEdge) % ChunkEdge;
            int idx = lx + ly * ChunkEdge + lz * ChunkEdge * ChunkEdge;
            return chunk[idx];
        }

        public static uint phenotype_gfx_voxel_chunk_count(long handle)
        {
            if (!_worlds.TryGetValue(handle, out var w))
                throw new InvalidOperationException($"voxel_chunk_count: invalid handle {handle}");
            return (uint)w.Chunks.Count;
        }

        public static long phenotype_gfx_voxel_mesh_build(long handle, int cx, int cy, int cz)
        {
            if (!_worlds.TryGetValue(handle, out var w))
                throw new InvalidOperationException($"voxel_mesh_build: invalid handle {handle}");

            var key = (cx, cy, cz);
            if (!w.Chunks.TryGetValue(key, out var chunk))
                return 0; // null handle → mesh_build failed (matches Rust behaviour)

            var vertices = new List<MeshVertexRecord>();
            var indices = new List<uint>();

            for (int z = 0; z < ChunkEdge; z++)
                for (int y = 0; y < ChunkEdge; y++)
                    for (int x = 0; x < ChunkEdge; x++)
                    {
                        int idx = x + y * ChunkEdge + z * ChunkEdge * ChunkEdge;
                        byte voxel = chunk[idx];
                        if (voxel == 0) continue;

                        uint baseIdx = (uint)vertices.Count;
                        float fx = x, fy = y, fz = z;
                        // 8 cube corners
                        vertices.Add(new MeshVertexRecord { px = fx,     py = fy,     pz = fz,     material = voxel });
                        vertices.Add(new MeshVertexRecord { px = fx + 1, py = fy,     pz = fz,     material = voxel });
                        vertices.Add(new MeshVertexRecord { px = fx + 1, py = fy + 1, pz = fz,     material = voxel });
                        vertices.Add(new MeshVertexRecord { px = fx,     py = fy + 1, pz = fz,     material = voxel });
                        vertices.Add(new MeshVertexRecord { px = fx,     py = fy,     pz = fz + 1, material = voxel });
                        vertices.Add(new MeshVertexRecord { px = fx + 1, py = fy,     pz = fz + 1, material = voxel });
                        vertices.Add(new MeshVertexRecord { px = fx + 1, py = fy + 1, pz = fz + 1, material = voxel });
                        vertices.Add(new MeshVertexRecord { px = fx,     py = fy + 1, pz = fz + 1, material = voxel });

                        // 12 triangles (6 faces × 2)
                        uint[][] faces = new uint[][]
                        {
                            new uint[] { 0, 1, 2 }, new uint[] { 0, 2, 3 }, // front
                            new uint[] { 4, 6, 5 }, new uint[] { 4, 7, 6 }, // back
                            new uint[] { 0, 4, 5 }, new uint[] { 0, 5, 1 }, // bottom
                            new uint[] { 2, 6, 7 }, new uint[] { 2, 7, 3 }, // top
                            new uint[] { 0, 3, 7 }, new uint[] { 0, 7, 4 }, // left
                            new uint[] { 1, 5, 6 }, new uint[] { 1, 6, 2 }, // right
                        };
                        foreach (var face in faces)
                            foreach (var vi in face)
                                indices.Add(baseIdx + vi);
                    }

            if (vertices.Count == 0)
                return 0; // empty mesh — matches Rust null contract

            var id = NextId();
            _meshes[id] = new MeshBufferState(vertices.ToArray(), indices.ToArray());
            return id;
        }

        public static void phenotype_gfx_voxel_mesh_destroy(long handle)
        {
            if (handle == 0) return;
            _meshes.Remove(handle);
        }

        public static MeshVertexRecord[] phenotype_gfx_voxel_vertices(long handle)
        {
            if (!_meshes.TryGetValue(handle, out var m))
                throw new InvalidOperationException($"voxel_vertices: invalid handle {handle}");
            return m.Vertices;
        }

        public static uint[] phenotype_gfx_voxel_indices(long handle)
        {
            if (!_meshes.TryGetValue(handle, out var m))
                throw new InvalidOperationException($"voxel_indices: invalid handle {handle}");
            return m.Indices;
        }

        public static uint phenotype_gfx_voxel_vertex_count(long handle)
        {
            if (!_meshes.TryGetValue(handle, out var m))
                throw new InvalidOperationException($"voxel_vertex_count: invalid handle {handle}");
            return (uint)m.Vertices.Length;
        }

        public static uint phenotype_gfx_voxel_index_count(long handle)
        {
            if (!_meshes.TryGetValue(handle, out var m))
                throw new InvalidOperationException($"voxel_index_count: invalid handle {handle}");
            return (uint)m.Indices.Length;
        }

        // ----------------------------------------------------------------
        // Material API (5 functions)
        // ----------------------------------------------------------------

        public static long phenotype_gfx_material_create()
        {
            var id = NextId();
            _palettes[id] = new MaterialPaletteState();
            return id;
        }

        public static void phenotype_gfx_material_destroy(long handle)
        {
            if (handle == 0) return;
            _palettes.Remove(handle);
        }

        public static ushort phenotype_gfx_material_set_property(long handle, string name, float hardness)
        {
            if (!_palettes.TryGetValue(handle, out var p))
                throw new InvalidOperationException($"material_set_property: invalid handle {handle}");
            if (p.Materials.Count >= ushort.MaxValue)
                return ushort.MaxValue;
            p.Materials.Add(new MaterialRecord(name ?? string.Empty, hardness));
            return (ushort)(p.Materials.Count - 1);
        }

        public static int phenotype_gfx_material_get_property(long handle, ushort id, out float outHardness)
        {
            outHardness = 0f;
            if (!_palettes.TryGetValue(handle, out var p))
                throw new InvalidOperationException($"material_get_property: invalid handle {handle}");
            if (id >= p.Materials.Count) return 0;
            outHardness = p.Materials[id].Hardness;
            return 1;
        }

        public static uint phenotype_gfx_material_count(long handle)
        {
            if (!_palettes.TryGetValue(handle, out var p))
                throw new InvalidOperationException($"material_count: invalid handle {handle}");
            return (uint)p.Materials.Count;
        }

        // ----------------------------------------------------------------
        // Streaming API (6 functions)
        // ----------------------------------------------------------------

        public static long phenotype_gfx_streaming_create()
        {
            var id = NextId();
            _streaming[id] = new StreamingState();
            return id;
        }

        public static void phenotype_gfx_streaming_destroy(long handle)
        {
            if (handle == 0) return;
            _streaming.Remove(handle);
        }

        public static uint phenotype_gfx_streaming_load(long handle, int cx, int cy, int cz)
        {
            if (!_streaming.TryGetValue(handle, out var s))
                throw new InvalidOperationException($"streaming_load: invalid handle {handle}");
            var key = (cx, cy, cz);
            if (s.Loaded.ContainsKey(key)) return 0;
            s.NextId++;
            s.Loaded[key] = s.NextId;
            s.AccessOrder.Add(key);
            return s.NextId;
        }

        public static int phenotype_gfx_streaming_unload(long handle, int cx, int cy, int cz)
        {
            if (!_streaming.TryGetValue(handle, out var s))
                throw new InvalidOperationException($"streaming_unload: invalid handle {handle}");
            var key = (cx, cy, cz);
            if (s.Loaded.Remove(key))
            {
                s.AccessOrder.Remove(key);
                return 1;
            }
            return 0;
        }

        public static uint phenotype_gfx_streaming_loaded_count(long handle)
        {
            if (!_streaming.TryGetValue(handle, out var s))
                throw new InvalidOperationException($"streaming_loaded_count: invalid handle {handle}");
            return (uint)s.Loaded.Count;
        }

        public static int phenotype_gfx_streaming_evict_oldest(long handle)
        {
            if (!_streaming.TryGetValue(handle, out var s))
                throw new InvalidOperationException($"streaming_evict_oldest: invalid handle {handle}");
            if (s.AccessOrder.Count == 0) return 0;
            var oldest = s.AccessOrder[0];
            s.AccessOrder.RemoveAt(0);
            s.Loaded.Remove(oldest);
            return 1;
        }

        // ----------------------------------------------------------------
        // Observability API (3 functions)
        // ----------------------------------------------------------------

        public static void phenotype_gfx_obs_init()
        {
            // Idempotent no-op in the mock — matches the Rust contract.
        }

        public static void phenotype_gfx_obs_counter_inc(uint delta)
        {
            ObsCounter += delta;
        }

        public static void phenotype_gfx_obs_gauge_set(uint value)
        {
            ObsGauge = value;
        }

        // ----------------------------------------------------------------
        // Test helpers — let the test suite reset state between cases.
        // ----------------------------------------------------------------

        public static void ResetAll()
        {
            _worlds.Clear();
            _meshes.Clear();
            _palettes.Clear();
            _streaming.Clear();
            ObsCounter = 0;
            ObsGauge = 0;
        }

        // ----------------------------------------------------------------
        // Internal state records
        // ----------------------------------------------------------------

        private sealed class VoxelWorldState
        {
            public long VoxelSpan;
            public Dictionary<(int, int, int), byte[]> Chunks = new();
            public VoxelWorldState(long voxelSpan) { VoxelSpan = voxelSpan; }
        }

        private sealed class MeshBufferState
        {
            public MeshVertexRecord[] Vertices;
            public uint[] Indices;
            public MeshBufferState(MeshVertexRecord[] v, uint[] i) { Vertices = v; Indices = i; }
        }

        private sealed class MaterialPaletteState
        {
            public List<MaterialRecord> Materials = new();
        }

        private sealed class StreamingState
        {
            public Dictionary<(int, int, int), uint> Loaded = new();
            public List<(int, int, int)> AccessOrder = new();
            public uint NextId;
        }

        /// <summary>Mirrors the Rust MeshVertex POD layout.</summary>
        public struct MeshVertexRecord
        {
            public float px, py, pz;   // position
            public float nx, ny, nz;   // normal (mock: +Y up)
            public float u, v;         // uv
            public ushort material;
        }

        private readonly struct MaterialRecord
        {
            public readonly string Name;
            public readonly float Hardness;
            public MaterialRecord(string name, float hardness) { Name = name; Hardness = hardness; }
        }
    }

    // ====================================================================
    // IDisposable wrappers — these mirror unity/PhenotypeGfx.cs's
    // VoxelWorld / MeshBuffer / MaterialPalette / StreamingManager so the
    // public surface used by consumers is identical to what they'd touch
    // from inside Unity.
    // ====================================================================

    /// <summary>
    /// Mock-backed mirror of <c>PhenotypeGfx.VoxelWorld</c>.
    /// </summary>
    public sealed class VoxelWorld : IDisposable
    {
        private long _handle;
        private bool _disposed;

        public VoxelWorld(long voxelSpan = MockNative.FixedScale)
        {
            _handle = MockNative.phenotype_gfx_voxel_create(voxelSpan);
            if (_handle == 0)
                throw new InvalidOperationException("phenotype_gfx_voxel_create returned 0.");
        }

        public long Handle
        {
            get { ThrowIfDisposed(); return _handle; }
        }

        public void Set(long x, long y, long z, byte value)
        {
            ThrowIfDisposed();
            MockNative.phenotype_gfx_voxel_set(_handle, x, y, z, value);
        }

        public byte Get(long x, long y, long z)
        {
            ThrowIfDisposed();
            return MockNative.phenotype_gfx_voxel_get(_handle, x, y, z);
        }

        public void SetUnity(UnityEngine.Vector3 pos, byte value)
        {
            Set((long)(pos.x * MockNative.FixedScale),
                (long)(pos.y * MockNative.FixedScale),
                (long)(pos.z * MockNative.FixedScale),
                value);
        }

        public byte GetUnity(UnityEngine.Vector3 pos)
        {
            return Get((long)(pos.x * MockNative.FixedScale),
                       (long)(pos.y * MockNative.FixedScale),
                       (long)(pos.z * MockNative.FixedScale));
        }

        public uint ChunkCount
        {
            get { ThrowIfDisposed(); return MockNative.phenotype_gfx_voxel_chunk_count(_handle); }
        }

        public MeshBuffer BuildMesh(int cx, int cy, int cz)
        {
            ThrowIfDisposed();
            long meshHandle = MockNative.phenotype_gfx_voxel_mesh_build(_handle, cx, cy, cz);
            if (meshHandle == 0) return null;
            return new MeshBuffer(meshHandle);
        }

        public void Dispose()
        {
            if (!_disposed)
            {
                if (_handle != 0)
                {
                    MockNative.phenotype_gfx_voxel_destroy(_handle);
                    _handle = 0;
                }
                _disposed = true;
            }
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(VoxelWorld));
        }
    }

    /// <summary>
    /// Mock-backed mirror of <c>PhenotypeGfx.MeshBuffer</c>.
    /// </summary>
    public sealed class MeshBuffer : IDisposable
    {
        private long _handle;
        private bool _disposed;

        public readonly uint VertexCount;
        public readonly uint IndexCount;

        internal MeshBuffer(long handle)
        {
            _handle = handle;
            VertexCount = MockNative.phenotype_gfx_voxel_vertex_count(_handle);
            IndexCount = MockNative.phenotype_gfx_voxel_index_count(_handle);
        }

        public long Handle
        {
            get { ThrowIfDisposed(); return _handle; }
        }

        public MockNative.MeshVertexRecord[] GetVertices()
        {
            ThrowIfDisposed();
            if (VertexCount == 0) return Array.Empty<MockNative.MeshVertexRecord>();
            return MockNative.phenotype_gfx_voxel_vertices(_handle);
        }

        public uint[] GetIndices()
        {
            ThrowIfDisposed();
            if (IndexCount == 0) return Array.Empty<uint>();
            return MockNative.phenotype_gfx_voxel_indices(_handle);
        }

        public void FillUnityMesh(UnityEngine.Mesh mesh)
        {
            ThrowIfDisposed();
            if (mesh == null) throw new ArgumentNullException(nameof(mesh));

            var verts = GetVertices();
            var idxs = GetIndices();

            var positions = new UnityEngine.Vector3[verts.Length];
            var normals = new UnityEngine.Vector3[verts.Length];
            var uvs = new UnityEngine.Vector2[verts.Length];

            for (int i = 0; i < verts.Length; i++)
            {
                positions[i] = new UnityEngine.Vector3(verts[i].px, verts[i].py, verts[i].pz);
                normals[i] = new UnityEngine.Vector3(verts[i].nx, verts[i].ny, verts[i].nz);
                uvs[i] = new UnityEngine.Vector2(verts[i].u, verts[i].v);
            }

            mesh.Clear();
            mesh.vertices = positions;
            mesh.normals = normals;
            mesh.uv = uvs;
            mesh.triangles = Array.ConvertAll(idxs, x => (int)x);
        }

        public void Dispose()
        {
            if (!_disposed)
            {
                if (_handle != 0)
                {
                    MockNative.phenotype_gfx_voxel_mesh_destroy(_handle);
                    _handle = 0;
                }
                _disposed = true;
            }
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(MeshBuffer));
        }
    }

    /// <summary>
    /// Mock-backed mirror of <c>PhenotypeGfx.MaterialPalette</c>.
    /// </summary>
    public sealed class MaterialPalette : IDisposable
    {
        private long _handle;
        private bool _disposed;

        public MaterialPalette()
        {
            _handle = MockNative.phenotype_gfx_material_create();
            if (_handle == 0)
                throw new InvalidOperationException("phenotype_gfx_material_create returned 0.");
        }

        public long Handle
        {
            get { ThrowIfDisposed(); return _handle; }
        }

        public ushort Add(string name, float hardness)
        {
            ThrowIfDisposed();
            return MockNative.phenotype_gfx_material_set_property(_handle, name, hardness);
        }

        public bool TryGetHardness(ushort id, out float hardness)
        {
            ThrowIfDisposed();
            return MockNative.phenotype_gfx_material_get_property(_handle, id, out hardness) == 1;
        }

        public uint Count
        {
            get { ThrowIfDisposed(); return MockNative.phenotype_gfx_material_count(_handle); }
        }

        public void Dispose()
        {
            if (!_disposed)
            {
                if (_handle != 0)
                {
                    MockNative.phenotype_gfx_material_destroy(_handle);
                    _handle = 0;
                }
                _disposed = true;
            }
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(MaterialPalette));
        }
    }

    /// <summary>
    /// Mock-backed mirror of <c>PhenotypeGfx.StreamingManager</c>.
    /// </summary>
    public sealed class StreamingManager : IDisposable
    {
        private long _handle;
        private bool _disposed;

        public StreamingManager()
        {
            _handle = MockNative.phenotype_gfx_streaming_create();
            if (_handle == 0)
                throw new InvalidOperationException("phenotype_gfx_streaming_create returned 0.");
        }

        public long Handle
        {
            get { ThrowIfDisposed(); return _handle; }
        }

        public uint Load(int cx, int cy, int cz)
        {
            ThrowIfDisposed();
            return MockNative.phenotype_gfx_streaming_load(_handle, cx, cy, cz);
        }

        public bool Unload(int cx, int cy, int cz)
        {
            ThrowIfDisposed();
            return MockNative.phenotype_gfx_streaming_unload(_handle, cx, cy, cz) == 1;
        }

        public uint LoadedCount
        {
            get { ThrowIfDisposed(); return MockNative.phenotype_gfx_streaming_loaded_count(_handle); }
        }

        public bool EvictOldest()
        {
            ThrowIfDisposed();
            return MockNative.phenotype_gfx_streaming_evict_oldest(_handle) == 1;
        }

        public void Dispose()
        {
            if (!_disposed)
            {
                if (_handle != 0)
                {
                    MockNative.phenotype_gfx_streaming_destroy(_handle);
                    _handle = 0;
                }
                _disposed = true;
            }
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(StreamingManager));
        }
    }

    /// <summary>
    /// Mock-backed mirror of <c>PhenotypeGfx.Observability</c>.
    /// </summary>
    public static class Observability
    {
        public static void Init()
        {
            MockNative.phenotype_gfx_obs_init();
        }

        public static void CounterInc(uint delta)
        {
            MockNative.phenotype_gfx_obs_counter_inc(delta);
        }

        public static void GaugeSet(uint value)
        {
            MockNative.phenotype_gfx_obs_gauge_set(value);
        }
    }
}
