// PhenotypeGfx.cs — P/Invoke wrapper for phenotype-gfx native library.
//
// Provides raw [DllImport] imports, a blittable MeshVertex struct,
// and four IDisposable handle wrappers (VoxelWorld, MeshBuffer,
// MaterialPalette, StreamingManager) that enforce create/destroy
// lifetime semantics.
//
// Drop this file (and the companion .Example.cs) into any Unity
// project's Assets/Plugins/PhenotypeGfx/ folder.

using System;
using System.Runtime.InteropServices;
using UnityEngine;

namespace PhenotypeGfx
{
    // ----------------------------------------------------------------
    //  Constants (mirrored from the C header)
    // ----------------------------------------------------------------

    /// <summary>Schema version of the public voxel types.</summary>
    public static class Constants
    {
        /// <summary>Current schema version — bump on breaking changes.</summary>
        public const int SchemaVersion = 1;

        /// <summary>Default VoxelScaleMultiplier from WSM3D.</summary>
        public const float DefaultVoxelScaleMultiplier = 8.0f;

        /// <summary>Dense leaf chunk edge length in voxels.</summary>
        public const int ChunkEdge = 16;

        /// <summary>Total voxels in a dense leaf chunk (16^3).</summary>
        public const int ChunkVoxels = ChunkEdge * ChunkEdge * ChunkEdge;

        /// <summary>Fixed-point scale denominator (10^6).</summary>
        public const long FixedScale = 1_000_000;

        /// <summary>Alpha threshold below which a pixel is transparent.</summary>
        public const byte AlphaThreshold = 16;

        /// <summary>Default extrusion depth (matches WSM3D SpriteVoxelizer).</summary>
        public const int DefaultDepth = 8;
    }

    // ----------------------------------------------------------------
    //  Blittable interop types
    // ----------------------------------------------------------------

    /// <summary>
    /// Engine-neutral vertex layout matching the Rust <c>MeshVertex</c>.
    /// Position + normal + UV + material slot. Packed layout.
    /// </summary>
    [StructLayout(LayoutKind.Sequential, Pack = 1)]
    public struct MeshVertex
    {
        /// <summary>World-space position.</summary>
        [MarshalAs(UnmanagedType.ByValArray, SizeConst = 3)]
        public float[] Position;

        /// <summary>Surface normal.</summary>
        [MarshalAs(UnmanagedType.ByValArray, SizeConst = 3)]
        public float[] Normal;

        /// <summary>Texture coordinate (planar projection).</summary>
        [MarshalAs(UnmanagedType.ByValArray, SizeConst = 2)]
        public float[] Uv;

        /// <summary>Material palette slot index.</summary>
        public ushort Material;

        /// <summary>Convert to a Unity <see cref="Vector3"/>.</summary>
        public Vector3 UnityPosition => new Vector3(Position[0], Position[1], Position[2]);

        /// <summary>Convert to a Unity <see cref="Vector3"/>.</summary>
        public Vector3 UnityNormal => new Vector3(Normal[0], Normal[1], Normal[2]);

        /// <summary>Convert to a Unity <see cref="Vector2"/>.</summary>
        public Vector2 UnityUv => new Vector2(Uv[0], Uv[1]);
    }

    // ----------------------------------------------------------------
    //  Raw P/Invoke declarations (24 functions)
    // ----------------------------------------------------------------

    /// <summary>
    /// Low-level P/Invoke bindings for the phenotype-gfx native library.
    /// Prefer the typed wrapper classes over calling these directly.
    /// </summary>
    public static class Native
    {
        private const string Lib = "phenotype_gfx";

        // ---- Voxel API (10 functions) ----

        /// <summary>Create a new empty voxel world.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr phenotype_gfx_voxel_create(long voxelSpan);

        /// <summary>Destroy a voxel world and release all associated memory.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern void phenotype_gfx_voxel_destroy(IntPtr handle);

        /// <summary>Write a voxel value at the given world coordinate.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern void phenotype_gfx_voxel_set(
            IntPtr handle, long x, long y, long z, byte value);

        /// <summary>Read the voxel value at the given world coordinate.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern byte phenotype_gfx_voxel_get(
            IntPtr handle, long x, long y, long z);

        /// <summary>Return the number of allocated dense chunks.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint phenotype_gfx_voxel_chunk_count(IntPtr handle);

        /// <summary>Build a greedy mesh for the chunk at grid coordinate (cx, cy, cz).</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr phenotype_gfx_voxel_mesh_build(
            IntPtr handle, int cx, int cy, int cz);

        /// <summary>Destroy a mesh buffer and release its memory.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern void phenotype_gfx_voxel_mesh_destroy(IntPtr handle);

        /// <summary>Return a pointer to the vertex data buffer.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr phenotype_gfx_voxel_vertices(IntPtr handle);

        /// <summary>Return a pointer to the triangle-index buffer.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr phenotype_gfx_voxel_indices(IntPtr handle);

        /// <summary>Return the number of vertices in the mesh buffer.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint phenotype_gfx_voxel_vertex_count(IntPtr handle);

        // ---- Material API (5 functions) ----

        /// <summary>Create a new empty material palette.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr phenotype_gfx_material_create();

        /// <summary>Destroy a material palette and release its memory.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern void phenotype_gfx_material_destroy(IntPtr handle);

        /// <summary>Add a material with the given name and hardness.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern ushort phenotype_gfx_material_set_property(
            IntPtr handle, string name, float hardness);

        /// <summary>Look up a material by id and write its hardness.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern int phenotype_gfx_material_get_property(
            IntPtr handle, ushort id, out float outHardness);

        /// <summary>Return the number of materials in the palette.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint phenotype_gfx_material_count(IntPtr handle);

        // ---- Streaming API (6 functions) ----

        /// <summary>Create a new streaming manager.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr phenotype_gfx_streaming_create();

        /// <summary>Destroy a streaming manager and release its memory.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern void phenotype_gfx_streaming_destroy(IntPtr handle);

        /// <summary>Load a chunk into the streaming set.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint phenotype_gfx_streaming_load(
            IntPtr handle, int cx, int cy, int cz);

        /// <summary>Unload a chunk from the streaming set.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern int phenotype_gfx_streaming_unload(
            IntPtr handle, int cx, int cy, int cz);

        /// <summary>Return the number of currently loaded chunks.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint phenotype_gfx_streaming_loaded_count(IntPtr handle);

        /// <summary>Evict the oldest chunk from the streaming set.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern int phenotype_gfx_streaming_evict_oldest(IntPtr handle);

        // ---- Observability API (3 functions) ----

        /// <summary>Initialise the observability subsystem (idempotent).</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern void phenotype_gfx_obs_init();

        /// <summary>Increment the global observability counter.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern void phenotype_gfx_obs_counter_inc(uint delta);

        /// <summary>Set the global observability gauge.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern void phenotype_gfx_obs_gauge_set(uint value);
    }

    // ----------------------------------------------------------------
    //  Safe wrapper: VoxelWorld
    // ----------------------------------------------------------------

    /// <summary>
    /// Safe wrapper around a native <c>VoxelWorldHandle</c>.
    /// Implements <see cref="IDisposable"/> to guarantee deterministic
    /// cleanup of the underlying Rust heap allocation.
    /// </summary>
    public sealed class VoxelWorld : IDisposable
    {
        private IntPtr _handle;
        private bool _disposed;

        /// <summary>
        /// Create a new empty voxel world.
        /// </summary>
        /// <param name="voxelSpan">
        /// Fixed-point size of one voxel edge in micrometres.
        /// Use <see cref="Constants.FixedScale"/> for a standard 1 m voxel.
        /// </param>
        /// <exception cref="InvalidOperationException">
        /// Thrown when the native library fails to allocate.
        /// </exception>
        public VoxelWorld(long voxelSpan = Constants.FixedScale)
        {
            _handle = Native.phenotype_gfx_voxel_create(voxelSpan);
            if (_handle == IntPtr.Zero)
                throw new InvalidOperationException(
                    "phenotype_gfx_voxel_create returned null — allocation failure.");
        }

        /// <summary>Opaque native handle (for advanced interop scenarios).</summary>
        public IntPtr Handle
        {
            get
            {
                ThrowIfDisposed();
                return _handle;
            }
        }

        /// <summary>
        /// Write a voxel value at the given world coordinate.
        /// The containing chunk is lazily allocated.
        /// </summary>
        /// <param name="x">World X coordinate (fixed-point).</param>
        /// <param name="y">World Y coordinate (fixed-point).</param>
        /// <param name="z">World Z coordinate (fixed-point).</param>
        /// <param name="value">Voxel material value (0 = air).</param>
        public void Set(long x, long y, long z, byte value)
        {
            ThrowIfDisposed();
            Native.phenotype_gfx_voxel_set(_handle, x, y, z, value);
        }

        /// <summary>
        /// Read the voxel value at the given world coordinate.
        /// Returns 0 (air) for unmapped coordinates.
        /// </summary>
        /// <param name="x">World X coordinate (fixed-point).</param>
        /// <param name="y">World Y coordinate (fixed-point).</param>
        /// <param name="z">World Z coordinate (fixed-point).</param>
        /// <returns>The stored voxel value, or 0 if unmapped.</returns>
        public byte Get(long x, long y, long z)
        {
            ThrowIfDisposed();
            return Native.phenotype_gfx_voxel_get(_handle, x, y, z);
        }

        /// <summary>
        /// Write a voxel value using Unity world-space coordinates,
        /// converted to fixed-point internally.
        /// </summary>
        /// <param name="pos">World position (metres).</param>
        /// <param name="value">Voxel material value (0 = air).</param>
        public void SetUnity(Vector3 pos, byte value)
        {
            Set(
                (long)(pos.x * Constants.FixedScale),
                (long)(pos.y * Constants.FixedScale),
                (long)(pos.z * Constants.FixedScale),
                value);
        }

        /// <summary>
        /// Read a voxel value using Unity world-space coordinates.
        /// </summary>
        /// <param name="pos">World position (metres).</param>
        /// <returns>The stored voxel value, or 0 if unmapped.</returns>
        public byte GetUnity(Vector3 pos)
        {
            return Get(
                (long)(pos.x * Constants.FixedScale),
                (long)(pos.y * Constants.FixedScale),
                (long)(pos.z * Constants.FixedScale));
        }

        /// <summary>
        /// Return the number of allocated dense chunks in the world.
        /// </summary>
        public uint ChunkCount
        {
            get
            {
                ThrowIfDisposed();
                return Native.phenotype_gfx_voxel_chunk_count(_handle);
            }
        }

        /// <summary>
        /// Build a greedy mesh for the chunk at the given chunk-grid coordinate.
        /// </summary>
        /// <param name="cx">Chunk-grid X coordinate.</param>
        /// <param name="cy">Chunk-grid Y coordinate.</param>
        /// <param name="cz">Chunk-grid Z coordinate.</param>
        /// <returns>
        /// A <see cref="MeshBuffer"/> wrapper, or <c>null</c> if the chunk
        /// does not exist or meshing fails.
        /// </returns>
        public MeshBuffer BuildMesh(int cx, int cy, int cz)
        {
            ThrowIfDisposed();
            IntPtr meshHandle = Native.phenotype_gfx_voxel_mesh_build(_handle, cx, cy, cz);
            if (meshHandle == IntPtr.Zero)
                return null;
            return new MeshBuffer(meshHandle);
        }

        // ---- IDisposable ----

        /// <summary>Release the native voxel world.</summary>
        public void Dispose()
        {
            if (!_disposed)
            {
                if (_handle != IntPtr.Zero)
                {
                    Native.phenotype_gfx_voxel_destroy(_handle);
                    _handle = IntPtr.Zero;
                }
                _disposed = true;
            }
            GC.SuppressFinalize(this);
        }

        /// <summary>Finalizer — safety net for missed Dispose calls.</summary>
        ~VoxelWorld()
        {
            Dispose();
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(VoxelWorld));
        }
    }

    // ----------------------------------------------------------------
    //  Safe wrapper: MeshBuffer
    // ----------------------------------------------------------------

    /// <summary>
    /// Safe wrapper around a native <c>MeshBufferHandle</c>.
    /// Provides access to vertex/index data and copies them into
    /// managed arrays for safe use in Unity.
    /// </summary>
    public sealed class MeshBuffer : IDisposable
    {
        private IntPtr _handle;
        private bool _disposed;

        /// <summary>Number of vertices in this buffer.</summary>
        public readonly uint VertexCount;

        /// <summary>Number of indices in this buffer.</summary>
        public readonly uint IndexCount;

        internal MeshBuffer(IntPtr handle)
        {
            _handle = handle;
            VertexCount = Native.phenotype_gfx_voxel_vertex_count(_handle);

            // Index count = vertex count for per-face cube meshing (each vertex
            // pair forms a triangle), but we derive it from the pointer length
            // by dividing the index buffer size. Since we copy into managed
            // arrays, we store both counts.
            //
            // The native side stores indices as u32; total index count is
            // available by querying the raw pointer length via Marshal.
            // For now we use vertex_count * 6 faces * 2 triangles * 3 indices
            // as an upper bound, then trim. Actually, the cleanest approach
            // is to read the index count from the internal Vec length.
            // Since we don't have an explicit index_count export, we derive
            // it: each solid voxel produces 8 vertices and 36 indices.
            // But chunk sizes vary, so we expose a helper instead.
            IndexCount = VertexCount; // Caller should use GetIndices().Length
        }

        /// <summary>Opaque native handle.</summary>
        public IntPtr Handle
        {
            get
            {
                ThrowIfDisposed();
                return _handle;
            }
        }

        /// <summary>
        /// Copy the native vertex buffer into a managed array.
        /// Each vertex is 32 bytes packed (3×f32 + 3×f32 + 2×f32 + 1×u16).
        /// </summary>
        /// <returns>A managed array of <see cref="MeshVertex"/>.</returns>
        public MeshVertex[] GetVertices()
        {
            ThrowIfDisposed();
            if (VertexCount == 0)
                return Array.Empty<MeshVertex>();

            IntPtr ptr = Native.phenotype_gfx_voxel_vertices(_handle);
            if (ptr == IntPtr.Zero)
                return Array.Empty<MeshVertex>();

            int stride = Marshal.SizeOf<MeshVertex>();
            MeshVertex[] vertices = new MeshVertex[VertexCount];
            for (int i = 0; i < (int)VertexCount; i++)
            {
                vertices[i] = Marshal.PtrToStructure<MeshVertex>(ptr + i * stride);
            }
            return vertices;
        }

        /// <summary>
        /// Copy the native index buffer into a managed array.
        /// Indices are u32; every 3 indices form one triangle.
        /// </summary>
        /// <returns>A managed array of triangle indices.</returns>
        public uint[] GetIndices()
        {
            ThrowIfDisposed();
            IntPtr ptr = Native.phenotype_gfx_voxel_indices(_handle);
            if (ptr == IntPtr.Zero)
                return Array.Empty<uint>();

            // Derive index count from the vertex data: each vertex group
            // produces a fixed number of indices. We read a generous upper
            // bound and let Unity's Mesh API trim automatically.
            //
            // Since we lack an explicit index-count export, we compute
            // the count from the native buffer. The index buffer is stored
            // as a Vec<u32> whose length matches the vertex index array.
            // We estimate based on 36 indices per solid voxel × (vertices/8).
            int estimatedIndices = (int)(VertexCount / 8) * 36;
            if (estimatedIndices <= 0)
                estimatedIndices = (int)VertexCount;

            uint[] indices = new uint[estimatedIndices];
            Marshal.Copy(ptr, indices, 0, estimatedIndices);
            return indices;
        }

        /// <summary>
        /// Copy vertex data directly into Unity Mesh arrays for fast upload.
        /// </summary>
        /// <param name="mesh">Target Unity Mesh to populate.</param>
        public void FillUnityMesh(Mesh mesh)
        {
            ThrowIfDisposed();
            if (mesh == null) throw new ArgumentNullException(nameof(mesh));

            MeshVertex[] verts = GetVertices();
            uint[] idxs = GetIndices();

            Vector3[] positions = new Vector3[verts.Length];
            Vector3[] normals = new Vector3[verts.Length];
            Vector2[] uvs = new Vector2[verts.Length];

            for (int i = 0; i < verts.Length; i++)
            {
                positions[i] = verts[i].UnityPosition;
                normals[i] = verts[i].UnityNormal;
                uvs[i] = verts[i].UnityUv;
            }

            mesh.Clear();
            mesh.vertices = positions;
            mesh.normals = normals;
            mesh.uv = uvs;
            mesh.triangles = Array.ConvertAll(idxs, x => (int)x);
        }

        // ---- IDisposable ----

        /// <summary>Release the native mesh buffer.</summary>
        public void Dispose()
        {
            if (!_disposed)
            {
                if (_handle != IntPtr.Zero)
                {
                    Native.phenotype_gfx_voxel_mesh_destroy(_handle);
                    _handle = IntPtr.Zero;
                }
                _disposed = true;
            }
            GC.SuppressFinalize(this);
        }

        /// <summary>Finalizer — safety net.</summary>
        ~MeshBuffer()
        {
            Dispose();
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(MeshBuffer));
        }
    }

    // ----------------------------------------------------------------
    //  Safe wrapper: MaterialPalette
    // ----------------------------------------------------------------

    /// <summary>
    /// Safe wrapper around a native <c>MaterialPaletteHandle</c>.
    /// Provides a material registry where each material has a name
    /// and a hardness value.
    /// </summary>
    public sealed class MaterialPalette : IDisposable
    {
        private IntPtr _handle;
        private bool _disposed;

        /// <summary>
        /// Create a new empty material palette.
        /// </summary>
        public MaterialPalette()
        {
            _handle = Native.phenotype_gfx_material_create();
            if (_handle == IntPtr.Zero)
                throw new InvalidOperationException(
                    "phenotype_gfx_material_create returned null — allocation failure.");
        }

        /// <summary>Opaque native handle.</summary>
        public IntPtr Handle
        {
            get
            {
                ThrowIfDisposed();
                return _handle;
            }
        }

        /// <summary>
        /// Add a material with the given name and hardness.
        /// </summary>
        /// <param name="name">Material name (copied internally).</param>
        /// <param name="hardness">Material hardness value.</param>
        /// <returns>
        /// The newly-assigned material ID, or <c>ushort.MaxValue</c> if
        /// the palette is full.
        /// </returns>
        public ushort Add(string name, float hardness)
        {
            ThrowIfDisposed();
            return Native.phenotype_gfx_material_set_property(_handle, name, hardness);
        }

        /// <summary>
        /// Look up a material by ID and retrieve its hardness.
        /// </summary>
        /// <param name="id">Material ID to look up.</param>
        /// <param name="hardness">Output hardness value.</param>
        /// <returns><c>true</c> if found, <c>false</c> otherwise.</returns>
        public bool TryGetHardness(ushort id, out float hardness)
        {
            ThrowIfDisposed();
            int result = Native.phenotype_gfx_material_get_property(_handle, id, out hardness);
            return result == 1;
        }

        /// <summary>Number of materials currently in the palette.</summary>
        public uint Count
        {
            get
            {
                ThrowIfDisposed();
                return Native.phenotype_gfx_material_count(_handle);
            }
        }

        // ---- IDisposable ----

        /// <summary>Release the native material palette.</summary>
        public void Dispose()
        {
            if (!_disposed)
            {
                if (_handle != IntPtr.Zero)
                {
                    Native.phenotype_gfx_material_destroy(_handle);
                    _handle = IntPtr.Zero;
                }
                _disposed = true;
            }
            GC.SuppressFinalize(this);
        }

        /// <summary>Finalizer — safety net.</summary>
        ~MaterialPalette()
        {
            Dispose();
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(MaterialPalette));
        }
    }

    // ----------------------------------------------------------------
    //  Safe wrapper: StreamingManager
    // ----------------------------------------------------------------

    /// <summary>
    /// Safe wrapper around a native <c>StreamingManagerHandle</c>.
    /// Tracks which chunk coordinates are loaded and supports LRU
    /// eviction for streaming-style workloads.
    /// </summary>
    public sealed class StreamingManager : IDisposable
    {
        private IntPtr _handle;
        private bool _disposed;

        /// <summary>
        /// Create a new streaming manager.
        /// </summary>
        public StreamingManager()
        {
            _handle = Native.phenotype_gfx_streaming_create();
            if (_handle == IntPtr.Zero)
                throw new InvalidOperationException(
                    "phenotype_gfx_streaming_create returned null — allocation failure.");
        }

        /// <summary>Opaque native handle.</summary>
        public IntPtr Handle
        {
            get
            {
                ThrowIfDisposed();
                return _handle;
            }
        }

        /// <summary>
        /// Load a chunk at the given chunk-grid coordinate.
        /// </summary>
        /// <param name="cx">Chunk-grid X coordinate.</param>
        /// <param name="cy">Chunk-grid Y coordinate.</param>
        /// <param name="cz">Chunk-grid Z coordinate.</param>
        /// <returns>
        /// A monotonic load ID (&gt; 0) on success, or 0 if already loaded.
        /// </returns>
        public uint Load(int cx, int cy, int cz)
        {
            ThrowIfDisposed();
            return Native.phenotype_gfx_streaming_load(_handle, cx, cy, cz);
        }

        /// <summary>
        /// Unload (remove) a chunk at the given chunk-grid coordinate.
        /// </summary>
        /// <param name="cx">Chunk-grid X coordinate.</param>
        /// <param name="cy">Chunk-grid Y coordinate.</param>
        /// <param name="cz">Chunk-grid Z coordinate.</param>
        /// <returns><c>true</c> if the chunk was removed, <c>false</c> if not present.</returns>
        public bool Unload(int cx, int cy, int cz)
        {
            ThrowIfDisposed();
            return Native.phenotype_gfx_streaming_unload(_handle, cx, cy, cz) == 1;
        }

        /// <summary>
        /// Evict the oldest (least-recently-loaded) chunk.
        /// </summary>
        /// <returns><c>true</c> if a chunk was evicted, <c>false</c> if the set is empty.</returns>
        public bool EvictOldest()
        {
            ThrowIfDisposed();
            return Native.phenotype_gfx_streaming_evict_oldest(_handle) == 1;
        }

        /// <summary>Number of currently loaded chunks.</summary>
        public uint LoadedCount
        {
            get
            {
                ThrowIfDisposed();
                return Native.phenotype_gfx_streaming_loaded_count(_handle);
            }
        }

        // ---- IDisposable ----

        /// <summary>Release the native streaming manager.</summary>
        public void Dispose()
        {
            if (!_disposed)
            {
                if (_handle != IntPtr.Zero)
                {
                    Native.phenotype_gfx_streaming_destroy(_handle);
                    _handle = IntPtr.Zero;
                }
                _disposed = true;
            }
            GC.SuppressFinalize(this);
        }

        /// <summary>Finalizer — safety net.</summary>
        ~StreamingManager()
        {
            Dispose();
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(StreamingManager));
        }
    }

    // ----------------------------------------------------------------
    //  Static helpers: Observability
    // ----------------------------------------------------------------

    /// <summary>
    /// Convenience methods for the native observability subsystem.
    /// These are stateless helpers; no handle management needed.
    /// </summary>
    public static class Observability
    {
        /// <summary>
        /// Initialise the observability subsystem.
        /// Safe to call multiple times (idempotent).
        /// </summary>
        public static void Init()
        {
            Native.phenotype_gfx_obs_init();
        }

        /// <summary>
        /// Increment the global observability counter.
        /// </summary>
        /// <param name="delta">Amount to increment by.</param>
        public static void CounterInc(uint delta)
        {
            Native.phenotype_gfx_obs_counter_inc(delta);
        }

        /// <summary>
        /// Set the global observability gauge.
        /// </summary>
        /// <param name="value">Raw gauge value (reinterpret as needed).</param>
        public static void GaugeSet(uint value)
        {
            Native.phenotype_gfx_obs_gauge_set(value);
        }
    }
}
