//! Gerstner (trochoidal) wave bank.
//!
//! Ported from C# `GerstnerWaveBank.cs` + `GerstnerWave` struct. Gerstner
//! waves are a more realistic approximation of ocean waves than simple sine
//! waves. They produce sharp crests and broad troughs, and displace vertices
//! horizontally as well as vertically.

use std::f32::consts::PI;

use crate::water::error::WaterError;

const TWO_PI: f32 = 2.0 * PI;
const NORMAL_EPSILON: f32 = 1e-10;

/// A single Gerstner (trochoidal) wave definition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GerstnerWave {
    /// Crest amplitude in world units.
    pub amplitude: f32,
    /// Distance between successive crests in world units.
    pub wavelength: f32,
    /// Steepness (Q). Range `[0, 1]`. 0 = sinusoidal, 1 = sharp trochoid.
    pub steepness: f32,
    /// Normalised XZ propagation direction. (x, z) tuple.
    pub direction: (f32, f32),
    /// Phase speed in world units per second.
    pub speed: f32,
}

impl GerstnerWave {
    /// Create a new Gerstner wave.
    ///
    /// `steepness` is clamped to `[0, 1]`. `direction` is normalised; if a
    /// zero-length direction is supplied, falls back to `(1, 0)` (east).
    pub fn new(amplitude: f32, wavelength: f32, steepness: f32, direction: (f32, f32), speed: f32) -> Self {
        let steepness = steepness.clamp(0.0, 1.0);
        let mag2 = direction.0 * direction.0 + direction.1 * direction.1;
        let direction = if mag2 > 1e-10 {
            let inv_mag = 1.0 / mag2.sqrt();
            (direction.0 * inv_mag, direction.1 * inv_mag)
        } else {
            (1.0, 0.0)
        };
        Self { amplitude, wavelength, steepness, direction, speed }
    }
}

/// Manages a bank of Gerstner wave parameters for ocean/lake surface animation.
#[derive(Debug, Clone, Default)]
pub struct GerstnerWaveBank {
    waves: Vec<GerstnerWave>,
}

impl GerstnerWaveBank {
    /// Create an empty bank.
    pub fn new() -> Self { Self::default() }

    /// Create a bank pre-populated with `waves`.
    pub fn from_waves(waves: Vec<GerstnerWave>) -> Self {
        Self { waves }
    }

    /// Read-only view of the current wave set.
    pub fn waves(&self) -> &[GerstnerWave] { &self.waves }

    /// Number of waves in the bank.
    pub fn len(&self) -> usize { self.waves.len() }

    /// Whether the bank has no waves.
    pub fn is_empty(&self) -> bool { self.waves.is_empty() }

    /// Adds a wave to the bank and returns `self` for chaining.
    pub fn add(&mut self, wave: GerstnerWave) -> &mut Self {
        self.waves.push(wave);
        self
    }

    /// Sums the horizontal (XZ) and vertical (Y) Gerstner displacement at
    /// `world_xz` for the given `time`.
    pub fn sample_displacement(&self, world_xz: glam::Vec2, time: f32) -> glam::Vec3 {
        let mut dx = 0.0f32;
        let mut dy = 0.0f32;
        let mut dz = 0.0f32;

        for w in &self.waves {
            if w.amplitude <= 0.0 || w.wavelength <= 0.0 { continue; }

            let k = TWO_PI / w.wavelength;
            let phi = w.speed * k * time;
            let dot = k * (w.direction.0 * world_xz.x + w.direction.1 * world_xz.y) - phi;
            let sin_d = dot.sin();
            let cos_d = dot.cos();
            let qa = w.steepness * w.amplitude;

            dx += -w.direction.0 * qa * sin_d;
            dz += -w.direction.1 * qa * sin_d;
            dy += w.amplitude * cos_d;
        }

        glam::Vec3::new(dx, dy, dz)
    }

    /// Returns the analytic unit normal of the displaced surface at
    /// `world_xz` at the given `time`. If the normal length is near zero,
    /// returns `glam::Vec3::Y`.
    pub fn sample_normal(&self, world_xz: glam::Vec2, time: f32) -> glam::Vec3 {
        let mut d_xdx = 1.0f32;
        let mut d_ydx = 0.0f32;
        let mut d_zdx = 0.0f32;
        let mut d_xdz = 0.0f32;
        let mut d_ydz = 0.0f32;
        let mut d_zdz = 1.0f32;

        for w in &self.waves {
            if w.amplitude <= 0.0 || w.wavelength <= 0.0 { continue; }

            let k = TWO_PI / w.wavelength;
            let phi = w.speed * k * time;
            let dot = k * (w.direction.0 * world_xz.x + w.direction.1 * world_xz.y) - phi;
            let sin_d = dot.sin();
            let cos_d = dot.cos();
            let wx = w.direction.0;
            let wz = w.direction.1;
            let qa = w.steepness * w.amplitude;
            let ak = w.amplitude * k;
            let qak = qa * k;

            d_xdx -= qak * wx * wx * cos_d;
            d_ydx -= ak * wx * sin_d;
            d_zdx -= qak * wx * wz * cos_d;
            d_xdz -= qak * wx * wz * cos_d;
            d_ydz -= ak * wz * sin_d;
            d_zdz -= qak * wz * wz * cos_d;
        }

        let tangent_x = glam::Vec3::new(d_xdx, d_ydx, d_zdx);
        let tangent_z = glam::Vec3::new(d_xdz, d_ydz, d_zdz);
        let normal = tangent_z.cross(tangent_x);
        if normal.length() > NORMAL_EPSILON { normal.normalize() } else { glam::Vec3::Y }
    }

    /// Create a default open-ocean preset with four varied waves covering a
    /// range of scales and directions.
    pub fn create_ocean_preset() -> Self {
        Self::from_waves(vec![
            GerstnerWave::new(0.8, 60.0, 0.45, (0.7,  0.7), 6.5),
            GerstnerWave::new(0.4, 24.0, 0.5,  (-0.6, 0.8), 4.0),
            GerstnerWave::new(0.25, 15.0, 0.55, (-0.8, -0.6), 3.2),
            GerstnerWave::new(0.08, 4.0, 0.35, (0.4, -0.9), 2.0),
        ])
    }

    /// Create a calm lake preset with two gentle, low-steepness waves.
    pub fn create_lake_preset() -> Self {
        Self::from_waves(vec![
            GerstnerWave::new(0.05, 10.0, 0.2,  (1.0, 0.3),  1.5),
            GerstnerWave::new(0.03,  6.0, 0.15, (-0.5, 1.0), 1.0),
        ])
    }
}

/// Validate a `time` value. Negative times are allowed in the simulation
/// (waves can be evaluated at t=0 in the past) but `NaN` / `±∞` is not.
pub fn validate_time(time: f32) -> Result<(), WaterError> {
    if time.is_nan() || time.is_infinite() {
        return Err(WaterError::OutOfBounds {
            msg: format!("time must be finite, got {}", time),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, tol: f32) -> bool { (a - b).abs() < tol }

    #[test]
    fn empty_bank_has_no_waves() {
        let b = GerstnerWaveBank::new();
        assert!(b.is_empty());
        assert_eq!(b.len(), 0);
    }

    #[test]
    fn add_waves_grows_bank() {
        let mut b = GerstnerWaveBank::new();
        b.add(GerstnerWave::new(1.0, 10.0, 0.5, (1.0, 0.0), 2.0));
        b.add(GerstnerWave::new(0.5, 5.0, 0.3, (0.0, 1.0), 1.0));
        assert_eq!(b.len(), 2);
    }

    #[test]
    fn empty_bank_displacement_is_zero() {
        let b = GerstnerWaveBank::new();
        let d = b.sample_displacement(glam::Vec2::ZERO, 1.0);
        assert!(approx(d.x, 0.0, 1e-6));
        assert!(approx(d.y, 0.0, 1e-6));
        assert!(approx(d.z, 0.0, 1e-6));
    }

    #[test]
    fn empty_bank_normal_is_unit_y() {
        let b = GerstnerWaveBank::new();
        let n = b.sample_normal(glam::Vec2::ZERO, 1.0);
        assert!(approx(n.x, 0.0, 1e-6));
        assert!(approx(n.y, 1.0, 1e-6));
        assert!(approx(n.z, 0.0, 1e-6));
    }

    #[test]
    fn ocean_preset_has_four_waves() {
        assert_eq!(GerstnerWaveBank::create_ocean_preset().len(), 4);
    }

    #[test]
    fn lake_preset_has_two_waves() {
        assert_eq!(GerstnerWaveBank::create_lake_preset().len(), 2);
    }

    #[test]
    fn ocean_preset_normal_is_unit_length() {
        let b = GerstnerWaveBank::create_ocean_preset();
        let n = b.sample_normal(glam::Vec2::new(7.3, -4.1), 2.5);
        assert!(approx(n.length(), 1.0, 1e-4));
    }

    #[test]
    fn wave_with_zero_amplitude_is_skipped() {
        let mut b = GerstnerWaveBank::new();
        b.add(GerstnerWave::new(0.0, 10.0, 0.5, (1.0, 0.0), 1.0));
        b.add(GerstnerWave::new(1.0, 10.0, 0.5, (0.0, 1.0), 1.0));
        let d = b.sample_displacement(glam::Vec2::ZERO, 0.0);
        // Only the second wave contributes; we just check it's not zero.
        assert!(d.y.abs() > 0.0);
    }

    #[test]
    fn zero_direction_falls_back_to_east() {
        let w = GerstnerWave::new(1.0, 10.0, 0.5, (0.0, 0.0), 1.0);
        assert_eq!(w.direction, (1.0, 0.0));
    }

    #[test]
    fn steepness_is_clamped() {
        let w = GerstnerWave::new(1.0, 10.0, 5.0, (1.0, 0.0), 1.0);
        assert_eq!(w.steepness, 1.0);
        let w = GerstnerWave::new(1.0, 10.0, -1.0, (1.0, 0.0), 1.0);
        assert_eq!(w.steepness, 0.0);
    }

    #[test]
    fn validate_time_rejects_nan() {
        assert!(validate_time(f32::NAN).is_err());
    }
}
