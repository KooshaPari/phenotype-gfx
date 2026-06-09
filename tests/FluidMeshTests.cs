using System;
using UnityEngine;
using Xunit;
using Phenotype.Water;

namespace Phenotype.Water.Tests
{
    public class FluidMeshTests
    {
        private const float Tolerance = 1e-4f;

        // ──────────────────────────────────────────────────────────────────────
        // Vertex / index count
        // ──────────────────────────────────────────────────────────────────────

        [Theory]
        [InlineData(1)]
        [InlineData(4)]
        [InlineData(8)]
        [InlineData(16)]
        public void VertexCount_MatchesResolution(int res)
        {
            var bank = new GerstnerWaveBank();
            var mesh = FluidMesh.Build(bank, res, 10f, 0f);

            int expected = (res + 1) * (res + 1);
            Assert.Equal(expected, mesh.Vertices.Length);
            Assert.Equal(expected, mesh.Normals.Length);
            Assert.Equal(expected, mesh.UVs.Length);
        }

        [Theory]
        [InlineData(1)]
        [InlineData(4)]
        [InlineData(8)]
        [InlineData(16)]
        public void IndexCount_MatchesResolution(int res)
        {
            var bank = new GerstnerWaveBank();
            var mesh = FluidMesh.Build(bank, res, 10f, 0f);

            int expected = res * res * 6;
            Assert.Equal(expected, mesh.Indices.Length);
        }

        // ──────────────────────────────────────────────────────────────────────
        // No degenerate triangles (watertight)
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void NoDegenerateTriangles_EmptyBank()
        {
            var bank = new GerstnerWaveBank();
            var mesh = FluidMesh.Build(bank, 8, 20f, 0f);

            for (int i = 0; i < mesh.Indices.Length; i += 3)
            {
                int a = mesh.Indices[i];
                int b = mesh.Indices[i + 1];
                int c = mesh.Indices[i + 2];

                // All indices must be distinct
                Assert.NotEqual(a, b);
                Assert.NotEqual(b, c);
                Assert.NotEqual(a, c);

                // All indices in range
                Assert.InRange(a, 0, mesh.Vertices.Length - 1);
                Assert.InRange(b, 0, mesh.Vertices.Length - 1);
                Assert.InRange(c, 0, mesh.Vertices.Length - 1);

                // Non-zero area: cross product of two edges must not be near-zero
                var v0 = mesh.Vertices[a];
                var v1 = mesh.Vertices[b];
                var v2 = mesh.Vertices[c];
                var cross = Vector3.Cross(v1 - v0, v2 - v0);
                Assert.True(cross.magnitude > 1e-8f,
                    $"Degenerate tri at index {i}: area ~ 0");
            }
        }

        [Fact]
        public void NoDegenerateTriangles_OceanPreset()
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            var mesh = FluidMesh.Build(bank, 8, 20f, 1.5f);

            for (int i = 0; i < mesh.Indices.Length; i += 3)
            {
                int a = mesh.Indices[i];
                int b = mesh.Indices[i + 1];
                int c = mesh.Indices[i + 2];

                Assert.NotEqual(a, b);
                Assert.NotEqual(b, c);
                Assert.NotEqual(a, c);
            }
        }

        // ──────────────────────────────────────────────────────────────────────
        // Normals are unit length
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void AllNormals_AreUnitLength()
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            var mesh = FluidMesh.Build(bank, 8, 50f, 3.3f);

            foreach (var n in mesh.Normals)
            {
                float mag = n.magnitude;
                Assert.InRange(mag, 1f - 1e-4f, 1f + 1e-4f);
            }
        }

        [Fact]
        public void AllNormals_AreUnitLength_EmptyBank()
        {
            var bank = new GerstnerWaveBank();
            var mesh = FluidMesh.Build(bank, 4, 10f, 0f);

            foreach (var n in mesh.Normals)
            {
                float mag = n.magnitude;
                Assert.InRange(mag, 1f - Tolerance, 1f + Tolerance);
            }
        }

        // ──────────────────────────────────────────────────────────────────────
        // Displacement matches wave bank at sample points
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void Vertices_DisplacementMatchesWaveBank()
        {
            var bank = GerstnerWaveBank.CreateLakePreset();
            const int res = 4;
            const float size = 8f;
            const float time = 2.1f;

            var mesh = FluidMesh.Build(bank, res, size, time);

            float step = size / res;
            float half = size * 0.5f;

            for (int row = 0; row <= res; row++)
            {
                for (int col = 0; col <= res; col++)
                {
                    float x = col * step - half;
                    float z = row * step - half;
                    var xz = new Vector2(x, z);
                    var disp = bank.SampleDisplacement(xz, time);

                    int idx = row * (res + 1) + col;
                    var v = mesh.Vertices[idx];

                    Assert.InRange(v.x, x + disp.x - Tolerance, x + disp.x + Tolerance);
                    Assert.InRange(v.y, disp.y - Tolerance, disp.y + Tolerance);
                    Assert.InRange(v.z, z + disp.z - Tolerance, z + disp.z + Tolerance);
                }
            }
        }

        [Fact]
        public void Normals_MatchWaveBank()
        {
            var bank = GerstnerWaveBank.CreateLakePreset();
            const int res = 4;
            const float size = 8f;
            const float time = 2.1f;

            var mesh = FluidMesh.Build(bank, res, size, time);

            float step = size / res;
            float half = size * 0.5f;

            for (int row = 0; row <= res; row++)
            {
                for (int col = 0; col <= res; col++)
                {
                    float x = col * step - half;
                    float z = row * step - half;
                    var xz = new Vector2(x, z);
                    var expN = bank.SampleNormal(xz, time);

                    int idx = row * (res + 1) + col;
                    var n = mesh.Normals[idx];

                    Assert.InRange(n.x, expN.x - Tolerance, expN.x + Tolerance);
                    Assert.InRange(n.y, expN.y - Tolerance, expN.y + Tolerance);
                    Assert.InRange(n.z, expN.z - Tolerance, expN.z + Tolerance);
                }
            }
        }

        // ──────────────────────────────────────────────────────────────────────
        // UVs in [0,1] range
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void UVs_AreInZeroToOneRange()
        {
            var bank = new GerstnerWaveBank();
            var mesh = FluidMesh.Build(bank, 4, 10f, 0f);

            foreach (var uv in mesh.UVs)
            {
                Assert.InRange(uv.x, 0f, 1f);
                Assert.InRange(uv.y, 0f, 1f);
            }
        }

        // ──────────────────────────────────────────────────────────────────────
        // Empty bank → flat grid (Y == 0 everywhere)
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void EmptyBank_FlatGrid_AllYZero()
        {
            var bank = new GerstnerWaveBank();
            var mesh = FluidMesh.Build(bank, 4, 10f, 0f);

            foreach (var v in mesh.Vertices)
                Assert.InRange(v.y, -Tolerance, Tolerance);
        }

        // ──────────────────────────────────────────────────────────────────────
        // Guard argument validation
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void NullBank_Throws()
        {
            Assert.Throws<ArgumentNullException>(() =>
                FluidMesh.Build(null, 4, 10f, 0f));
        }

        [Fact]
        public void ZeroResolution_Throws()
        {
            Assert.Throws<ArgumentOutOfRangeException>(() =>
                FluidMesh.Build(new GerstnerWaveBank(), 0, 10f, 0f));
        }

        [Fact]
        public void NegativeSize_Throws()
        {
            Assert.Throws<ArgumentOutOfRangeException>(() =>
                FluidMesh.Build(new GerstnerWaveBank(), 4, -1f, 0f));
        }
    }
}
