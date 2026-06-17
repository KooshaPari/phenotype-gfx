using UnityEngine;

namespace Phenotype.PostFx
{
    // ---------------------------------------------------------------------------
    // Shader-availability abstraction — injectable for tests
    // ---------------------------------------------------------------------------

    /// <summary>
    /// Abstracts shader/material availability so tests can inject mock results
    /// without requiring a live Unity runtime.
    /// </summary>
    public interface IShaderAvailabilityProvider
    {
        /// <summary>Returns true when the shader for the given effect is loaded
        /// and all required variants are present in the build.</summary>
        bool IsAvailable(PostFxEffect effect);
    }

    /// <summary>Identifies each post-processing effect.</summary>
    public enum PostFxEffect
    {
        SSAO,
        SSGI,
        Bloom,
        ACES,
        Vignette,
        ChromaticAberration,
        LUT,
    }

    /// <summary>
    /// Shared per-pass metadata used by the render pipeline.
    /// </summary>
    internal readonly struct PostFxPass
    {
        public readonly string ShaderName;
        public readonly Material? Material;
        public readonly bool Enabled;
        public readonly bool Supported;

        public PostFxPass(string shaderName, Material? material, bool enabled, bool supported)
        {
            ShaderName = shaderName;
            Material = material;
            Enabled = enabled;
            Supported = supported;
        }

        public bool IsActive => Enabled && Supported && Material;
    }

    /// <summary>
    /// Production implementation: an effect is available when its material was
    /// successfully loaded (non-null), which is exactly what TryLoad / the LUT
    /// path guarantee at init time.
    /// </summary>
    internal sealed class DefaultShaderAvailabilityProvider : IShaderAvailabilityProvider
    {
        readonly PostStack _owner;
        internal DefaultShaderAvailabilityProvider(PostStack owner) => _owner = owner;

        public bool IsAvailable(PostFxEffect effect) => effect switch
        {
            PostFxEffect.SSAO  => _owner._ssaoMat  != null,
            PostFxEffect.SSGI  => _owner._ssgiMat  != null,
            PostFxEffect.Bloom => _owner._bloomMat != null,
            PostFxEffect.ACES  => _owner._acesMat  != null,
            PostFxEffect.Vignette => _owner._vignetteMat != null,
            PostFxEffect.ChromaticAberration => _owner._chromaticAberrationMat != null,
            PostFxEffect.LUT   => _owner._lutMat   != null,
            _ => false,
        };
    }

    // ---------------------------------------------------------------------------

    public sealed class PostStack : MonoBehaviour
    {
        static readonly int BloomTexId = Shader.PropertyToID("_BloomTex");
        static readonly int ExposureId = Shader.PropertyToID("_Exposure");

        [Header("Pass Toggles")]
        public bool EnableSSAO = true;
        public bool EnableSSGI;
        public bool EnableBloom;
        public bool EnableACES = true;
        public bool EnableVignette;
        public bool EnableChromaticAberration;
        public bool EnableLUT = true;

        [Header("SSAO")]
        public int SSAOSamples = 12;
        public float SSAORadius = 2.0f;
        public float SSAOBias = 0.0012f;
        public float SSAOIntensity = 1.0f;

        [Header("SSGI")]
        public int SSGISamples = 12;
        public float SSGIRadius = 1.8f;
        public float SSGIIntensity = 0.45f;

        [Header("ACES")]
        public float Exposure = 1.0f;

        [Header("Vignette")]
        public Vector2 VignetteCenter = new Vector2 { x = 0.5f, y = 0.5f };
        public float VignetteIntensity = 0.45f;
        public float VignetteSmoothness = 0.6f;
        public float VignetteRoundness = 1.0f;

        [Header("Chromatic Aberration")]
        public float ChromaticAberrationIntensity = 0.15f;

        [Header("LUT")]
        public Texture2D LutTexture;

        // Materials — internal so DefaultShaderAvailabilityProvider can read them
        internal Material _ssaoMat;
        internal Material _ssgiMat;
        internal Material _bloomMat;
        internal Material _acesMat;
        internal Material _vignetteMat;
        internal Material _chromaticAberrationMat;
        internal Material _lutMat;

        RenderTexture _ping;
        bool _initialized;
        static readonly Vector4[] _ssaoKernel = new Vector4[16];
        static readonly Vector4[] _ssgiKernel = new Vector4[12];
        static bool _kernelsBuilt;

        // Validated availability flags — set by ValidateShaderVariants()
        bool _ssaoSupported;
        bool _ssgiSupported;
        bool _bloomSupported;
        bool _acesSupported;
        bool _vignetteSupported;
        bool _chromaticAberrationSupported;
        bool _lutSupported;

        /// <summary>
        /// Injectable shader-availability provider.  Defaults to the production
        /// implementation; replace in tests via <see cref="SetAvailabilityProvider"/>.
        /// </summary>
        IShaderAvailabilityProvider _availabilityProvider;

        // ------------------------------------------------------------------
        // Public API for test injection
        // ------------------------------------------------------------------

        /// <summary>
        /// Override the shader-availability provider used by
        /// <see cref="ValidateShaderVariants"/>.  Call before <c>Awake</c> or
        /// immediately after construction in tests.
        /// </summary>
        public void SetAvailabilityProvider(IShaderAvailabilityProvider provider)
            => _availabilityProvider = provider ?? throw new System.ArgumentNullException(nameof(provider));

        // ------------------------------------------------------------------

        void Awake()
        {
            Camera cam = GetComponent<Camera>();
            if (cam != null) cam.depthTextureMode |= DepthTextureMode.Depth;
            BuildKernels();
            InitMaterials();
        }

        void OnDestroy()
        {
            ReleaseMaterials();
            ReleasePingPong();
        }

        static void BuildKernels()
        {
            if (_kernelsBuilt) return;
            var rng = new System.Random(1337);
            for (int i = 0; i < _ssaoKernel.Length; i++)
            {
                Vector2 s;
                s.x = Mathf.Lerp(-1f, 1f, (float)rng.NextDouble());
                s.y = Mathf.Lerp(-1f, 1f, (float)rng.NextDouble());
                if (s.sqrMagnitude < 0.0001f) s = Vector2.right;
                s.Normalize();
                _ssaoKernel[i] = new Vector4(s.x, s.y, 0f, (i + 1f) / _ssaoKernel.Length);
            }
            rng = new System.Random(4242);
            for (int i = 0; i < _ssgiKernel.Length; i++)
            {
                Vector2 s;
                s.x = Mathf.Lerp(-1f, 1f, (float)rng.NextDouble());
                s.y = Mathf.Lerp(-1f, 1f, (float)rng.NextDouble());
                if (s.sqrMagnitude < 0.0001f) s = Vector2.one;
                s.Normalize();
                _ssgiKernel[i] = new Vector4(s.x, s.y, 0f, 0f);
            }
            _kernelsBuilt = true;
        }

        void InitMaterials()
        {
            ReleaseMaterials();
            _ssaoMat = TryLoadPass("Shaders/ScreenSpaceAO", "Hidden/ScreenSpaceAO");
            _ssgiMat = TryLoadPass("Shaders/ScreenSpaceGI", "Hidden/ScreenSpaceGI");
            _bloomMat = TryLoadPass("Shaders/BrpBloom", "Hidden/Phenotype/BrpBloom");
            _acesMat = TryLoadPass("Shaders/BrpACES", "Hidden/Phenotype/BrpACES");
            _vignetteMat = TryLoadPass("Shaders/Vignette", "Hidden/WSM3D/Vignette");
            _chromaticAberrationMat = TryLoadPass("Shaders/ChromaticAberration", "Hidden/WSM3D/ChromaticAberration");

            Shader lutShader = Resources.Load<Shader>("Shaders/ColorGradingLUT");
            lutShader ??= Shader.Find("Hidden/ColorGradingLUT");
            if (lutShader != null)
            {
                _lutMat = new Material(lutShader);
                if (LutTexture != null)
                {
                    if (_lutMat.HasProperty("_LutTex")) _lutMat.SetTexture("_LutTex", LutTexture);
                    else if (_lutMat.HasProperty("_LookupTex")) _lutMat.SetTexture("_LookupTex", LutTexture);
                }
                if (_lutMat.HasProperty("_LutParams"))
                    _lutMat.SetVector("_LutParams", new Vector4(16f / 256f, 1f / 16f, 1f, 0f));
            }

            if (_ssaoMat != null) ApplySSAOParams();
            if (_ssgiMat != null) ApplySSGIParams();

            // Resolve the provider lazily so tests can inject before Awake()
            _availabilityProvider ??= new DefaultShaderAvailabilityProvider(this);
            ValidateShaderVariants();

            _initialized = true;
        }

        // ------------------------------------------------------------------
        // Shader-variant validation
        // ------------------------------------------------------------------

        /// <summary>
        /// Audits each enabled effect against the availability provider.
        /// On failure the effect is silently disabled for this session and a
        /// <see cref="Debug.LogWarning"/> is emitted — preventing black/pink
        /// renders in stripped builds.
        /// </summary>
        internal void ValidateShaderVariants()
        {
            _ssaoSupported  = ValidatePass(PostFxEffect.SSAO,  "ScreenSpaceAO",  nameof(EnableSSAO));
            _ssgiSupported  = ValidatePass(PostFxEffect.SSGI,  "ScreenSpaceGI",  nameof(EnableSSGI));
            _bloomSupported = ValidatePass(PostFxEffect.Bloom, "BrpBloom",       nameof(EnableBloom));
            _acesSupported  = ValidatePass(PostFxEffect.ACES,  "BrpACES",        nameof(EnableACES));
            _vignetteSupported = ValidatePass(PostFxEffect.Vignette, "Vignette", nameof(EnableVignette));
            _chromaticAberrationSupported = ValidatePass(PostFxEffect.ChromaticAberration, "ChromaticAberration", nameof(EnableChromaticAberration));
            _lutSupported   = ValidatePass(PostFxEffect.LUT,   "ColorGradingLUT",nameof(EnableLUT));
        }

        bool ValidatePass(PostFxEffect effect, string shaderName, string toggleName)
        {
            bool available = _availabilityProvider.IsAvailable(effect);
            if (!available)
            {
                Debug.LogWarning(
                    $"[PostStack] Shader variant unavailable: {shaderName}. " +
                    $"{toggleName} will be skipped this session. " +
                    $"Ensure the shader is included in your build's ShaderVariantCollection " +
                    $"(phenotype-postfx-variants.shadervariants).");
            }
            return available;
        }

        void ReleaseMaterials()
        {
            if (_ssaoMat != null) { Destroy(_ssaoMat); _ssaoMat = null; }
            if (_ssgiMat != null) { Destroy(_ssgiMat); _ssgiMat = null; }
            if (_bloomMat != null) { Destroy(_bloomMat); _bloomMat = null; }
            if (_acesMat != null) { Destroy(_acesMat); _acesMat = null; }
            if (_vignetteMat != null) { Destroy(_vignetteMat); _vignetteMat = null; }
            if (_chromaticAberrationMat != null) { Destroy(_chromaticAberrationMat); _chromaticAberrationMat = null; }
            if (_lutMat != null) { Destroy(_lutMat); _lutMat = null; }
        }

        static Material? TryLoadPass(string resourcePath, string fallbackName)
        {
            Shader shader = Resources.Load<Shader>(resourcePath);
            shader ??= Shader.Find(fallbackName);
            return shader != null ? new Material(shader) : null;
        }

        void ApplySSAOParams()
        {
            _ssaoMat.SetInt("_SampleCount", SSAOSamples);
            _ssaoMat.SetVectorArray("_Samples", _ssaoKernel);
            _ssaoMat.SetFloat("_Radius", SSAORadius);
            _ssaoMat.SetFloat("_Bias", SSAOBias);
            _ssaoMat.SetFloat("_Intensity", SSAOIntensity);
        }

        void ApplySSGIParams()
        {
            _ssgiMat.SetInt("_SampleCount", SSGISamples);
            _ssgiMat.SetVectorArray("_Samples", _ssgiKernel);
            _ssgiMat.SetFloat("_Radius", SSGIRadius);
            _ssgiMat.SetFloat("_Intensity", SSGIIntensity);
        }

        void ApplyACESParams()
        {
            _acesMat.SetFloat(ExposureId, Exposure);
        }

        void ApplyVignetteParams()
        {
            _vignetteMat.SetVector("_Center", new Vector4(VignetteCenter.x, VignetteCenter.y, 0f, 0f));
            _vignetteMat.SetFloat("_Intensity", VignetteIntensity);
            _vignetteMat.SetFloat("_Smoothness", VignetteSmoothness);
            _vignetteMat.SetFloat("_Roundness", VignetteRoundness);
        }

        void ApplyChromaticAberrationParams()
        {
            _chromaticAberrationMat.SetFloat("_Intensity", ChromaticAberrationIntensity);
        }

        void EnsurePingPong(RenderTexture src)
        {
            if (_ping != null && _ping.width == src.width && _ping.height == src.height) return;
            ReleasePingPong();
            _ping = RenderTexture.GetTemporary(src.descriptor);
        }

        void ReleasePingPong()
        {
            if (_ping != null) { RenderTexture.ReleaseTemporary(_ping); _ping = null; }
        }

        static bool BlitIfEnabled(RenderTexture src, RenderTexture dst, in PostFxPass pass, System.Action? beforeBlit = null)
        {
            if (!pass.IsActive) return false;
            beforeBlit?.Invoke();
            Graphics.Blit(src, dst, pass.Material!);
            return true;
        }

        void OnRenderImage(RenderTexture src, RenderTexture dst)
        {
            if (src == null || dst == null) return;
            if (!_initialized)
            {
                Graphics.Blit(src, dst);
                return;
            }

            PostFxPass ssaoPass = new PostFxPass("ScreenSpaceAO", _ssaoMat, EnableSSAO, _ssaoSupported);
            PostFxPass ssgiPass = new PostFxPass("ScreenSpaceGI", _ssgiMat, EnableSSGI, _ssgiSupported);
            PostFxPass bloomPass = new PostFxPass("BrpBloom", _bloomMat, EnableBloom, _bloomSupported);
            PostFxPass acesPass = new PostFxPass("BrpACES", _acesMat, EnableACES, _acesSupported);
            PostFxPass vignettePass = new PostFxPass("Vignette", _vignetteMat, EnableVignette, _vignetteSupported);
            PostFxPass chromaticAberrationPass = new PostFxPass("ChromaticAberration", _chromaticAberrationMat, EnableChromaticAberration, _chromaticAberrationSupported);
            PostFxPass lutPass = new PostFxPass("ColorGradingLUT", _lutMat, EnableLUT, _lutSupported);

            bool anyPass = ssaoPass.IsActive ||
                           ssgiPass.IsActive ||
                           bloomPass.IsActive ||
                           acesPass.IsActive ||
                           vignettePass.IsActive ||
                           chromaticAberrationPass.IsActive ||
                           (lutPass.IsActive && LutTexture);
            if (!anyPass)
            {
                Graphics.Blit(src, dst);
                return;
            }

            EnsurePingPong(src);
            try
            {
                RenderTexture cur = src, next = _ping;

                if (BlitIfEnabled(cur, next, ssaoPass, ApplySSAOParams))
                {
                    Swap(ref cur, ref next);
                }

                if (BlitIfEnabled(cur, next, ssgiPass, ApplySSGIParams))
                {
                    Swap(ref cur, ref next);
                }

                if (bloomPass.IsActive)
                {
                    int w = Mathf.Max(1, src.width / 4);
                    int h = Mathf.Max(1, src.height / 4);
                    RenderTexture bA = RenderTexture.GetTemporary(w, h, 0, src.format);
                    RenderTexture bB = RenderTexture.GetTemporary(w, h, 0, src.format);
                    try
                    {
                        Graphics.Blit(cur, bA, _bloomMat, 0);
                        Graphics.Blit(bA, bB, _bloomMat, 1);
                        Graphics.Blit(bB, bA, _bloomMat, 2);
                        _bloomMat.SetTexture(BloomTexId, bA);
                        Graphics.Blit(cur, next, _bloomMat, 3);
                        Swap(ref cur, ref next);
                    }
                    finally
                    {
                        _bloomMat.SetTexture(BloomTexId, null);
                        RenderTexture.ReleaseTemporary(bA);
                        RenderTexture.ReleaseTemporary(bB);
                    }
                }

                if (BlitIfEnabled(cur, next, acesPass, ApplyACESParams))
                {
                    Swap(ref cur, ref next);
                }

                if (BlitIfEnabled(cur, next, vignettePass, ApplyVignetteParams))
                {
                    Swap(ref cur, ref next);
                }

                if (BlitIfEnabled(cur, next, chromaticAberrationPass, ApplyChromaticAberrationParams))
                {
                    Swap(ref cur, ref next);
                }

                if (lutPass.IsActive && LutTexture)
                    Graphics.Blit(cur, dst, lutPass.Material!);
                else
                    Graphics.Blit(cur, dst);
            }
            finally
            {
                ReleasePingPong();
            }
        }

        static void Swap(ref RenderTexture a, ref RenderTexture b) => (a, b) = (b, a);
    }
}
