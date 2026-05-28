using System;
using UnityEngine;
using Xunit;
using Phenotype.Water;

namespace Phenotype.Water.Tests
{
    public class GerstnerWaveBankTests
    {
        // ──────────────────────────────────────────────────────────────────────
        // Helpers
        // ──────────────────────────────────────────────────────────────────────

        private const float Tolerance = 1e-5f;

        private static GerstnerWaveBank SingleWave(
            float amplitude   = 1f,
            float wavelength  = 10f,
            float steepness   = 0.5f,
            float dirX        = 1f,
            float dirZ        = 0f,
            float speed       = 1f)
        {
            return new GerstnerWaveBank(new[]
            {
                new GerstnerWave(amplitude, wavelength, steepness,
                                 new Vector2(dirX, dirZ), speed)
            });
        }

        // ──────────────────────────────────────────────────────────────────────
        // Zero-time baseline
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// At t=0 and position=0 the phase is 0 → cos(0)=1, so Y displacement
        /// should equal the wave amplitude and horizontal displacement should be 0.
        /// </summary>
        [Fact]
        public void ZeroTimeAtOrigin_YDisplacementEqualsAmplitude()
        {
            const float A = 2.5f;
            var bank = SingleWave(amplitude: A, steepness: 0f); // Q=0 → no horizontal shift

            var d = bank.SampleDisplacement(Vector2.zero, 0f);

            Assert.InRange(d.y, A - Tolerance, A + Tolerance);
        }

        /// <summary>
        /// With steepness=0 there is no horizontal displacement — even at t=0.
        /// </summary>
        [Fact]
        public void ZeroSteepness_NoHorizontalDisplacement()
        {
            var bank = SingleWave(steepness: 0f);

            var d = bank.SampleDisplacement(Vector2.zero, 0f);

            Assert.InRange(d.x, -Tolerance, Tolerance);
            Assert.InRange(d.z, -Tolerance, Tolerance);
        }

        // ──────────────────────────────────────────────────────────────────────
        // Displacement determinism
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// Same inputs must always yield bit-identical results (no randomness).
        /// </summary>
        [Fact]
        public void SampleDisplacement_IsDeterministic()
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            var pos  = new Vector2(3.7f, -11.2f);
            const float t = 4.56f;

            var d1 = bank.SampleDisplacement(pos, t);
            var d2 = bank.SampleDisplacement(pos, t);

            Assert.Equal(d1.x, d2.x);
            Assert.Equal(d1.y, d2.y);
            Assert.Equal(d1.z, d2.z);
        }

        /// <summary>
        /// SampleNormal must also be deterministic.
        /// </summary>
        [Fact]
        public void SampleNormal_IsDeterministic()
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            var pos  = new Vector2(1.1f, 99.9f);
            const float t = 12.3f;

            var n1 = bank.SampleNormal(pos, t);
            var n2 = bank.SampleNormal(pos, t);

            Assert.Equal(n1.x, n2.x);
            Assert.Equal(n1.y, n2.y);
            Assert.Equal(n1.z, n2.z);
        }

        // ──────────────────────────────────────────────────────────────────────
        // Normal is unit length
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// The analytic normal must be unit length at multiple sample sites.
        /// </summary>
        [Theory]
        [InlineData(  0f,    0f,  0.0f)]
        [InlineData(  5f,   -3f,  1.5f)]
        [InlineData(-10f,   20f,  9.9f)]
        [InlineData( 50f,  100f, 33.3f)]
        public void SampleNormal_IsUnitLength(float x, float z, float t)
        {
            var bank = GerstnerWaveBank.CreateOceanPreset();
            var n    = bank.SampleNormal(new Vector2(x, z), t);
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
            var n    = bank.SampleNormal(new Vector2(5f, 5f), 1f);
            Assert.InRange(n.y, 1f - Tolerance, 1f + Tolerance);
            Assert.InRange(n.x, -Tolerance, Tolerance);
            Assert.InRange(n.z, -Tolerance, Tolerance);
        }

        // ──────────────────────────────────────────────────────────────────────
        // Amplitude scaling
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// Doubling the amplitude of a single sinusoidal wave (Q=0) must
        /// double the vertical displacement at the crest.
        /// </summary>
        [Fact]
        public void DoubleAmplitude_DoublesVerticalDisplacement()
        {
            const float A1 = 1f;
            const float A2 = 2f;

            // Use Q=0 so horizontal displacement is zero and crest is at origin/t=0.
            var bank1 = SingleWave(amplitude: A1, steepness: 0f);
            var bank2 = SingleWave(amplitude: A2, steepness: 0f);

            float y1 = bank1.SampleDisplacement(Vector2.zero, 0f).y;
            float y2 = bank2.SampleDisplacement(Vector2.zero, 0f).y;

            Assert.InRange(y2 / y1, 2f - Tolerance * 10, 2f + Tolerance * 10);
        }

        /// <summary>
        /// Displacement of a zero-amplitude wave should be the zero vector.
        /// </summary>
        [Fact]
        public void ZeroAmplitude_ZeroDisplacement()
        {
            var bank = SingleWave(amplitude: 0f);
            var d    = bank.SampleDisplacement(new Vector2(1f, 1f), 5f);

            Assert.InRange(d.x, -Tolerance, Tolerance);
            Assert.InRange(d.y, -Tolerance, Tolerance);
            Assert.InRange(d.z, -Tolerance, Tolerance);
        }

        // ──────────────────────────────────────────────────────────────────────
        // Multi-wave superposition
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// Displacement of a bank with two opposite-direction identical waves
        /// at t=0, position=0 (both at crest) sums both amplitudes vertically.
        /// </summary>
        [Fact]
        public void TwoWavesAtCrest_SumAmplitudesVertically()
        {
            const float A = 1f;
            var bank = new GerstnerWaveBank(new[]
            {
                new GerstnerWave(A, 10f, 0f, new Vector2( 1f, 0f), 1f),
                new GerstnerWave(A, 10f, 0f, new Vector2(-1f, 0f), 1f),
            });

            float y = bank.SampleDisplacement(Vector2.zero, 0f).y;
            Assert.InRange(y, 2 * A - Tolerance, 2 * A + Tolerance);
        }

        // ──────────────────────────────────────────────────────────────────────
        // Factory presets smoke test
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
            var n    = bank.SampleNormal(new Vector2(7.3f, -4.1f), 2.5f);
            Assert.InRange(n.magnitude, 1f - 1e-4f, 1f + 1e-4f);
        }
    }
}
