using UnityEngine;
using Phenotype.PostFx.Ports;

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
        /// <summary>
        /// Returns <see langword="true"/> when the shader for the given effect is loaded
        /// and all required variants are present in the build.
        /// </summary>
        /// <param name="effect">The post-processing effect to check.</param>
        /// <returns>
        /// <see langword="true"/> if the shader is available; otherwise, <see langword="false"/>.
        /// </returns>
        bool IsAvailable(PostFxEffect effect);
    }

    /// <summary>Identifies each post-processing effect.</summary>
    public enum PostFxEffect
    {
        /// <summary>Screen Space Ambient Occlusion.</summary>
        SSAO,
        /// <summary>Screen Space Global Illumination.</summary>
        SSGI,
        /// <summary>Bloom glow effect.</summary>
        Bloom,
        /// <summary>ACES tone mapping.</summary>
        ACES,
        /// <summary>Vignette darkening.</summary>
        Vignette,
        /// <summary>Chromatic aberration distortion.</summary>
        ChromaticAberration,
        /// <summary>Color Look-Up Table grading.</summary>
        LUT,
    }

    /// <summary>
    /// Shared per-pass metadata used by the render pipeline.
    /// </summary>
    internal readonly struct PostFxPass
    {
        /// <summary>
        /// Gets the name of the shader used by this pass.
        /// </summary>
        /// <value>The shader name.</value>
        public readonly string ShaderName;

        /// <summary>
        /// Gets the material instance for this pass.
        /// </summary>
        /// <value>The material, or <see langword="null"/> if unavailable.</value>
        public readonly Material? Material;

        /// <summary>
        /// Gets a value indicating whether the pass is enabled.
        /// </summary>
        /// <value>
        /// <see langword="true"/> if enabled; otherwise, <see langword="false"/>.
        /// </value>
        public readonly bool Enabled;

        /// <summary>
        /// Gets a value indicating whether the pass is supported in this build.
        /// </summary>
        /// <value>
        /// <see langword="true"/> if supported; otherwise, <see langword="false"/>.
        /// </value>
        public readonly bool Supported;

        /// <summary>
        /// Initializes a new instance of the <see cref="PostFxPass"/> struct.
        /// </summary>
        /// <param name="shaderName">The shader name.</param>
        /// <param name="material">The material instance.</param>
        /// <param name="enabled">Whether the pass is enabled.</param>
        /// <param name="supported">Whether the pass is supported.</param>
        public PostFxPass(string shaderName, Material? material, bool enabled, bool supported)
        {
            ShaderName = shaderName;
            Material = material;
            Enabled = enabled;
            Supported = supported;
        }

        /// <summary>
        /// Gets a value indicating whether this pass is active.
        /// A pass is active when it is enabled, supported, and has a valid material.
        /// </summary>
        /// <value>
        /// <see langword="true"/> if the pass should execute; otherwise, <see langword="false"/>.
        /// </value>
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

        /// <summary>
        /// Initializes a new instance of the <see cref="DefaultShaderAvailabilityProvider"/> class.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> to query for material availability.</param>
        internal DefaultShaderAvailabilityProvider(PostStack owner) => _owner = owner;

        /// <summary>
        /// Checks whether the specified effect's material is loaded.
        /// </summary>
        /// <param name="effect">The effect to check.</param>
        /// <returns>
        /// <see langword="true"/> if the material is non-null; otherwise, <see langword="false"/>.
        /// </returns>
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

    /// <summary>
    /// Central driver for the post-processing stack.
    /// Attach this <see cref="MonoBehaviour"/> to a camera to apply a configurable
    /// chain of post-processing effects (SSAO, Bloom, ACES, Vignette, etc.).
    /// </summary>
    /// <remarks>
    /// <para>
    /// The stack initialises materials on <c>Awake</c>, validates shader variants,
    /// and composites enabled passes via <c>OnRenderImage</c>.  Pass order is
    /// controlled by <see cref="PostFxPassRegistry"/>.
    /// </para>
    /// <para>
    /// Example usage in a scene:
    /// <code>
    /// var stack = camera.gameObject.AddComponent&lt;PostStack&gt;();
    /// stack.EnableBloom = true;
    /// stack.EnableACES = true;
    /// </code>
    /// </para>
    /// </remarks>
    public sealed class PostStack : MonoBehaviour
    {
        /// <summary>
        /// Shader property ID for the bloom texture (<c>_BloomTex</c>).
        /// Used by <see cref="BloomPassProvider"/> to bind the blurred intermediate.
        /// </summary>
        public static readonly int BloomTexId = Shader.PropertyToID("_BloomTex");
        static readonly int ExposureId = Shader.PropertyToID("_Exposure");

        [Header("Pass Toggles")]
        /// <summary>
        /// Enables or disables the Screen Space Ambient Occlusion (SSAO) pass.
        /// </summary>
        /// <value>
        /// <see langword="true"/> to enable SSAO; otherwise, <see langword="false"/>.
        /// Default is <see langword="true"/>.
        /// </value>
        public bool EnableSSAO = true;

        /// <summary>
        /// Enables or disables the Screen Space Global Illumination (SSGI) pass.
        /// </summary>
        /// <value>
        /// <see langword="true"/> to enable SSGI; otherwise, <see langword="false"/>.
        /// Default is <see langword="false"/>.
        /// </value>
        public bool EnableSSGI;

        /// <summary>
        /// Enables or disables the Bloom pass.
        /// </summary>
        /// <value>
        /// <see langword="true"/> to enable Bloom; otherwise, <see langword="false"/>.
        /// Default is <see langword="false"/>.
        /// </value>
        public bool EnableBloom;

        /// <summary>
        /// Enables or disables the ACES tone mapping pass.
        /// </summary>
        /// <value>
        /// <see langword="true"/> to enable ACES; otherwise, <see langword="false"/>.
        /// Default is <see langword="true"/>.
        /// </value>
        public bool EnableACES = true;

        /// <summary>
        /// Enables or disables the Vignette pass.
        /// </summary>
        /// <value>
        /// <see langword="true"/> to enable Vignette; otherwise, <see langword="false"/>.
        /// Default is <see langword="false"/>.
        /// </value>
        public bool EnableVignette;

        /// <summary>
        /// Enables or disables the Chromatic Aberration pass.
        /// </summary>
        /// <value>
        /// <see langword="true"/> to enable Chromatic Aberration; otherwise, <see langword="false"/>.
        /// Default is <see langword="false"/>.
        /// </value>
        public bool EnableChromaticAberration;

        /// <summary>
        /// Enables or disables the Color Look-Up Table (LUT) grading pass.
        /// </summary>
        /// <value>
        /// <see langword="true"/> to enable LUT; otherwise, <see langword="false"/>.
        /// Default is <see langword="true"/>.
        /// </value>
        public bool EnableLUT = true;

        [Header("Quality")]
        /// <summary>
        /// The overall quality preset for the post-processing stack.
        /// </summary>
        /// <value>
        /// A <see cref="PostFxQuality"/> value. Default is <see cref="PostFxQuality.High"/>.
        /// </value>
        public PostFxQuality Quality = PostFxQuality.High;

        [Header("SSAO")]
        /// <summary>
        /// Number of sample points used by the SSAO kernel.
        /// Higher values improve quality at the cost of performance.
        /// </summary>
        /// <value>The sample count. Default is 12.</value>
        public int SSAOSamples = 12;

        /// <summary>
        /// World-space radius of the SSAO sampling sphere.
        /// </summary>
        /// <value>The radius in world units. Default is 2.0f.</value>
        public float SSAORadius = 2.0f;

        /// <summary>
        /// Depth bias for SSAO to avoid self-occlusion artifacts.
        /// </summary>
        /// <value>The bias value. Default is 0.0012f.</value>
        public float SSAOBias = 0.0012f;

        /// <summary>
        /// Intensity multiplier for the SSAO occlusion mask.
        /// </summary>
        /// <value>The intensity. Default is 1.0f.</value>
        public float SSAOIntensity = 1.0f;

        [Header("SSGI")]
        /// <summary>
        /// Number of sample points used by the SSGI kernel.
        /// </summary>
        /// <value>The sample count. Default is 12.</value>
        public int SSGISamples = 12;

        /// <summary>
        /// World-space radius of the SSGI sampling sphere.
        /// </summary>
        /// <value>The radius in world units. Default is 1.8f.</value>
        public float SSGIRadius = 1.8f;

        /// <summary>
        /// Intensity multiplier for the SSGI indirect lighting contribution.
        /// </summary>
        /// <value>The intensity. Default is 0.45f.</value>
        public float SSGIIntensity = 0.45f;

        [Header("ACES")]
        /// <summary>
        /// Exposure value applied before ACES tone mapping.
        /// </summary>
        /// <value>The exposure. Default is 1.0f (no change).</value>
        public float Exposure = 1.0f;

        [Header("Vignette")]
        /// <summary>
        /// Normalized screen-space center of the vignette effect.
        /// </summary>
        /// <value>The center position. Default is (0.5, 0.5).</value>
        public Vector2 VignetteCenter = new Vector2 { x = 0.5f, y = 0.5f };

        /// <summary>
        /// Strength of the vignette darkening.
        /// </summary>
        /// <value>The intensity. Default is 0.45f.</value>
        public float VignetteIntensity = 0.45f;

        /// <summary>
        /// Falloff smoothness of the vignette edge.
        /// </summary>
        /// <value>The smoothness. Default is 0.6f.</value>
        public float VignetteSmoothness = 0.6f;

        /// <summary>
        /// Roundness factor of the vignette shape (0 = square, 1 = circular).
        /// </summary>
        /// <value>The roundness. Default is 1.0f.</value>
        public float VignetteRoundness = 1.0f;

        [Header("Chromatic Aberration")]
        /// <summary>
        /// Strength of the RGB channel separation.
        /// </summary>
        /// <value>The intensity. Default is 0.15f.</value>
        public float ChromaticAberrationIntensity = 0.15f;

        [Header("LUT")]
        /// <summary>
        /// The color grading Look-Up Table texture.
        /// Set to <see langword="null"/> to disable LUT grading.
        /// </summary>
        /// <value>The LUT <see cref="Texture2D"/>.</value>
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
        internal bool _ssaoSupported;
        internal bool _ssgiSupported;
        internal bool _bloomSupported;
        internal bool _acesSupported;
        internal bool _vignetteSupported;
        internal bool _chromaticAberrationSupported;
        internal bool _lutSupported;

        /// <summary>
        /// Injectable shader-availability provider. Defaults to the production
        /// implementation; replace in tests via <see cref="SetAvailabilityProvider"/>.
        /// </summary>
        IShaderAvailabilityProvider _availabilityProvider;

        /// <summary>
        /// Composio-style pass registry. Replace or extend at runtime to add
        /// custom post-processing passes without modifying <see cref="PostStack"/>.
        /// </summary>
        /// <value>The current pass registry.</value>
        public PostFxPassRegistry PassRegistry { get; private set; } = PostFxPassRegistry.CreateDefault();

        /// <summary>
        /// Initializes a new instance of the <see cref="PostStack"/> class.
        /// Binds the owner to <see cref="PassRegistry"/> so providers that cache
        /// the owner (e.g. <see cref="BlitPassProvider"/>) have it before
        /// <c>Awake</c> or <c>InitMaterials</c> run.
        /// </summary>
        public PostStack()
        {
            PassRegistry.Init(this);
        }

        // ------------------------------------------------------------------
        // Public API for test injection
        // ------------------------------------------------------------------

        /// <summary>
        /// Overrides the shader-availability provider used by
        /// <see cref="ValidateShaderVariants"/>.
        /// </summary>
        /// <param name="provider">The provider to use.</param>
        /// <exception cref="System.ArgumentNullException">
        /// Thrown when <paramref name="provider"/> is <see langword="null"/>.
        /// </exception>
        /// <remarks>
        /// Call before <c>Awake</c> or immediately after construction in tests.
        /// </remarks>
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
            PassRegistry.Dispose();
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

        internal void ApplySSAOParams()
        {
            _ssaoMat.SetInt("_SampleCount", SSAOSamples);
            _ssaoMat.SetVectorArray("_Samples", _ssaoKernel);
            _ssaoMat.SetFloat("_Radius", SSAORadius);
            _ssaoMat.SetFloat("_Bias", SSAOBias);
            _ssaoMat.SetFloat("_Intensity", SSAOIntensity);
        }

        internal void ApplySSGIParams()
        {
            _ssgiMat.SetInt("_SampleCount", SSGISamples);
            _ssgiMat.SetVectorArray("_Samples", _ssgiKernel);
            _ssgiMat.SetFloat("_Radius", SSGIRadius);
            _ssgiMat.SetFloat("_Intensity", SSGIIntensity);
        }

        internal void ApplyACESParams()
        {
            _acesMat.SetFloat(ExposureId, Exposure);
        }

        internal void ApplyVignetteParams()
        {
            _vignetteMat.SetVector("_Center", new Vector4(VignetteCenter.x, VignetteCenter.y, 0f, 0f));
            _vignetteMat.SetFloat("_Intensity", VignetteIntensity);
            _vignetteMat.SetFloat("_Smoothness", VignetteSmoothness);
            _vignetteMat.SetFloat("_Roundness", VignetteRoundness);
        }

        internal void ApplyChromaticAberrationParams()
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

            if (!PassRegistry.HasAnyActivePass(this))
            {
                Graphics.Blit(src, dst);
                return;
            }

            EnsurePingPong(src);
            try
            {
                RenderTexture cur = src, next = _ping;

                foreach (var effect in PassRegistry.Providers)
                {
                    if (!effect.IsEnabled(this) || !effect.IsSupported(this))
                        continue;

                    // LUT is applied as a final composite pass to dst
                    if (effect.Effect == PostFxEffect.LUT)
                    {
                        if (LutTexture != null && effect.Material != null)
                            Graphics.Blit(cur, dst, effect.Material);
                        else
                            Graphics.Blit(cur, dst);
                        continue;
                    }

                    effect.ApplyParams(this);
                    if (effect.Render(cur, next))
                    {
                        Swap(ref cur, ref next);
                    }
                }

                // If LUT was not active, blit the final result to dst
                var lutProvider = PassRegistry.GetProvider(PostFxEffect.LUT);
                if (lutProvider == null || !lutProvider.IsEnabled(this) || !lutProvider.IsSupported(this) || LutTexture == null)
                {
                    Graphics.Blit(cur, dst);
                }
            }
            finally
            {
                ReleasePingPong();
            }
        }

        static void Swap(ref RenderTexture a, ref RenderTexture b) => (a, b) = (b, a);
    }
}
