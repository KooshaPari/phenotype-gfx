using Xunit;
using Phenotype.Water.Rendering;

namespace Phenotype.Water.Tests
{
    public class WaterShaderTests
    {
        [Fact]
        public void Constructor_DoesNotThrow()
        {
            var shader = new WaterShader("Water/Ocean");
            Assert.NotNull(shader);
        }
    }
}
