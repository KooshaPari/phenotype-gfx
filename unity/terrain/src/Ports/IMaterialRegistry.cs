// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! IMaterialRegistry — hexagonal port for terrain material / asset lookup.
//!
//! The chunk-mesh builder, the editor inspector, and the save/load path
//! all need to resolve a [`TerrainMaterial`] by id.  This port abstracts the
//! concrete asset backend (in-memory, file-backed, Unity Addressables, …)
//! so the domain code stays engine-agnostic.
//!
//! Reference: phenotype-postfx/Runtime/Ports/IMaterialRegistry.cs (Unity port),
//! phenotype-voxel/src/ports/material.rs (Rust port).

using System;
using System.Collections.Generic;
using Phenotype.Terrain.Materials;

namespace Phenotype.Terrain.Ports
{
    /// <summary>
    /// Hexagonal port: registry of terrain materials.
    /// Adapters include <see cref="InMemoryTerrainMaterialRegistry"/> and
    /// the future <c>AddressablesTerrainMaterialRegistry</c>.
    /// </summary>
    public interface IMaterialRegistry
    {
        /// <summary>
        /// Returns all materials currently registered.
        /// </summary>
        /// <returns>Read-only view of the registry contents.</returns>
        IReadOnlyList<TerrainMaterial> List();

        /// <summary>
        /// Looks up a material by id.
        /// </summary>
        /// <param name="id">The material id (a <see cref="Guid"/>).</param>
        /// <returns>The matching <see cref="TerrainMaterial"/>, or <see langword="null"/> if absent.</returns>
        TerrainMaterial Find(Guid id);

        /// <summary>
        /// Registers a material.  If an entry with the same id already exists, it is replaced.
        /// </summary>
        /// <param name="material">The material to register.</param>
        /// <exception cref="ArgumentNullException">Thrown when <paramref name="material"/> is null.</exception>
        void Register(TerrainMaterial material);

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
    /// Used by the editor inspector, by tests, and as the canonical null-
    /// adapter when no engine asset system is wired in.
    /// </summary>
    public sealed class InMemoryTerrainMaterialRegistry : IMaterialRegistry
    {
        private readonly Dictionary<Guid, TerrainMaterial> _byId =
            new Dictionary<Guid, TerrainMaterial>();

        /// <inheritdoc/>
        public IReadOnlyList<TerrainMaterial> List()
        {
            var result = new List<TerrainMaterial>(_byId.Count);
            foreach (var m in _byId.Values) result.Add(m);
            return result;
        }

        /// <inheritdoc/>
        public TerrainMaterial Find(Guid id)
        {
            return _byId.TryGetValue(id, out var m) ? m : null;
        }

        /// <inheritdoc/>
        public void Register(TerrainMaterial material)
        {
            if (material == null) throw new ArgumentNullException(nameof(material));
            _byId[material.Id] = material;
        }

        /// <inheritdoc/>
        public bool Unregister(Guid id)
        {
            return _byId.Remove(id);
        }
    }

    /// <summary>
    /// Recording mock used by domain tests to assert on registry interaction
    /// order.  Each operation is logged to a list the test can replay.
    /// </summary>
    public sealed class RecordingTerrainMaterialRegistry : IMaterialRegistry
    {
        private readonly Dictionary<Guid, TerrainMaterial> _byId =
            new Dictionary<Guid, TerrainMaterial>();
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
        public IReadOnlyList<TerrainMaterial> List()
        {
            _calls.Add(nameof(List));
            var result = new List<TerrainMaterial>(_byId.Count);
            foreach (var m in _byId.Values) result.Add(m);
            return result;
        }

        /// <inheritdoc/>
        public TerrainMaterial Find(Guid id)
        {
            _calls.Add($"{nameof(Find)}({id})");
            return _byId.TryGetValue(id, out var m) ? m : null;
        }

        /// <inheritdoc/>
        public void Register(TerrainMaterial material)
        {
            if (material == null) throw new ArgumentNullException(nameof(material));
            _calls.Add($"{nameof(Register)}({material.Id})");
            _byId[material.Id] = material;
        }

        /// <inheritdoc/>
        public bool Unregister(Guid id)
        {
            _calls.Add($"{nameof(Unregister)}({id})");
            return _byId.Remove(id);
        }
    }
}
