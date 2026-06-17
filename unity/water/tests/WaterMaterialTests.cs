using UnityEngine;
using Xunit;
using Phenotype.Water.Rendering;

namespace Phenotype.Water.Tests
{
    public class WaterMaterialTests
    {
        [Fact]
        public void Constructor_DoesNotThrow()
        {
            var shader = new WaterShader("Water/Ocean");
            var material = new WaterMaterial(shader);
            Assert.NotNull(material.Material);
        }

        [Fact]
        public void SetFloat_DoesNotThrow()
        {
            var shader = new WaterShader("Water/Ocean");
            var material = new WaterMaterial(shader);
            material.SetFloat("_WaveSpeed", 2.0f);
        }

        [Fact]
        public void SetVector_DoesNotThrow()
        {
            var shader = new WaterShader("Water/Ocean");
            var material = new WaterMaterial(shader);
            material.SetVector("_Color", new Vector4(0, 0.5f, 1, 1));
        }
    }
}
