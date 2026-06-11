using System;
using UnityEngine;
using Xunit;
using Phenotype.Water;

namespace Phenotype.Water.Tests
{
    /// <summary>
    /// Tests covering the construction of <see cref="GerstnerWaveBank"/>
    /// instances, including the built-in factory presets and smoke tests
    /// that confirm those presets behave like valid banks.
    /// </summary>
    public class GerstnerWaveBankConstructionTests
    {
        // ──────────────────────────────────────────────────────────────────────
        // Factory presets
        // ──────────────────────────────────────────────────────────────────────

        [Fact]
        public void OceanPreset_HasFourWaves()
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            Assert.Equal(4, bank.Waves.Count);
        }

        [Fact]
        public void LakePreset_HasTwoWaves()
        {
            var bank = GerstnerWaveBank.CreateLakePreset();
            Assert.Equal(2, bank.Waves.Count);
        }

        [Fact]
        public void OceanPreset_NormalIsUnitLength_AtArbitraryPoint()
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            var n = bank.SampleNormal(new Vector2(7.3f, -4.1f), 2.5f);
            Assert.InRange(n.magnitude, 1f - 1e-4f, 1f + 1e-4f);
        }
    }
}
