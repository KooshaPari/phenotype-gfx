using System;
using UnityEngine;

namespace Phenotype.Water
{
    /// <summary>
    /// Snapshot of a generated water grid mesh at a single point in time.
    /// All arrays are sized consistently: vertices[i] and normals[i] correspond
    /// to the same grid vertex, and indices form a list of CCW triangles.
    /// </summary>
    public struct MeshData
    {
        /// <summary>Displaced world-space vertex positions (length = (resolution+1)^2).</summary>
        public Vector3[] Vertices;

        /// <summary>Analytic unit normals per vertex.</summary>
        public Vector3[] Normals;

        /// <summary>
        /// Triangle index list (length = resolution^2 * 6).
        /// Each triplet is one triangle in counter-clockwise winding order.
        /// </summary>
        public int[] Indices;

        /// <summary>UV coordinates in [0,1] matching each vertex.</summary>
        public Vector2[] UVs;
    }

    /// <summary>
    /// Generates and maintains the procedural mesh grid used for water surface rendering.
    /// Handles vertex layout, UV tiling, and per-frame vertex displacement driven by
    /// <see cref="GerstnerWaveBank"/>.
    /// </summary>
    public class FluidMesh
    {
        /// <summary>
        /// Builds a tiling water grid mesh displaced by the given wave bank.
        /// </summary>
        /// <param name="bank">Wave bank used to displace and shade each vertex.</param>
        /// <param name="resolution">Number of quad columns (and rows) in the grid. Must be >= 1.</param>
        /// <param name="size">World-space side length of the square grid in metres.</param>
        /// <param name="time">Simulation time in seconds passed to the wave bank.</param>
        /// <returns>
        /// A <see cref="MeshData"/> whose vertex count is (resolution+1)^2 and
        /// index count is resolution^2 * 6.
        /// </returns>
        /// <exception cref="ArgumentNullException">Thrown if <paramref name="bank"/> is null.</exception>
        /// <exception cref="ArgumentOutOfRangeException">Thrown if resolution &lt; 1 or size &lt;= 0.</exception>
        public static MeshData Build(GerstnerWaveBank bank, int resolution, float size, float time)
        {
            if (bank == null)
                throw new ArgumentNullException(nameof(bank));
            if (resolution < 1)
                throw new ArgumentOutOfRangeException(nameof(resolution), "resolution must be >= 1");
            if (size <= 0f)
                throw new ArgumentOutOfRangeException(nameof(size), "size must be > 0");

            int verts = (resolution + 1) * (resolution + 1);
            int tris = resolution * resolution * 6;

            var vertices = new Vector3[verts];
            var normals = new Vector3[verts];
            var uvs = new Vector2[verts];
            var indices = new int[tris];

            float step = size / resolution;
            float half = size * 0.5f;

            // Build vertices and normals
            for (int row = 0; row <= resolution; row++)
            {
                for (int col = 0; col <= resolution; col++)
                {
                    int idx = row * (resolution + 1) + col;

                    // Undisplaced XZ position, centred at origin
                    float x = col * step - half;
                    float z = row * step - half;

                    var xz = new Vector2(x, z);
                    var disp = bank.SampleDisplacement(xz, time);

                    vertices[idx] = new Vector3(x + disp.x, disp.y, z + disp.z);
                    normals[idx] = bank.SampleNormal(xz, time);
                    uvs[idx] = new Vector2((float)col / resolution, (float)row / resolution);
                }
            }

            // Build triangle indices (two CCW triangles per quad)
            int ti = 0;
            for (int row = 0; row < resolution; row++)
            {
                for (int col = 0; col < resolution; col++)
                {
                    int bl = row * (resolution + 1) + col;       // bottom-left
                    int br = bl + 1;                                     // bottom-right
                    int tl = (row + 1) * (resolution + 1) + col;       // top-left
                    int tr = tl + 1;                                     // top-right

                    // Triangle 1: bottom-left, top-left, top-right
                    indices[ti++] = bl;
                    indices[ti++] = tl;
                    indices[ti++] = tr;

                    // Triangle 2: bottom-left, top-right, bottom-right
                    indices[ti++] = bl;
                    indices[ti++] = tr;
                    indices[ti++] = br;
                }
            }

            return new MeshData
            {
                Vertices = vertices,
                Normals = normals,
                UVs = uvs,
                Indices = indices,
            };
        }
    }
}
