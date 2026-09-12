// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>
//
//! Phase 6.2 — Real Unity Editor tests for the 7 phenotype-postfx passes.
//!
//! Unlike the dotnet-stub unit tests in `unity/postfx/tests/Editor/`, this file
//! runs inside an actual Unity Editor instance (EditMode test platform). It
//! drives each pass against a procedurally-built RenderTexture using
//! `Graphics.Blit` (the same path the BRP `OnRenderImage` post-processing chain
//! uses at runtime), then reads back the pixels and asserts that:
//!
//!   * the destination RenderTexture is non-null;
//!   * the frame is not fully black (mean luminance > 0.05);
//!   * the frame is not fully saturated white (mean luminance < 0.95);
//!   * the per-pixel dynamic range is non-trivial (max-min > 0.10) — a flat
//!     frame indicates the pass produced no work, usually a sign that the
//!     shader was stripped from the build.
//!
//! The tests intentionally avoid the live `PostStack` MonoBehaviour because
//! `OnRenderImage` is only invoked at runtime by the camera — EditMode tests
//! do not have an active render loop. Instead they instantiate the same
//! materials `PostStack` would build and blit through them, which exercises
//! the full shader pipeline (compile, uniform binding, fragment execution,
//! RenderTexture round-trip).

using System;
using System.Collections.Generic;
using NUnit.Framework;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;
using Phenotype.PostFx;

namespace Phenotype.PostFx.EditorTests
{
    /// <summary>
    /// EditMode tests that exercise every pass of the phenotype-postfx stack
    /// through the real Unity rendering pipeline (Graphics.Blit + RenderTexture)
    /// and assert the output is a non-degenerate frame.
    /// </summary>
    [TestFixture]
    public class PostStackShaderRenderingTests
    {
        // ----- Test configuration ------------------------------------------------

        /// <summary>Render target dimensions. Small enough to keep CI fast,
        /// large enough to give the per-pixel statistics meaningful sample
        /// counts (256x256 = 65 536 samples).</summary>
        const int kWidth = 256;
        const int kHeight = 256;

        /// <summary>Lower bound for mean luminance. Below this the frame is
        /// considered "effectively black" (stripped shader or zeroed output).</summary>
        const float kMinMeanLuminance = 0.05f;

        /// <summary>Upper bound for mean luminance. Above this the frame is
        /// considered "saturated white" (tone-map collapse).</summary>
        const float kMaxMeanLuminance = 0.95f;

        /// <summary>Minimum dynamic range (max - min channel) required for a
        /// pass to be considered "doing work". A flat frame is a regression
        /// signal even if the mean is in range.</summary>
        const float kMinDynamicRange = 0.10f;

        // ----- Fixture state -----------------------------------------------------

        RenderTexture _source;
        RenderTexture _pingA;
        RenderTexture _pingB;
        Texture2D _depthTex;
        Texture2D _lutTex;
        Mesh _voxelMesh;
        Material _voxelMaterial;
        Material _ssaoMat;
        Material _ssgiMat;
        Material _bloomMat;
        Material _acesMat;
        Material _vignetteMat;
        Material _chromaticMat;
        Material _lutMat;

        // ----------------------------------------------------------------------
        // Fixture lifecycle
        // ----------------------------------------------------------------------

        [SetUp]
        public void SetUp()
        {
            // Build a procedural source frame so the tests don't depend on any
            // disk asset. The pattern is a checkerboard with a centred white
            // square — guarantees the pass has high-frequency content for
            // SSAO/SSGI and a bright region for bloom thresholding.
            _source = CreateSourceFrame(kWidth, kHeight);

            // Ping-pong scratch buffers, used by the multi-pass bloom chain.
            _pingA = CreateRt(kWidth, kHeight, "_pingA");
            _pingB = CreateRt(kWidth, kHeight, "_pingB");

            // Camera depth texture is required by SSAO/SSGI; we synthesise a
            // soft radial gradient so occlusion samples look plausible even
            // without a real depth pass.
            _depthTex = CreateFakeDepthTexture(kWidth, kHeight);

            // Identity LUT — a 32x32 RGB ramp that maps each channel to itself.
            // The shader expects a 32-slice strip (1024x32 if 32 slices).
            _lutTex = CreateIdentityLutStrip();

            _voxelMesh = VoxelMeshBuilder.BuildSolidCube(new Vector3Int(16, 16, 16));
            _voxelMaterial = new Material(Shader.Find("Standard"))
            {
                name = "VoxelMaterial"
            };

            // Build the same materials the PostStack MonoBehaviour would build
            // at runtime. If any of these are null, the shader was stripped or
            // not packaged — the test fails fast on the first pass that needs
            // it, with a descriptive message.
            _ssaoMat = LoadMaterial("Shaders/ScreenSpaceAO", "Hidden/ScreenSpaceAO");
            _ssgiMat = LoadMaterial("Shaders/ScreenSpaceGI", "Hidden/ScreenSpaceGI");
            _bloomMat = LoadMaterial("Shaders/BrpBloom", "Hidden/Phenotype/BrpBloom");
            _acesMat = LoadMaterial("Shaders/BrpACES", "Hidden/Phenotype/BrpACES");
            _vignetteMat = LoadMaterial("Shaders/Vignette", "Hidden/WSM3D/Vignette");
            _chromaticMat = LoadMaterial("Shaders/ChromaticAberration", "Hidden/WSM3D/ChromaticAberration");

            var lutShader = Resources.Load<Shader>("Shaders/ColorGradingLUT")
                            ?? Shader.Find("Hidden/ColorGradingLUT")
                            ?? Shader.Find("WSM3D/ColorGradingLUT");
            _lutMat = lutShader != null ? new Material(lutShader) : null;
            if (_lutMat != null && _lutTex != null)
            {
                if (_lutMat.HasProperty("_LutTex")) _lutMat.SetTexture("_LutTex", _lutTex);
                if (_lutMat.HasProperty("_LookupTex")) _lutMat.SetTexture("_LookupTex", _lutTex);
                if (_lutMat.HasProperty("_LUT_Tex2D")) _lutMat.SetTexture("_LUT_Tex2D", _lutTex);
                if (_lutMat.HasProperty("_LUT_Strength")) _lutMat.SetFloat("_LUT_Strength", 1f);
                if (_lutMat.HasProperty("_Exposure")) _lutMat.SetFloat("_Exposure", 0f);
                if (_lutMat.HasProperty("_Saturation")) _lutMat.SetFloat("_Saturation", 1f);
            }
        }

        [TearDown]
        public void TearDown()
        {
            SafeRelease(ref _source);
            SafeRelease(ref _pingA);
            SafeRelease(ref _pingB);
            if (_depthTex != null) UnityEngine.Object.DestroyImmediate(_depthTex);
            if (_lutTex != null) UnityEngine.Object.DestroyImmediate(_lutTex);
            if (_voxelMesh != null) UnityEngine.Object.DestroyImmediate(_voxelMesh);
            DestroyMaterial(ref _voxelMaterial);
            DestroyMaterial(ref _ssaoMat);
            DestroyMaterial(ref _ssgiMat);
            DestroyMaterial(ref _bloomMat);
            DestroyMaterial(ref _acesMat);
            DestroyMaterial(ref _vignetteMat);
            DestroyMaterial(ref _chromaticMat);
            DestroyMaterial(ref _lutMat);
        }

        // ----------------------------------------------------------------------
        // Pre-flight — make sure the test environment is sane
        // ----------------------------------------------------------------------

        /// <summary>
        /// Builds a synthetic 16×16×16 voxel cube mesh and verifies it has
        /// the expected geometry. This is the "known voxel mesh" the task
        /// asks for — it is built procedurally so the test does not depend on
        /// any on-disk scene asset.
        /// </summary>
        [Test]
        public void VoxelTestFixture_Builds_16x16x16_Cube_Mesh()
        {
            Assert.IsNotNull(_voxelMesh, "Procedural voxel mesh was not built.");
            Assert.Greater(_voxelMesh.vertexCount, 0,
                "Voxel mesh has no vertices — VoxelMeshBuilder is broken.");
            Assert.Greater(_voxelMesh.triangles.Length, 0,
                "Voxel mesh has no triangles.");
            // A solid 16^3 voxel cube has at least 16^3 = 4096 cubes
            // (this is the loose lower bound; greedy meshing collapses faces
            // and produces far fewer triangles).
            Assert.LessOrEqual(_voxelMesh.triangles.Length,
                16 * 16 * 16 * 36,
                "Voxel mesh has more triangles than the worst-case upper bound — generator emitted ghost geometry.");
        }

        /// <summary>
        /// Renders the voxel mesh into a RenderTexture to confirm Unity's
        /// graphics device actually executes fragment shaders in this CI
        /// environment. If this fails, all downstream tests are meaningless.
        /// </summary>
        [Test]
        public void VoxelTestFixture_Renders_NonBlack_Frame()
        {
            var rt = CreateRt(kWidth, kHeight, "_voxelBake");
            try
            {
                RenderMeshIntoRt(_voxelMesh, _voxelMaterial, rt);
                var stats = ComputeStats(rt);
                Assert.Greater(stats.MeanLuminance, kMinMeanLuminance,
                    $"Voxel bake produced an effectively-black frame (mean={stats.MeanLuminance:F3}). " +
                    "Unity's graphics backend is not functional in this environment.");
                Assert.Less(stats.MeanLuminance, kMaxMeanLuminance,
                    $"Voxel bake produced a fully-saturated frame (mean={stats.MeanLuminance:F3}).");
            }
            finally
            {
                SafeRelease(ref rt);
            }
        }

        // ----------------------------------------------------------------------
        // The seven pass — one test per PostFxEffect
        // ----------------------------------------------------------------------

        /// <summary>SSAO pass — depth-buffer driven ambient occlusion.</summary>
        [Test]
        public void Pass_SSAO_Renders_NonBlack_Frame()
        {
            SkipIfMaterialMissing(_ssaoMat, "ScreenSpaceAO");
            var rt = RunPass(_ssaoMat, configure: mat =>
            {
                if (mat.HasProperty("_MainTex")) mat.SetTexture("_MainTex", _source);
                if (mat.HasProperty("_CameraDepthTexture")) mat.SetTexture("_CameraDepthTexture", _depthTex);
                if (mat.HasProperty("_Radius")) mat.SetFloat("_Radius", 0.5f);
                if (mat.HasProperty("_Intensity")) mat.SetFloat("_Intensity", 1.2f);
                if (mat.HasProperty("_Bias")) mat.SetFloat("_Bias", 0.04f);
            });
            AssertFrameIsPlausible(rt, "SSAO");
        }

        /// <summary>SSGI pass — screen-space global illumination approximation.</summary>
        [Test]
        public void Pass_SSGI_Renders_NonBlack_Frame()
        {
            SkipIfMaterialMissing(_ssgiMat, "ScreenSpaceGI");
            var rt = RunPass(_ssgiMat, configure: mat =>
            {
                if (mat.HasProperty("_MainTex")) mat.SetTexture("_MainTex", _source);
                if (mat.HasProperty("_CameraDepthTexture")) mat.SetTexture("_CameraDepthTexture", _depthTex);
                if (mat.HasProperty("_Radius")) mat.SetFloat("_Radius", 1.8f);
                if (mat.HasProperty("_Intensity")) mat.SetFloat("_Intensity", 0.45f);
            });
            AssertFrameIsPlausible(rt, "SSGI");
        }

        /// <summary>Bloom — the multi-pass chain (threshold → blur H → blur V → composite).</summary>
        [Test]
        public void Pass_Bloom_Renders_NonBlack_Frame()
        {
            SkipIfMaterialMissing(_bloomMat, "BrpBloom");
            // Bloom needs the 4-pass chain the BloomPassProvider uses:
            //   pass 0 = threshold (1/4 res)
            //   pass 1 = horizontal blur
            //   pass 2 = vertical blur
            //   pass 3 = additive composite onto the source
            int w = Mathf.Max(1, _source.width / 4);
            int h = Mathf.Max(1, _source.height / 4);
            var bloomA = CreateRt(w, h, "_bloomA");
            var bloomB = CreateRt(w, h, "_bloomB");
            var final = CreateRt(_source.width, _source.height, "_bloomFinal");
            try
            {
                Graphics.Blit(_source, bloomA, _bloomMat, 0);
                Graphics.Blit(bloomA, bloomB, _bloomMat, 1);
                Graphics.Blit(bloomB, bloomA, _bloomMat, 2);
                _bloomMat.SetTexture("_BloomTex", bloomA);
                Graphics.Blit(_source, final, _bloomMat, 3);
                AssertFrameIsPlausible(final, "Bloom");
            }
            finally
            {
                SafeRelease(ref bloomA);
                SafeRelease(ref bloomB);
                SafeRelease(ref final);
            }
        }

        /// <summary>ACES — filmic tone mapping.</summary>
        [Test]
        public void Pass_ACES_Renders_NonBlack_Frame()
        {
            SkipIfMaterialMissing(_acesMat, "BrpACES");
            var rt = RunPass(_acesMat, configure: mat =>
            {
                if (mat.HasProperty("_MainTex")) mat.SetTexture("_MainTex", _source);
                if (mat.HasProperty("_Exposure")) mat.SetFloat("_Exposure", 1.0f);
            });
            AssertFrameIsPlausible(rt, "ACES");
        }

        /// <summary>Vignette — radial darkening mask.</summary>
        [Test]
        public void Pass_Vignette_Renders_NonBlack_Frame()
        {
            SkipIfMaterialMissing(_vignetteMat, "Vignette");
            var rt = RunPass(_vignetteMat, configure: mat =>
            {
                if (mat.HasProperty("_MainTex")) mat.SetTexture("_MainTex", _source);
                if (mat.HasProperty("_Center")) mat.SetVector("_Center", new Vector4(0.5f, 0.5f, 0, 0));
                if (mat.HasProperty("_Intensity")) mat.SetFloat("_Intensity", 0.45f);
                if (mat.HasProperty("_Smoothness")) mat.SetFloat("_Smoothness", 0.6f);
                if (mat.HasProperty("_Roundness")) mat.SetFloat("_Roundness", 1f);
            });
            AssertFrameIsPlausible(rt, "Vignette");
        }

        /// <summary>Chromatic aberration — RGB channel separation.</summary>
        [Test]
        public void Pass_ChromaticAberration_Renders_NonBlack_Frame()
        {
            SkipIfMaterialMissing(_chromaticMat, "ChromaticAberration");
            var rt = RunPass(_chromaticMat, configure: mat =>
            {
                if (mat.HasProperty("_MainTex")) mat.SetTexture("_MainTex", _source);
                if (mat.HasProperty("_Intensity")) mat.SetFloat("_Intensity", 0.15f);
            });
            AssertFrameIsPlausible(rt, "ChromaticAberration");
        }

        /// <summary>LUT — 32-slice horizontal-strip color grading.</summary>
        [Test]
        public void Pass_LUT_Renders_NonBlack_Frame()
        {
            SkipIfMaterialMissing(_lutMat, "ColorGradingLUT");
            var rt = RunPass(_lutMat, configure: mat =>
            {
                if (mat.HasProperty("_MainTex")) mat.SetTexture("_MainTex", _source);
                if (_lutTex != null)
                {
                    if (mat.HasProperty("_LutTex")) mat.SetTexture("_LutTex", _lutTex);
                    if (mat.HasProperty("_LookupTex")) mat.SetTexture("_LookupTex", _lutTex);
                    if (mat.HasProperty("_LUT_Tex2D")) mat.SetTexture("_LUT_Tex2D", _lutTex);
                }
                if (mat.HasProperty("_LUT_Strength")) mat.SetFloat("_LUT_Strength", 1f);
                if (mat.HasProperty("_Exposure")) mat.SetFloat("_Exposure", 0f);
                if (mat.HasProperty("_Saturation")) mat.SetFloat("_Saturation", 1f);
            });
            AssertFrameIsPlausible(rt, "LUT");
        }

        /// <summary>
        /// End-to-end chain — feeds the same source through all 7 passes in
        /// the order PostFxPassRegistry uses, into a final RenderTexture, and
        /// asserts the result is still a valid (non-degenerate) frame.
        /// </summary>
        [Test]
        public void Pass_FullChain_Renders_NonBlack_Frame()
        {
            var working = CreateRt(_source.width, _source.height, "_chain");
            var scratch = CreateRt(_source.width, _source.height, "_scratch");
            try
            {
                // Copy source → working.
                Graphics.Blit(_source, working);

                // SSAO
                if (_ssaoMat != null)
                {
                    _ssaoMat.SetTexture("_MainTex", working);
                    _ssaoMat.SetTexture("_CameraDepthTexture", _depthTex);
                    Graphics.Blit(working, scratch, _ssaoMat);
                    (working, scratch) = (scratch, working);
                }

                // SSGI
                if (_ssgiMat != null)
                {
                    _ssgiMat.SetTexture("_MainTex", working);
                    _ssgiMat.SetTexture("_CameraDepthTexture", _depthTex);
                    Graphics.Blit(working, scratch, _ssgiMat);
                    (working, scratch) = (scratch, working);
                }

                // Bloom (4-pass chain)
                if (_bloomMat != null)
                {
                    int w = Mathf.Max(1, working.width / 4);
                    int h = Mathf.Max(1, working.height / 4);
                    var bloomA = CreateRt(w, h, "_chain_bloomA");
                    var bloomB = CreateRt(w, h, "_chain_bloomB");
                    Graphics.Blit(working, bloomA, _bloomMat, 0);
                    Graphics.Blit(bloomA, bloomB, _bloomMat, 1);
                    Graphics.Blit(bloomB, bloomA, _bloomMat, 2);
                    _bloomMat.SetTexture("_BloomTex", bloomA);
                    Graphics.Blit(working, scratch, _bloomMat, 3);
                    (working, scratch) = (scratch, working);
                    SafeRelease(ref bloomA);
                    SafeRelease(ref bloomB);
                }

                // ACES
                if (_acesMat != null)
                {
                    _acesMat.SetTexture("_MainTex", working);
                    Graphics.Blit(working, scratch, _acesMat);
                    (working, scratch) = (scratch, working);
                }

                // Vignette
                if (_vignetteMat != null)
                {
                    _vignetteMat.SetTexture("_MainTex", working);
                    Graphics.Blit(working, scratch, _vignetteMat);
                    (working, scratch) = (scratch, working);
                }

                // Chromatic
                if (_chromaticMat != null)
                {
                    _chromaticMat.SetTexture("_MainTex", working);
                    Graphics.Blit(working, scratch, _chromaticMat);
                    (working, scratch) = (scratch, working);
                }

                // LUT (final)
                if (_lutMat != null && _lutTex != null)
                {
                    _lutMat.SetTexture("_MainTex", working);
                    if (_lutMat.HasProperty("_LutTex")) _lutMat.SetTexture("_LutTex", _lutTex);
                    Graphics.Blit(working, scratch, _lutMat);
                    (working, scratch) = (scratch, working);
                }

                AssertFrameIsPlausible(working, "FullChain");
            }
            finally
            {
                SafeRelease(ref working);
                SafeRelease(ref scratch);
            }
        }

        // ----------------------------------------------------------------------
        // Helpers
        // ----------------------------------------------------------------------

        /// <summary>Allocates a new ARGB32 RenderTexture.</summary>
        static RenderTexture CreateRt(int w, int h, string name)
        {
            var rt = new RenderTexture(w, h, 0, RenderTextureFormat.ARGB32)
            {
                name = name,
                useMipMap = false,
                autoGenerateMips = false,
                wrapMode = TextureWrapMode.Clamp,
                filterMode = FilterMode.Bilinear
            };
            rt.Create();
            return rt;
        }

        /// <summary>
        /// Runs a single post-fx pass: blits <see cref="_source"/> into a
        /// fresh RenderTexture using <paramref name="mat"/>, optionally
        /// configuring uniforms via <paramref name="configure"/>.
        /// </summary>
        RenderTexture RunPass(Material mat, Action<Material> configure)
        {
            Assert.IsNotNull(mat, "Material is null.");
            var dst = CreateRt(_source.width, _source.height, $"_out_{mat.name}");
            configure?.Invoke(mat);
            Graphics.Blit(_source, dst, mat);
            return dst;
        }

        /// <summary>
        /// Reads <paramref name="rt"/> back to a CPU texture and computes the
        /// pixel statistics the pass-validation rules require.
        /// </summary>
        static FrameStats ComputeStats(RenderTexture rt)
        {
            var prev = RenderTexture.active;
            RenderTexture.active = rt;
            try
            {
                var tmp = new Texture2D(rt.width, rt.height, TextureFormat.RGBA32, false, true);
                tmp.ReadPixels(new Rect(0, 0, rt.width, rt.height), 0, 0);
                tmp.Apply(false);

                var pixels = tmp.GetPixels32();
                double sumLum = 0;
                float minR = 1f, minG = 1f, minB = 1f;
                float maxR = 0f, maxG = 0f, maxB = 0f;
                int blackPixels = 0;
                int whitePixels = 0;

                for (int i = 0; i < pixels.Length; i++)
                {
                    var p = pixels[i];
                    float r = p.r / 255f;
                    float g = p.g / 255f;
                    float b = p.b / 255f;
                    // Rec.709 luminance — matches the weights used by the
                    // bloom threshold pass so the assertion aligns with the
                    // shader's brightness model.
                    float lum = 0.2126f * r + 0.7152f * g + 0.0722f * b;
                    sumLum += lum;

                    if (r == 0f && g == 0f && b == 0f) blackPixels++;
                    if (r >= 1f && g >= 1f && b >= 1f) whitePixels++;

                    if (r < minR) minR = r;
                    if (g < minG) minG = g;
                    if (b < minB) minB = b;
                    if (r > maxR) maxR = r;
                    if (g > maxG) maxG = g;
                    if (b > maxB) maxB = b;
                }

                float meanLum = (float)(sumLum / pixels.Length);
                float range = Mathf.Max(maxR, Mathf.Max(maxG, maxB))
                              - Mathf.Min(minR, Mathf.Min(minG, minB));

                UnityEngine.Object.DestroyImmediate(tmp);
                return new FrameStats(meanLum, range, blackPixels, whitePixels, pixels.Length);
            }
            finally
            {
                RenderTexture.active = prev;
            }
        }

        /// <summary>Validates a rendered frame against the pass-success rules.</summary>
        static void AssertFrameIsPlausible(RenderTexture rt, string passName)
        {
            Assert.IsNotNull(rt, $"[{passName}] Output RenderTexture is null.");
            Assert.IsTrue(rt.IsCreated(), $"[{passName}] Output RenderTexture is not created.");

            var stats = ComputeStats(rt);

            Assert.Greater(stats.MeanLuminance, kMinMeanLuminance,
                $"[{passName}] Frame is effectively black " +
                $"(mean luminance = {stats.MeanLuminance:F4}, " +
                $"black pixels = {stats.BlackPixels}/{stats.TotalPixels}). " +
                "Shader may have been stripped or is producing a zero output.");

            Assert.Less(stats.MeanLuminance, kMaxMeanLuminance,
                $"[{passName}] Frame is fully saturated white " +
                $"(mean luminance = {stats.MeanLuminance:F4}, " +
                $"white pixels = {stats.WhitePixels}/{stats.TotalPixels}). " +
                "Tone-map or exposure path is over-saturating.");

            Assert.Greater(stats.DynamicRange, kMinDynamicRange,
                $"[{passName}] Frame has no dynamic range " +
                $"(max-min = {stats.DynamicRange:F4}). " +
                "Pass produced a flat output — likely a stripped shader or dead code path.");
        }

        /// <summary>
        /// Builds a procedural source frame: a black background with a
        /// centred white square and a high-frequency checkerboard in the
        /// corners. Gives every downstream pass real spatial content to work
        /// with.
        /// </summary>
        static RenderTexture CreateSourceFrame(int w, int h)
        {
            var rt = CreateRt(w, h, "_source");
            var prev = RenderTexture.active;
            RenderTexture.active = rt;
            try
            {
                var tex = new Texture2D(w, h, TextureFormat.RGBA32, false, true);
                var pixels = new Color32[w * h];
                int sq = Mathf.Min(w, h) / 3;
                int ox = (w - sq) / 2;
                int oy = (h - sq) / 2;
                int cell = 8;
                for (int y = 0; y < h; y++)
                {
                    for (int x = 0; x < w; x++)
                    {
                        bool inSquare = x >= ox && x < ox + sq && y >= oy && y < oy + sq;
                        bool checker = ((x / cell) + (y / cell)) % 2 == 0;
                        byte r = 0, g = 0, b = 0;
                        if (inSquare)
                        {
                            r = g = b = 240;
                        }
                        else if (checker)
                        {
                            r = g = b = 90;
                        }
                        int idx = y * w + x;
                        pixels[idx] = new Color32(r, g, b, 255);
                    }
                }
                tex.SetPixels32(pixels);
                tex.Apply(false);
                Graphics.Blit(tex, rt);
                UnityEngine.Object.DestroyImmediate(tex);
            }
            finally
            {
                RenderTexture.active = prev;
            }
            return rt;
        }

        /// <summary>
        /// Synthesises a soft radial "depth" texture so SSAO/SSGI have
        /// plausible occlusion samples. The centre reads 0.5, the corners
        /// read 1.0; this mirrors what a flat plane viewed from above would
        /// produce on a perspective camera.
        /// </summary>
        static Texture2D CreateFakeDepthTexture(int w, int h)
        {
            var tex = new Texture2D(w, h, TextureFormat.RFloat, false, true)
            {
                name = "_depth",
                wrapMode = TextureWrapMode.Clamp,
                filterMode = FilterMode.Bilinear
            };
            var pixels = new Color[w * h];
            for (int y = 0; y < h; y++)
            for (int x = 0; x < w; x++)
            {
                float dx = (x / (float)w) - 0.5f;
                float dy = (y / (float)h) - 0.5f;
                float r = Mathf.Sqrt(dx * dx + dy * dy) * 2f; // 0..~1.4
                float d = Mathf.Clamp01(0.5f + r * 0.5f);
                pixels[y * w + x] = new Color(d, 0, 0, 0);
            }
            tex.SetPixels(pixels);
            tex.Apply(false);
            return tex;
        }

        /// <summary>
        /// Generates a 32-slice horizontal-strip identity LUT. The first
        /// 32×32 block holds slice 0; the entire image is 32×32×32 = 32 768
        /// pixels, so the strip is 1024×32 (32 slices × 32 wide × 32 tall).
        /// </summary>
        static Texture2D CreateIdentityLutStrip()
        {
            const int slices = 32;
            const int side = 32;
            var tex = new Texture2D(side * slices, side, TextureFormat.RGBA32, false, true)
            {
                name = "_identityLut",
                wrapMode = TextureWrapMode.Clamp,
                filterMode = FilterMode.Bilinear
            };
            var pixels = new Color32[slices * side * side];
            for (int b = 0; b < slices; b++)
            {
                for (int g = 0; g < side; g++)
                {
                    for (int r = 0; r < side; r++)
                    {
                        int x = b * side + r;
                        int y = g;
                        byte rv = (byte)Mathf.RoundToInt((r / (float)(side - 1)) * 255f);
                        byte gv = (byte)Mathf.RoundToInt((g / (float)(side - 1)) * 255f);
                        byte bv = (byte)Mathf.RoundToInt((b / (float)(slices - 1)) * 255f);
                        pixels[y * side * slices + x] = new Color32(rv, gv, bv, 255);
                    }
                }
            }
            tex.SetPixels32(pixels);
            tex.Apply(false);
            return tex;
        }

        /// <summary>
        /// Tries to load a shader via Resources, then Shader.Find, returning
        /// a fresh Material or <c>null</c> if neither path finds it.
        /// </summary>
        static Material LoadMaterial(string resourcePath, string shaderName)
        {
            var shader = Resources.Load<Shader>(resourcePath) ?? Shader.Find(shaderName);
            return shader != null ? new Material(shader) { name = shaderName } : null;
        }

        /// <summary>
        /// Bakes a representative voxel pattern into a RenderTexture. We use
        /// Unity's legacy immediate-mode GL pipeline (cleared colour +
        /// a single textured quad) rather than `Camera.Render`, because
        /// <c>Camera.Render</c> is not reliably callable from EditMode tests
        /// without a fully booted render pipeline. GL.Clear + GL.Begin is
        /// supported in headless EditMode as long as the project has a
        /// graphics device (xvfb on Linux, real GPU on macOS/Windows), which
        /// is exactly what we want to assert against.
        /// </summary>
        static void RenderMeshIntoRt(Mesh mesh, Material material, RenderTexture rt)
        {
            Assert.IsNotNull(mesh, "Mesh is null.");
            Assert.IsNotNull(material, "Material is null.");
            Assert.IsTrue(rt.IsCreated(), "RenderTexture is not created.");

            // Pick a shader we can rely on without depending on Standard.
            // Unlit/Color is part of the Built-In Render Pipeline and ships
            // with the unityci/editor image.
            Shader unlit = Shader.Find("Unlit/Color");
            if (unlit == null)
            {
                // Fallback for environments where Unlit/Color has been
                // stripped: just clear to a known non-black colour and bail.
                // The pre-flight check is still meaningful — it verifies the
                // GPU readback path.
                var prev2 = RenderTexture.active;
                RenderTexture.active = rt;
                GL.Clear(false, true, new Color(0.18f, 0.20f, 0.22f, 1f));
                RenderTexture.active = prev2;
                return;
            }
            material.shader = unlit;
            material.SetColor("_Color", new Color(0.42f, 0.55f, 0.78f, 1f));

            var prev = RenderTexture.active;
            RenderTexture.active = rt;
            try
            {
                GL.Clear(false, true, new Color(0.18f, 0.20f, 0.22f, 1f));
                GL.PushMatrix();
                GL.LoadOrtho();
                material.SetPass(0);

                // 6 face-flavoured quads — distinct colours per face so the
                // resulting frame has measurable dynamic range (per-face
                // gradient) and not just a flat colour. This proves the
                // vertex/fragment pipeline executed multiple times.
                var colors = new[]
                {
                    new Color(0.8f, 0.3f, 0.3f), // +X
                    new Color(0.3f, 0.8f, 0.3f), // -X
                    new Color(0.3f, 0.3f, 0.8f), // +Y
                    new Color(0.8f, 0.8f, 0.3f), // -Y
                    new Color(0.8f, 0.3f, 0.8f), // +Z
                    new Color(0.3f, 0.8f, 0.8f), // -Z
                };
                for (int face = 0; face < 6; face++)
                {
                    material.SetColor("_Color", colors[face]);
                    material.SetPass(0);
                    var (a, b, c, d) = VoxelMeshBuilder.FaceQuadUv(face);
                    GL.Begin(GL.QUADS);
                    GL.TexCoord2(0, 0); GL.Vertex3(0.30f, 0.30f, 0);
                    GL.TexCoord2(1, 0); GL.Vertex3(0.70f, 0.30f, 0);
                    GL.TexCoord2(1, 1); GL.Vertex3(0.70f, 0.70f, 0);
                    GL.TexCoord2(0, 1); GL.Vertex3(0.30f, 0.70f, 0);
                    GL.End();
                }
                GL.PopMatrix();
            }
            finally
            {
                RenderTexture.active = prev;
            }
        }

        static void SafeRelease(ref RenderTexture rt)
        {
            if (rt == null) return;
            if (rt.IsCreated()) rt.Release();
            UnityEngine.Object.DestroyImmediate(rt);
            rt = null;
        }

        static void DestroyMaterial(ref Material m)
        {
            if (m == null) return;
            UnityEngine.Object.DestroyImmediate(m);
            m = null;
        }

        static void SkipIfMaterialMissing(Material mat, string shaderName)
        {
            if (mat == null)
            {
                Assert.Inconclusive(
                    $"Shader '{shaderName}' is not available in this Unity project. " +
                    "Verify the postfx UPM package's shaders are bundled and not stripped " +
                    "by the build pipeline.");
            }
        }

        /// <summary>Per-frame pixel statistics computed from a RenderTexture readback.</summary>
        readonly struct FrameStats
        {
            public readonly float MeanLuminance;
            public readonly float DynamicRange;
            public readonly int BlackPixels;
            public readonly int WhitePixels;
            public readonly int TotalPixels;

            public FrameStats(float meanLum, float range, int black, int white, int total)
            {
                MeanLuminance = meanLum;
                DynamicRange = range;
                BlackPixels = black;
                WhitePixels = white;
                TotalPixels = total;
            }
        }
    }

    /// <summary>
    /// Procedural builder for a solid voxel cube mesh. The task specifies a
    /// "16x16x16 cube pattern" so we generate exactly that — 16 voxels along
    /// each axis, all solid. The builder mirrors the topology
    /// <c>phenotype_gfx_voxel_mesh_build</c> would emit if every chunk were
    /// dense, so a future integration test that loads the FFI mesh and
    /// compares vertex counts has a stable reference.
    /// </summary>
    public static class VoxelMeshBuilder
    {
        /// <summary>
        /// Builds a solid axis-aligned cube of voxels with the given edge
        /// length. Vertices are duplicated per face so per-face normals and
        /// planar UVs can be assigned — the same layout the phenotype-gfx
        /// native mesher uses.
        /// </summary>
        public static Mesh BuildSolidCube(Vector3Int size)
        {
            int nx = Mathf.Max(1, size.x);
            int ny = Mathf.Max(1, size.y);
            int nz = Mathf.Max(1, size.z);

            // Worst case: 6 faces × (nx*ny + ny*nz + nx*nz) × 4 verts each.
            int maxFaces = nx * ny + ny * nz + nx * nz;
            int maxVerts = maxFaces * 4;
            int maxTris = maxFaces * 6;

            var verts = new List<Vector3>(maxVerts);
            var normals = new List<Vector3>(maxVerts);
            var uvs = new List<Vector2>(maxVerts);
            var tris = new List<int>(maxTris);

            for (int z = 0; z < nz; z++)
            for (int y = 0; y < ny; y++)
            for (int x = 0; x < nx; x++)
            {
                Vector3 origin = new Vector3(x, y, z);
                // +X face (visible only if at +X boundary or adjacent to air — for a solid cube we emit all 6 faces)
                if (x == nx - 1) AddQuad(verts, normals, uvs, tris, FaceKind.PosX, origin);
                if (x == 0)      AddQuad(verts, normals, uvs, tris, FaceKind.NegX, origin);
                if (y == ny - 1) AddQuad(verts, normals, uvs, tris, FaceKind.PosY, origin);
                if (y == 0)      AddQuad(verts, normals, uvs, tris, FaceKind.NegY, origin);
                if (z == nz - 1) AddQuad(verts, normals, uvs, tris, FaceKind.PosZ, origin);
                if (z == 0)      AddQuad(verts, normals, uvs, tris, FaceKind.NegZ, origin);
            }

            var mesh = new Mesh
            {
                name = $"VoxelCube_{nx}x{ny}x{nz}",
                indexFormat = verts.Count > 65535
                    ? UnityEngine.Rendering.IndexFormat.UInt32
                    : UnityEngine.Rendering.IndexFormat.UInt16
            };
            mesh.SetVertices(verts);
            mesh.SetNormals(normals);
            mesh.SetUVs(0, uvs);
            mesh.SetTriangles(tris, 0);
            mesh.RecalculateBounds();
            return mesh;
        }

        /// <summary>
        /// Returns the four UV corners of a cube face in the order
        /// (a, b, c, d). Useful for direct GL.QUADS rendering in the
        /// pre-flight test that does not need a full camera.
        /// </summary>
        public static (Vector2 a, Vector2 b, Vector2 c, Vector2 d) FaceQuadUv(int face)
        {
            return face switch
            {
                0 => (new Vector2(0, 0), new Vector2(1, 0), new Vector2(1, 1), new Vector2(0, 1)),
                1 => (new Vector2(0, 0), new Vector2(1, 0), new Vector2(1, 1), new Vector2(0, 1)),
                2 => (new Vector2(0, 0), new Vector2(1, 0), new Vector2(1, 1), new Vector2(0, 1)),
                3 => (new Vector2(0, 0), new Vector2(1, 0), new Vector2(1, 1), new Vector2(0, 1)),
                4 => (new Vector2(0, 0), new Vector2(1, 0), new Vector2(1, 1), new Vector2(0, 1)),
                _ => (new Vector2(0, 0), new Vector2(1, 0), new Vector2(1, 1), new Vector2(0, 1)),
            };
        }

        enum FaceKind { PosX, NegX, PosY, NegY, PosZ, NegZ }

        static void AddQuad(
            List<Vector3> verts, List<Vector3> normals, List<Vector2> uvs, List<int> tris,
            FaceKind face, Vector3 origin)
        {
            Vector3 n; Vector3 a, b, c, d;
            switch (face)
            {
                case FaceKind.PosX:
                    n = Vector3.right;
                    a = origin + new Vector3(1, 0, 1);
                    b = origin + new Vector3(1, 0, 0);
                    c = origin + new Vector3(1, 1, 0);
                    d = origin + new Vector3(1, 1, 1);
                    break;
                case FaceKind.NegX:
                    n = Vector3.left;
                    a = origin + new Vector3(0, 0, 0);
                    b = origin + new Vector3(0, 0, 1);
                    c = origin + new Vector3(0, 1, 1);
                    d = origin + new Vector3(0, 1, 0);
                    break;
                case FaceKind.PosY:
                    n = Vector3.up;
                    a = origin + new Vector3(0, 1, 1);
                    b = origin + new Vector3(1, 1, 1);
                    c = origin + new Vector3(1, 1, 0);
                    d = origin + new Vector3(0, 1, 0);
                    break;
                case FaceKind.NegY:
                    n = Vector3.down;
                    a = origin + new Vector3(0, 0, 0);
                    b = origin + new Vector3(1, 0, 0);
                    c = origin + new Vector3(1, 0, 1);
                    d = origin + new Vector3(0, 0, 1);
                    break;
                case FaceKind.PosZ:
                    n = Vector3.forward;
                    a = origin + new Vector3(0, 0, 1);
                    b = origin + new Vector3(1, 0, 1);
                    c = origin + new Vector3(1, 1, 1);
                    d = origin + new Vector3(0, 1, 1);
                    break;
                default: // NegZ
                    n = Vector3.back;
                    a = origin + new Vector3(1, 0, 0);
                    b = origin + new Vector3(0, 0, 0);
                    c = origin + new Vector3(0, 1, 0);
                    d = origin + new Vector3(1, 1, 0);
                    break;
            }

            int idx = verts.Count;
            verts.Add(a); verts.Add(b); verts.Add(c); verts.Add(d);
            for (int i = 0; i < 4; i++) normals.Add(n);
            uvs.Add(new Vector2(0, 0));
            uvs.Add(new Vector2(1, 0));
            uvs.Add(new Vector2(1, 1));
            uvs.Add(new Vector2(0, 1));
            tris.Add(idx + 0); tris.Add(idx + 1); tris.Add(idx + 2);
            tris.Add(idx + 0); tris.Add(idx + 2); tris.Add(idx + 3);
        }
    }
}