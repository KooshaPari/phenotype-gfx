// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! IMaterialRegistry — hexagonal port for post-fx material / shader-asset lookup.
//!
//! Each post-fx pass needs a `Material` instance whose shader is loaded with the
//! correct variant keywords.  This port abstracts the asset-loading backend
//! (Resources, Addressables, AssetBundle, …) so the pass can stay engine-agnostic.
//!
//! Reference: phenotype-voxel/src/ports/material.rs (Rust port), phenotype-infra/REUSE.toml (T20).

using System;
using System.Collections.Generic;

namespace Phenotype.PostFx.Ports
{
    /// <summary>
    /// Logical classification of a managed post-fx material asset.
    /// </summary>
    public enum PostFxMaterialKind
    {
        /// <summary>Generic copy / passthrough material (used by several passes).</summary>
        Copy = 0,
        /// <summary>Bloom prefilter / downsample / upscale material.</summary>
        Bloom = 1,
        /// <summary>Color grading (LUT apply) material.</summary>
        ColorGrade = 2,
        /// <summary>Ambient-occlusion / SSAO composite material.</summary>
        Ssao = 3,
        /// <summary>Tonemap / ACES material.</summary>
        Tonemap = 4,
        /// <summary>Anything not covered above — escape hatch.</summary>
        Other = 99,
    }

    /// <summary>
    /// Metadata for a single managed post-fx material.
    /// </summary>
    public sealed class PostFxMaterialInfo
    {
        /// <summary>
        /// Stable id of this material (e.g. <c>"bloom-prefilter-v1"</c>).
        /// </summary>
        public string Id { get; }

        /// <summary>
        /// Logical kind of this material.
        /// </summary>
        public PostFxMaterialKind Kind { get; }

        /// <summary>
        /// Addressable key, AssetBundle path, or Resources path.
        /// </summary>
        public string AssetPath { get; }

        /// <summary>
        /// Required shader-keyword variants that must be enabled.
        /// </summary>
        public IReadOnlyList<string> RequiredKeywords { get; }

        /// <summary>
        /// Initializes a new instance of the <see cref="PostFxMaterialInfo"/> class.
        /// </summary>
        /// <param name="id">Stable id of the material.</param>
        /// <param name="kind">Logical kind.</param>
        /// <param name="assetPath">Addressable / Resources path.</param>
        /// <param name="requiredKeywords">Required shader keywords (may be empty).</param>
        /// <exception cref="ArgumentNullException">Thrown when <paramref name="id"/> or <paramref name="assetPath"/> is null.</exception>
        public PostFxMaterialInfo(
            string id,
            PostFxMaterialKind kind,
            string assetPath,
            IReadOnlyList<string>? requiredKeywords = null)
        {
            Id = id ?? throw new ArgumentNullException(nameof(id));
            Kind = kind;
            AssetPath = assetPath ?? throw new ArgumentNullException(nameof(assetPath));
            RequiredKeywords = requiredKeywords ?? Array.Empty<string>();
        }
    }

    /// <summary>
    /// Hexagonal port: post-fx material / asset registry.
    /// Adapters include <see cref="InMemoryMaterialRegistry"/>, the future
    /// <c>AddressablesMaterialRegistry</c>, and the <c>AssetBundleMaterialRegistry</c>.
    /// </summary>
    public interface IMaterialRegistry
    {
        /// <summary>
        /// Returns all materials currently registered.
        /// </summary>
        /// <returns>Read-only view of the registry contents.</returns>
        IReadOnlyList<PostFxMaterialInfo> List();

        /// <summary>
        /// Looks up a material by id.
        /// </summary>
        /// <param name="id">Stable id of the material.</param>
        /// <returns>The matching <see cref="PostFxMaterialInfo"/>, or <see langword="null"/> if absent.</returns>
        PostFxMaterialInfo? Find(string id);

        /// <summary>
        /// Registers a material.  If an entry with the same id already exists, it is replaced.
        /// </summary>
        /// <param name="info">The material metadata to register.</param>
        /// <exception cref="ArgumentNullException">Thrown when <paramref name="info"/> is null.</exception>
        void Register(PostFxMaterialInfo info);

        /// <summary>
        /// Removes a material by id.
        /// </summary>
        /// <param name="id">Stable id of the material to remove.</param>
        /// <returns>
        /// <see langword="true"/> if the material was present and removed;
        /// otherwise, <see langword="false"/>.
        /// </returns>
        bool Unregister(string id);
    }

    /// <summary>
    /// Default in-memory adapter for <see cref="IMaterialRegistry"/>.
    /// Used by the editor stack, by tests, and as the canonical null-adapter
    /// when no engine asset system is wired in.
    /// </summary>
    public sealed class InMemoryMaterialRegistry : IMaterialRegistry
    {
        private readonly Dictionary<string, PostFxMaterialInfo> _byId =
            new Dictionary<string, PostFxMaterialInfo>(StringComparer.Ordinal);

        /// <inheritdoc/>
        public IReadOnlyList<PostFxMaterialInfo> List()
        {
            // Materialize to a list so callers can't mutate the internal map.
            var result = new List<PostFxMaterialInfo>(_byId.Count);
            foreach (var info in _byId.Values) result.Add(info);
            return result;
        }

        /// <inheritdoc/>
        public PostFxMaterialInfo? Find(string id)
        {
            if (id == null) return null;
            return _byId.TryGetValue(id, out var info) ? info : null;
        }

        /// <inheritdoc/>
        public void Register(PostFxMaterialInfo info)
        {
            if (info == null) throw new ArgumentNullException(nameof(info));
            _byId[info.Id] = info;
        }

        /// <inheritdoc/>
        public bool Unregister(string id)
        {
            if (id == null) return false;
            return _byId.Remove(id);
        }
    }

    /// <summary>
    /// Recording mock used by domain tests to assert on registry interaction
    /// order.  Each operation is logged to a list the test can replay.
    /// </summary>
    public sealed class RecordingMaterialRegistry : IMaterialRegistry
    {
        private readonly Dictionary<string, PostFxMaterialInfo> _byId =
            new Dictionary<string, PostFxMaterialInfo>(StringComparer.Ordinal);
        private readonly List<string> _calls = new List<string>();

        /// <summary>
        /// Returns the sequence of method names invoked on this mock.
        /// </summary>
        public IReadOnlyList<string> Calls => _calls;

        /// <summary>
        /// Resets the call log (keeps the registry contents intact).
        /// </summary>
        public void ResetCalls() => _calls.Clear();

        /// <inheritdoc/>
        public IReadOnlyList<PostFxMaterialInfo> List()
        {
            _calls.Add(nameof(List));
            var result = new List<PostFxMaterialInfo>(_byId.Count);
            foreach (var info in _byId.Values) result.Add(info);
            return result;
        }

        /// <inheritdoc/>
        public PostFxMaterialInfo? Find(string id)
        {
            _calls.Add($"{nameof(Find)}({id})");
            if (id == null) return null;
            return _byId.TryGetValue(id, out var info) ? info : null;
        }

        /// <inheritdoc/>
        public void Register(PostFxMaterialInfo info)
        {
            if (info == null) throw new ArgumentNullException(nameof(info));
            _calls.Add($"{nameof(Register)}({info.Id})");
            _byId[info.Id] = info;
        }

        /// <inheritdoc/>
        public bool Unregister(string id)
        {
            _calls.Add($"{nameof(Unregister)}({id})");
            if (id == null) return false;
            return _byId.Remove(id);
        }
    }

    /// <summary>
    /// Test fixture for <see cref="IMaterialRegistry"/> adapters.
    /// </summary>
    [NUnit.Framework.TestFixture]
    public class MaterialRegistryPortTests
    {
        /// <summary>
        /// FR-POSTFX-PORT-MATERIAL-000 — registering two materials and
        /// finding each by id round-trips.
        /// </summary>
        [NUnit.Framework.Test]
        public void InMemory_Register_And_Find_RoundTrip()
        {
            IMaterialRegistry reg = new InMemoryMaterialRegistry();
            reg.Register(new PostFxMaterialInfo(
                "bloom-prefilter-v1",
                PostFxMaterialKind.Bloom,
                "Shaders/PostFx/Bloom/Prefilter",
                new[] { "_BLOOM_HQ" }));
            reg.Register(new PostFxMaterialInfo(
                "tonemap-aces-v1",
                PostFxMaterialKind.Tonemap,
                "Shaders/PostFx/TonemapAces"));

            var found = reg.Find("bloom-prefilter-v1");
            NUnit.Framework.Assert.That(found, NUnit.Framework.Is.Not.Null);
            NUnit.Framework.Assert.That(found!.Kind, NUnit.Framework.Is.EqualTo(PostFxMaterialKind.Bloom));
            NUnit.Framework.Assert.That(found.RequiredKeywords, NUnit.Framework.Is.EquivalentTo(new[] { "_BLOOM_HQ" }));

            var missing = reg.Find("does-not-exist");
            NUnit.Framework.Assert.That(missing, NUnit.Framework.Is.Null);
        }

        /// <summary>
        /// FR-POSTFX-PORT-MATERIAL-001 — unregister returns true when the
        /// material existed, false otherwise.
        /// </summary>
        [NUnit.Framework.Test]
        public void InMemory_Unregister_Removes_Entry()
        {
            IMaterialRegistry reg = new InMemoryMaterialRegistry();
            reg.Register(new PostFxMaterialInfo(
                "tonemap-aces-v1",
                PostFxMaterialKind.Tonemap,
                "Shaders/PostFx/TonemapAces"));

            NUnit.Framework.Assert.That(reg.Unregister("tonemap-aces-v1"), NUnit.Framework.Is.True);
            NUnit.Framework.Assert.That(reg.Unregister("tonemap-aces-v1"), NUnit.Framework.Is.False);
        }

        /// <summary>
        /// FR-POSTFX-PORT-MATERIAL-002 — recording mock logs each call in
        /// invocation order so tests can assert on the sequence.
        /// </summary>
        [NUnit.Framework.Test]
        public void RecordingMock_Captures_Call_Sequence()
        {
            var mock = new RecordingMaterialRegistry();
            mock.Register(new PostFxMaterialInfo(
                "bloom-prefilter-v1",
                PostFxMaterialKind.Bloom,
                "Shaders/PostFx/Bloom/Prefilter"));
            mock.Find("bloom-prefilter-v1");
            mock.Unregister("bloom-prefilter-v1");

            NUnit.Framework.Assert.That(mock.Calls, NUnit.Framework.Is.EqualTo(new[]
            {
                "Register(bloom-prefilter-v1)",
                "Find(bloom-prefilter-v1)",
                "Unregister(bloom-prefilter-v1)",
            }));
        }
    }
}
