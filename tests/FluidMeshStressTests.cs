using System;
using System.Diagnostics;
using UnityEngine;
using Xunit;
using Phenotype.Water;
using Phenotype.Water.Rendering;

namespace Phenotype.Water.Tests
{
    public class FluidMeshStressTests
    {
        private const float Tolerance = 1e-4f;

        // ──────────────────────────────────────────────────────────────────────
        // 1. High vertex count meshes (10000+ vertices)
        // ──────────────────────────────────────────────────────────────────────

        [Theory]
        [InlineData(100)]   // 101*101 = 10 201 vertices
        [InlineData(150)]   // 151*151 = 22 801 vertices
        [InlineData(200)]   // 201*201 = 40 401 vertices
        public void HighVertexCount_MeshBuildsSuccessfully(int resolution)
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            var mesh = FluidMesh.Build(bank, resolution, 100f, 1.5f);

            int expectedVerts = (resolution + 1) * (resolution + 1);
            int expectedIndices = resolution * resolution * 6;

            Assert.Equal(expectedVerts, mesh.Vertices.Length);
            Assert.Equal(expectedVerts, mesh.Normals.Length);
            Assert.Equal(expectedVerts, mesh.UVs.Length);
            Assert.Equal(expectedIndices, mesh.Indices.Length);

            // Ensure no degenerate triangles
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
        // 2. Mesh regeneration performance
        // ──────────────────────────────────────────────────────────────────────

        [Theory]
        [InlineData(100)]
        [InlineData(150)]
        public void MeshRegeneration_PerformanceWithinBudget(int resolution)
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            const int iterations = 20;
            var sw = Stopwatch.StartNew();

            for (int i = 0; i < iterations; i++)
            {
                float t = i * 0.1f;
                var mesh = FluidMesh.Build(bank, resolution, 100f, t);
                // Force materialization to prevent the compiler from optimizing away
                Assert.True(mesh.Vertices.Length > 0);
            }

            sw.Stop();
            double avgMs = sw.Elapsed.TotalMilliseconds / iterations;

            // Each iteration should average under 500 ms even for large meshes
            Assert.True(avgMs < 500.0,
                $"Average mesh generation time ({avgMs:F2} ms) exceeded budget for resolution {resolution}");
        }

        // ──────────────────────────────────────────────────────────────────────
        // 3. Memory usage for large meshes
        // ──────────────────────────────────────────────────────────────────────

        [Theory]
        [InlineData(100)]
        [InlineData(150)]
        [InlineData(200)]
        public void LargeMesh_MemoryUsageIsPredictable(int resolution)
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();

            GC.Collect();
            GC.WaitForPendingFinalizers();
            GC.Collect();
            long before = GC.GetTotalMemory(true);

            var mesh = FluidMesh.Build(bank, resolution, 100f, 2f);

            // Materialize to avoid compiler optimization
            Assert.Equal((resolution + 1) * (resolution + 1), mesh.Vertices.Length);

            long after = GC.GetTotalMemory(true);
            long delta = after - before;

            // Expected allocation: ~((resolution+1)^2 * (3*4 + 3*4 + 2*4)) + (resolution^2 * 6 * 4)
            int vertCount = (resolution + 1) * (resolution + 1);
            int idxCount = resolution * resolution * 6;
            long expectedBytes = (long)vertCount * (12 + 12 + 8) + (long)idxCount * 4;

            // Allow 2x overhead for GC / array object headers
            Assert.True(delta <= expectedBytes * 2,
                $"Memory delta ({delta} bytes) exceeded 2x expected ({expectedBytes * 2} bytes) for resolution {resolution}");
            Assert.True(delta >= expectedBytes / 2,
                $"Memory delta ({delta} bytes) was unexpectedly low (< 0.5x expected) for resolution {resolution}");
        }

        // ──────────────────────────────────────────────────────────────────────
        // 4. Boundary conditions for fluid simulation
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void BoundaryConditions_MinimumResolution_ProducesValidMesh()
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            var mesh = FluidMesh.Build(bank, 1, 10f, 0f);

            Assert.Equal(4, mesh.Vertices.Length);   // (1+1)^2
            Assert.Equal(6, mesh.Indices.Length);    // 1*1*6

            // All indices in range
            foreach (int idx in mesh.Indices)
                Assert.InRange(idx, 0, mesh.Vertices.Length - 1);

            // Normals still unit length
            foreach (var n in mesh.Normals)
                Assert.InRange(n.magnitude, 1f - 1e-4f, 1f + 1e-4f);
        }

        [Fact]
        public void BoundaryConditions_LargeSize_ProducesValidMesh()
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            var mesh = FluidMesh.Build(bank, 8, 1000f, 5f);

            // Ensure vertices span the expected range
            float half = 500f;
            foreach (var v in mesh.Vertices)
            {
                Assert.True(Math.Abs(v.x) <= half + 10f,
                    "Vertex X coordinate out of expected range");
                Assert.True(Math.Abs(v.z) <= half + 10f,
                    "Vertex Z coordinate out of expected range");
            }
        }

        [Fact]
        public void BoundaryConditions_ZeroTime_ProducesDeterministicMesh()
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            var mesh1 = FluidMesh.Build(bank, 16, 50f, 0f);
            var mesh2 = FluidMesh.Build(bank, 16, 50f, 0f);

            Assert.Equal(mesh1.Vertices.Length, mesh2.Vertices.Length);
            for (int i = 0; i < mesh1.Vertices.Length; i++)
            {
                Assert.Equal(mesh1.Vertices[i].x, mesh2.Vertices[i].x);
                Assert.Equal(mesh1.Vertices[i].y, mesh2.Vertices[i].y);
                Assert.Equal(mesh1.Vertices[i].z, mesh2.Vertices[i].z);
            }
        }

        [Fact]
        public void BoundaryConditions_EmptyBank_FlatGridAtAnyResolution()
        {
            var bank = new GerstnerWaveBank();
            var mesh = FluidMesh.Build(bank, 32, 20f, 99f);

            foreach (var v in mesh.Vertices)
                Assert.InRange(v.y, -Tolerance, Tolerance);
        }

        // ──────────────────────────────────────────────────────────────────────
        // 5. Stress test with 100 iterations of mesh update
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void StressTest_100Iterations_MeshUpdateStaysConsistent()
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            const int resolution = 32;
            const float size = 50f;
            const int iterations = 100;

            var firstMesh = FluidMesh.Build(bank, resolution, size, 0f);
            var lastMesh = FluidMesh.Build(bank, resolution, size, (iterations - 1) * 0.05f);

            // All iterations must produce meshes of the same dimensions
            Assert.Equal(firstMesh.Vertices.Length, lastMesh.Vertices.Length);
            Assert.Equal(firstMesh.Normals.Length, lastMesh.Normals.Length);
            Assert.Equal(firstMesh.UVs.Length, lastMesh.UVs.Length);
            Assert.Equal(firstMesh.Indices.Length, lastMesh.Indices.Length);

            // Run 100 iterations and assert no exceptions
            for (int i = 0; i < iterations; i++)
            {
                float t = i * 0.05f;
                var mesh = FluidMesh.Build(bank, resolution, size, t);

                Assert.Equal((resolution + 1) * (resolution + 1), mesh.Vertices.Length);
                Assert.Equal(resolution * resolution * 6, mesh.Indices.Length);

                // Validate every normal is unit length
                foreach (var n in mesh.Normals)
                {
                    float mag = n.magnitude;
                    Assert.InRange(mag, 1f - 1e-4f, 1f + 1e-4f);
                }

                // Validate UVs in [0,1]
                foreach (var uv in mesh.UVs)
                {
                    Assert.InRange(uv.x, 0f, 1f);
                    Assert.InRange(uv.y, 0f, 1f);
                }

                // Validate indices in range
                foreach (int idx in mesh.Indices)
                    Assert.InRange(idx, 0, mesh.Vertices.Length - 1);
            }
        }

        [Fact]
        public void StressTest_100Iterations_PerformanceIsStable()
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            const int resolution = 32;
            const float size = 50f;
            const int iterations = 100;

            var sw = Stopwatch.StartNew();

            for (int i = 0; i < iterations; i++)
            {
                float t = i * 0.05f;
                var mesh = FluidMesh.Build(bank, resolution, size, t);
                Assert.True(mesh.Vertices.Length > 0);
            }

            sw.Stop();
            double avgMs = sw.Elapsed.TotalMilliseconds / iterations;

            // Should average well under 50 ms per iteration at resolution 32
            Assert.True(avgMs < 50.0,
                $"Average iteration time ({avgMs:F2} ms) exceeded stable budget");
        }
    }
}
