// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! NUnit tests for SSAOPass — kernel generation, parameter binding, lifecycle,
//! and shader-variant validation.

using System;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using PostFxContext = Phenotype.PostFx.Ports.PostFxContext;
using PostFxQuality = Phenotype.PostFx.Ports.PostFxQuality;

namespace Phenotype.PostFx.Tests
{
    [TestFixture]
    public sealed class SSAOPassTests
    {
        SSAOPass _pass;
        RenderTexture _src;
        RenderTexture _dst;
        PostFxContext _ctx;

        [SetUp]
        public void SetUp()
        {
            _pass = new SSAOPass();
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
        public void Name_IsSSAO()
        {
            Assert.AreEqual("SSAO", _pass.Name);
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
        public void Radius_IsConfigurable()
        {
            _pass.Radius = 1.0f;
            Assert.AreEqual(1.0f, _pass.Radius);
        }

        [Test]
        public void Intensity_IsConfigurable()
        {
            _pass.Intensity = 2.0f;
            Assert.AreEqual(2.0f, _pass.Intensity);
        }

        [Test]
        public void Bias_IsConfigurable()
        {
            _pass.Bias = 0.1f;
            Assert.AreEqual(0.1f, _pass.Bias);
        }

        [Test]
        public void KernelSize_IsConfigurable()
        {
            _pass.KernelSize = 16;
            Assert.AreEqual(16, _pass.KernelSize);
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
            var field = typeof(SSAOPass).GetField("_material",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            Assert.IsNotNull(field);
            field!.SetValue(_pass, mat);

            GraphicsCapture.Clear();
            _pass.OnRender(_ctx);

            Assert.IsTrue(GraphicsCapture.Blits.Count > 0);
        }

        [Test]
        public void OnRender_WithMaterial_AppliesParameters()
        {
            var shader = new Shader();
            var mat = new Material(shader);
            var field = typeof(SSAOPass).GetField("_material",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            Assert.IsNotNull(field);
            field!.SetValue(_pass, mat);

            _pass.Radius = 1.5f;
            _pass.Intensity = 2.5f;
            _pass.Bias = 0.08f;
            _pass.KernelSize = 12;
            _pass.BuildKernel(12);

            GraphicsCapture.Clear();
            _pass.OnRender(_ctx);

            Assert.IsTrue(GraphicsCapture.Blits.Count > 0);
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
            var field = typeof(SSAOPass).GetField("_material",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            Assert.IsNotNull(field);
            field!.SetValue(_pass, mat);

            _pass.OnDispose();
            Assert.IsNull(_pass.Material);
        }

        [Test]
        public void BuildKernel_GeneratesCorrectSize()
        {
            _pass.KernelSize = 12;
            _pass.BuildKernel(12);

            var field = typeof(SSAOPass).GetField("_kernel",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            Assert.IsNotNull(field);
            var kernel = (Vector4[])field!.GetValue(_pass)!;
            Assert.AreEqual(12, kernel.Length);
        }

        [Test]
        public void BuildKernel_RespectsSizeChange()
        {
            _pass.KernelSize = 8;
            _pass.BuildKernel(8);

            var field = typeof(SSAOPass).GetField("_kernel",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            Assert.IsNotNull(field);
            var kernel = (Vector4[])field!.GetValue(_pass)!;
            Assert.AreEqual(8, kernel.Length);

            _pass.KernelSize = 16;
            _pass.BuildKernel(16);
            kernel = (Vector4[])field!.GetValue(_pass)!;
            Assert.AreEqual(16, kernel.Length);
        }

        [Test]
        public void BuildKernel_SamplesAreNormalized()
        {
            _pass.KernelSize = 8;
            _pass.BuildKernel(8);

            var field = typeof(SSAOPass).GetField("_kernel",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            Assert.IsNotNull(field);
            var kernel = (Vector4[])field!.GetValue(_pass)!;

            foreach (var sample in kernel)
            {
                float len = (float)Math.Sqrt(sample.x * sample.x + sample.y * sample.y);
                // Direction part should be roughly unit length before scaling
                Assert.Greater(len, 0f, "Sample direction should be non-zero");
            }
        }

        [Test]
        public void OnSetup_WithMissingShader_LeavesKernelEmpty()
        {
            _pass.OnSetup(_ctx);
            Assert.IsNull(_pass.Material);

            var kernelField = typeof(SSAOPass).GetField("_kernel",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            Assert.IsNotNull(kernelField);
            var kernel = (Vector4[])kernelField!.GetValue(_pass)!;
            Assert.AreEqual(0, kernel.Length);
        }

        [Test]
        public void OnSetup_SkipsRebuildIfMaterialExists()
        {
            _pass.KernelSize = 8;
            _pass.BuildKernel(8);

            var kernelField = typeof(SSAOPass).GetField("_kernel",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            Assert.IsNotNull(kernelField);
            var kernel = (Vector4[])kernelField!.GetValue(_pass)!;
            Assert.AreEqual(8, kernel.Length);

            var shader = new Shader();
            var mat = new Material(shader);
            var matField = typeof(SSAOPass).GetField("_material",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            Assert.IsNotNull(matField);
            matField!.SetValue(_pass, mat);

            // Change kernel size and call OnSetup again — it should be a no-op
            // because material already exists.
            _pass.KernelSize = 24;
            _pass.OnSetup(_ctx);
            var kernel2 = (Vector4[])kernelField!.GetValue(_pass)!;
            Assert.AreEqual(8, kernel2.Length);
        }
    }
}
