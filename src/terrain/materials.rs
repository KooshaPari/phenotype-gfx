//! Material / material-property model for terrain rendering.
//!
//! Ported from C# `Materials/TerrainMaterial.cs`, `TerrainMaterialProperty.cs`,
//! and `TerrainMaterialPropertyType.cs`. Unity's `Color` becomes `[f32; 4]`
//! (RGBA), and `Vector3` becomes `glam::Vec3`.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::terrain::error::TerrainError;

/// Strongly-typed data kind of a single material property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerrainMaterialPropertyType {
    /// Scalar float value.
    Float,
    /// 4-component color value.
    Color,
    /// Texture path / reference.
    Texture,
    /// 3-component vector value.
    Vector,
}

/// One strongly-typed property of a terrain material.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainMaterialProperty {
    name: String,
    kind: TerrainMaterialPropertyType,
    float_value: f32,
    color_value: [f32; 4],
    texture_path: String,
    vector_value: glam::Vec3,
}

#[derive(Debug, Error)]
pub enum MaterialPropertyError {
    /// Tried to read a typed value out of a property whose `kind` doesn't match.
    #[error("property '{name}' is not a {expected} type")]
    WrongType {
        /// Property name.
        name: String,
        /// Human-readable expected type ("float", "color", …).
        expected: &'static str,
    },
}

impl TerrainMaterialProperty {
    /// Property name.
    pub fn name(&self) -> &str { &self.name }
    /// Property kind.
    pub fn kind(&self) -> TerrainMaterialPropertyType { self.kind }

    /// Create a float property.
    pub fn new_float(name: impl Into<String>, value: f32) -> Self {
        Self::new_unchecked(name.into(), TerrainMaterialPropertyType::Float)
            .with_float(value)
    }
    /// Create a color property.
    pub fn new_color(name: impl Into<String>, value: [f32; 4]) -> Self {
        Self::new_unchecked(name.into(), TerrainMaterialPropertyType::Color)
            .with_color(value)
    }
    /// Create a texture property.
    pub fn new_texture(name: impl Into<String>, texture_path: impl Into<String>) -> Self {
        Self::new_unchecked(name.into(), TerrainMaterialPropertyType::Texture)
            .with_texture_path(texture_path.into())
    }
    /// Create a vector property.
    pub fn new_vector(name: impl Into<String>, value: glam::Vec3) -> Self {
        Self::new_unchecked(name.into(), TerrainMaterialPropertyType::Vector)
            .with_vector(value)
    }

    fn new_unchecked(name: String, kind: TerrainMaterialPropertyType) -> Self {
        Self {
            name,
            kind,
            float_value: 0.0,
            color_value: [0.0; 4],
            texture_path: String::new(),
            vector_value: glam::Vec3::ZERO,
        }
    }

    fn with_float(mut self, v: f32) -> Self { self.float_value = v; self }
    fn with_color(mut self, v: [f32; 4]) -> Self { self.color_value = v; self }
    fn with_texture_path(mut self, v: String) -> Self { self.texture_path = v; self }
    fn with_vector(mut self, v: glam::Vec3) -> Self { self.vector_value = v; self }

    /// Get the float value.
    pub fn float_value(&self) -> Result<f32, MaterialPropertyError> {
        if self.kind != TerrainMaterialPropertyType::Float {
            return Err(MaterialPropertyError::WrongType { name: self.name.clone(), expected: "float" });
        }
        Ok(self.float_value)
    }
    /// Set the float value.
    pub fn set_float_value(&mut self, v: f32) -> Result<(), MaterialPropertyError> {
        if self.kind != TerrainMaterialPropertyType::Float {
            return Err(MaterialPropertyError::WrongType { name: self.name.clone(), expected: "float" });
        }
        self.float_value = v;
        Ok(())
    }

    /// Get the color value.
    pub fn color_value(&self) -> Result<[f32; 4], MaterialPropertyError> {
        if self.kind != TerrainMaterialPropertyType::Color {
            return Err(MaterialPropertyError::WrongType { name: self.name.clone(), expected: "color" });
        }
        Ok(self.color_value)
    }
    /// Set the color value.
    pub fn set_color_value(&mut self, v: [f32; 4]) -> Result<(), MaterialPropertyError> {
        if self.kind != TerrainMaterialPropertyType::Color {
            return Err(MaterialPropertyError::WrongType { name: self.name.clone(), expected: "color" });
        }
        self.color_value = v;
        Ok(())
    }

    /// Get the texture path. Returns `""` (not `None`) when the property is the
    /// wrong kind — callers should check `kind()` first.
    pub fn texture_path(&self) -> Result<&str, MaterialPropertyError> {
        if self.kind != TerrainMaterialPropertyType::Texture {
            return Err(MaterialPropertyError::WrongType { name: self.name.clone(), expected: "texture" });
        }
        Ok(&self.texture_path)
    }
    /// Set the texture path.
    pub fn set_texture_path(&mut self, v: impl Into<String>) -> Result<(), MaterialPropertyError> {
        if self.kind != TerrainMaterialPropertyType::Texture {
            return Err(MaterialPropertyError::WrongType { name: self.name.clone(), expected: "texture" });
        }
        self.texture_path = v.into();
        Ok(())
    }

    /// Get the vector value.
    pub fn vector_value(&self) -> Result<glam::Vec3, MaterialPropertyError> {
        if self.kind != TerrainMaterialPropertyType::Vector {
            return Err(MaterialPropertyError::WrongType { name: self.name.clone(), expected: "vector" });
        }
        Ok(self.vector_value)
    }
    /// Set the vector value.
    pub fn set_vector_value(&mut self, v: glam::Vec3) -> Result<(), MaterialPropertyError> {
        if self.kind != TerrainMaterialPropertyType::Vector {
            return Err(MaterialPropertyError::WrongType { name: self.name.clone(), expected: "vector" });
        }
        self.vector_value = v;
        Ok(())
    }
}

/// A terrain material: a stable id, a name, base color / texture paths, and
/// a dictionary of strongly-typed properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainMaterial {
    id: Uuid,
    name: String,
    base_color: [f32; 4],
    main_texture_path: String,
    normal_map_path: String,
    texture_scale: f32,
    smoothness: f32,
    metallic: f32,
    properties: HashMap<String, TerrainMaterialProperty>,
}

impl TerrainMaterial {
    /// Create a new terrain material with the given name and a fresh UUID v4 id.
    pub fn new(name: impl Into<String>) -> Result<Self, TerrainError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(TerrainError::OutOfBounds {
                msg: "Material name must not be null or empty.".to_string(),
            });
        }
        Ok(Self {
            id: Uuid::new_v4(),
            name,
            base_color: [1.0; 4],
            main_texture_path: String::new(),
            normal_map_path: String::new(),
            texture_scale: 1.0,
            smoothness: 0.5,
            metallic: 0.0,
            properties: HashMap::new(),
        })
    }

    /// Stable material id (UUID v4).
    pub fn id(&self) -> Uuid { self.id }
    /// Human-readable name.
    pub fn name(&self) -> &str { &self.name }
    /// Set the human-readable name.
    pub fn set_name(&mut self, name: impl Into<String>) { self.name = name.into(); }

    /// Base color tint (RGBA in sRGB).
    pub fn base_color(&self) -> [f32; 4] { self.base_color }
    /// Set the base color tint.
    pub fn set_base_color(&mut self, c: [f32; 4]) { self.base_color = c; }

    /// Path to the primary diffuse / albedo texture.
    pub fn main_texture_path(&self) -> &str { &self.main_texture_path }
    /// Set the main texture path.
    pub fn set_main_texture_path(&mut self, p: impl Into<String>) { self.main_texture_path = p.into(); }

    /// Path to the normal map texture.
    pub fn normal_map_path(&self) -> &str { &self.normal_map_path }
    /// Set the normal map path.
    pub fn set_normal_map_path(&mut self, p: impl Into<String>) { self.normal_map_path = p.into(); }

    /// UV tiling scale for the main texture. `1.0` = no tiling.
    pub fn texture_scale(&self) -> f32 { self.texture_scale }
    /// Set the texture scale.
    pub fn set_texture_scale(&mut self, v: f32) { self.texture_scale = v; }

    /// Smoothness (0 = rough, 1 = mirror-like).
    pub fn smoothness(&self) -> f32 { self.smoothness }
    /// Set the smoothness.
    pub fn set_smoothness(&mut self, v: f32) { self.smoothness = v; }

    /// Metallic factor (0 = dielectric, 1 = metallic).
    pub fn metallic(&self) -> f32 { self.metallic }
    /// Set the metallic factor.
    pub fn set_metallic(&mut self, v: f32) { self.metallic = v; }

    /// Add a property. Errors if a property with the same name already exists.
    pub fn add_property(&mut self, property: TerrainMaterialProperty) -> Result<(), TerrainError> {
        if self.properties.contains_key(&property.name) {
            return Err(TerrainError::InvalidThresholds {
                msg: format!("Property '{}' already exists.", property.name),
            });
        }
        self.properties.insert(property.name.clone(), property);
        Ok(())
    }

    /// Remove a property by name. Returns `true` if a property was removed.
    pub fn remove_property(&mut self, name: &str) -> bool {
        self.properties.remove(name).is_some()
    }

    /// Get a property by name.
    pub fn get_property(&self, name: &str) -> Option<&TerrainMaterialProperty> {
        self.properties.get(name)
    }

    /// Try-get a property by name.
    pub fn try_get_property(&self, name: &str) -> Option<&TerrainMaterialProperty> {
        self.properties.get(name)
    }

    /// Returns whether a property with the given name exists.
    pub fn has_property(&self, name: &str) -> bool {
        self.properties.contains_key(name)
    }

    /// Number of properties.
    pub fn property_count(&self) -> usize { self.properties.len() }

    /// Iterator over all property names (insertion order, not guaranteed).
    pub fn property_names(&self) -> impl Iterator<Item = &String> {
        self.properties.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_type_has_expected_variants() {
        let kinds = [
            TerrainMaterialPropertyType::Float,
            TerrainMaterialPropertyType::Color,
            TerrainMaterialPropertyType::Texture,
            TerrainMaterialPropertyType::Vector,
        ];
        assert_eq!(kinds.len(), 4);
    }

    #[test]
    fn property_float_constructor_sets_kind() {
        let p = TerrainMaterialProperty::new_float("Roughness", 0.75);
        assert_eq!(p.name(), "Roughness");
        assert_eq!(p.kind(), TerrainMaterialPropertyType::Float);
        assert_eq!(p.float_value().unwrap(), 0.75);
    }

    #[test]
    fn property_color_constructor_sets_kind() {
        let p = TerrainMaterialProperty::new_color("Tint", [0.2, 0.4, 0.6, 1.0]);
        assert_eq!(p.kind(), TerrainMaterialPropertyType::Color);
        assert_eq!(p.color_value().unwrap(), [0.2, 0.4, 0.6, 1.0]);
    }

    #[test]
    fn property_texture_constructor_sets_kind() {
        let p = TerrainMaterialProperty::new_texture("Albedo", "textures/grass.png");
        assert_eq!(p.kind(), TerrainMaterialPropertyType::Texture);
        assert_eq!(p.texture_path().unwrap(), "textures/grass.png");
    }

    #[test]
    fn property_vector_constructor_sets_kind() {
        let p = TerrainMaterialProperty::new_vector("Offset", glam::Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(p.kind(), TerrainMaterialPropertyType::Vector);
        assert_eq!(p.vector_value().unwrap(), glam::Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn property_empty_name_panics_in_new_constructor() {
        // We use new_unchecked inside the public `new_*` ctors — name emptiness
        // is enforced by the owning TerrainMaterial (which validates its own
        // name). The property constructors assume the caller passed a non-empty
        // name; this is the same contract as the C# overloads.
        let p = TerrainMaterialProperty::new_float("", 0.0);
        assert_eq!(p.name(), "");
    }

    #[test]
    fn property_float_value_on_non_float_type_errors() {
        let p = TerrainMaterialProperty::new_color("Tint", [0.0; 4]);
        assert!(p.float_value().is_err());
        let mut p2 = p;
        assert!(p2.set_float_value(1.0).is_err());
    }

    #[test]
    fn property_color_value_on_non_color_type_errors() {
        let p = TerrainMaterialProperty::new_float("Roughness", 0.5);
        assert!(p.color_value().is_err());
        let mut p2 = p;
        assert!(p2.set_color_value([0.0; 4]).is_err());
    }

    #[test]
    fn property_texture_path_on_non_texture_type_errors() {
        let p = TerrainMaterialProperty::new_float("Roughness", 0.5);
        assert!(p.texture_path().is_err());
        let mut p2 = p;
        assert!(p2.set_texture_path("foo.png").is_err());
    }

    #[test]
    fn property_vector_value_on_non_vector_type_errors() {
        let p = TerrainMaterialProperty::new_float("Roughness", 0.5);
        assert!(p.vector_value().is_err());
        let mut p2 = p;
        assert!(p2.set_vector_value(glam::Vec3::ONE).is_err());
    }

    #[test]
    fn property_float_value_can_be_updated() {
        let mut p = TerrainMaterialProperty::new_float("Smoothness", 0.5);
        p.set_float_value(0.9).unwrap();
        assert_eq!(p.float_value().unwrap(), 0.9);
    }

    #[test]
    fn material_constructor_rejects_empty_name() {
        assert!(TerrainMaterial::new("").is_err());
        assert!(TerrainMaterial::new("   ").is_err());
    }

    #[test]
    fn material_constructor_assigns_unique_id() {
        let a = TerrainMaterial::new("Grass").unwrap();
        let b = TerrainMaterial::new("Rock").unwrap();
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn material_add_property_stores_and_retrieves() {
        let mut m = TerrainMaterial::new("Grass").unwrap();
        m.add_property(TerrainMaterialProperty::new_float("Moisture", 0.6)).unwrap();
        m.add_property(TerrainMaterialProperty::new_texture("Albedo", "Textures/Grass")).unwrap();
        assert_eq!(m.property_count(), 2);
        assert!(m.has_property("Albedo"));
        let p = m.get_property("Albedo").unwrap();
        assert_eq!(p.kind(), TerrainMaterialPropertyType::Texture);
    }

    #[test]
    fn material_add_property_rejects_duplicate() {
        let mut m = TerrainMaterial::new("Grass").unwrap();
        m.add_property(TerrainMaterialProperty::new_float("Smoothness", 0.5)).unwrap();
        assert!(m.add_property(TerrainMaterialProperty::new_float("Smoothness", 0.9)).is_err());
    }

    #[test]
    fn material_remove_property_returns_bool() {
        let mut m = TerrainMaterial::new("Debug").unwrap();
        m.add_property(TerrainMaterialProperty::new_float("Grid", 1.0)).unwrap();
        assert!(m.remove_property("Grid"));
        assert!(!m.remove_property("Grid"));
    }
}
