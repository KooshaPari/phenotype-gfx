// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! IMaterialRegistry — hexagonal port for water material / asset lookup.
//!
//! The fluid-mesh builder, the renderer, and the save/load path all need to
//! resolve a [`WaterMaterial`] by id.  This port abstracts the concrete asset
//! backend (in-memory, file-backed, Unity Addressables, …) so the domain code
//! stays engine-agnostic.
//!
//! Reference: phenotype-postfx/Runtime/Ports/IMaterialRegistry.cs (Unity port),
//! phenotype-voxel/src/ports/material.rs (Rust port).

using System;
using System.Collections.Generic;
using Phenotype.Water.Rendering;

namespace Phenotype.Water.Ports
{
    /// <summary>
    /// Hexagonal port: registry of water materials.
    /// Adapters include <see cref="InMemoryWaterMaterialRegistry"/> and the
    /// future <c>AddressablesWaterMaterialRegistry</c>.
    /// </summary>
    public interface IMaterialRegistry
    {
        /// <summary>
        /// Returns all materials currently registered.
        /// </summary>
        /// <returns>Read-only view of the registry contents.</returns>
        IReadOnlyList<WaterMaterial> List();

        /// <summary>
        /// Looks up a material by id.
        /// </summary>
        /// <param name="id">The material id (a <see cref="Guid"/>).</param>
        /// <returns>The matching <see cref="WaterMaterial"/>, or <see langword="null"/> if absent.</returns>
        WaterMaterial Find(Guid id);

        /// <summary>
        /// Registers a material.  If an entry with the same id already exists, it is replaced.
        /// </summary>
        /// <param name="material">The material to register.</param>
        /// <exception cref="ArgumentNullException">Thrown when <paramref name="material"/> is null.</exception>
        void Register(WaterMaterial material);

        /// <summary>
        /// Removes a material by id.
        /// </summary>
        /// <param name="id">The material id.</param>
        /// <returns>
        /// <see langword="true"/> if the material was present and removed;
        /// otherwise, <see langword="false"/>.
        /// </returns>
        bool Unregister(Guid id);
    }

    /// <summary>
    /// Default in-memory adapter for <see cref="IMaterialRegistry"/>.
    /// Used by the renderer, by tests, and as the canonical null-adapter
    /// when no engine asset system is wired in.
    /// </summary>
    public sealed class InMemoryWaterMaterialRegistry : IMaterialRegistry
    {
        private readonly Dictionary<Guid, WaterMaterial> _byId =
            new Dictionary<Guid, WaterMaterial>();

        /// <inheritdoc/>
        public IReadOnlyList<WaterMaterial> List()
        {
            var result = new List<WaterMaterial>(_byId.Count);
            foreach (var m in _byId.Values) result.Add(m);
            return result;
        }

        /// <inheritdoc/>
        public WaterMaterial Find(Guid id)
        {
            return _byId.TryGetValue(id, out var m) ? m : null;
        }

        /// <inheritdoc/>
        public void Register(WaterMaterial material)
        {
            if (material == null) throw new ArgumentNullException(nameof(material));
            _byId[GetMaterialId(material)] = material;
        }

        /// <inheritdoc/>
        public bool Unregister(Guid id)
        {
            return _byId.Remove(id);
        }

        /// <summary>
        /// Stable id derivation for a material.  We key by reference identity
        /// hashed into a Guid so the same material always maps to the same id.
        /// </summary>
        private static Guid GetMaterialId(WaterMaterial material)
        {
            // Use the runtime hash of the underlying UnityEngine.Material to
            // produce a stable per-instance id; this is enough for an in-memory
            // adapter.  Persisted ids (e.g., for serialization) are produced
            // by a separate addressables-style adapter.
            return new Guid(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, (byte)material.GetHashCode());
        }
    }

    /// <summary>
    /// Recording mock used by domain tests to assert on registry interaction
    /// order.  Each operation is logged to a list the test can replay.
    /// </summary>
    public sealed class RecordingWaterMaterialRegistry : IMaterialRegistry
    {
        private readonly Dictionary<Guid, WaterMaterial> _byId =
            new Dictionary<Guid, WaterMaterial>();
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
        public IReadOnlyList<WaterMaterial> List()
        {
            _calls.Add(nameof(List));
            var result = new List<WaterMaterial>(_byId.Count);
            foreach (var m in _byId.Values) result.Add(m);
            return result;
        }

        /// <inheritdoc/>
        public WaterMaterial Find(Guid id)
        {
            _calls.Add($"{nameof(Find)}({id})");
            return _byId.TryGetValue(id, out var m) ? m : null;
        }

        /// <inheritdoc/>
        public void Register(WaterMaterial material)
        {
            if (material == null) throw new ArgumentNullException(nameof(material));
            _calls.Add($"{nameof(Register)}({material.GetHashCode()})");
            _byId[material.GetHashCodeGuid()] = material;
        }

        /// <inheritdoc/>
        public bool Unregister(Guid id)
        {
            _calls.Add($"{nameof(Unregister)}({id})");
            return _byId.Remove(id);
        }
    }

    internal static class WaterMaterialIdExtensions
    {
        /// <summary>
        /// Produces a stable <see cref="Guid"/> id from a <see cref="WaterMaterial"/>
        /// instance.  This is the canonical id used by the recording mock.
        /// </summary>
        public static Guid GetHashCodeGuid(this WaterMaterial material)
        {
            return new Guid(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, (byte)material.GetHashCode());
        }
    }
}
