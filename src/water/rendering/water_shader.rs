//! `WaterShader` — pass-through name-only handle.
//!
//! Deprecated: the C# `WaterShader` wrapped a Unity `Shader` instance loaded
//! via `Shader.Find`. In the single Rust core the shader is just a name; the
//! engine-side binding is the consumer's responsibility. Kept as a struct so
//! the C# edge (and the registry tests) can still pass a stable identity
//! through the port.

/// Thin pass-through for a Unity shader name. Deprecated; consumers should
/// rely on the engine-side shader lookup at render time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[deprecated(note = "Use the engine-side shader lookup; this is a name-only pass-through.")]
pub struct WaterShader {
    name: String,
}

impl WaterShader {
    /// Create a new shader handle by name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
    /// The shader name (Unity `Shader.Find` lookup key).
    pub fn name(&self) -> &str {
        &self.name
    }
}
