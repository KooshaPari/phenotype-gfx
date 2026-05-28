using System;
using System.Collections.Generic;
using UnityEngine;

namespace Phenotype.Water
{
    /// <summary>
    /// A single Gerstner (trochoidal) wave definition.
    /// </summary>
    public struct GerstnerWave
    {
        /// <summary>Crest amplitude in world units.</summary>
        public float Amplitude;

        /// <summary>Distance between successive crests in world units.</summary>
        public float Wavelength;

        /// <summary>
        /// Steepness (also called Q). Range [0, 1].
        /// 0 = sinusoidal; 1 = sharp trochoid with potential looping.
        /// </summary>
        public float Steepness;

        /// <summary>Normalised XZ propagation direction.</summary>
        public Vector2 Direction;

        /// <summary>Phase speed in world units per second.</summary>
        public float Speed;

        /// <param name="amplitude">Crest height in world units.</param>
        /// <param name="wavelength">Crest-to-crest distance in world units.</param>
        /// <param name="steepness">Q in [0,1].</param>
        /// <param name="direction">Propagation direction (will be normalised).</param>
        /// <param name="speed">Phase speed in world units/s.</param>
        public GerstnerWave(float amplitude, float wavelength, float steepness,
                            Vector2 direction, float speed)
        {
            Amplitude   = amplitude;
            Wavelength  = wavelength;
            Steepness   = Mathf.Clamp01(steepness);
            Direction   = direction.sqrMagnitude > 1e-10f ? direction.normalized : Vector2.right;
            Speed       = speed;
        }
    }

    /// <summary>
    /// Manages a bank of Gerstner wave parameters for ocean/lake surface animation.
    /// Each wave is defined by amplitude, wavelength, steepness, and direction.
    /// The bank is evaluated per-vertex to produce displaced positions and normals.
    /// </summary>
    public class GerstnerWaveBank
    {
        private readonly List<GerstnerWave> _waves;

        /// <summary>Read-only view of the current wave set.</summary>
        public IReadOnlyList<GerstnerWave> Waves => _waves;

        /// <summary>Creates an empty bank.</summary>
        public GerstnerWaveBank()
        {
            _waves = new List<GerstnerWave>();
        }

        /// <summary>Creates a bank pre-populated with <paramref name="waves"/>.</summary>
        public GerstnerWaveBank(IEnumerable<GerstnerWave> waves)
        {
            _waves = new List<GerstnerWave>(waves);
        }

        /// <summary>Adds a wave to the bank and returns <c>this</c> for chaining.</summary>
        public GerstnerWaveBank Add(GerstnerWave wave)
        {
            _waves.Add(wave);
            return this;
        }

        // ──────────────────────────────────────────────────────────────────────
        // Core evaluation
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// Sums the horizontal (XZ) and vertical (Y) Gerstner displacement at
        /// world position <paramref name="worldXZ"/> for the given <paramref name="time"/>.
        /// </summary>
        /// <param name="worldXZ">Undisplaced surface position in XZ.</param>
        /// <param name="time">Elapsed time in seconds.</param>
        /// <returns>
        /// World-space displacement vector. Add to the undisplaced position to get
        /// the final displaced position.
        /// </returns>
        public Vector3 SampleDisplacement(Vector2 worldXZ, float time)
        {
            float dx = 0f, dy = 0f, dz = 0f;

            foreach (var w in _waves)
            {
                if (w.Amplitude <= 0f || w.Wavelength <= 0f) continue;

                float k   = TwoPi / w.Wavelength;               // wave number
                float phi = w.Speed * k * time;                  // phase offset = ω·t, ω = speed·k
                float dot = k * (w.Direction.x * worldXZ.x
                               + w.Direction.y * worldXZ.y)
                           - phi;

                float sinD = Mathf.Sin(dot);
                float cosD = Mathf.Cos(dot);

                // Q·A controls horizontal crest sharpening
                float qa = w.Steepness * w.Amplitude;

                dx += -w.Direction.x * qa * sinD;
                dz += -w.Direction.y * qa * sinD;
                dy +=  w.Amplitude   * cosD;
            }

            return new Vector3(dx, dy, dz);
        }

        /// <summary>
        /// Returns the analytic unit normal of the displaced surface at
        /// <paramref name="worldXZ"/> at the given <paramref name="time"/>.
        /// Computed from first-order partial derivatives of the displacement — no
        /// finite-difference approximation.
        /// </summary>
        public Vector3 SampleNormal(Vector2 worldXZ, float time)
        {
            // Tangents in X and Z: dP/dx and dP/dz of the Gerstner sum.
            // Normal = cross(dP/dx, dP/dz), normalised.
            //
            // For wave i with direction d=(dx,dz), wavenumber k, amplitude A,
            // steepness Q, phase θ = k(d·p) - ω·t:
            //   ∂X/∂x = 1 - Q·A·k·dx²·cos θ   ← from horizontal displacement contribution
            //   ∂Y/∂x = A·k·dx·(-sin θ)        ← from vertical displacement contribution
            //   ∂Z/∂x = -Q·A·k·dx·dz·cos θ
            //   (and symmetrically for ∂/∂z)
            //
            // We accumulate the summed partial terms then build the cross product.

            float dXdx = 1f, dYdx = 0f, dZdx = 0f;
            float dXdz = 0f, dYdz = 0f, dZdz = 1f;

            foreach (var w in _waves)
            {
                if (w.Amplitude <= 0f || w.Wavelength <= 0f) continue;

                float k   = TwoPi / w.Wavelength;
                float phi = w.Speed * k * time;
                float dot = k * (w.Direction.x * worldXZ.x
                               + w.Direction.y * worldXZ.y)
                           - phi;

                float sinD = Mathf.Sin(dot);
                float cosD = Mathf.Cos(dot);

                float wx = w.Direction.x;
                float wz = w.Direction.y;
                float qa = w.Steepness * w.Amplitude;
                float ak = w.Amplitude * k;
                float qak = qa * k;

                // ∂displacement/∂x
                dXdx -= qak * wx * wx * cosD;
                dYdx -= ak  * wx      * sinD;
                dZdx -= qak * wx * wz * cosD;

                // ∂displacement/∂z
                dXdz -= qak * wx * wz * cosD;
                dYdz -= ak  * wz      * sinD;
                dZdz -= qak * wz * wz * cosD;
            }

            // Tangent vectors in world space (dP = undisplaced + displacement)
            var tangentX = new Vector3(dXdx, dYdx, dZdx);
            var tangentZ = new Vector3(dXdz, dYdz, dZdz);

            // Normal = tangentZ × tangentX  (right-hand rule, Y-up)
            var normal = Vector3.Cross(tangentZ, tangentX);
            float len  = normal.magnitude;
            return len > 1e-10f ? normal / len : Vector3.up;
        }

        // ──────────────────────────────────────────────────────────────────────
        // Factory presets
        // ──────────────────────────────────────────────────────────────────────

        /// <summary>
        /// Creates a default open-ocean preset with four varied waves covering
        /// a range of scales and directions.
        /// </summary>
        /// <remarks>
        /// Wave parameters are tuned for a world-unit = 1 m scale:
        /// long swell from the south-west, mid-frequency chop from south-east
        /// and north-west, plus a short high-frequency detail wave.
        /// </remarks>
        public static GerstnerWaveBank CreateOceanPreset()
        {
            return new GerstnerWaveBank(new[]
            {
                // Primary swell — long, low-steepness, south-west to north-east
                new GerstnerWave(
                    amplitude:  0.8f,
                    wavelength: 60f,
                    steepness:  0.45f,
                    direction:  new Vector2(0.7f,  0.7f),
                    speed:      6.5f),

                // Secondary chop — shorter, south-east to north-west
                new GerstnerWave(
                    amplitude:  0.4f,
                    wavelength: 24f,
                    steepness:  0.5f,
                    direction:  new Vector2(-0.6f, 0.8f),
                    speed:      4.0f),

                // Cross-swell from north-west
                new GerstnerWave(
                    amplitude:  0.25f,
                    wavelength: 15f,
                    steepness:  0.55f,
                    direction:  new Vector2(-0.8f, -0.6f),
                    speed:      3.2f),

                // High-frequency surface ripple
                new GerstnerWave(
                    amplitude:  0.08f,
                    wavelength: 4f,
                    steepness:  0.35f,
                    direction:  new Vector2(0.4f, -0.9f),
                    speed:      2.0f),
            });
        }

        /// <summary>
        /// Creates a calm lake preset with two gentle, low-steepness waves.
        /// </summary>
        public static GerstnerWaveBank CreateLakePreset()
        {
            return new GerstnerWaveBank(new[]
            {
                new GerstnerWave(
                    amplitude:  0.05f,
                    wavelength: 10f,
                    steepness:  0.2f,
                    direction:  new Vector2(1f, 0.3f),
                    speed:      1.5f),

                new GerstnerWave(
                    amplitude:  0.03f,
                    wavelength: 6f,
                    steepness:  0.15f,
                    direction:  new Vector2(-0.5f, 1f),
                    speed:      1.0f),
            });
        }

        // ──────────────────────────────────────────────────────────────────────
        // Helpers
        // ──────────────────────────────────────────────────────────────────────

        private const float TwoPi = 2f * Mathf.PI;
    }
}
