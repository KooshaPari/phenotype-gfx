using System;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using PostFxContext = Phenotype.PostFx.Ports.PostFxContext;
using PostFxQuality = Phenotype.PostFx.Ports.PostFxQuality;

namespace Phenotype.PostFx.Tests
{
    [TestFixture]
    public sealed class BloomPassTests
    {
        BloomPass _pass;
        RenderTexture _src;
        RenderTexture _dst;
        PostFxContext _ctx;

        [SetUp]
        public void SetUp()
        {
            _pass = new BloomPass();
            _src = new RenderTexture { width = 64, height = 64 };
            _dst = new RenderTexture { width = 64, height = 64 };
            var camera = new Camera();
            _ctx = new PostFxContext(_src, _dst, camera, PostFxQuality.High, new MaterialPropertyBlock());
        }

        [TearDown]
        public void TearDown()
        {
            _pass.OnDispose();
        }

        [Test]
        public void Name_IsBloom()
        {
            Assert.AreEqual("Bloom", _pass.Name);
        }

        [Test]
        public void Cost_IsNonZero()
        {
            Assert.Greater(_pass.Cost, 0f);
        }

        [Test]
        public void IsEnabled_DefaultsTrue()
        {
            Assert.IsTrue(_pass.IsEnabled);
        }

        [Test]
        public void Threshold_IsConfigurable()
        {
            _pass.Threshold = 0.5f;
            Assert.AreEqual(0.5f, _pass.Threshold);
        }

        [Test]
        public void Intensity_IsConfigurable()
        {
            _pass.Intensity = 1.2f;
            Assert.AreEqual(1.2f, _pass.Intensity);
        }

        [Test]
        public void Iterations_IsConfigurable()
        {
            _pass.Iterations = 4;
            Assert.AreEqual(4, _pass.Iterations);
        }

        [Test]
        public void OnSetup_WithMissingShader_LeavesMaterialNull()
        {
            _pass.OnSetup(_ctx);
            Assert.IsNull(_pass.Material);
        }

        [Test]
        public void OnRender_WithNullMaterial_DoesNotThrow()
        {
            Assert.DoesNotThrow(() => _pass.OnRender(_ctx));
        }

        [Test]
        public void OnRender_WithMaterial_CapturesBlits()
        {
            var shader = new Shader();
            var mat = new Material(shader);
            var field = typeof(BloomPass).GetField("_material",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            Assert.IsNotNull(field);
            field!.SetValue(_pass, mat);

            GraphicsCapture.Clear();
            _pass.OnRender(_ctx);

            Assert.IsTrue(GraphicsCapture.Blits.Count > 0);
        }

        [Test]
        public void OnRender_WithIterations_CorrectBlitCount()
        {
            _pass.Iterations = 3;
            var shader = new Shader();
            var mat = new Material(shader);
            var field = typeof(BloomPass).GetField("_material",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            Assert.IsNotNull(field);
            field!.SetValue(_pass, mat);

            GraphicsCapture.Clear();
            _pass.OnRender(_ctx);

            // Threshold (1) + iterations * 2 (blur H + blur V) + composite (1)
            int expected = 1 + (_pass.Iterations * 2) + 1;
            Assert.AreEqual(expected, GraphicsCapture.Blits.Count);
        }

        [Test]
        public void ValidateVariants_Available_DoesNotThrow()
        {
            var provider = new Phenotype.PostFx.Ports.DefaultShaderAvailabilityProvider();
            Assert.DoesNotThrow(() => _pass.ValidateVariants(provider));
        }

        [Test]
        public void ValidateVariants_Unavailable_Throws()
        {
            var provider = new MockShaderProvider(false);
            Assert.Throws<InvalidOperationException>(() => _pass.ValidateVariants(provider));
        }

        [Test]
        public void OnDispose_ClearsMaterial()
        {
            var shader = new Shader();
            var mat = new Material(shader);
            var field = typeof(BloomPass).GetField("_material",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            Assert.IsNotNull(field);
            field!.SetValue(_pass, mat);

            _pass.OnDispose();
            Assert.IsNull(_pass.Material);
        }

        [Test]
        public void QualityKeyword_UpdatesOnRender()
        {
            var shader = new Shader();
            var mat = new Material(shader);
            var field = typeof(BloomPass).GetField("_material",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            Assert.IsNotNull(field);
            field!.SetValue(_pass, mat);

            var lowCtx = new PostFxContext(_src, _dst, new Camera(), PostFxQuality.Low, new MaterialPropertyBlock());
            _pass.OnRender(lowCtx);
            Assert.IsTrue(mat.IsKeywordEnabled("BLOOM_LOW"));
            Assert.IsFalse(mat.IsKeywordEnabled("BLOOM_MEDIUM"));

            var highCtx = new PostFxContext(_src, _dst, new Camera(), PostFxQuality.High, new MaterialPropertyBlock());
            _pass.OnRender(highCtx);
            Assert.IsTrue(mat.IsKeywordEnabled("BLOOM_HIGH"));
            Assert.IsFalse(mat.IsKeywordEnabled("BLOOM_LOW"));
        }
    }

    internal sealed class MockShaderProvider : Phenotype.PostFx.Ports.IShaderAvailabilityProvider
    {
        public bool Available { get; }

        public MockShaderProvider(bool available) => Available = available;

        public bool IsAvailable(string shaderName, string keyword) => Available;
    }
}
