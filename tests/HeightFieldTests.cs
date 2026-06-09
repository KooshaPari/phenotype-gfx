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
            var hf = new HeightField();
            Assert.NotNull(hf);
        }
    }
}
