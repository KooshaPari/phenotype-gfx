using System;
using UnityEngine;
using Xunit;
using Phenotype.Water;
using Phenotype.Water.Rendering;

namespace Phenotype.Water.Tests
{
    public class WaterRendererTests
    {
        [Fact]
        public void BuildMesh_WithoutWaveBank_Throws()
        {
            var renderer = new WaterRenderer();
            Assert.Throws<InvalidOperationException>(() => renderer.BuildMesh(0f, 0f));
        }

        [Fact]
        public void BuildMesh_WithWaveBank_ReturnsValidMesh()
        {
            var renderer = new WaterRenderer
            {
                WaveBank = GerstnerWaveBank.CreateOceanPreset(),
                PatchSize = 50f,
            };
            var mesh = renderer.BuildMesh(1f, 0f);
            Assert.True(mesh.Vertices.Length > 0);
        }

        [Fact]
        public void BuildMesh_CulledDistance_ReturnsDefaultMesh()
        {
            var renderer = new WaterRenderer
            {
                WaveBank = GerstnerWaveBank.CreateOceanPreset(),
            };
            var mesh = renderer.BuildMesh(0f, 500f);
            Assert.Empty(mesh.Vertices);
        }

        [Fact]
        public void Lod_IsNotNull()
        {
            var renderer = new WaterRenderer();
            Assert.NotNull(renderer.Lod);
        }

        [Fact]
        public void BuildMesh_ResolutionMatchesLod()
        {
            var renderer = new WaterRenderer
            {
                WaveBank = GerstnerWaveBank.CreateOceanPreset(),
                PatchSize = 20f,
            };
            var mesh = renderer.BuildMesh(0f, 0f);
            int expectedResolution = renderer.Lod.NearResolution;
            int expectedVerts = (expectedResolution + 1) * (expectedResolution + 1);
            Assert.Equal(expectedVerts, mesh.Vertices.Length);
        }
    }
}
