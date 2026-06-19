// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! SSAOPass — Screen Space Ambient Occlusion using the IPostFxPass hexagonal port.
//!
//! Generates a configurable sample kernel and applies an occlusion mask via
//! depth-buffer comparison.  Parameters are exposed as mutable properties
//! so the driver or test code can tune them without re-creating the pass.

using System;
using UnityEngine;
using Phenotype.PostFx.Ports;

namespace Phenotype.PostFx
{
    /// <summary>
    /// Screen-space ambient occlusion pass using the <see cref="IPostFxPass"/> hexagonal port.
    /// Supports configurable radius, intensity, bias, and kernel size, and generates
    /// a deterministic sample kernel for depth-buffer sampling.
    /// </summary>
    /// <example>
    /// <code>
    /// var ssao = new SSAOPass();
    /// ssao.Radius = 1.0f;
    /// ssao.Intensity = 1.5f;
    /// ssao.OnSetup(ctx);
    /// ssao.OnRender(ctx);
    /// </code>
    /// </example>
    public sealed class SSAOPass : Phenotype.PostFx.Ports.IPostFxPass
    {
        /// <summary>
        /// Gets the stable name of this pass.
        /// </summary>
        /// <value>"SSAO".</value>
        public string Name => "SSAO";

        /// <summary>
        /// Gets the relative cost hint.
        /// </summary>
        /// <value>0.25f (quarter-frame cost estimate).</value>
        public float Cost => 0.25f;

        /// <summary>
        /// Gets or sets a value indicating whether this pass is enabled.
        /// </summary>
        /// <value>
        /// <see langword="true"/> if the pass is enabled; otherwise, <see langword="false"/>.
        /// Default is <see langword="true"/>.
        /// </value>
        public bool IsEnabled { get; set; } = true;

        /// <summary>
        /// Gets or sets the world-space radius of the SSAO sampling sphere.
        /// </summary>
        /// <value>The radius. Default is 0.5f.</value>
        public float Radius { get; set; } = 0.5f;

        /// <summary>
        /// Gets or sets the intensity multiplier for the occlusion mask.
        /// </summary>
        /// <value>The intensity. Default is 1.2f.</value>
        public float Intensity { get; set; } = 1.2f;

        /// <summary>
        /// Gets or sets the depth bias to avoid self-occlusion artifacts.
        /// </summary>
        /// <value>The bias. Default is 0.04f.</value>
        public float Bias { get; set; } = 0.04f;

        /// <summary>
        /// Gets or sets the number of samples in the SSAO kernel.
        /// </summary>
        /// <value>The kernel size. Default is 8.</value>
        public int KernelSize { get; set; } = 8;

        /// <summary>
        /// Gets the material for this pass, or <see langword="null"/> if the shader was not found.
        /// </summary>
        /// <value>The <see cref="Material"/> instance.</value>
        public Material? Material => _material;

        Material? _material;
        Vector4[] _kernel = Array.Empty<Vector4>();

        static readonly int RadiusId = Shader.PropertyToID("_Radius");
        static readonly int IntensityId = Shader.PropertyToID("_Intensity");
        static readonly int BiasId = Shader.PropertyToID("_Bias");
        static readonly int SampleCountId = Shader.PropertyToID("_SampleCount");
        static readonly int SamplesId = Shader.PropertyToID("_Samples");

        const string ShaderName = "Hidden/Phenotype/SSAOPass";
        const string PassKeyword = "SSAOPASS";

        /// <summary>
        /// Creates the material and builds the sample kernel if needed.
        /// </summary>
        /// <param name="ctx">Per-camera context (ignored in this implementation).</param>
        public void OnSetup(PostFxContext ctx)
        {
            if (_material == null)
            {
                Shader? shader = Shader.Find(ShaderName);
                if (shader == null) return;
                _material = new Material(shader);
            }

            if (_kernel.Length == 0)
            {
                BuildKernel(KernelSize);
            }

            ApplyParams();
        }

        /// <summary>
        /// Renders the SSAO pass from <c>ctx.Source</c> to <c>ctx.Destination</c>.
        /// </summary>
        /// <param name="ctx">Per-camera context containing source and destination render targets.</param>
        public void OnRender(PostFxContext ctx)
        {
            if (_material == null) return;

            ApplyParams();
            Graphics.Blit(ctx.Source, ctx.Destination, _material);
        }

        /// <summary>
        /// Destroys the material and resets internal state.
        /// </summary>
        public void OnDispose()
        {
            if (_material != null)
            {
                UnityEngine.Object.Destroy(_material);
                _material = null;
            }
        }

        /// <summary>
        /// Validates that the required shader variant is available.
        /// </summary>
        /// <param name="shaderProvider">The provider to query for shader availability.</param>
        /// <exception cref="InvalidOperationException">
        /// Thrown when the shader variant is unavailable.
        /// </exception>
        public void ValidateVariants(Phenotype.PostFx.Ports.IShaderAvailabilityProvider shaderProvider)
        {
            if (!shaderProvider.IsAvailable(ShaderName, PassKeyword))
                throw new InvalidOperationException($"Shader variant unavailable: {ShaderName} ({PassKeyword})");
        }

        /// <summary>
        /// Generates a deterministic sample kernel of the given size.
        /// Samples are distributed on a unit disk and scaled so that inner samples
        /// are closer to the origin and outer samples are farther away.
        /// </summary>
        /// <param name="size">The number of samples in the kernel.</param>
        public void BuildKernel(int size)
        {
            _kernel = new Vector4[size];
            var rng = new System.Random(1337);
            for (int i = 0; i < size; i++)
            {
                Vector2 s;
                s.x = Mathf.Lerp(-1f, 1f, (float)rng.NextDouble());
                s.y = Mathf.Lerp(-1f, 1f, (float)rng.NextDouble());
                if (s.sqrMagnitude < 0.0001f) s = Vector2.right;
                s.Normalize();
                float scale = (i + 1f) / size;
                _kernel[i] = new Vector4(s.x, s.y, 0f, scale);
            }
        }

        void ApplyParams()
        {
            if (_material == null) return;
            _material.SetFloat(RadiusId, Radius);
            _material.SetFloat(IntensityId, Intensity);
            _material.SetFloat(BiasId, Bias);
            _material.SetInt(SampleCountId, _kernel.Length);
            _material.SetVectorArray(SamplesId, _kernel);
        }
    }
}
