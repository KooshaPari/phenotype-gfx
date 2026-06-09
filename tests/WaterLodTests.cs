using System;
using Xunit;
using Phenotype.Water;
using Phenotype.Water.Rendering;
using Phenotype.Terrain;

namespace Phenotype.Water.Tests
{
    public class WaterLodTests
    {
        // ──────────────────────────────────────────────────────────────────────
        // Default thresholds: near<50, mid<150, far<400, culled>=400
        // ──────────────────────────────────────────────────────────────────────

        [Theory]
        [InlineData(0f, LodTier.Near)]
        [InlineData(25f, LodTier.Near)]
        [InlineData(49.9f, LodTier.Near)]
        [InlineData(50f, LodTier.Mid)]
        [InlineData(100f, LodTier.Mid)]
        [InlineData(149.9f, LodTier.Mid)]
        [InlineData(150f, LodTier.Far)]
        [InlineData(300f, LodTier.Far)]
        [InlineData(399.9f, LodTier.Far)]
        [InlineData(400f, LodTier.Culled)]
        [InlineData(999f, LodTier.Culled)]
        public void SelectTier_ReturnsCorrectTier(float distance, LodTier expected)
        {
            var lod = new WaterLod();
            Assert.Equal(expected, lod.SelectTier(distance));
        }

        // ──────────────────────────────────────────────────────────────────────
        // Resolution matches tier
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void NearDistance_ReturnsNearResolution()
        {
            var lod = new WaterLod();
            Assert.Equal(lod.NearResolution, lod.SelectResolution(0f));
            Assert.Equal(lod.NearResolution, lod.SelectResolution(25f));
        }

        [Fact]
        public void MidDistance_ReturnsMidResolution()
        {
            var lod = new WaterLod();
            Assert.Equal(lod.MidResolution, lod.SelectResolution(75f));
        }

        [Fact]
        public void FarDistance_ReturnsFarResolution()
        {
            var lod = new WaterLod();
            Assert.Equal(lod.FarResolution, lod.SelectResolution(200f));
        }

        [Fact]
        public void BeyondCull_ReturnsZero()
        {
            var lod = new WaterLod();
            Assert.Equal(0, lod.SelectResolution(500f));
        }

        // ──────────────────────────────────────────────────────────────────────
        // Default resolutions are monotonically decreasing (near > mid > far)
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void DefaultResolutions_AreMonotonicallyDecreasing()
        {
            var lod = new WaterLod();
            Assert.True(lod.NearResolution > lod.MidResolution,
                $"Near({lod.NearResolution}) should be > Mid({lod.MidResolution})");
            Assert.True(lod.MidResolution > lod.FarResolution,
                $"Mid({lod.MidResolution}) should be > Far({lod.FarResolution})");
            Assert.True(lod.FarResolution > 0,
                "Far resolution should be > 0");
        }

        // ──────────────────────────────────────────────────────────────────────
        // Monotonic: farther distance → same-or-coarser resolution
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void Monotonic_FartherDistanceNeverFinerResolution()
        {
            var lod = new WaterLod();
            float[] distances = { 0f, 10f, 49f, 50f, 100f, 149f, 150f, 300f, 399f, 400f, 800f };

            int prev = int.MaxValue;
            foreach (float d in distances)
            {
                int res = lod.SelectResolution(d);
                Assert.True(res <= prev,
                    $"At distance {d} resolution {res} is finer than previous {prev} — not monotonic");
                prev = res;
            }
        }

        // ──────────────────────────────────────────────────────────────────────
        // Configurable thresholds
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void CustomThresholds_RespectedBySelectTier()
        {
            var lod = new WaterLod
            {
                NearDistance = 10f,
                MidDistance = 30f,
                CullDistance = 60f,
            };

            Assert.Equal(LodTier.Near, lod.SelectTier(5f));
            Assert.Equal(LodTier.Mid, lod.SelectTier(15f));
            Assert.Equal(LodTier.Far, lod.SelectTier(40f));
            Assert.Equal(LodTier.Culled, lod.SelectTier(60f));
        }

        [Fact]
        public void CustomResolutions_RespectedBySelectResolution()
        {
            var lod = new WaterLod
            {
                NearResolution = 128,
                MidResolution = 64,
                FarResolution = 32,
            };

            Assert.Equal(128, lod.SelectResolution(0f));
            Assert.Equal(64, lod.SelectResolution(75f));
            Assert.Equal(32, lod.SelectResolution(200f));
            Assert.Equal(0, lod.SelectResolution(500f));
        }

        // ──────────────────────────────────────────────────────────────────────
        // Cull beyond max distance
        // ──────────────────────────────────────────────────────────────────────

        [Theory]
        [InlineData(400f)]
        [InlineData(1000f)]
        [InlineData(float.MaxValue)]
        public void BeyondCullDistance_AlwaysCulled(float distance)
        {
            var lod = new WaterLod();
            Assert.Equal(LodTier.Culled, lod.SelectTier(distance));
            Assert.Equal(0, lod.SelectResolution(distance));
        }

        // ──────────────────────────────────────────────────────────────────────
        // Threshold validation
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void ValidThresholds_DoesNotThrow()
        {
            var lod = new WaterLod(); // defaults are valid
            lod.ValidateThresholds(); // must not throw
        }

        [Fact]
        public void InvalidThresholds_NearGeMid_Throws()
        {
            var lod = new WaterLod { NearDistance = 200f, MidDistance = 100f };
            Assert.Throws<InvalidOperationException>(() => lod.ValidateThresholds());
        }

        [Fact]
        public void InvalidThresholds_MidGeCull_Throws()
        {
            var lod = new WaterLod { MidDistance = 500f, CullDistance = 300f };
            Assert.Throws<InvalidOperationException>(() => lod.ValidateThresholds());
        }

        // ──────────────────────────────────────────────────────────────────────
        // Guard: negative distance
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void NegativeDistance_Throws()
        {
            var lod = new WaterLod();
            Assert.Throws<ArgumentOutOfRangeException>(() => lod.SelectTier(-1f));
            Assert.Throws<ArgumentOutOfRangeException>(() => lod.SelectResolution(-0.001f));
        }
    }
}
