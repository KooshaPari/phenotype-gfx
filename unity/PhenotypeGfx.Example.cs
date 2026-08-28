// PhenotypeGfx.Example.cs — MonoBehaviour demonstrating the phenotype-gfx
// P/Invoke wrapper lifecycle: create world, set voxels, build mesh,
// read vertices/indices, and dispose.
//
// Attach this component to any GameObject in your scene, then press Play.

using UnityEngine;
using PhenotypeGfx;

/// <summary>
/// Demo MonoBehaviour that exercises the full phenotype-gfx lifecycle.
/// </summary>
public class PhenotypeGfxExample : MonoBehaviour
{
    [Header("Voxel Settings")]
    [Tooltip("Number of solid voxels to place along each axis.")]
    [SerializeField] private int _blockSize = 4;

    [Tooltip("Mesh filter to receive the generated mesh.")]
    [SerializeField] private MeshFilter _meshFilter;

    [Tooltip("Mesh renderer to display the result.")]
    [SerializeField] private MeshRenderer _meshRenderer;

    // Native handles — disposed in OnDestroy.
    private VoxelWorld _world;
    private MaterialPalette _palette;
    private StreamingManager _streaming;

    private void Start()
    {
        Debug.Log("[PhenotypeGfx] Initialising observability subsystem …");
        Observability.Init();

        // --- Material palette ---
        _palette = new MaterialPalette();
        ushort stoneId = _palette.Add("stone", 5.0f);
        ushort woodId  = _palette.Add("wood",  1.0f);
        Debug.Log($"[PhenotypeGfx] Palette has {_palette.Count} materials " +
                  $"(stone id={stoneId}, wood id={woodId})");

        // --- Voxel world (1 m voxels) ---
        _world = new VoxelWorld(Constants.FixedScale);
        Debug.Log($"[PhenotypeGfx] Created voxel world, " +
                  $"chunk count={_world.ChunkCount}");

        // Fill a solid block at the origin.
        for (int x = 0; x < _blockSize; x++)
        for (int y = 0; y < _blockSize; y++)
        for (int z = 0; z < _blockSize; z++)
        {
            // Alternate between stone and wood for visual interest.
            byte val = (x + y + z) % 2 == 0 ? stoneId : woodId;
            _world.SetUnity(new Vector3(x, y, z), val);
        }

        Debug.Log($"[PhenotypeGfx] Placed {_blockSize}^3 voxels, " +
                  $"chunk count={_world.ChunkCount}");

        // Verify a round-trip read.
        byte readBack = _world.GetUnity(new Vector3(0, 0, 0));
        Debug.Log($"[PhenotypeGfx] Read-back at (0,0,0) = {readBack}");

        // --- Streaming manager demo ---
        _streaming = new StreamingManager();
        uint loadId = _streaming.Load(0, 0, 0);
        Debug.Log($"[PhenotypeGfx] Loaded chunk (0,0,0), loadId={loadId}, " +
                  $"loaded={_streaming.LoadedCount}");

        // --- Build mesh for chunk (0,0,0) ---
        MeshBuffer meshBuffer = _world.BuildMesh(0, 0, 0);
        if (meshBuffer != null)
        {
            Debug.Log($"[PhenotypeGfx] Mesh built: {meshBuffer.VertexCount} vertices");

            // Copy data into a Unity Mesh.
            Mesh mesh = new Mesh { name = "VoxelMesh" };
            meshBuffer.FillUnityMesh(mesh);

            if (_meshFilter != null)
                _meshFilter.mesh = mesh;

            // Optional: tint by reading vertex data.
            MeshVertex[] verts = meshBuffer.GetVertices();
            uint[] indices = meshBuffer.GetIndices();
            Debug.Log($"[PhenotypeGfx] Unity mesh ready: " +
                      $"{verts.Length} verts, {indices.Length} indices");

            meshBuffer.Dispose();
        }
        else
        {
            Debug.LogWarning("[PhenotypeGfx] Mesh build returned null — " +
                             "chunk (0,0,0) may not exist.");
        }

        Observability.CounterInc(1);
        Debug.Log("[PhenotypeGfx] Example complete.");
    }

    private void OnDestroy()
    {
        // Deterministic cleanup — order does not matter since
        // each wrapper destroys an independent native allocation.
        _streaming?.Dispose();
        _palette?.Dispose();
        _world?.Dispose();
        Debug.Log("[PhenotypeGfx] All native handles released.");
    }
}
