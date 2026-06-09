using System;
using Phenotype.Terrain;
using Xunit;

namespace Phenotype.Terrain.Tests
{
    public class LodBaseTests
    {
        private class TestLodBase : LodBase
        {
            public override float NearDistance { get; } = 10f;
            public override float MidDistance { get; } = 50f;
            public override float CullDistance { get; } = 100f;
        }

        [Fact]
        public void SelectTier_Near_ReturnsNear()
        {
            var lod = new TestLodBase();
            Assert.Equal(LodTier.Near, lod.SelectTier(5f));
            Assert.Equal(LodTier.Near, lod.SelectTier(0f));
        }

        [Fact]
        public void SelectTier_Mid_ReturnsMid()
        {
            var lod = new TestLodBase();
            Assert.Equal(LodTier.Mid, lod.SelectTier(25f));
            Assert.Equal(LodTier.Mid, lod.SelectTier(10f));
        }

        [Fact]
        public void SelectTier_Far_ReturnsFar()
        {
            var lod = new TestLodBase();
            Assert.Equal(LodTier.Far, lod.SelectTier(75f));
            Assert.Equal(LodTier.Far, lod.SelectTier(50f));
        }

        [Fact]
        public void SelectTier_Culled_ReturnsCulled()
        {
            var lod = new TestLodBase();
            Assert.Equal(LodTier.Culled, lod.SelectTier(100f));
            Assert.Equal(LodTier.Culled, lod.SelectTier(150f));
        }

        [Fact]
        public void SelectTier_Negative_ThrowsArgumentOutOfRangeException()
        {
            var lod = new TestLodBase();
            Assert.Throws<ArgumentOutOfRangeException>(() => lod.SelectTier(-1f));
        }

        [Fact]
        public void ValidateThresholds_Valid_DoesNotThrow()
        {
            var lod = new TestLodBase();
            lod.ValidateThresholds();
        }

        [Fact]
        public void ValidateThresholds_InvalidNearMid_ThrowsInvalidOperationException()
        {
            var lod = new InvalidNearMidLodBase();
            Assert.Throws<InvalidOperationException>(() => lod.ValidateThresholds());
        }

        [Fact]
        public void ValidateThresholds_InvalidMidCull_ThrowsInvalidOperationException()
        {
            var lod = new InvalidMidCullLodBase();
            Assert.Throws<InvalidOperationException>(() => lod.ValidateThresholds());
        }

        private class InvalidNearMidLodBase : LodBase
        {
            public override float NearDistance { get; } = 50f;
            public override float MidDistance { get; } = 50f;
            public override float CullDistance { get; } = 100f;
        }

        private class InvalidMidCullLodBase : LodBase
        {
            public override float NearDistance { get; } = 10f;
            public override float MidDistance { get; } = 100f;
            public override float CullDistance { get; } = 100f;
        }
    }
}
