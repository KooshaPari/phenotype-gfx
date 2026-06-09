using System;
using Phenotype.Terrain;
using UnityEngine;
using Xunit;

namespace Phenotype.Terrain.Tests
{
    public class ChunkMeshBuilderTests
    {
        [Fact]
        public void BuildMesh_SimpleQuad_ProducesCorrectCounts()
        {
            var builder = new ChunkMeshBuilder();
            var mesh = builder.BuildMesh(resolution: 1, size: 1f);

            // 1x1 grid => (1+1)*(1+1) = 4 vertices
            Assert.Equal(4, mesh.Vertices.Length);
            Assert.Equal(4, mesh.UVs.Length);
            Assert.Equal(4, mesh.Normals.Length);

            // 1x1 grid => 2 triangles => 6 indices
            Assert.Equal(6, mesh.Indices.Length);
        }

        [Fact]
        public void BuildMesh_SimpleQuad_VerticesAreAtExpectedPositions()
        {
            var builder = new ChunkMeshBuilder();
            var mesh = builder.BuildMesh(resolution: 1, size: 2f);

            // With size=2 and resolution=1, cell size is 2
            // Expected vertices: (0,0,0), (2,0,0), (0,0,2), (2,0,2)
            AssertVertex(mesh.Vertices[0], 0f, 0f, 0f);
            AssertVertex(mesh.Vertices[1], 2f, 0f, 0f);
            AssertVertex(mesh.Vertices[2], 0f, 0f, 2f);
            AssertVertex(mesh.Vertices[3], 2f, 0f, 2f);
        }

        [Fact]
        public void BuildMesh_SimpleQuad_IndicesFormTwoTriangles()
        {
            var builder = new ChunkMeshBuilder();
            var mesh = builder.BuildMesh(resolution: 1, size: 1f);

            // First triangle: bottom-left, top-left, top-right
            Assert.Equal(0, mesh.Indices[0]);
            Assert.Equal(2, mesh.Indices[1]);
            Assert.Equal(3, mesh.Indices[2]);

            // Second triangle: bottom-left, top-right, bottom-right
            Assert.Equal(0, mesh.Indices[3]);
            Assert.Equal(3, mesh.Indices[4]);
            Assert.Equal(1, mesh.Indices[5]);
        }

        [Fact]
        public void BuildMesh_TriangleWindingOrder_IsConsistentAndUpward()
        {
            var builder = new ChunkMeshBuilder();
            var mesh = builder.BuildMesh(resolution: 4, size: 4f);

            int triangleCount = mesh.Indices.Length / 3;
            for (int t = 0; t < triangleCount; t++)
            {
                int i0 = mesh.Indices[t * 3 + 0];
                int i1 = mesh.Indices[t * 3 + 1];
                int i2 = mesh.Indices[t * 3 + 2];

                Vector3 v0 = mesh.Vertices[i0];
                Vector3 v1 = mesh.Vertices[i1];
                Vector3 v2 = mesh.Vertices[i2];

                // Compute face normal via cross product
                Vector3 edge1 = v1 - v0;
                Vector3 edge2 = v2 - v0;
                Vector3 normal = Cross(edge1, edge2);

                // For a flat mesh, all triangles should face upward (+Y)
                Assert.True(normal.y > 0,
                    $"Triangle {t} has downward-facing normal ({normal.y}).");
            }
        }

        [Fact]
        public void BuildMesh_VertexCount_NearResolution_MatchesTerrainLod()
        {
            var builder = new ChunkMeshBuilder();
            var mesh = builder.BuildMesh(resolution: 64, size: 64f);

            // (64+1)^2 = 4225 vertices
            Assert.Equal(4225, mesh.Vertices.Length);
        }

        [Fact]
        public void BuildMesh_VertexCount_MidResolution_MatchesTerrainLod()
        {
            var builder = new ChunkMeshBuilder();
            var mesh = builder.BuildMesh(resolution: 32, size: 64f);

            // (32+1)^2 = 1089 vertices
            Assert.Equal(1089, mesh.Vertices.Length);
        }

        [Fact]
        public void BuildMesh_VertexCount_FarResolution_MatchesTerrainLod()
        {
            var builder = new ChunkMeshBuilder();
            var mesh = builder.BuildMesh(resolution: 16, size: 64f);

            // (16+1)^2 = 289 vertices
            Assert.Equal(289, mesh.Vertices.Length);
        }

        [Fact]
        public void BuildMesh_IndexBuffer_LengthIsCorrect()
        {
            var builder = new ChunkMeshBuilder();
            var mesh = builder.BuildMesh(resolution: 4, size: 4f);

            // 4x4 grid => 16 quads => 32 triangles => 96 indices
            Assert.Equal(96, mesh.Indices.Length);
        }

        [Fact]
        public void BuildMesh_IndexBuffer_AllIndicesInValidRange()
        {
            var builder = new ChunkMeshBuilder();
            var mesh = builder.BuildMesh(resolution: 8, size: 8f);

            int maxIndex = mesh.Vertices.Length - 1;
            foreach (int index in mesh.Indices)
            {
                Assert.True(index >= 0, "Index must be non-negative.");
                Assert.True(index <= maxIndex, "Index must be within vertex array bounds.");
            }
        }

        [Fact]
        public void BuildMesh_IndexBuffer_NoDegenerateTriangles()
        {
            var builder = new ChunkMeshBuilder();
            var mesh = builder.BuildMesh(resolution: 4, size: 4f);

            int triangleCount = mesh.Indices.Length / 3;
            for (int t = 0; t < triangleCount; t++)
            {
                int i0 = mesh.Indices[t * 3 + 0];
                int i1 = mesh.Indices[t * 3 + 1];
                int i2 = mesh.Indices[t * 3 + 2];

                Assert.NotEqual(i0, i1);
                Assert.NotEqual(i1, i2);
                Assert.NotEqual(i2, i0);
            }
        }

        [Fact]
        public void BuildMesh_Normals_AreAllUpward()
        {
            var builder = new ChunkMeshBuilder();
            var mesh = builder.BuildMesh(resolution: 4, size: 4f);

            foreach (var normal in mesh.Normals)
            {
                Assert.Equal(0f, normal.x);
                Assert.Equal(1f, normal.y);
                Assert.Equal(0f, normal.z);
            }
        }

        [Fact]
        public void BuildMesh_UVs_CoverFullRange()
        {
            var builder = new ChunkMeshBuilder();
            var mesh = builder.BuildMesh(resolution: 2, size: 2f);

            // Check that UVs range from 0 to 1
            foreach (var uv in mesh.UVs)
            {
                Assert.True(uv.x >= 0f && uv.x <= 1f, "UV.x must be in [0,1].");
                Assert.True(uv.y >= 0f && uv.y <= 1f, "UV.y must be in [0,1].");
            }

            // Corners should be exactly 0 or 1
            Assert.Equal(0f, mesh.UVs[0].x);
            Assert.Equal(0f, mesh.UVs[0].y);

            int last = mesh.UVs.Length - 1;
            Assert.Equal(1f, mesh.UVs[last].x);
            Assert.Equal(1f, mesh.UVs[last].y);
        }

        [Fact]
        public void BuildMesh_ZeroResolution_ThrowsArgumentOutOfRangeException()
        {
            var builder = new ChunkMeshBuilder();
            Assert.Throws<ArgumentOutOfRangeException>(() => builder.BuildMesh(resolution: 0));
        }

        [Fact]
        public void BuildMesh_NegativeResolution_ThrowsArgumentOutOfRangeException()
        {
            var builder = new ChunkMeshBuilder();
            Assert.Throws<ArgumentOutOfRangeException>(() => builder.BuildMesh(resolution: -1));
        }

        private static void AssertVertex(Vector3 actual, float x, float y, float z)
        {
            Assert.Equal(x, actual.x);
            Assert.Equal(y, actual.y);
            Assert.Equal(z, actual.z);
        }

        private static Vector3 Cross(Vector3 a, Vector3 b)
        {
            return new Vector3(
                a.y * b.z - a.z * b.y,
                a.z * b.x - a.x * b.z,
                a.x * b.y - a.y * b.x);
        }
    }
}
