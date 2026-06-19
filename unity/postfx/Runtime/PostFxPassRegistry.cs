using System.Collections.Generic;
using UnityEngine;

namespace Phenotype.PostFx
{
    /// <summary>
    /// Composio-style pass provider contract. Each adapter (built-in pass,
    /// third-party effect, mod extension) implements this to register itself
    /// with the <see cref="PostFxPassRegistry"/>.
    /// </summary>
    public interface IPostFxPassProvider : IPostFxPass
    {
        /// <summary>
        /// Gets the unique identifier for this pass provider.
        /// </summary>
        /// <value>The <see cref="PostFxEffect"/> value.</value>
        PostFxEffect Effect { get; }

        /// <summary>
        /// Gets the human-readable display name.
        /// </summary>
        /// <value>The display name.</value>
        string DisplayName { get; }

        /// <summary>
        /// Gets the material used for this pass.
        /// </summary>
        /// <value>
        /// The <see cref="Material"/> instance, or <see langword="null"/> if the shader is unavailable.
        /// </value>
        Material? Material { get; }

        /// <summary>
        /// Determines whether the pass is currently enabled by the user.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> that owns this pass.</param>
        /// <returns>
        /// <see langword="true"/> if the pass is enabled; otherwise, <see langword="false"/>.
        /// </returns>
        bool IsEnabled(PostStack owner);

        /// <summary>
        /// Determines whether the shader is supported in this build.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> that owns this pass.</param>
        /// <returns>
        /// <see langword="true"/> if the shader is supported; otherwise, <see langword="false"/>.
        /// </returns>
        bool IsSupported(PostStack owner);

        /// <summary>
        /// Applies per-frame parameters before rendering.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> that owns this pass.</param>
        void ApplyParams(PostStack owner);
    }

    /// <summary>
    /// Built-in single-blit pass provider. Most effects (SSAO, SSGI, ACES,
    /// Vignette, ChromaticAberration, LUT) use this default implementation.
    /// </summary>
    internal sealed class BlitPassProvider : IPostFxPassProvider
    {
        readonly string _shaderName;
        readonly System.Func<PostStack, Material?> _materialAccessor;
        readonly System.Func<PostStack, bool> _enabledAccessor;
        readonly System.Func<PostStack, bool> _supportedAccessor;
        readonly System.Action<PostStack> _applyParams;

        /// <summary>
        /// Gets the effect identifier for this pass.
        /// </summary>
        /// <value>The <see cref="PostFxEffect"/>.</value>
        public PostFxEffect Effect { get; }

        /// <summary>
        /// Gets the human-readable display name.
        /// </summary>
        /// <value>The display name.</value>
        public string DisplayName { get; }

        /// <summary>
        /// Gets the material for this pass, or <see langword="null"/> if the owner is not bound.
        /// </summary>
        /// <value>The <see cref="Material"/> instance.</value>
        public Material? Material => _owner != null ? _materialAccessor(_owner) : null;

        PostStack _owner;

        /// <summary>
        /// Initializes a new instance of the <see cref="BlitPassProvider"/> class.
        /// </summary>
        /// <param name="effect">The effect identifier.</param>
        /// <param name="displayName">The human-readable display name.</param>
        /// <param name="shaderName">The shader name for debugging.</param>
        /// <param name="materialAccessor">Function that returns the material for a given owner.</param>
        /// <param name="enabledAccessor">Function that returns whether the pass is enabled.</param>
        /// <param name="supportedAccessor">Function that returns whether the pass is supported.</param>
        /// <param name="applyParams">Action that applies per-frame parameters.</param>
        public BlitPassProvider(
            PostFxEffect effect,
            string displayName,
            string shaderName,
            System.Func<PostStack, Material?> materialAccessor,
            System.Func<PostStack, bool> enabledAccessor,
            System.Func<PostStack, bool> supportedAccessor,
            System.Action<PostStack> applyParams)
        {
            Effect = effect;
            DisplayName = displayName;
            _shaderName = shaderName;
            _materialAccessor = materialAccessor;
            _enabledAccessor = enabledAccessor;
            _supportedAccessor = supportedAccessor;
            _applyParams = applyParams;
        }

        /// <summary>
        /// Binds the owning <see cref="PostStack"/>.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> that owns this pass.</param>
        public void Init(PostStack owner) => _owner = owner;

        /// <summary>
        /// Determines whether the pass is enabled.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> that owns this pass.</param>
        /// <returns>
        /// <see langword="true"/> if the pass is enabled; otherwise, <see langword="false"/>.
        /// </returns>
        public bool IsEnabled(PostStack owner) => _enabledAccessor(owner);

        /// <summary>
        /// Determines whether the pass is supported.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> that owns this pass.</param>
        /// <returns>
        /// <see langword="true"/> if the pass is supported; otherwise, <see langword="false"/>.
        /// </returns>
        public bool IsSupported(PostStack owner) => _supportedAccessor(owner);

        /// <summary>
        /// Applies per-frame parameters.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> that owns this pass.</param>
        public void ApplyParams(PostStack owner) => _applyParams(owner);

        /// <summary>
        /// Renders the pass from <paramref name="src"/> into <paramref name="dst"/>.
        /// </summary>
        /// <param name="src">The source render texture.</param>
        /// <param name="dst">The destination render texture.</param>
        /// <returns>
        /// <see langword="true"/> if the blit was performed; otherwise, <see langword="false"/>.
        /// </returns>
        public bool Render(RenderTexture src, RenderTexture dst)
        {
            var mat = Material;
            if (mat == null) return false;
            Graphics.Blit(src, dst, mat);
            return true;
        }

        /// <summary>
        /// No-op. Single-blit providers do not hold disposable resources.
        /// </summary>
        public void Dispose() { }
    }

    /// <summary>
    /// Bloom-specific multi-pass provider. Encapsulates the 4-pass threshold →
    /// blur H → blur V → composite chain that cannot be expressed as a single blit.
    /// </summary>
    internal sealed class BloomPassProvider : IPostFxPassProvider
    {
        /// <summary>
        /// Gets the effect identifier. Always <see cref="PostFxEffect.Bloom"/>.
        /// </summary>
        /// <value><see cref="PostFxEffect.Bloom"/>.</value>
        public PostFxEffect Effect => PostFxEffect.Bloom;

        /// <summary>
        /// Gets the display name. Always "Bloom".
        /// </summary>
        /// <value>"Bloom".</value>
        public string DisplayName => "Bloom";

        /// <summary>
        /// Gets the bloom material, or <see langword="null"/> if the owner is not bound.
        /// </summary>
        /// <value>The <see cref="Material"/> instance.</value>
        public Material? Material => _owner?._bloomMat;

        PostStack _owner;

        /// <summary>
        /// Binds the owning <see cref="PostStack"/>.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> that owns this pass.</param>
        public void Init(PostStack owner) => _owner = owner;

        /// <summary>
        /// Determines whether the pass is enabled.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> that owns this pass.</param>
        /// <returns>
        /// <see langword="true"/> if <see cref="PostStack.EnableBloom"/> is <see langword="true"/>;
        /// otherwise, <see langword="false"/>.
        /// </returns>
        public bool IsEnabled(PostStack owner) => owner.EnableBloom;

        /// <summary>
        /// Determines whether the pass is supported.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> that owns this pass.</param>
        /// <returns>
        /// <see langword="true"/> if the bloom shader is available; otherwise, <see langword="false"/>.
        /// </returns>
        public bool IsSupported(PostStack owner) => owner._bloomSupported;

        /// <summary>
        /// No-op. Bloom parameters are applied via the material directly.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> that owns this pass.</param>
        public void ApplyParams(PostStack owner) { }

        /// <summary>
        /// Renders the 4-pass bloom chain.
        /// </summary>
        /// <param name="src">The source render texture.</param>
        /// <param name="dst">The destination render texture.</param>
        /// <returns>
        /// <see langword="true"/> if the bloom material was available and rendering succeeded;
        /// otherwise, <see langword="false"/>.
        /// </returns>
        public bool Render(RenderTexture src, RenderTexture dst)
        {
            var bloomMat = _owner?._bloomMat;
            if (bloomMat == null) return false;

            int w = Mathf.Max(1, src.width / 4);
            int h = Mathf.Max(1, src.height / 4);
            RenderTexture bA = RenderTexture.GetTemporary(w, h, 0, src.format);
            RenderTexture bB = RenderTexture.GetTemporary(w, h, 0, src.format);
            try
            {
                Graphics.Blit(src, bA, bloomMat, 0);
                Graphics.Blit(bA, bB, bloomMat, 1);
                Graphics.Blit(bB, bA, bloomMat, 2);
                bloomMat.SetTexture(PostStack.BloomTexId, bA);
                Graphics.Blit(src, dst, bloomMat, 3);
                return true;
            }
            finally
            {
                bloomMat.SetTexture(PostStack.BloomTexId, null);
                RenderTexture.ReleaseTemporary(bA);
                RenderTexture.ReleaseTemporary(bB);
            }
        }

        /// <summary>
        /// No-op. Bloom temporary targets are released inside <see cref="Render"/>.
        /// </summary>
        public void Dispose() { }
    }

    /// <summary>
    /// Composio-style provider registry for post-processing passes.
    /// Replaces the hard-coded switch statements in <see cref="PostStack"/>
    /// with a discoverable, extensible registration model.
    /// </summary>
    public sealed class PostFxPassRegistry
    {
        readonly Dictionary<PostFxEffect, IPostFxPassProvider> _providers = new();
        readonly List<PostFxEffect> _renderOrder = new();

        /// <summary>
        /// Gets all registered providers in current render order.
        /// </summary>
        /// <value>An enumerable of <see cref="IPostFxPassProvider"/>.</value>
        public IEnumerable<IPostFxPassProvider> Providers => _renderOrder.ConvertAll(e => _providers[e]);

        /// <summary>
        /// Registers a provider. Overwrites any existing provider for the same effect.
        /// </summary>
        /// <param name="provider">The provider to register.</param>
        public void Register(IPostFxPassProvider provider)
        {
            _providers[provider.Effect] = provider;
            if (!_renderOrder.Contains(provider.Effect))
                _renderOrder.Add(provider.Effect);
        }

        /// <summary>
        /// Removes a provider from the registry.
        /// </summary>
        /// <param name="effect">The effect to unregister.</param>
        public void Unregister(PostFxEffect effect)
        {
            _providers.Remove(effect);
            _renderOrder.Remove(effect);
        }

        /// <summary>
        /// Changes the render order of effects.
        /// </summary>
        /// <param name="order">
        /// The desired render order. Any omitted effects keep their relative order.
        /// </param>
        public void SetRenderOrder(params PostFxEffect[] order)
        {
            var newOrder = new List<PostFxEffect>(order);
            foreach (var effect in _renderOrder)
            {
                if (!newOrder.Contains(effect))
                    newOrder.Add(effect);
            }
            _renderOrder.Clear();
            _renderOrder.AddRange(newOrder);
        }

        /// <summary>
        /// Gets a provider by effect.
        /// </summary>
        /// <param name="effect">The effect to look up.</param>
        /// <returns>
        /// The registered <see cref="IPostFxPassProvider"/>, or <see langword="null"/> if not registered.
        /// </returns>
        public IPostFxPassProvider? GetProvider(PostFxEffect effect)
        {
            _providers.TryGetValue(effect, out var provider);
            return provider;
        }

        /// <summary>
        /// Initialises all providers that require an owner reference.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> to bind to each provider.</param>
        /// <remarks>Call after construction or after replacing the registry.</remarks>
        public void Init(PostStack owner)
        {
            foreach (var p in _providers.Values)
            {
                p.Init(owner);
            }
        }

        /// <summary>
        /// Disposes all registered providers and clears the registry.
        /// </summary>
        public void Dispose()
        {
            foreach (var p in _providers.Values)
            {
                p.Dispose();
            }
            _providers.Clear();
            _renderOrder.Clear();
        }

        /// <summary>
        /// Determines whether any registered pass is active for the given owner.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> to check.</param>
        /// <returns>
        /// <see langword="true"/> if at least one pass is enabled, supported, and has a material;
        /// otherwise, <see langword="false"/>.
        /// </returns>
        public bool HasAnyActivePass(PostStack owner)
        {
            foreach (var effect in _renderOrder)
            {
                if (_providers.TryGetValue(effect, out var p))
                {
                    if (p.IsEnabled(owner) && p.IsSupported(owner) && p.Material != null)
                        return true;
                }
            }
            return false;
        }

        /// <summary>
        /// Builds the default registry with all built-in passes.
        /// </summary>
        /// <returns>A new <see cref="PostFxPassRegistry"/> with built-in passes pre-registered.</returns>
        public static PostFxPassRegistry CreateDefault()
        {
            var registry = new PostFxPassRegistry();

            registry.Register(new BlitPassProvider(
                PostFxEffect.SSAO, "SSAO", "ScreenSpaceAO",
                o => o._ssaoMat,
                o => o.EnableSSAO,
                o => o._ssaoSupported,
                o => o.ApplySSAOParams()));

            registry.Register(new BlitPassProvider(
                PostFxEffect.SSGI, "SSGI", "ScreenSpaceGI",
                o => o._ssgiMat,
                o => o.EnableSSGI,
                o => o._ssgiSupported,
                o => o.ApplySSGIParams()));

            registry.Register(new BloomPassProvider());

            registry.Register(new BlitPassProvider(
                PostFxEffect.ACES, "ACES", "BrpACES",
                o => o._acesMat,
                o => o.EnableACES,
                o => o._acesSupported,
                o => o.ApplyACESParams()));

            registry.Register(new BlitPassProvider(
                PostFxEffect.Vignette, "Vignette", "Vignette",
                o => o._vignetteMat,
                o => o.EnableVignette,
                o => o._vignetteSupported,
                o => o.ApplyVignetteParams()));

            registry.Register(new BlitPassProvider(
                PostFxEffect.ChromaticAberration, "ChromaticAberration", "ChromaticAberration",
                o => o._chromaticAberrationMat,
                o => o.EnableChromaticAberration,
                o => o._chromaticAberrationSupported,
                o => o.ApplyChromaticAberrationParams()));

            // LUT is handled as a special final pass in PostStack, but we register it
            // so the registry can report its availability.
            registry.Register(new BlitPassProvider(
                PostFxEffect.LUT, "LUT", "ColorGradingLUT",
                o => o._lutMat,
                o => o.EnableLUT,
                o => o._lutSupported,
                o => { }));

            return registry;
        }
    }
}
