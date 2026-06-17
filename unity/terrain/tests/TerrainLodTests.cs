using Phenotype.Terrain;
using Xunit;

namespace Phenotype.Terrain.Tests
{
    public class TerrainLodTests
    {
        [Fact]
        public void TerrainLod_DefaultThresholds_AreExpected()
        {
            var lod = new TerrainLod();
            Assert.Equal(50f, lod.NearDistance);
            Assert.Equal(150f, lod.MidDistance);
            Assert.Equal(400f, lod.CullDistance);
        }

        [Fact]
        public void TerrainLod_DefaultResolutions_AreExpected()
        {
            var lod = new TerrainLod();
            Assert.Equal(64, lod.NearResolution);
            Assert.Equal(32, lod.MidResolution);
            Assert.Equal(16, lod.FarResolution);
        }

        [Fact]
        public void SelectResolution_NearDistance_ReturnsNearResolution()
        {
            var lod = new TerrainLod();
            Assert.Equal(64, lod.SelectResolution(25f));
        }

        [Fact]
        public void SelectResolution_MidDistance_ReturnsMidResolution()
        {
            var lod = new TerrainLod();
            Assert.Equal(32, lod.SelectResolution(100f));
        }

        [Fact]
        public void SelectResolution_FarDistance_ReturnsFarResolution()
        {
            var lod = new TerrainLod();
            Assert.Equal(16, lod.SelectResolution(250f));
        }

        [Fact]
        public void SelectResolution_Culled_ReturnsZero()
        {
            var lod = new TerrainLod();
            Assert.Equal(0, lod.SelectResolution(500f));
        }

        [Fact]
        public void ValidateThresholds_Default_DoesNotThrow()
        {
            var lod = new TerrainLod();
            lod.ValidateThresholds();
        }

        [Fact]
        public void TerrainLod_IsLodBase()
        {
            var lod = new TerrainLod();
            Assert.IsAssignableFrom<LodBase>(lod);
        }
    }
}
