using System;
using UnityEngine;
using Xunit;
using Phenotype.Water;

namespace Phenotype.Water.Tests
{
    /// <summary>
    /// Core simulation tests for <see cref="GerstnerWaveBank"/>: displacement
    /// and normal sampling, determinism, superposition, amplitude/frequency
    /// scaling, phase offsets, and time-based animation.
    /// </summary>
    public class GerstnerWaveBankSimulationTests
    {
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
            var bank = GerstnerWaveBankTestHelpers.SingleWave(amplitude: A, steepness: 0f); // Q=0 → no horizontal shift

            var d = bank.SampleDisplacement(Vector2.zero, 0f);

            Assert.InRange(d.y, A - GerstnerWaveBankTestHelpers.Tolerance, A + GerstnerWaveBankTestHelpers.Tolerance);
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
            var pos = new Vector2(3.7f, -11.2f);
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
            var pos = new Vector2(1.1f, 99.9f);
            const float t = 12.3f;

            var n1 = bank.SampleNormal(pos, t);
            var n2 = bank.SampleNormal(pos, t);

            Assert.Equal(n1.x, n2.x);
            Assert.Equal(n1.y, n2.y);
            Assert.Equal(n1.z, n2.z);
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
            var bank1 = GerstnerWaveBankTestHelpers.SingleWave(amplitude: A1, steepness: 0f);
            var bank2 = GerstnerWaveBankTestHelpers.SingleWave(amplitude: A2, steepness: 0f);

            float y1 = bank1.SampleDisplacement(Vector2.zero, 0f).y;
            float y2 = bank2.SampleDisplacement(Vector2.zero, 0f).y;

            Assert.InRange(y2 / y1, 2f - GerstnerWaveBankTestHelpers.Tolerance * 10, 2f + GerstnerWaveBankTestHelpers.Tolerance * 10);
        }

        // ──────────────────────────────────────────────────────────────────────
        // Single wave generation
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// A single wave at a quarter wavelength has phase = π/2, so
        /// vertical displacement is zero.
        /// </summary>
        [Fact]
        public void SingleWave_AtQuarterWavelength_ZeroVerticalDisplacement()
        {
            const float wavelength = 10f;
            const float A = 1.5f;
            var bank = GerstnerWaveBankTestHelpers.SingleWave(amplitude: A, wavelength: wavelength, steepness: 0f);

            var pos = new Vector2(wavelength / 4f, 0f);
            var d = bank.SampleDisplacement(pos, 0f);

            Assert.InRange(d.y, -GerstnerWaveBankTestHelpers.Tolerance, GerstnerWaveBankTestHelpers.Tolerance);
        }

        /// <summary>
        /// With non-zero steepness, horizontal displacement at quarter wavelength
        /// equals -Q·A (direction is (1,0), sin(π/2)=1).
        /// </summary>
        [Fact]
        public void SingleWave_WithSteepness_ProducesHorizontalDisplacement()
        {
            const float wavelength = 8f;
            const float A = 1f;
            const float Q = 0.5f;
            var bank = GerstnerWaveBankTestHelpers.SingleWave(amplitude: A, wavelength: wavelength, steepness: Q);

            var pos = new Vector2(wavelength / 4f, 0f);
            var d = bank.SampleDisplacement(pos, 0f);

            float expectedX = -Q * A; // -dx * qa * sin(π/2)
            Assert.InRange(d.x, expectedX - GerstnerWaveBankTestHelpers.Tolerance, expectedX + GerstnerWaveBankTestHelpers.Tolerance);
            Assert.InRange(d.z, -GerstnerWaveBankTestHelpers.Tolerance, GerstnerWaveBankTestHelpers.Tolerance);
        }

        /// <summary>
        /// A single wave at half wavelength has phase = π, so
        /// vertical displacement is -A (trough).
        /// </summary>
        [Fact]
        public void SingleWave_AtHalfWavelength_IsTrough()
        {
            const float A = 2f;
            const float wavelength = 12f;
            var bank = GerstnerWaveBankTestHelpers.SingleWave(amplitude: A, wavelength: wavelength, steepness: 0f);

            var pos = new Vector2(wavelength / 2f, 0f);
            var d = bank.SampleDisplacement(pos, 0f);

            Assert.InRange(d.y, -A - GerstnerWaveBankTestHelpers.Tolerance, -A + GerstnerWaveBankTestHelpers.Tolerance);
        }

        // ──────────────────────────────────────────────────────────────────────
        // Multiple wave superposition
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// Displacement of a bank with multiple waves equals the sum of
        /// displacements from each individual wave bank.
        /// </summary>
        [Fact]
        public void MultipleWaves_SuperpositionIsLinear()
        {
            var w1 = new GerstnerWave(1.2f, 10f, 0.3f, new Vector2(0.6f, 0.8f), 2f);
            var w2 = new GerstnerWave(0.8f, 6f, 0.5f, new Vector2(-0.4f, 0.9f), 1.5f);
            var w3 = new GerstnerWave(0.5f, 14f, 0.2f, new Vector2(0.9f, -0.3f), 3f);

            var combined = new GerstnerWaveBank(new[] { w1, w2, w3 });
            var bank1 = new GerstnerWaveBank(new[] { w1 });
            var bank2 = new GerstnerWaveBank(new[] { w2 });
            var bank3 = new GerstnerWaveBank(new[] { w3 });

            var pos = new Vector2(3.5f, -2.1f);
            const float t = 1.7f;

            var dCombined = combined.SampleDisplacement(pos, t);
            var d1 = bank1.SampleDisplacement(pos, t);
            var d2 = bank2.SampleDisplacement(pos, t);
            var d3 = bank3.SampleDisplacement(pos, t);

            Assert.InRange(dCombined.x, d1.x + d2.x + d3.x - GerstnerWaveBankTestHelpers.Tolerance, d1.x + d2.x + d3.x + GerstnerWaveBankTestHelpers.Tolerance);
            Assert.InRange(dCombined.y, d1.y + d2.y + d3.y - GerstnerWaveBankTestHelpers.Tolerance, d1.y + d2.y + d3.y + GerstnerWaveBankTestHelpers.Tolerance);
            Assert.InRange(dCombined.z, d1.z + d2.z + d3.z - GerstnerWaveBankTestHelpers.Tolerance, d1.z + d2.z + d3.z + GerstnerWaveBankTestHelpers.Tolerance);
        }

        /// <summary>
        /// Three waves at different directions and wavelengths produce
        /// non-zero displacement at the origin.
        /// </summary>
        [Fact]
        public void MultipleWaves_AtOrigin_NonZeroDisplacement()
        {
            var bank = new GerstnerWaveBank(new[]
            {
                new GerstnerWave(1.0f, 10f, 0f, new Vector2(1f, 0f), 1f),
                new GerstnerWave(0.5f, 5f, 0f, new Vector2(0f, 1f), 2f),
                new GerstnerWave(0.3f, 20f, 0f, new Vector2(0.7f, 0.7f), 0.5f),
            });

            var d = bank.SampleDisplacement(Vector2.zero, 0f);

            // At t=0, position=0, each wave is at crest: y = 1.0 + 0.5 + 0.3 = 1.8
            Assert.InRange(d.y, 1.8f - GerstnerWaveBankTestHelpers.Tolerance, 1.8f + GerstnerWaveBankTestHelpers.Tolerance);
        }

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
            Assert.InRange(y, 2 * A - GerstnerWaveBankTestHelpers.Tolerance, 2 * A + GerstnerWaveBankTestHelpers.Tolerance);
        }

        // ──────────────────────────────────────────────────────────────────────
        // Wave amplitude and frequency
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// Halving the wavelength doubles the wave number k, producing twice
        /// as many spatial oscillations over the same distance.
        /// </summary>
        [Fact]
        public void Frequency_ShorterWavelength_MoreOscillations()
        {
            const float longWavelength = 20f;
            const float shortWavelength = 10f;
            const float A = 1f;

            var bankLong = GerstnerWaveBankTestHelpers.SingleWave(amplitude: A, wavelength: longWavelength, steepness: 0f);
            var bankShort = GerstnerWaveBankTestHelpers.SingleWave(amplitude: A, wavelength: shortWavelength, steepness: 0f);

            // Over a distance of 20 units, the long wave completes 1 cycle,
            // the short wave completes 2 cycles.
            // At x = 5, long wave phase = 2π/20 * 5 = π/2  → y = 0
            // At x = 5, short wave phase = 2π/10 * 5 = π   → y = -1
            var pos = new Vector2(5f, 0f);
            float yLong = bankLong.SampleDisplacement(pos, 0f).y;
            float yShort = bankShort.SampleDisplacement(pos, 0f).y;

            Assert.InRange(yLong, -GerstnerWaveBankTestHelpers.Tolerance, GerstnerWaveBankTestHelpers.Tolerance);
            Assert.InRange(yShort, -A - GerstnerWaveBankTestHelpers.Tolerance, -A + GerstnerWaveBankTestHelpers.Tolerance);
        }

        /// <summary>
        /// Tripling the amplitude triples the vertical displacement at the crest.
        /// </summary>
        [Fact]
        public void Amplitude_TripleAmplitude_TriplesVerticalDisplacement()
        {
            const float A = 1.5f;
            var bank = GerstnerWaveBankTestHelpers.SingleWave(amplitude: A, steepness: 0f);
            var bank3x = GerstnerWaveBankTestHelpers.SingleWave(amplitude: 3f * A, steepness: 0f);

            float y = bank.SampleDisplacement(Vector2.zero, 0f).y;
            float y3x = bank3x.SampleDisplacement(Vector2.zero, 0f).y;

            Assert.InRange(y3x / y, 3f - GerstnerWaveBankTestHelpers.Tolerance * 10, 3f + GerstnerWaveBankTestHelpers.Tolerance * 10);
        }

        // ──────────────────────────────────────────────────────────────────────
        // Phase offset
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// After one full period T = λ / speed, the wave returns to the same
        /// displacement state.
        /// </summary>
        [Fact]
        public void PhaseOffset_PeriodicRepetition()
        {
            const float wavelength = 10f;
            const float speed = 2f;
            const float period = wavelength / speed;
            var bank = GerstnerWaveBankTestHelpers.SingleWave(wavelength: wavelength, speed: speed, steepness: 0f);

            var pos = new Vector2(2.5f, 1.5f);
            var d0 = bank.SampleDisplacement(pos, 0f);
            var dPeriod = bank.SampleDisplacement(pos, period);

            Assert.InRange(dPeriod.x, d0.x - GerstnerWaveBankTestHelpers.Tolerance, d0.x + GerstnerWaveBankTestHelpers.Tolerance);
            Assert.InRange(dPeriod.y, d0.y - GerstnerWaveBankTestHelpers.Tolerance, d0.y + GerstnerWaveBankTestHelpers.Tolerance);
            Assert.InRange(dPeriod.z, d0.z - GerstnerWaveBankTestHelpers.Tolerance, d0.z + GerstnerWaveBankTestHelpers.Tolerance);
        }

        /// <summary>
        /// At a quarter period, the phase has advanced by π/2, so the crest
        /// at the origin becomes a zero-crossing.
        /// </summary>
        [Fact]
        public void PhaseOffset_QuarterPeriod_ShiftsCrestToZero()
        {
            const float wavelength = 10f;
            const float speed = 2f;
            const float period = wavelength / speed;
            const float A = 1f;
            var bank = GerstnerWaveBankTestHelpers.SingleWave(amplitude: A, wavelength: wavelength, speed: speed, steepness: 0f);

            var d0 = bank.SampleDisplacement(Vector2.zero, 0f);
            var dQuarter = bank.SampleDisplacement(Vector2.zero, period / 4f);

            // At t=0, origin is at crest: y = A
            Assert.InRange(d0.y, A - GerstnerWaveBankTestHelpers.Tolerance, A + GerstnerWaveBankTestHelpers.Tolerance);

            // At t=T/4, phase = -π/2, cos(-π/2) = 0
            Assert.InRange(dQuarter.y, -GerstnerWaveBankTestHelpers.Tolerance, GerstnerWaveBankTestHelpers.Tolerance);
        }

        // ──────────────────────────────────────────────────────────────────────
        // Time-based animation
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// Displacement at a fixed position changes over time (the surface moves).
        /// </summary>
        [Fact]
        public void TimeBasedAnimation_DisplacementChangesOverTime()
        {
            var bank = GerstnerWaveBankTestHelpers.SingleWave(amplitude: 1f, wavelength: 10f, speed: 2f, steepness: 0f);
            var pos = new Vector2(1.5f, 2.5f);

            var d1 = bank.SampleDisplacement(pos, 0.5f);
            var d2 = bank.SampleDisplacement(pos, 1.2f);
            var d3 = bank.SampleDisplacement(pos, 2.8f);

            // All three times should produce different displacements
            Assert.NotEqual(d1.y, d2.y);
            Assert.NotEqual(d2.y, d3.y);
        }

        /// <summary>
        /// After two full periods the wave returns to the same displacement state.
        /// </summary>
        [Fact]
        public void TimeBasedAnimation_TwoPeriods_ReturnsToSameState()
        {
            const float wavelength = 8f;
            const float speed = 1.5f;
            const float period = wavelength / speed;
            var bank = GerstnerWaveBankTestHelpers.SingleWave(wavelength: wavelength, speed: speed, steepness: 0.3f);

            var pos = new Vector2(4f, -3f);
            var d0 = bank.SampleDisplacement(pos, 0.7f);
            var d2T = bank.SampleDisplacement(pos, 0.7f + 2f * period);

            Assert.InRange(d2T.x, d0.x - GerstnerWaveBankTestHelpers.Tolerance, d0.x + GerstnerWaveBankTestHelpers.Tolerance);
            Assert.InRange(d2T.y, d0.y - GerstnerWaveBankTestHelpers.Tolerance, d0.y + GerstnerWaveBankTestHelpers.Tolerance);
            Assert.InRange(d2T.z, d0.z - GerstnerWaveBankTestHelpers.Tolerance, d0.z + GerstnerWaveBankTestHelpers.Tolerance);
        }
    }
}
