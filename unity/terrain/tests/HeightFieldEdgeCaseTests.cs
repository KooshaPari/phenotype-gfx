using System;
using Phenotype.Terrain;
using Xunit;

namespace Phenotype.Terrain.Tests
{
    public class HeightFieldEdgeCaseTests
    {
        // 1. Negative heights
        [Fact]
        public void HeightField_NegativeHeight_StoredAndRetrievedCorrectly()
        {
            var hf = new HeightField(4, 4);
            hf.SetHeight(1, 1, -42.5f);
            hf.SetHeight(2, 3, -999.99f);

            Assert.Equal(-42.5f, hf.GetHeight(1, 1));
            Assert.Equal(-999.99f, hf.GetHeight(2, 3));
        }

        [Fact]
        public void HeightField_NegativeHeights_InDataArray()
        {
            float[] data = new float[] { 0f, -1f, -2f, -3f };
            var hf = new HeightField(2, 2, data);

            Assert.Equal(-1f, hf.GetHeight(1, 0));
            Assert.Equal(-2f, hf.GetHeight(0, 1));
            Assert.Equal(-3f, hf.GetHeight(1, 1));
        }

        // 2. Zero-size height field
        [Fact]
        public void HeightField_ZeroWidthZeroHeight_CanBeInstantiated()
        {
            var hf = new HeightField(0, 0);
            Assert.NotNull(hf);
            Assert.Equal(0, hf.Width);
            Assert.Equal(0, hf.Height);
        }

        [Fact]
        public void HeightField_ZeroWidthNonZeroHeight_CanBeInstantiated()
        {
            var hf = new HeightField(0, 5);
            Assert.NotNull(hf);
            Assert.Equal(0, hf.Width);
            Assert.Equal(5, hf.Height);
        }

        [Fact]
        public void HeightField_NonZeroWidthZeroHeight_CanBeInstantiated()
        {
            var hf = new HeightField(5, 0);
            Assert.NotNull(hf);
            Assert.Equal(5, hf.Width);
            Assert.Equal(0, hf.Height);
        }

        [Fact]
        public void HeightField_ZeroSize_GetHeight_Throws()
        {
            var hf = new HeightField(0, 0);
            Assert.Throws<ArgumentOutOfRangeException>(() => hf.GetHeight(0, 0));
        }

        [Fact]
        public void HeightField_ZeroSize_SetHeight_Throws()
        {
            var hf = new HeightField(0, 0);
            Assert.Throws<ArgumentOutOfRangeException>(() => hf.SetHeight(0, 0, 1f));
        }

        // 3. Very large height field (1024x1024)
        [Fact]
        public void HeightField_Large1024x1024_CanBeInstantiated()
        {
            var hf = new HeightField(1024, 1024);
            Assert.NotNull(hf);
            Assert.Equal(1024, hf.Width);
            Assert.Equal(1024, hf.Height);
        }

        [Fact]
        public void HeightField_Large1024x1024_ReadWriteRoundTrip()
        {
            var hf = new HeightField(1024, 1024);
            hf.SetHeight(0, 0, 1f);
            hf.SetHeight(1023, 1023, 2f);
            hf.SetHeight(512, 512, 3f);

            Assert.Equal(1f, hf.GetHeight(0, 0));
            Assert.Equal(2f, hf.GetHeight(1023, 1023));
            Assert.Equal(3f, hf.GetHeight(512, 512));
        }

        [Fact]
        public void HeightField_Large1024x1024_DataArrayLength()
        {
            var hf = new HeightField(1024, 1024);
            var data = hf.GetData();
            Assert.Equal(1024 * 1024, data.Length);
        }

        // 4. Null/empty input handling
        [Fact]
        public void HeightField_NullData_CreatesZeroInitializedField()
        {
            var hf = new HeightField(3, 3, null);
            Assert.Equal(0f, hf.GetHeight(0, 0));
            Assert.Equal(0f, hf.GetHeight(2, 2));
        }

        [Fact]
        public void HeightField_EmptyDataArray_WithZeroSize_Succeeds()
        {
            var hf = new HeightField(0, 0, Array.Empty<float>());
            Assert.NotNull(hf);
        }

        [Fact]
        public void HeightField_EmptyDataArray_WithNonZeroSize_Throws()
        {
            Assert.Throws<ArgumentException>(() => new HeightField(2, 2, Array.Empty<float>()));
        }

        [Fact]
        public void HeightField_DataArrayNull_WithZeroSize_Succeeds()
        {
            var hf = new HeightField(0, 0, null);
            Assert.NotNull(hf);
        }

        // 5. Boundary conditions (min/max values)
        [Fact]
        public void HeightField_NegativeWidth_ThrowsArgumentOutOfRangeException()
        {
            Assert.Throws<ArgumentOutOfRangeException>(() => new HeightField(-1, 4));
        }

        [Fact]
        public void HeightField_NegativeHeight_ThrowsArgumentOutOfRangeException()
        {
            Assert.Throws<ArgumentOutOfRangeException>(() => new HeightField(4, -1));
        }

        [Fact]
        public void HeightField_GetHeight_NegativeX_ThrowsArgumentOutOfRangeException()
        {
            var hf = new HeightField(4, 4);
            Assert.Throws<ArgumentOutOfRangeException>(() => hf.GetHeight(-1, 0));
        }

        [Fact]
        public void HeightField_GetHeight_NegativeZ_ThrowsArgumentOutOfRangeException()
        {
            var hf = new HeightField(4, 4);
            Assert.Throws<ArgumentOutOfRangeException>(() => hf.GetHeight(0, -1));
        }

        [Fact]
        public void HeightField_GetHeight_XEqualsWidth_ThrowsArgumentOutOfRangeException()
        {
            var hf = new HeightField(4, 4);
            Assert.Throws<ArgumentOutOfRangeException>(() => hf.GetHeight(4, 0));
        }

        [Fact]
        public void HeightField_GetHeight_ZEqualsHeight_ThrowsArgumentOutOfRangeException()
        {
            var hf = new HeightField(4, 4);
            Assert.Throws<ArgumentOutOfRangeException>(() => hf.GetHeight(0, 4));
        }

        [Fact]
        public void HeightField_GetHeight_XGreaterThanWidth_ThrowsArgumentOutOfRangeException()
        {
            var hf = new HeightField(4, 4);
            Assert.Throws<ArgumentOutOfRangeException>(() => hf.GetHeight(100, 0));
        }

        [Fact]
        public void HeightField_GetHeight_ZGreaterThanHeight_ThrowsArgumentOutOfRangeException()
        {
            var hf = new HeightField(4, 4);
            Assert.Throws<ArgumentOutOfRangeException>(() => hf.GetHeight(0, 100));
        }

        [Fact]
        public void HeightField_SetHeight_MaxIntegerCoordinates_Succeeds()
        {
            var hf = new HeightField(4, 4);
            hf.SetHeight(3, 3, float.MaxValue);
            Assert.Equal(float.MaxValue, hf.GetHeight(3, 3));
        }

        [Fact]
        public void HeightField_SetHeight_MinFloatValue_Succeeds()
        {
            var hf = new HeightField(4, 4);
            hf.SetHeight(0, 0, float.MinValue);
            Assert.Equal(float.MinValue, hf.GetHeight(0, 0));
        }

        [Fact]
        public void HeightField_SetHeight_MaxFloatValue_Succeeds()
        {
            var hf = new HeightField(4, 4);
            hf.SetHeight(0, 0, float.MaxValue);
            Assert.Equal(float.MaxValue, hf.GetHeight(0, 0));
        }

        [Fact]
        public void HeightField_SetHeight_PositiveInfinity_Succeeds()
        {
            var hf = new HeightField(4, 4);
            hf.SetHeight(0, 0, float.PositiveInfinity);
            Assert.Equal(float.PositiveInfinity, hf.GetHeight(0, 0));
        }

        [Fact]
        public void HeightField_SetHeight_NegativeInfinity_Succeeds()
        {
            var hf = new HeightField(4, 4);
            hf.SetHeight(0, 0, float.NegativeInfinity);
            Assert.Equal(float.NegativeInfinity, hf.GetHeight(0, 0));
        }

        [Fact]
        public void HeightField_SetHeight_NaN_Succeeds()
        {
            var hf = new HeightField(4, 4);
            hf.SetHeight(0, 0, float.NaN);
            Assert.True(float.IsNaN(hf.GetHeight(0, 0)));
        }
    }
}
