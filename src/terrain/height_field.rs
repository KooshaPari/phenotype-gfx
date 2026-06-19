//! 2D elevation grid.
//!
//! Ported from C# `HeightField.cs` (137 lines). Stores a flat `Vec<f32>`
//! indexed as `z * width + x`. World coordinates are 32-bit signed tile
//! indices; elevations are `f32` world units.

use serde::{Deserialize, Serialize};

use crate::terrain::error::{TerrainError, TerrainResult};

/// 2D elevation field indexed as `z * width + x`.
///
/// Coordinates are zero-based and range from `(0, 0)` to `(width - 1, height - 1)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeightField {
    width: i32,
    height: i32,
    data: Vec<f32>,
}

impl HeightField {
    /// Create a new height field with the given dimensions and zero-initialized data.
    pub fn new(width: i32, height: i32) -> TerrainResult<Self> {
        Self::with_data(width, height, None)
    }

    /// Create with optional initial data. `data.len()` must equal `width * height`
    /// when supplied.
    pub fn with_data(width: i32, height: i32, data: Option<Vec<f32>>) -> TerrainResult<Self> {
        if width < 0 {
            return Err(TerrainError::OutOfBounds {
                msg: format!("width must be non-negative, got {width}"),
            });
        }
        if height < 0 {
            return Err(TerrainError::OutOfBounds {
                msg: format!("height must be non-negative, got {height}"),
            });
        }
        let expected = (width as usize).saturating_mul(height as usize);
        let data = match data {
            Some(d) => {
                if d.len() != expected {
                    return Err(TerrainError::InvalidDataLength {
                        got: d.len(),
                        expected,
                    });
                }
                d
            }
            None => vec![0.0; expected],
        };
        Ok(Self { width, height, data })
    }

    /// Width of the height field in tiles.
    pub fn width(&self) -> i32 { self.width }

    /// Height of the height field in tiles.
    pub fn height(&self) -> i32 { self.height }

    /// Returns the elevation at the given tile coordinate.
    pub fn get_height(&self, x: i32, z: i32) -> TerrainResult<f32> {
        if x < 0 || x >= self.width {
            return Err(TerrainError::OutOfBounds { msg: format!("x={x} out of [0, {})", self.width) });
        }
        if z < 0 || z >= self.height {
            return Err(TerrainError::OutOfBounds { msg: format!("z={z} out of [0, {})", self.height) });
        }
        Ok(self.data[z as usize * self.width as usize + x as usize])
    }

    /// Sets the elevation at the given tile coordinate.
    pub fn set_height(&mut self, x: i32, z: i32, value: f32) -> TerrainResult<()> {
        if x < 0 || x >= self.width {
            return Err(TerrainError::OutOfBounds { msg: format!("x={x} out of [0, {})", self.width) });
        }
        if z < 0 || z >= self.height {
            return Err(TerrainError::OutOfBounds { msg: format!("z={z} out of [0, {})", self.height) });
        }
        self.data[z as usize * self.width as usize + x as usize] = value;
        Ok(())
    }

    /// Returns a copy of the internal elevation data.
    pub fn get_data(&self) -> Vec<f32> {
        self.data.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heightfield_can_be_instantiated() {
        let hf = HeightField::new(16, 16).unwrap();
        assert_eq!(hf.width(), 16);
        assert_eq!(hf.height(), 16);
    }

    #[test]
    fn heightfield_get_height_returns_zero_for_default_data() {
        let hf = HeightField::new(4, 4).unwrap();
        assert_eq!(hf.get_height(0, 0).unwrap(), 0.0);
        assert_eq!(hf.get_height(3, 3).unwrap(), 0.0);
    }

    #[test]
    fn heightfield_set_get_round_trip() {
        let mut hf = HeightField::new(4, 4).unwrap();
        hf.set_height(1, 2, 42.5).unwrap();
        assert_eq!(hf.get_height(1, 2).unwrap(), 42.5);
    }

    #[test]
    fn heightfield_get_data_returns_copy() {
        let mut hf = HeightField::new(2, 2).unwrap();
        hf.set_height(0, 0, 5.0).unwrap();
        let mut data = hf.get_data();
        data[0] = 99.0;
        // original unchanged because get_data cloned
        assert_eq!(hf.get_height(0, 0).unwrap(), 5.0);
    }

    // ---- Edge cases (from HeightFieldEdgeCaseTests) ----

    #[test]
    fn heightfield_negative_height_round_trip() {
        let mut hf = HeightField::new(4, 4).unwrap();
        hf.set_height(1, 1, -42.5).unwrap();
        hf.set_height(2, 3, -999.99).unwrap();
        assert_eq!(hf.get_height(1, 1).unwrap(), -42.5);
        assert_eq!(hf.get_height(2, 3).unwrap(), -999.99);
    }

    #[test]
    fn heightfield_negative_heights_via_data_array() {
        let data = vec![0.0, -1.0, -2.0, -3.0];
        let hf = HeightField::with_data(2, 2, Some(data)).unwrap();
        assert_eq!(hf.get_height(1, 0).unwrap(), -1.0);
        assert_eq!(hf.get_height(0, 1).unwrap(), -2.0);
        assert_eq!(hf.get_height(1, 1).unwrap(), -3.0);
    }

    #[test]
    fn heightfield_zero_size_can_be_instantiated() {
        let hf = HeightField::new(0, 0).unwrap();
        assert_eq!(hf.width(), 0);
        assert_eq!(hf.height(), 0);
    }

    #[test]
    fn heightfield_zero_size_get_height_throws() {
        let hf = HeightField::new(0, 0).unwrap();
        assert!(hf.get_height(0, 0).is_err());
    }

    #[test]
    fn heightfield_zero_size_set_height_throws() {
        let mut hf = HeightField::new(0, 0).unwrap();
        assert!(hf.set_height(0, 0, 1.0).is_err());
    }

    #[test]
    fn heightfield_large_1024_can_be_instantiated() {
        let hf = HeightField::new(1024, 1024).unwrap();
        assert_eq!(hf.width(), 1024);
        assert_eq!(hf.height(), 1024);
    }

    #[test]
    fn heightfield_large_1024_read_write_round_trip() {
        let mut hf = HeightField::new(1024, 1024).unwrap();
        hf.set_height(0, 0, 1.0).unwrap();
        hf.set_height(1023, 1023, 2.0).unwrap();
        hf.set_height(512, 512, 3.0).unwrap();
        assert_eq!(hf.get_height(0, 0).unwrap(), 1.0);
        assert_eq!(hf.get_height(1023, 1023).unwrap(), 2.0);
        assert_eq!(hf.get_height(512, 512).unwrap(), 3.0);
    }

    #[test]
    fn heightfield_invalid_data_length() {
        let result = HeightField::with_data(2, 2, Some(vec![0.0, 1.0]));
        assert!(matches!(result, Err(TerrainError::InvalidDataLength { .. })));
    }
}
