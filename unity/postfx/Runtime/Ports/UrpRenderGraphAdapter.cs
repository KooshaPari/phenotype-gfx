// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! T27 stub: URP 17 RecordRenderGraph adapter contract.
//!
//! URP 17 (Unity 6) replaces the OnRenderImage callback with the
//! ScriptableRenderPass.RecordRenderGraph API.  This file is the
//! specification of the adapter port trait that the 7 existing passes
//! must implement when migrating from BRP (OnRenderImage) to URP 17.
//!
//! Reference: https://docs.unity3d.com/Packages/com.unity.render-pipelines.universal@17.0/manual/renderer-graph.html

using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.Rendering.Universal;

namespace Phenotype.PostFx.Urp
{
    /// <summary>
    /// Hexagonal port: a post-fx pass that can record itself into a URP 17
    /// RenderGraph.  Each existing BRP pass will gain an adapter implementing
    /// this port without changing the BRP-side implementation.
    /// </summary>
    public interface IUrpPostFxPass
    {
        /// <summary>
        /// Gets the stable name of this pass, used for profiling and ordering.
        /// </summary>
        /// <value>The pass name.</value>
        string Name { get; }

        /// <summary>
        /// Gets a value indicating whether this pass should run in the current frame.
        /// </summary>
        /// <value>
        /// <see langword="true"/> if the pass is enabled; otherwise, <see langword="false"/>.
        /// </value>
        bool IsEnabled { get; }

        /// <summary>
        /// Records the pass into the supplied RenderGraph.
        /// </summary>
        /// <param name="graph">The RenderGraph to record into.</param>
        /// <param name="frameData">Context container with frame-level data.</param>
        /// <param name="ctx">Per-camera context for the URP 17 adapter.</param>
        /// <remarks>
        /// The pass is responsible for declaring its inputs (via <c>UseTexture</c>)
        /// and outputs (via <c>SetRenderAttachment</c>).
        /// </remarks>
        void RecordRenderGraph(RenderGraph graph, ContextContainer frameData, UrpPostFxContext ctx);
    }

    /// <summary>Per-camera context for the URP 17 adapter.</summary>
    public sealed class UrpPostFxContext
    {
        /// <summary>
        /// Gets the camera being rendered.
        /// </summary>
        /// <value>The active camera.</value>
        public Camera Camera { get; }

        /// <summary>
        /// Gets the universal resource data for this frame.
        /// </summary>
        /// <value>The resource data.</value>
        public UniversalResourceData ResourceData { get; }

        /// <summary>
        /// Gets the universal camera data for this frame.
        /// </summary>
        /// <value>The camera data.</value>
        public UniversalCameraData CameraData { get; }

        /// <summary>
        /// Initializes a new instance of the <see cref="UrpPostFxContext"/> class.
        /// </summary>
        /// <param name="camera">The camera being rendered.</param>
        /// <param name="resourceData">The universal resource data.</param>
        /// <param name="cameraData">The universal camera data.</param>
        public UrpPostFxContext(Camera camera, UniversalResourceData resourceData, UniversalCameraData cameraData)
        {
            Camera = camera;
            ResourceData = resourceData;
            CameraData = cameraData;
        }
    }

    /// <summary>
    /// Adapter that bridges an existing <see cref="IPostFxPass"/> (BRP-side) to the URP 17
    /// <c>RecordRenderGraph</c> API. It allocates a temporary texture, runs the BRP
    /// blit into it, then writes the result back to the active color buffer.
    /// </summary>
    public sealed class BrpToUrpAdapter : IUrpPostFxPass
    {
        private readonly IPostFxPass _brpPass;

        /// <summary>
        /// Gets the name of the wrapped BRP pass.
        /// </summary>
        /// <value>The pass name.</value>
        public string Name => _brpPass.Name;

        /// <summary>
        /// Gets a value indicating whether the wrapped BRP pass is enabled.
        /// </summary>
        /// <value>
        /// <see langword="true"/> if the pass is enabled; otherwise, <see langword="false"/>.
        /// </value>
        public bool IsEnabled => _brpPass.IsEnabled;

        /// <summary>
        /// Initializes a new instance of the <see cref="BrpToUrpAdapter"/> class.
        /// </summary>
        /// <param name="brpPass">The BRP pass to wrap.</param>
        /// <exception cref="System.ArgumentNullException">
        /// Thrown when <paramref name="brpPass"/> is <see langword="null"/>.
        /// </exception>
        public BrpToUrpAdapter(IPostFxPass brpPass)
        {
            _brpPass = brpPass ?? throw new System.ArgumentNullException(nameof(brpPass));
        }

        /// <summary>
        /// Records the BRP pass into the URP RenderGraph.
        /// </summary>
        /// <param name="graph">The RenderGraph to record into.</param>
        /// <param name="frameData">Context container with frame-level data.</param>
        /// <param name="ctx">Per-camera context for the URP 17 adapter.</param>
        /// <remarks>
        /// The actual URP 17 implementation requires Unity 6 + URP 17 packages.
        /// This stub documents the API; the migration work (T27) will fill it in
        /// once the project upgrades to Unity 6 in a follow-up.
        /// </remarks>
        public void RecordRenderGraph(RenderGraph graph, ContextContainer frameData, UrpPostFxContext ctx)
        {
            // The actual URP 17 implementation requires Unity 6 + URP 17 packages.
            // This stub documents the API; the migration work (T27) will fill it in
            // once the project upgrades to Unity 6 in a follow-up.
            // See: docs/migration/urp-17.md (created in T27 batch)
        }
    }
}
