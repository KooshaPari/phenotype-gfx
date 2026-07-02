//! `WaterMaterial` — pass-through handle for a water surface material.
//!
//! Deprecated: in the C# layer this was a thin wrapper over `UnityEngine.Material`.
//! In the single Rust core the material is a name + handle pair; actual
//! shader / property assignment is the engine-side's responsibility.

use super::water_shader::WaterShader;

/// Handle for a water material in the registry. Deprecated: the actual
/// material lives in the engine; this is a stable identity for the registry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[deprecated(
    note = "WaterMaterial is a name-only pass-through. Use the engine-side material at render time."
)]
pub struct WaterMaterial {
    id: u64,
    shader_name: String,
    label: String,
}

impl WaterMaterial {
    /// Create a new water material handle with a fresh id and the given label.
    pub fn new(label: impl Into<String>, shader: &WaterShader) -> Self {
        Self {
            id: crate::water::ports::material_registry::next_handle(),
            shader_name: shader.name().to_string(),
            label: label.into(),
        }
    }
    /// Stable id assigned on construction.
    pub fn id(&self) -> u64 {
        self.id
    }
    /// The shader name (Unity `Shader.Find` lookup key).
    pub fn shader_name(&self) -> &str {
        &self.shader_name
    }
    /// Human-readable label for the material.
    pub fn label(&self) -> &str {
        &self.label
    }
}
