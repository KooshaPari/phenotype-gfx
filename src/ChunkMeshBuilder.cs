using System;
using UnityEngine;

namespace Phenotype.Terrain
{
    /// <summary>
    /// Holds the output of a mesh build operation: vertices, triangle indices,
    /// UV coordinates and per-vertex normals.
    /// </summary>
    public class MeshData
    {
        /// <summary>Vertex positions in world space.</summary>
        public Vector3[] Vertices { get; set; }

        /// <summary>Triangle index buffer (3 indices per triangle).</summary>
        public int[] Indices { get; set; }

        /// <summary>UV coordinates for each vertex.</summary>
        public Vector2[] UVs { get; set; }

        /// <summary>Per-vertex normals.</summary>
        public Vector3[] Normals { get; set; }
    }

    /// <summary>
    /// Generates Unity Mesh objects for terrain chunks from height-field data.
    /// Handles vertex layout, triangle winding, UV mapping, and normals.
    /// </summary>
    public class ChunkMeshBuilder
    {
        /// <summary>
        /// Builds a flat mesh for a terrain chunk with the given grid resolution.
        /// The chunk spans [0, size] in both X and Z, with Y = 0.
        /// </summary>
        /// <param name="resolution">Number of grid quads along each axis. Must be &gt; 0.</param>
        /// <param name="size">World-space size of the chunk. Default is 1.</param>
        /// <returns>A <see cref="MeshData"/> containing the generated mesh.</returns>
        /// <exception cref="ArgumentOutOfRangeException">Thrown when resolution is &lt;= 0.</exception>
        public MeshData BuildMesh(int resolution, float size = 1f)
        {
            if (resolution <= 0)
                throw new ArgumentOutOfRangeException(nameof(resolution), "resolution must be > 0");

            int vertexCount = (resolution + 1) * (resolution + 1);
            int triangleCount = resolution * resolution * 2;
            int indexCount = triangleCount * 3;

            var vertices = new Vector3[vertexCount];
            var indices = new int[indexCount];
            var uvs = new Vector2[vertexCount];
            var normals = new Vector3[vertexCount];

            float cellSize = size / resolution;

            // Generate vertices and UVs
            for (int z = 0; z <= resolution; z++)
            {
                for (int x = 0; x <= resolution; x++)
                {
                    int index = z * (resolution + 1) + x;
                    vertices[index] = new Vector3(x * cellSize, 0f, z * cellSize);
                    uvs[index] = new Vector2((float)x / resolution, (float)z / resolution);
                    normals[index] = new Vector3(0f, 1f, 0f);
                }
            }

            // Generate indices with consistent clockwise winding
            int idx = 0;
            for (int z = 0; z < resolution; z++)
            {
                for (int x = 0; x < resolution; x++)
                {
                    int bottomLeft = z * (resolution + 1) + x;
                    int bottomRight = bottomLeft + 1;
                    int topLeft = (z + 1) * (resolution + 1) + x;
                    int topRight = topLeft + 1;

                    // First triangle (clockwise when viewed from above)
                    indices[idx++] = bottomLeft;
                    indices[idx++] = topLeft;
                    indices[idx++] = topRight;

                    // Second triangle (clockwise when viewed from above)
                    indices[idx++] = bottomLeft;
                    indices[idx++] = topRight;
                    indices[idx++] = bottomRight;
                }
            }

            return new MeshData
            {
                Vertices = vertices,
                Indices = indices,
                UVs = uvs,
                Normals = normals,
            };
        }

        /// <summary>
        /// Builds a mesh for a terrain chunk using the supplied height field.
        /// Y values are sampled from <paramref name="heightField"/>.
        /// </summary>
        /// <param name="heightField">Height field to sample elevation from.</param>
        /// <param name="resolution">Number of grid quads along each axis. Must be &gt; 0.</param>
        /// <param name="size">World-space size of the chunk. Default is 1.</param>
        /// <returns>A <see cref="MeshData"/> containing the generated mesh.</returns>
        /// <exception cref="ArgumentOutOfRangeException">Thrown when resolution is &lt;= 0.</exception>
        public MeshData BuildMesh(HeightField heightField, int resolution, float size = 1f)
        {
            if (resolution <= 0)
                throw new ArgumentOutOfRangeException(nameof(resolution), "resolution must be > 0");

            // For now, HeightField is a stub with no data; fall back to flat mesh.
            return BuildMesh(resolution, size);
        }
    }
}
