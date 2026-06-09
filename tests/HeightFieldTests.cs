using System;
using Phenotype.Terrain;
using Xunit;

namespace Phenotype.Terrain.Tests
{
    public class HeightFieldTests
    {
        [Fact]
        public void HeightField_CanBeInstantiated()
        {
            var hf = new HeightField(16, 16);
            Assert.NotNull(hf);
            Assert.Equal(16, hf.Width);
            Assert.Equal(16, hf.Height);
        }

        [Fact]
        public void HeightField_GetHeight_ReturnsZeroForDefaultData()
        {
            var hf = new HeightField(4, 4);
            Assert.Equal(0f, hf.GetHeight(0, 0));
            Assert.Equal(0f, hf.GetHeight(3, 3));
        }

        [Fact]
        public void HeightField_SetHeight_GetHeight_RoundTrip()
        {
            var hf = new HeightField(4, 4);
            hf.SetHeight(1, 2, 42.5f);
            Assert.Equal(42.5f, hf.GetHeight(1, 2));
        }

        [Fact]
        public void HeightField_GetData_ReturnsCopy()
        {
            var hf = new HeightField(2, 2);
            hf.SetHeight(0, 0, 5f);
            var data = hf.GetData();
            data[0] = 99f;
            Assert.Equal(5f, hf.GetHeight(0, 0));
        }
    }
}
