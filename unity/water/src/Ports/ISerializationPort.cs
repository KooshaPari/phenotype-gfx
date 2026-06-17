// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! ISerializationPort — hexagonal port for water save / load.
//!
//! The water editor and the wave-bank inspector both need to persist their
//! state (wave parameters, foam colour, normal map paths, …) to disk so a
//! user can save a body of water and restore it in a later session.  The
//! port abstracts the concrete wire format (JSON, binary) and storage
//! backend (file, cloud, PlayerPrefs).
//!
//! Reference: phenotype-postfx/Runtime/Ports/ISerializationPort.cs (Unity port),
//! kmobile/crates/kmobile-core/src/ports/serialization.rs (Rust port).

using System;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Phenotype.Water.Ports
{
    /// <summary>
    /// Serializable snapshot of a body of water.
    /// </summary>
    /// <remarks>
    /// Engine-agnostic by design: it carries only primitive types, not Unity
    /// references.  The port is responsible for translating this snapshot into
    /// the engine's runtime representation on load.
    /// </remarks>
    [Serializable]
    public sealed class WaterSnapshot
    {
        /// <summary>
        /// Format version of this snapshot.  Bumped on breaking changes.
        /// </summary>
        [JsonPropertyName("version")]
        public int Version { get; set; } = 1;

        /// <summary>
        /// Human-readable name of the body of water.
        /// </summary>
        [JsonPropertyName("name")]
        public string Name { get; set; } = string.Empty;

        /// <summary>
        /// Sea level in world units.
        /// </summary>
        [JsonPropertyName("sea_level")]
        public float SeaLevel { get; set; } = 0f;

        /// <summary>
        /// Names of the materials referenced by this body of water.
        /// </summary>
        [JsonPropertyName("material_names")]
        public string[] MaterialNames { get; set; } = Array.Empty<string>();
    }

    /// <summary>
    /// Hexagonal port: save / load water snapshots.
    /// Adapters include <see cref="JsonFileSerializationPort"/> and the future
    /// binary / cloud-save adapter.
    /// </summary>
    public interface ISerializationPort
    {
        /// <summary>
        /// Serializes <paramref name="snapshot"/> to <paramref name="destination"/>.
        /// </summary>
        /// <param name="snapshot">The snapshot to serialize.</param>
        /// <param name="destination">Backend-specific destination (file path, key, …).</param>
        /// <exception cref="ArgumentNullException">Thrown when <paramref name="snapshot"/> is null.</exception>
        void Save(WaterSnapshot snapshot, string destination);

        /// <summary>
        /// Loads and deserializes a snapshot from <paramref name="destination"/>.
        /// </summary>
        /// <param name="destination">Backend-specific destination.</param>
        /// <returns>The deserialized snapshot.</returns>
        /// <exception cref="InvalidDataException">Thrown when the destination is empty or invalid.</exception>
        WaterSnapshot Load(string destination);

        /// <summary>
        /// Stable format identifier (e.g. <c>"water-json-v1"</c>).
        /// </summary>
        string FormatId { get; }
    }

    /// <summary>
    /// Default JSON-on-disk adapter. Used by the editor inspector and by
    /// the cloud-save CLI exporter.
    /// </summary>
    public sealed class JsonFileSerializationPort : ISerializationPort
    {
        private static readonly JsonSerializerOptions Options = new JsonSerializerOptions
        {
            WriteIndented = true,
            // Property names are pinned via [JsonPropertyName] on each field;
            // we leave the policy null to avoid relying on .NET 8+ APIs.
            DefaultIgnoreCondition = JsonIgnoreCondition.Never,
        };

        /// <inheritdoc/>
        public string FormatId => "water-json-v1";

        /// <inheritdoc/>
        public void Save(WaterSnapshot snapshot, string destination)
        {
            if (snapshot == null) throw new ArgumentNullException(nameof(snapshot));
            if (string.IsNullOrEmpty(destination))
                throw new ArgumentException("Destination must not be empty.", nameof(destination));

            string json = JsonSerializer.Serialize(snapshot, Options);
            File.WriteAllText(destination, json, Encoding.UTF8);
        }

        /// <inheritdoc/>
        public WaterSnapshot Load(string destination)
        {
            if (string.IsNullOrEmpty(destination))
                throw new ArgumentException("Destination must not be empty.", nameof(destination));
            if (!File.Exists(destination))
                throw new InvalidDataException($"Snapshot file not found: {destination}");

            string json = File.ReadAllText(destination, Encoding.UTF8);
            if (string.IsNullOrWhiteSpace(json))
                throw new InvalidDataException($"Snapshot file is empty: {destination}");

            var snapshot = JsonSerializer.Deserialize<WaterSnapshot>(json, Options);
            if (snapshot == null)
                throw new InvalidDataException($"Snapshot could not be deserialized: {destination}");
            return snapshot;
        }
    }

    /// <summary>
    /// In-memory mock for domain tests.  Records the most recent save and
    /// replays a pre-loaded snapshot on load.
    /// </summary>
    public sealed class MockSerializationPort : ISerializationPort
    {
        private WaterSnapshot _staged;
        private WaterSnapshot _lastSaved;

        /// <summary>
        /// Stages a snapshot to be returned by the next <see cref="Load"/> call.
        /// </summary>
        public void StageLoad(WaterSnapshot snapshot) => _staged = snapshot;

        /// <summary>
        /// Returns the snapshot captured by the most recent <see cref="Save"/> call.
        /// </summary>
        public WaterSnapshot LastSaved => _lastSaved;

        /// <inheritdoc/>
        public string FormatId => "mock-v0";

        /// <inheritdoc/>
        public void Save(WaterSnapshot snapshot, string destination)
        {
            if (snapshot == null) throw new ArgumentNullException(nameof(snapshot));
            _lastSaved = snapshot;
        }

        /// <inheritdoc/>
        public WaterSnapshot Load(string destination)
        {
            if (_staged == null)
                throw new InvalidDataException("MockSerializationPort: no snapshot staged.");
            return _staged;
        }
    }
}
