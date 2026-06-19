// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! ISerializationPort — hexagonal port for post-fx stack save / load.
//!
//! The PostStack driver and the editor inspector can persist the current
//! pipeline (effect toggles, quality settings, LUT path) so a user can save
//! a post-fx profile and restore it across sessions.  The port abstracts the
//! concrete wire format (JSON, YAML, binary) and storage backend (file,
//! PlayerPrefs, cloud save).
//!
//! Reference: kmobile/crates/kmobile-core/src/ports/serialization.rs (Rust port).

using System;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Phenotype.PostFx.Ports
{
    /// <summary>
    /// Serializable snapshot of a post-fx stack configuration.
    /// </summary>
    /// <remarks>
    /// Engine-agnostic by design: it carries only logical state, not Unity
    /// references.  The port is responsible for translating this snapshot
    /// into the engine's runtime representation on load.
    /// </remarks>
    [Serializable]
    public sealed class PostFxStackSnapshot
    {
        /// <summary>
        /// Format version of this snapshot.  Bumped on breaking changes.
        /// </summary>
        [JsonPropertyName("version")]
        public int Version { get; set; } = 1;

        /// <summary>
        /// Human-readable name of the snapshot (e.g. <c>"Cinematic-Moody"</c>).
        /// </summary>
        [JsonPropertyName("name")]
        public string Name { get; set; } = string.Empty;

        /// <summary>
        /// Currently loaded LUT path (or empty for identity).
        /// </summary>
        [JsonPropertyName("lut_path")]
        public string LutPath { get; set; } = string.Empty;

        /// <summary>
        /// Bloom intensity, normalised to <c>[0, 1]</c>.
        /// </summary>
        [JsonPropertyName("bloom_intensity")]
        public float BloomIntensity { get; set; }

        /// <summary>
        /// SSAO toggle.
        /// </summary>
        [JsonPropertyName("ssao_enabled")]
        public bool SsaoEnabled { get; set; }

        /// <summary>
        /// Vignette toggle.
        /// </summary>
        [JsonPropertyName("vignette_enabled")]
        public bool VignetteEnabled { get; set; }
    }

    /// <summary>
    /// Hexagonal port: save / load post-fx stack snapshots.
    /// Adapters include <see cref="JsonFileSerializationPort"/> and the future
    /// cloud-save adapter.
    /// </summary>
    public interface ISerializationPort
    {
        /// <summary>
        /// Serializes <paramref name="snapshot"/> to <paramref name="destination"/>.
        /// </summary>
        /// <param name="snapshot">The snapshot to serialize.</param>
        /// <param name="destination">Backend-specific destination (file path, key, …).</param>
        /// <exception cref="ArgumentNullException">Thrown when <paramref name="snapshot"/> is null.</exception>
        void Save(PostFxStackSnapshot snapshot, string destination);

        /// <summary>
        /// Loads and deserializes a snapshot from <paramref name="destination"/>.
        /// </summary>
        /// <param name="destination">Backend-specific destination.</param>
        /// <returns>The deserialized snapshot.</returns>
        /// <exception cref="InvalidDataException">Thrown when the destination is empty or invalid.</exception>
        PostFxStackSnapshot Load(string destination);

        /// <summary>
        /// Stable format identifier (e.g. <c>"postfx-json-v1"</c>).
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
        public string FormatId => "postfx-json-v1";

        /// <inheritdoc/>
        public void Save(PostFxStackSnapshot snapshot, string destination)
        {
            if (snapshot == null) throw new ArgumentNullException(nameof(snapshot));
            if (string.IsNullOrEmpty(destination))
                throw new ArgumentException("Destination must not be empty.", nameof(destination));

            string json = JsonSerializer.Serialize(snapshot, Options);
            File.WriteAllText(destination, json, Encoding.UTF8);
        }

        /// <inheritdoc/>
        public PostFxStackSnapshot Load(string destination)
        {
            if (string.IsNullOrEmpty(destination))
                throw new ArgumentException("Destination must not be empty.", nameof(destination));
            if (!File.Exists(destination))
                throw new InvalidDataException($"Snapshot file not found: {destination}");

            string json = File.ReadAllText(destination, Encoding.UTF8);
            if (string.IsNullOrWhiteSpace(json))
                throw new InvalidDataException($"Snapshot file is empty: {destination}");

            var snapshot = JsonSerializer.Deserialize<PostFxStackSnapshot>(json, Options);
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
        private PostFxStackSnapshot? _staged;
        private PostFxStackSnapshot? _lastSaved;

        /// <summary>
        /// Stages a snapshot to be returned by the next <see cref="Load"/> call.
        /// </summary>
        public void StageLoad(PostFxStackSnapshot snapshot) => _staged = snapshot;

        /// <summary>
        /// Returns the snapshot captured by the most recent <see cref="Save"/> call.
        /// </summary>
        public PostFxStackSnapshot? LastSaved => _lastSaved;

        /// <inheritdoc/>
        public string FormatId => "mock-v0";

        /// <inheritdoc/>
        public void Save(PostFxStackSnapshot snapshot, string destination)
        {
            if (snapshot == null) throw new ArgumentNullException(nameof(snapshot));
            _lastSaved = snapshot;
        }

        /// <inheritdoc/>
        public PostFxStackSnapshot Load(string destination)
        {
            if (_staged == null)
                throw new InvalidDataException("MockSerializationPort: no snapshot staged.");
            return _staged;
        }
    }

    /// <summary>
    /// Test fixture for <see cref="ISerializationPort"/> adapters.
    /// </summary>
    [NUnit.Framework.TestFixture]
    public class SerializationPortTests
    {
        /// <summary>
        /// FR-POSTFX-PORT-SERIAL-000 — JSON adapter round-trips a snapshot
        /// losslessly through a real file.
        /// </summary>
        [NUnit.Framework.Test]
        public void JsonAdapter_RoundTrips_Through_File()
        {
            string tmp = Path.Combine(Path.GetTempPath(), $"postfx-snap-{Guid.NewGuid():N}.json");
            try
            {
                ISerializationPort port = new JsonFileSerializationPort();
                NUnit.Framework.Assert.That(port.FormatId, NUnit.Framework.Is.EqualTo("postfx-json-v1"));

                var original = new PostFxStackSnapshot
                {
                    Name = "Cinematic-Moody",
                    LutPath = "LUTs/CinematicMoody.cube",
                    BloomIntensity = 0.7f,
                    SsaoEnabled = true,
                    VignetteEnabled = true,
                };
                port.Save(original, tmp);

                var recovered = port.Load(tmp);
                NUnit.Framework.Assert.That(recovered.Name, NUnit.Framework.Is.EqualTo("Cinematic-Moody"));
                NUnit.Framework.Assert.That(recovered.LutPath, NUnit.Framework.Is.EqualTo("LUTs/CinematicMoody.cube"));
                NUnit.Framework.Assert.That(recovered.BloomIntensity, NUnit.Framework.Is.EqualTo(0.7f));
                NUnit.Framework.Assert.That(recovered.SsaoEnabled, NUnit.Framework.Is.True);
                NUnit.Framework.Assert.That(recovered.VignetteEnabled, NUnit.Framework.Is.True);
            }
            finally
            {
                if (File.Exists(tmp)) File.Delete(tmp);
            }
        }

        /// <summary>
        /// FR-POSTFX-PORT-SERIAL-001 — JSON adapter raises <see cref="InvalidDataException"/>
        /// when the file is missing.
        /// </summary>
        [NUnit.Framework.Test]
        public void JsonAdapter_Load_Missing_File_Raises()
        {
            ISerializationPort port = new JsonFileSerializationPort();
            string missing = Path.Combine(Path.GetTempPath(), $"missing-{Guid.NewGuid():N}.json");
            NUnit.Framework.Assert.Throws<InvalidDataException>(() => port.Load(missing));
        }

        /// <summary>
        /// FR-POSTFX-PORT-SERIAL-002 — mock records the last save and replays
        /// the staged snapshot on load.
        /// </summary>
        [NUnit.Framework.Test]
        public void Mock_Records_And_Replays()
        {
            var mock = new MockSerializationPort();
            var staged = new PostFxStackSnapshot
            {
                Name = "Staged",
                BloomIntensity = 0.3f,
            };
            mock.StageLoad(staged);

            var saved = new PostFxStackSnapshot
            {
                Name = "Saved",
                BloomIntensity = 0.9f,
            };
            mock.Save(saved, "ignored");

            NUnit.Framework.Assert.That(mock.LastSaved, NUnit.Framework.Is.Not.Null);
            NUnit.Framework.Assert.That(mock.LastSaved!.Name, NUnit.Framework.Is.EqualTo("Saved"));
            NUnit.Framework.Assert.That(mock.LastSaved.BloomIntensity, NUnit.Framework.Is.EqualTo(0.9f));

            var loaded = mock.Load("ignored");
            NUnit.Framework.Assert.That(loaded, NUnit.Framework.Is.SameAs(staged));
        }

        /// <summary>
        /// FR-POSTFX-PORT-SERIAL-003 — load on an empty mock raises
        /// <see cref="InvalidDataException"/>.
        /// </summary>
        [NUnit.Framework.Test]
        public void Mock_Load_Without_Stage_Raises()
        {
            var mock = new MockSerializationPort();
            NUnit.Framework.Assert.Throws<InvalidDataException>(() => mock.Load("ignored"));
        }
    }
}
