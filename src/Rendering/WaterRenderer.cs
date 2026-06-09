using System;
using UnityEngine;
using Phenotype.Water;

namespace Phenotype.Water.Rendering
{
    /// <summary>
    /// Orchestrates the water rendering pipeline by combining LOD selection,
    /// mesh generation, and material application.
    /// </summary>
    public class WaterRenderer
    {
        /// <summary>
        /// LOD controller for the water mesh.
        /// </summary>
        public WaterLod Lod { get; } = new WaterLod();

        /// <summary>
        /// The wave bank driving vertex displacement.
        /// </summary>
        public GerstnerWaveBank WaveBank { get; set; }

        /// <summary>
        /// Optional material applied during rendering.
        /// </summary>
        public WaterMaterial Material { get; set; }

        /// <summary>
        /// The world-space size of the water patch in metres.
        /// </summary>
        public float PatchSize { get; set; } = 100f;

        /// <summary>
        /// Generates the water mesh for the given camera distance and time.
        /// Returns a default <see cref="MeshData"/> when the mesh is culled.
        /// </summary>
        /// <param name="time">Simulation time in seconds.</param>
        /// <param name="distance">Camera-to-water distance in world units.</param>
        /// <returns>Mesh data or default when culled.</returns>
        /// <exception cref="InvalidOperationException">Thrown if <see cref="WaveBank"/> is null.</exception>
        public MeshData BuildMesh(float time, float distance)
        {
            if (WaveBank == null)
                throw new InvalidOperationException("WaveBank must be set before rendering.");

            int resolution = Lod.SelectResolution(distance);
            if (resolution <= 0)
                return default;

            return FluidMesh.Build(WaveBank, resolution, PatchSize, time);
        }
    }
}
