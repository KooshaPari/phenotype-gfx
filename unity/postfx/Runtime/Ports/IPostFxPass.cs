// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! IPostFxPass — hexagonal port trait for post-processing passes.
//!
//! Every post-fx effect (Bloom, ACES, LUT, SSAO, SSGI, etc.) is an adapter
//! that implements this trait.  The PostStack driver calls them in order
//! without knowing the concrete type.
//!
//! Reference: phenotype-infra/REUSE.toml (T20), phenotype-voxel/src/ports/* (T2 SSOT pattern).

using System;
using UnityEngine;

namespace Phenotype.PostFx.Ports
{
    /// <summary>
    /// The hexagonal port for a single post-fx pass.
    /// Adapters (BrpBloomPass, UrpAcesPass, etc.) implement this trait.
    /// </summary>
    public interface IPostFxPass
    {
        /// <summary>
        /// Gets the stable name used for ordering, logging, and profiling.
        /// </summary>
        /// <value>The pass name.</value>
        string Name { get; }

        /// <summary>
        /// Gets the relative cost hint (0.0 = free, 1.0 = full-frame).
        /// Used by the scheduler to prioritize or cull passes.
        /// </summary>
        /// <value>A normalized cost value between 0.0 and 1.0.</value>
        float Cost { get; }

        /// <summary>
        /// Gets a value indicating whether this pass should run on the current frame
        /// (e.g., based on a toggle or quality setting).
        /// </summary>
        /// <value>
        /// <see langword="true"/> if the pass is enabled; otherwise, <see langword="false"/>.
        /// </value>
        bool IsEnabled { get; }

        /// <summary>
        /// Builds pass-specific material and camera target allocations.
        /// </summary>
        /// <param name="ctx">Per-camera context containing source, destination, camera, and quality settings.</param>
        /// <remarks>Called once per camera.</remarks>
        void OnSetup(PostFxContext ctx);

        /// <summary>
        /// Renders the pass into the supplied source texture.
        /// </summary>
        /// <param name="ctx">Per-camera context containing source and destination render targets.</param>
        /// <remarks>Output is written to <c>ctx.Destination</c>.</remarks>
        void OnRender(PostFxContext ctx);

        /// <summary>
        /// Frees materials and temporary targets owned by this pass.
        /// </summary>
        /// <remarks>Called when the camera is destroyed or the pass is disabled.</remarks>
        void OnDispose();

        /// <summary>
        /// Validates that all required shader variants are present.
        /// </summary>
        /// <param name="shaderProvider">Provider to query for shader availability.</param>
        /// <exception cref="InvalidOperationException">
        /// Thrown when a required shader variant is missing.
        /// </exception>
        void ValidateVariants(IShaderAvailabilityProvider shaderProvider);
    }

    /// <summary>
    /// Per-camera context passed to each pass. Immutable from the pass's perspective
    /// (the PostStack driver swaps the source/destination pair between passes).
    /// </summary>
    public sealed class PostFxContext
    {
        /// <summary>
        /// Gets the source render texture for this pass.
        /// </summary>
        /// <value>The source texture.</value>
        public RenderTexture Source { get; }

        /// <summary>
        /// Gets the destination render texture for this pass.
        /// </summary>
        /// <value>The destination texture.</value>
        public RenderTexture Destination { get; }

        /// <summary>
        /// Gets the camera being rendered.
        /// </summary>
        /// <value>The active camera.</value>
        public Camera Camera { get; }

        /// <summary>
        /// Gets the current quality setting.
        /// </summary>
        /// <value>The quality level.</value>
        public PostFxQuality Quality { get; }

        /// <summary>
        /// Gets the material property block for this pass.
        /// </summary>
        /// <value>The property block.</value>
        public MaterialPropertyBlock PropertyBlock { get; }

        /// <summary>
        /// Initializes a new instance of the <see cref="PostFxContext"/> class.
        /// </summary>
        /// <param name="source">The source render texture.</param>
        /// <param name="destination">The destination render texture.</param>
        /// <param name="camera">The camera being rendered.</param>
        /// <param name="quality">The current quality setting.</param>
        /// <param name="propertyBlock">The material property block.</param>
        /// <exception cref="ArgumentNullException">
        /// Thrown when <paramref name="source"/>, <paramref name="destination"/>,
        /// or <paramref name="camera"/> is <see langword="null"/>.
        /// </exception>
        public PostFxContext(
            RenderTexture source,
            RenderTexture destination,
            Camera camera,
            PostFxQuality quality,
            MaterialPropertyBlock propertyBlock)
        {
            Source = source ?? throw new ArgumentNullException(nameof(source));
            Destination = destination ?? throw new ArgumentNullException(nameof(destination));
            Camera = camera ?? throw new ArgumentNullException(nameof(camera));
            Quality = quality;
            PropertyBlock = propertyBlock ?? new MaterialPropertyBlock();
        }
    }

    /// <summary>Quality settings — the driver passes the current value to each pass.</summary>
    [Flags]
    public enum PostFxQuality : byte
    {
        /// <summary>Effect is disabled.</summary>
        Off        = 0,
        /// <summary>Low quality (fastest, minimal samples).</summary>
        Low        = 1 << 0,
        /// <summary>Medium quality (balanced performance).</summary>
        Medium     = 1 << 1,
        /// <summary>High quality (more samples, better visuals).</summary>
        High       = 1 << 2,
        /// <summary>Ultra quality (maximum samples, best visuals).</summary>
        Ultra      = 1 << 3,
        /// <summary>All quality levels combined.</summary>
        All        = Low | Medium | High | Ultra
    }

    /// <summary>
    /// Adapter that wraps the legacy <c>IPostFxPassProvider</c> (in PostFxPassRegistry.cs)
    /// so it can also participate in the new hexagonal <c>IPostFxPass</c> registry.
    /// This is the bridge for the T21 migration — existing providers don't need
    /// to be rewritten, just wrapped.
    /// </summary>
    public sealed class PostFxPassProviderAdapter : IPostFxPass
    {
        private readonly IPostFxPassProvider _provider;
        private readonly PostStack _owner;

        /// <summary>
        /// Gets the display name of the wrapped provider.
        /// </summary>
        /// <value>The pass name.</value>
        public string Name => _provider.DisplayName;

        /// <summary>
        /// Gets the relative cost of the wrapped provider.
        /// </summary>
        /// <value>A conservative default of 0.1; can be tuned per-effect.</value>
        public float Cost => 0.1f; // Conservative default; can be tuned per-effect.

        /// <summary>
        /// Gets a value indicating whether the wrapped provider is enabled.
        /// </summary>
        /// <value>
        /// <see langword="true"/> if the provider is enabled for the owner; otherwise, <see langword="false"/>.
        /// </value>
        public bool IsEnabled => _provider.IsEnabled(_owner);

        /// <summary>
        /// Initializes a new instance of the <see cref="PostFxPassProviderAdapter"/> class.
        /// </summary>
        /// <param name="provider">The legacy provider to wrap.</param>
        /// <param name="owner">The <see cref="PostStack"/> that owns this pass.</param>
        /// <exception cref="ArgumentNullException">
        /// Thrown when <paramref name="provider"/> or <paramref name="owner"/> is <see langword="null"/>.
        /// </exception>
        public PostFxPassProviderAdapter(IPostFxPassProvider provider, PostStack owner)
        {
            _provider = provider ?? throw new ArgumentNullException(nameof(provider));
            _owner = owner ?? throw new ArgumentNullException(nameof(owner));
            _provider.Init(owner);
        }

        /// <summary>
        /// No-op. Providers handle their own setup state.
        /// </summary>
        /// <param name="ctx">Per-camera context (ignored).</param>
        public void OnSetup(PostFxContext ctx) { /* providers handle their own state */ }

        /// <summary>
        /// Delegates rendering to the wrapped provider.
        /// </summary>
        /// <param name="ctx">Per-camera context containing source and destination render targets.</param>
        public void OnRender(PostFxContext ctx) => _provider.Render(ctx.Source, ctx.Destination);

        /// <summary>
        /// No-op. Providers own their materials.
        /// </summary>
        public void OnDispose() { /* providers own their materials */ }

        /// <summary>
        /// Validates that the wrapped provider is supported.
        /// </summary>
        /// <param name="p">The shader availability provider to query.</param>
        /// <exception cref="InvalidOperationException">
        /// Thrown when the wrapped provider is not supported for the owner.
        /// </exception>
        public void ValidateVariants(IShaderAvailabilityProvider p)
        {
            if (!_provider.IsSupported(_owner))
                throw new InvalidOperationException($"Pass not supported: {Name}");
        }
    }
}
