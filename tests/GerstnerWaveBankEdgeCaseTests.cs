using System;
using UnityEngine;
using Xunit;
using Phenotype.Water;

namespace Phenotype.Water.Tests
{
    /// <summary>
    /// Edge-case and boundary tests for <see cref="GerstnerWaveBank"/>:
    /// empty banks, zero-amplitude / zero-steepness waves, and analytic-normal
    /// invariants at the extreme corners of the sampling domain.
    /// </summary>
    public class GerstnerWaveBankEdgeCaseTests
    {
        // ──────────────────────────────────────────────────────────────────────
        // Zero-steepness baseline
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// With steepness=0 there is no horizontal displacement — even at t=0.
        /// </summary>
        [Fact]
        public void ZeroSteepness_NoHorizontalDisplacement()
        {
            var bank = GerstnerWaveBankTestHelpers.SingleWave(steepness: 0f);

            var d = bank.SampleDisplacement(Vector2.zero, 0f);

            Assert.InRange(d.x, -GerstnerWaveBankTestHelpers.Tolerance, GerstnerWaveBankTestHelpers.Tolerance);
            Assert.InRange(d.z, -GerstnerWaveBankTestHelpers.Tolerance, GerstnerWaveBankTestHelpers.Tolerance);
        }

        // ──────────────────────────────────────────────────────────────────────
        // Normal invariants
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// The analytic normal must be unit length at multiple sample sites.
        /// </summary>
        [Theory]
        [InlineData(0f, 0f, 0.0f)]
        [InlineData(5f, -3f, 1.5f)]
        [InlineData(-10f, 20f, 9.9f)]
        [InlineData(50f, 100f, 33.3f)]
        public void SampleNormal_IsUnitLength(float x, float z, float t)
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            var n = bank.SampleNormal(new Vector2(x, z), t);
            float mag = n.magnitude;
            Assert.InRange(mag, 1f - 1e-4f, 1f + 1e-4f);
        }

        /// <summary>
        /// Normal of an empty bank (flat surface) should be straight up.
        /// </summary>
        [Fact]
        public void EmptyBank_NormalIsUp()
        {
            var bank = new GerstnerWaveBank();
            var n = bank.SampleNormal(new Vector2(5f, 5f), 1f);
            Assert.InRange(n.y, 1f - GerstnerWaveBankTestHelpers.Tolerance, 1f + GerstnerWaveBankTestHelpers.Tolerance);
            Assert.InRange(n.x, -GerstnerWaveBankTestHelpers.Tolerance, GerstnerWaveBankTestHelpers.Tolerance);
            Assert.InRange(n.z, -GerstnerWaveBankTestHelpers.Tolerance, GerstnerWaveBankTestHelpers.Tolerance);
        }

        // ──────────────────────────────────────────────────────────────────────
        // Zero-amplitude
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// Displacement of a zero-amplitude wave should be the zero vector.
        /// </summary>
        [Fact]
        public void ZeroAmplitude_ZeroDisplacement()
        {
            var bank = GerstnerWaveBankTestHelpers.SingleWave(amplitude: 0f);
            var d = bank.SampleDisplacement(new Vector2(1f, 1f), 5f);

            Assert.InRange(d.x, -GerstnerWaveBankTestHelpers.Tolerance, GerstnerWaveBankTestHelpers.Tolerance);
            Assert.InRange(d.y, -GerstnerWaveBankTestHelpers.Tolerance, GerstnerWaveBankTestHelpers.Tolerance);
            Assert.InRange(d.z, -GerstnerWaveBankTestHelpers.Tolerance, GerstnerWaveBankTestHelpers.Tolerance);
        }
    }
}
