using UnityEngine;
using Phenotype.Water;

namespace Phenotype.Water.Tests
{
    /// <summary>
    /// Shared helpers used across the <see cref="GerstnerWaveBank"/> test files.
    /// Kept internal so the test assembly is the only consumer.
    /// </summary>
    internal static class GerstnerWaveBankTestHelpers
    {
        internal const float Tolerance = 1e-5f;

        /// <summary>
        /// Builds a single-wave <see cref="GerstnerWaveBank"/> using the
        /// provided wave parameters, with sensible defaults for a unit
        /// test fixture.
        /// </summary>
        internal static GerstnerWaveBank SingleWave(
            float amplitude = 1f,
            float wavelength = 10f,
            float steepness = 0.5f,
            float dirX = 1f,
            float dirZ = 0f,
            float speed = 1f)
        {
            return new GerstnerWaveBank(new[]
            {
                new GerstnerWave(amplitude, wavelength, steepness,
                                 new Vector2(dirX, dirZ), speed)
            });
        }
    }
}
