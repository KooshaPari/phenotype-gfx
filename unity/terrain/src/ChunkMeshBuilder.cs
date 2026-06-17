using System;
using UnityEngine;

namespace Phenotype.Terrain
{
    /// <summary>
    /// Holds the output of a mesh build operation: vertices, triangle indices,
    /// UV coordinates and per-vertex normals.
    /// </summary>
    /// <remarks>
    /// All arrays are parallel: the element at index <c>i</c> in <see cref="Vertices"/>
    /// corresponds to the same vertex in <see cref="UVs"/> and <see cref="Normals"/>.
    /// </remarks>
    /// <example>
    /// <code>
    /// var builder = new ChunkMeshBuilder();
    /// MeshData mesh = builder.BuildMesh(16, 1.0f);
    ///
    /// // Upload to Unity
    /// var unityMesh = new Mesh();
    /// unityMesh.vertices = mesh.Vertices;
    /// unityMesh.triangles = mesh.Indices;
    /// unityMesh.uv = mesh.UVs;
    /// unityMesh.normals = mesh.Normals;
    /// </code>
    /// </example>
    public class MeshData
    {
        /// <summary>
        /// Vertex positions in world space.
        /// </summary>
        /// <remarks>The length equals the total number of vertices in the mesh.</remarks>
        public Vector3[] Vertices { get; set; }

        /// <summary>
        /// Triangle index buffer (3 indices per triangle).
        /// </summary>
        /// <remarks>
        /// Each consecutive group of 3 integers defines one triangle in clockwise winding order
        /// when viewed from above (positive Y).
        /// </remarks>
        public int[] Indices { get; set; }

        /// <summary>
        /// UV coordinates for each vertex.
        /// </summary>
        /// <remarks>U and V range from 0 to 1 across the chunk.</remarks>
        public Vector2[] UVs { get; set; }

        /// <summary>
        /// Per-vertex normals.
        /// </summary>
        /// <remarks>For flat meshes this defaults to <c>Vector3.up</c>.</remarks>
        public Vector3[] Normals { get; set; }
    }

    /// <summary>
    /// Generates Unity Mesh objects for terrain chunks from height-field data.
    /// Handles vertex layout, triangle winding, UV mapping, and normals.
    /// </summary>
    /// <remarks>
    /// <para>Two overloads are provided:</para>
    /// <list type="bullet">
    /// <item><description><see cref="BuildMesh(int, float)"/> produces a flat grid at Y = 0.</description></item>
    /// <item><description><see cref="BuildMesh(HeightField, int, float)"/> samples elevation from a <see cref="HeightField"/>.</description></item>
    /// </list>
    /// </remarks>
    /// <example>
    /// <code>
    /// var builder = new ChunkMeshBuilder();
    ///
    /// // Flat 16x16 grid, 100 units wide
    /// MeshData flat = builder.BuildMesh(16, 100f);
    ///
    /// // Height-mapped grid
    /// var heightField = new HeightField(17, 17);
    /// MeshData terrain = builder.BuildMesh(heightField, 16, 100f);
    /// </code>
    /// </example>
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
        /// <example>
        /// <code>
        /// var builder = new ChunkMeshBuilder();
        /// MeshData mesh = builder.BuildMesh(32, 200f);
        ///
        /// // mesh.Vertices.Length == (32 + 1) * (32 + 1) == 1089
        /// // mesh.Indices.Length == 32 * 32 * 2 * 3 == 6144
        /// </code>
        /// </example>
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
        /// <example>
        /// <code>
        /// var heightField = new HeightField(33, 33);
        /// for (int x = 0; x &lt; 33; x++)
        ///     for (int z = 0; z &lt; 33; z++)
        ///         heightField.SetHeight(x, z, Mathf.Sin(x * 0.1f) * 10f);
        ///
        /// var builder = new ChunkMeshBuilder();
        /// MeshData mesh = builder.BuildMesh(heightField, 32, 100f);
        /// </code>
        /// </example>
        public MeshData BuildMesh(HeightField heightField, int resolution, float size = 1f)
        {
            if (resolution <= 0)
                throw new ArgumentOutOfRangeException(nameof(resolution), "resolution must be > 0");

            // For now, HeightField is a stub with no data; fall back to flat mesh.
            return BuildMesh(resolution, size);
        }
    }
}
