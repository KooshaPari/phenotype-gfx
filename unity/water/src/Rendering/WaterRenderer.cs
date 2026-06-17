using System;
using UnityEngine;
using Phenotype.Water;

namespace Phenotype.Water.Rendering
{
    /// <summary>
    /// Orchestrates the water rendering pipeline by combining LOD selection,
    /// mesh generation, and material application.
    /// </summary>
    /// <remarks>
    /// <see cref="WaterRenderer"/> acts as the high-level controller for a water patch.
    /// It owns the <see cref="WaterLod"/> and <see cref="GerstnerWaveBank"/> references,
    /// and delegates mesh construction to <see cref="FluidMesh.Build"/>.
    /// </remarks>
    /// <example>
    /// <code>
    /// var renderer = new WaterRenderer
    /// {
    ///     WaveBank = GerstnerWaveBank.CreateOceanPreset(),
    ///     Material = new WaterMaterial(new WaterShader("Phenotype/Water")),
    ///     PatchSize = 200f
    /// };
    /// MeshData mesh = renderer.BuildMesh(1.5f, 75f);
    /// </code>
    /// </example>
    public class WaterRenderer
    {
        /// <summary>
        /// LOD controller for the water mesh.
        /// </summary>
        /// <value>
        /// Defaults to a new <see cref="WaterLod"/> instance with standard thresholds.
        /// Modify the returned instance to customize LOD behaviour.
        /// </value>
        /// <example>
        /// <code>
        /// var renderer = new WaterRenderer();
        /// renderer.Lod.NearResolution = 128;
        /// renderer.Lod.MidDistance = 100f;
        /// </code>
        /// </example>
        public WaterLod Lod { get; } = new WaterLod();

        /// <summary>
        /// The wave bank driving vertex displacement.
        /// </summary>
        /// <value>
        /// Must be set before calling <see cref="BuildMesh(float, float)"/>.
        /// Typically initialised with a preset such as <see cref="GerstnerWaveBank.CreateOceanPreset"/>.
        /// </value>
        /// <example>
        /// <code>
        /// var renderer = new WaterRenderer();
        /// renderer.WaveBank = GerstnerWaveBank.CreateOceanPreset();
        /// </code>
        /// </example>
        public GerstnerWaveBank WaveBank { get; set; }

        /// <summary>
        /// Optional material applied during rendering.
        /// </summary>
        /// <value>
        /// The water material used for the mesh. Can be <c>null</c> if the caller
        /// manages material assignment externally.
        /// </value>
        /// <example>
        /// <code>
        /// var renderer = new WaterRenderer();
        /// renderer.Material = new WaterMaterial(new WaterShader("Phenotype/Water"));
        /// </code>
        /// </example>
        public WaterMaterial Material { get; set; }

        /// <summary>
        /// The world-space size of the water patch in metres.
        /// </summary>
        /// <value>Default: 100f. Must be positive.</value>
        /// <example>
        /// <code>
        /// var renderer = new WaterRenderer();
        /// renderer.PatchSize = 250f; // Large ocean patch
        /// </code>
        /// </example>
        public float PatchSize { get; set; } = 100f;

        /// <summary>
        /// Generates the water mesh for the given camera distance and time.
        /// </summary>
        /// <param name="time">Simulation time in seconds.</param>
        /// <param name="distance">Camera-to-water distance in world units.</param>
        /// <returns>
        /// Mesh data containing displaced vertices, normals, UVs, and triangle indices.
        /// Returns a default empty <see cref="MeshData"/> when the mesh is culled.
        /// </returns>
        /// <exception cref="InvalidOperationException">
        /// Thrown if <see cref="WaveBank"/> is null when this method is called.
        /// </exception>
        /// <remarks>
        /// When the mesh is culled, the returned <see cref="MeshData"/> contains empty arrays
        /// to avoid unnecessary allocations.
        /// </remarks>
        /// <example>
        /// <code>
        /// var renderer = new WaterRenderer();
        /// renderer.WaveBank = GerstnerWaveBank.CreateOceanPreset();
        /// MeshData mesh = renderer.BuildMesh(1.5f, 75f);
        /// if (mesh.Vertices.Length > 0)
        /// {
        ///     meshFilter.mesh.vertices = mesh.Vertices;
        ///     meshFilter.mesh.normals = mesh.Normals;
        ///     meshFilter.mesh.triangles = mesh.Indices;
        /// }
        /// </code>
        /// </example>
        public MeshData BuildMesh(float time, float distance)
        {
            if (WaveBank == null)
                throw new InvalidOperationException("WaveBank must be set before rendering.");

            int resolution = Lod.SelectResolution(distance);
            if (resolution <= 0)
                return new MeshData { Vertices = Array.Empty<Vector3>(), Normals = Array.Empty<Vector3>(), UVs = Array.Empty<Vector2>(), Indices = Array.Empty<int>() };

            return FluidMesh.Build(WaveBank, resolution, PatchSize, time);
        }
    }
}
