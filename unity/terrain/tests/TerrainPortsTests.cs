// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! xUnit tests for the terrain hexagonal ports.

using System;
using System.IO;
using Phenotype.Terrain.Materials;
using Phenotype.Terrain.Ports;
using Xunit;

namespace Phenotype.Terrain.Tests
{
    /// <summary>
    /// Tests for <see cref="IMaterialRegistry"/> adapters.
    /// </summary>
    public class MaterialRegistryPortTests
    {
        /// <summary>
        /// FR-TERRAIN-PORT-MATERIAL-000 — registering two materials and
        /// finding each by id round-trips.
        /// </summary>
        [Fact]
        public void InMemory_Register_And_Find_RoundTrip()
        {
            IMaterialRegistry reg = new InMemoryTerrainMaterialRegistry();
            var a = new TerrainMaterial("Grass");
            var b = new TerrainMaterial("Rock");
            reg.Register(a);
            reg.Register(b);

            var foundA = reg.Find(a.Id);
            var foundB = reg.Find(b.Id);
            Assert.NotNull(foundA);
            Assert.NotNull(foundB);
            Assert.Equal("Grass", foundA.Name);
            Assert.Equal("Rock", foundB.Name);

            Assert.Null(reg.Find(Guid.NewGuid()));
        }

        /// <summary>
        /// FR-TERRAIN-PORT-MATERIAL-001 — unregister returns true when the
        /// material existed, false otherwise.
        /// </summary>
        [Fact]
        public void InMemory_Unregister_Removes_Entry()
        {
            IMaterialRegistry reg = new InMemoryTerrainMaterialRegistry();
            var m = new TerrainMaterial("Dirt");
            reg.Register(m);

            Assert.True(reg.Unregister(m.Id));
            Assert.False(reg.Unregister(m.Id));
        }

        /// <summary>
        /// FR-TERRAIN-PORT-MATERIAL-002 — recording mock logs each call in
        /// invocation order.
        /// </summary>
        [Fact]
        public void RecordingMock_Captures_Call_Sequence()
        {
            var mock = new RecordingTerrainMaterialRegistry();
            var m = new TerrainMaterial("Sand");
            mock.Register(m);
            mock.Find(m.Id);
            mock.Unregister(m.Id);

            Assert.Equal(new[]
            {
                $"Register({m.Id})",
                $"Find({m.Id})",
                $"Unregister({m.Id})",
            }, mock.Calls);
        }
    }

    /// <summary>
    /// Tests for <see cref="ISerializationPort"/> adapters.
    /// </summary>
    public class SerializationPortTests
    {
        /// <summary>
        /// FR-TERRAIN-PORT-SERIAL-000 — JSON adapter round-trips a snapshot
        /// losslessly through a real file.
        /// </summary>
        [Fact]
        public void JsonAdapter_RoundTrips_Through_File()
        {
            string tmp = Path.Combine(Path.GetTempPath(), $"terrain-snap-{Guid.NewGuid():N}.json");
            try
            {
                ISerializationPort port = new JsonFileSerializationPort();
                Assert.Equal("terrain-json-v1", port.FormatId);

                var original = new TerrainSnapshot
                {
                    Width = 4,
                    Height = 4,
                    Elevations = new float[] { 0f, 1f, 2f, 3f, 4f, 5f, 6f, 7f,
                                                8f, 9f, 10f, 11f, 12f, 13f, 14f, 15f },
                    MaterialIds = new[] { "grass", "rock" },
                };
                port.Save(original, tmp);

                var recovered = port.Load(tmp);
                Assert.Equal(original.Width, recovered.Width);
                Assert.Equal(original.Height, recovered.Height);
                Assert.Equal(original.Elevations, recovered.Elevations);
                Assert.Equal(original.MaterialIds, recovered.MaterialIds);
            }
            finally
            {
                if (File.Exists(tmp)) File.Delete(tmp);
            }
        }

        /// <summary>
        /// FR-TERRAIN-PORT-SERIAL-001 — JSON adapter raises
        /// <see cref="InvalidDataException"/> when the file is missing.
        /// </summary>
        [Fact]
        public void JsonAdapter_Load_Missing_File_Raises()
        {
            ISerializationPort port = new JsonFileSerializationPort();
            string missing = Path.Combine(Path.GetTempPath(), $"missing-{Guid.NewGuid():N}.json");
            Assert.Throws<InvalidDataException>(() => port.Load(missing));
        }

        /// <summary>
        /// FR-TERRAIN-PORT-SERIAL-002 — mock records the last save and
        /// replays the staged snapshot on load.
        /// </summary>
        [Fact]
        public void Mock_Records_And_Replays()
        {
            var mock = new MockSerializationPort();
            var staged = new TerrainSnapshot { Width = 2, Height = 2 };
            mock.StageLoad(staged);

            var saved = new TerrainSnapshot
            {
                Width = 8,
                Height = 8,
                Elevations = new float[] { 1f, 2f, 3f, 4f, 5f, 6f, 7f, 8f,
                                            9f, 10f, 11f, 12f, 13f, 14f, 15f, 16f,
                                            17f, 18f, 19f, 20f, 21f, 22f, 23f, 24f,
                                            25f, 26f, 27f, 28f, 29f, 30f, 31f, 32f,
                                            33f, 34f, 35f, 36f, 37f, 38f, 39f, 40f,
                                            41f, 42f, 43f, 44f, 45f, 46f, 47f, 48f,
                                            49f, 50f, 51f, 52f, 53f, 54f, 55f, 56f,
                                            57f, 58f, 59f, 60f, 61f, 62f, 63f, 64f },
            };
            mock.Save(saved, "ignored");

            Assert.NotNull(mock.LastSaved);
            Assert.Equal(8, mock.LastSaved.Width);

            var loaded = mock.Load("ignored");
            Assert.Same(staged, loaded);
        }

        /// <summary>
        /// FR-TERRAIN-PORT-SERIAL-003 — load on an empty mock raises
        /// <see cref="InvalidDataException"/>.
        /// </summary>
        [Fact]
        public void Mock_Load_Without_Stage_Raises()
        {
            var mock = new MockSerializationPort();
            Assert.Throws<InvalidDataException>(() => mock.Load("ignored"));
        }
    }
}
