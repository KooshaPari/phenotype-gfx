// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! `PostFxPassRegistry` — composio-style provider registry for post-processing
//! passes.
//!
//! Mirrors the C# `PostFxPassRegistry.cs` in `phenotype-postfx` (L5-112 port).
//! Replaces the hard-coded switch statements in `PostStack` with a
//! discoverable, extensible registration model.

use std::collections::HashMap;

use crate::postfx::ports::post_fx_pass::{PassEffect, PostFxPass};

/// Static pass descriptor (effect + display name + shader name).
#[derive(Debug, Clone, PartialEq)]
pub struct BlitPassDescriptor {
    /// Effect identifier.
    pub effect: PassEffect,
    /// Display name (e.g. `"Bloom"`).
    pub display_name: String,
    /// Shader name (debug-only).
    pub shader_name: String,
}

/// Hexagonal provider: a `PostFxPass` plus effect/display metadata.
///
/// Mirrors the C# `IPostFxPassProvider`. The provider allows the driver to
/// ask "is this pass enabled for this owner?" and "is this pass supported?".
pub trait PostFxPassDescriptor: Send + Sync {
    /// Returns the static descriptor.
    fn descriptor(&self) -> BlitPassDescriptor;
    /// Whether the pass is currently enabled.
    fn is_enabled(&self) -> bool;
    /// Whether the pass is supported in this build.
    fn is_supported(&self) -> bool;
    /// Mutable handle to the underlying pass.
    fn pass_mut(&mut self) -> &mut dyn PostFxPass;
}

/// Built-in single-blit provider. Most effects (SSAO, SSGI, ACES, Vignette,
/// ChromaticAberration, LUT) use this default implementation.
pub struct BlitProvider<P: PostFxPass> {
    descriptor: BlitPassDescriptor,
    pass: P,
    supported: bool,
}

impl<P: PostFxPass> BlitProvider<P> {
    /// New blit provider wrapping a pass.
    pub fn new(descriptor: BlitPassDescriptor, pass: P) -> Self {
        Self {
            descriptor,
            pass,
            supported: true,
        }
    }

    /// Mark this provider as supported (or not) in the current build.
    pub fn with_supported(mut self, supported: bool) -> Self {
        self.supported = supported;
        self
    }
}

impl<P: PostFxPass + 'static> PostFxPassDescriptor for BlitProvider<P> {
    fn descriptor(&self) -> BlitPassDescriptor {
        self.descriptor.clone()
    }
    fn is_enabled(&self) -> bool {
        self.pass.is_enabled()
    }
    fn is_supported(&self) -> bool {
        self.supported
    }
    fn pass_mut(&mut self) -> &mut dyn PostFxPass {
        &mut self.pass
    }
}

/// Composable registry of post-fx providers.
#[derive(Default)]
pub struct PostFxPassRegistry {
    providers: HashMap<PassEffect, Box<dyn PostFxPassDescriptor>>,
    render_order: Vec<PassEffect>,
}

impl std::fmt::Debug for PostFxPassRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostFxPassRegistry")
            .field("render_order", &self.render_order)
            .field("provider_count", &self.providers.len())
            .finish()
    }
}

impl PostFxPassRegistry {
    /// New empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a provider. Overwrites any existing provider for the same
    /// effect.
    pub fn register<P: PostFxPassDescriptor + 'static>(&mut self, provider: P) {
        let effect = provider.descriptor().effect;
        self.providers.insert(effect, Box::new(provider));
        if !self.render_order.contains(&effect) {
            self.render_order.push(effect);
        }
    }

    /// Removes a provider from the registry.
    pub fn unregister(&mut self, effect: PassEffect) {
        self.providers.remove(&effect);
        self.render_order.retain(|e| *e != effect);
    }

    /// Reorders the render order. Any omitted effects keep their relative
    /// order.
    pub fn set_render_order(&mut self, order: &[PassEffect]) {
        let mut new_order: Vec<PassEffect> = order.to_vec();
        for effect in &self.render_order {
            if !new_order.contains(effect) {
                new_order.push(*effect);
            }
        }
        self.render_order = new_order;
    }

    /// Returns the render order.
    pub fn render_order(&self) -> &[PassEffect] {
        &self.render_order
    }

    /// Returns a provider by effect.
    pub fn get_provider(&self, effect: PassEffect) -> Option<&dyn PostFxPassDescriptor> {
        self.providers.get(&effect).map(|b| b.as_ref())
    }

    /// Returns a mutable provider by effect.
    pub fn get_provider_mut(
        &mut self,
        effect: PassEffect,
    ) -> Option<&mut (dyn PostFxPassDescriptor + '_)> {
        self.providers
            .get_mut(&effect)
            .map(|b| b.as_mut() as &mut dyn PostFxPassDescriptor)
    }

    /// Iterate over providers in the current render order.
    pub fn providers(&self) -> impl Iterator<Item = &dyn PostFxPassDescriptor> {
        let order = self.render_order.clone();
        Box::new(
            order
                .into_iter()
                .filter_map(move |e| self.providers.get(&e).map(|p| p.as_ref())),
        )
    }

    /// Returns `true` if at least one pass is enabled, supported, and has
    /// available material.
    pub fn has_any_active_pass(&self) -> bool {
        for effect in &self.render_order {
            if let Some(p) = self.providers.get(effect) {
                if p.is_enabled() && p.is_supported() {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postfx::error::{PostFxError, PostFxResult};
    use crate::postfx::ports::post_fx_pass::{PassEffect, PassQuality, PostFxContext, PostFxPass};

    struct MockPass {
        effect: PassEffect,
        enabled: bool,
    }

    impl PostFxPass for MockPass {
        fn name(&self) -> &str {
            self.effect.name()
        }
        fn effect(&self) -> PassEffect {
            self.effect
        }
        fn cost(&self) -> f32 {
            0.1
        }
        fn is_enabled(&self) -> bool {
            self.enabled
        }
        fn set_enabled(&mut self, e: bool) {
            self.enabled = e;
        }
        fn on_setup(&mut self, _: &PostFxContext) -> PostFxResult<()> {
            Ok(())
        }
        fn on_render(&mut self, _: &PostFxContext) -> PostFxResult<()> {
            Ok(())
        }
        fn on_dispose(&mut self) {}
        fn validate_variants(
            &self,
            _: &dyn crate::postfx::ports::shader_availability::PostFxShaderAvailability,
        ) -> Result<(), PostFxError> {
            Ok(())
        }
    }

    fn make_provider(effect: PassEffect) -> Box<dyn PostFxPassDescriptor> {
        Box::new(BlitProvider::new(
            BlitPassDescriptor {
                effect,
                display_name: effect.name().to_string(),
                shader_name: format!("Hidden/{}", effect.name()),
            },
            MockPass {
                effect,
                enabled: true,
            },
        ))
    }

    #[test]
    fn empty_registry_has_no_active_passes() {
        let reg = PostFxPassRegistry::new();
        assert_eq!(reg.render_order().len(), 0);
        assert!(!reg.has_any_active_pass());
    }

    #[test]
    fn register_adds_provider() {
        let mut reg = PostFxPassRegistry::new();
        reg.register(BlitProvider::new(
            BlitPassDescriptor {
                effect: PassEffect::Bloom,
                display_name: "Bloom".into(),
                shader_name: "Hidden/Bloom".into(),
            },
            MockPass {
                effect: PassEffect::Bloom,
                enabled: true,
            },
        ));
        assert!(reg.get_provider(PassEffect::Bloom).is_some());
    }

    #[test]
    fn unregister_removes_provider() {
        let mut reg = PostFxPassRegistry::new();
        reg.register(BlitProvider::new(
            BlitPassDescriptor {
                effect: PassEffect::Bloom,
                display_name: "Bloom".into(),
                shader_name: "Hidden/Bloom".into(),
            },
            MockPass {
                effect: PassEffect::Bloom,
                enabled: true,
            },
        ));
        reg.unregister(PassEffect::Bloom);
        assert!(reg.get_provider(PassEffect::Bloom).is_none());
    }

    #[test]
    fn render_order_set_then_keep() {
        let mut reg = PostFxPassRegistry::new();
        reg.register(BlitProvider::new(
            BlitPassDescriptor {
                effect: PassEffect::Ssao,
                display_name: "SSAO".into(),
                shader_name: "Hidden/SSAO".into(),
            },
            MockPass {
                effect: PassEffect::Ssao,
                enabled: true,
            },
        ));
        reg.register(BlitProvider::new(
            BlitPassDescriptor {
                effect: PassEffect::Bloom,
                display_name: "Bloom".into(),
                shader_name: "Hidden/Bloom".into(),
            },
            MockPass {
                effect: PassEffect::Bloom,
                enabled: true,
            },
        ));
        reg.set_render_order(&[PassEffect::Bloom, PassEffect::Ssao]);
        assert_eq!(reg.render_order(), &[PassEffect::Bloom, PassEffect::Ssao]);
    }

    #[test]
    fn has_any_active_pass_true_when_enabled_and_supported() {
        let mut reg = PostFxPassRegistry::new();
        reg.register(BlitProvider::new(
            BlitPassDescriptor {
                effect: PassEffect::Bloom,
                display_name: "Bloom".into(),
                shader_name: "Hidden/Bloom".into(),
            },
            MockPass {
                effect: PassEffect::Bloom,
                enabled: true,
            },
        ));
        assert!(reg.has_any_active_pass());
    }

    #[test]
    fn has_any_active_pass_false_when_disabled() {
        let mut reg = PostFxPassRegistry::new();
        reg.register(BlitProvider::new(
            BlitPassDescriptor {
                effect: PassEffect::Bloom,
                display_name: "Bloom".into(),
                shader_name: "Hidden/Bloom".into(),
            },
            MockPass {
                effect: PassEffect::Bloom,
                enabled: false,
            },
        ));
        assert!(!reg.has_any_active_pass());
    }

    #[test]
    fn providers_iteration_in_order() {
        let mut reg = PostFxPassRegistry::new();
        reg.register(BlitProvider::new(
            BlitPassDescriptor {
                effect: PassEffect::Ssao,
                display_name: "SSAO".into(),
                shader_name: "Hidden/SSAO".into(),
            },
            MockPass {
                effect: PassEffect::Ssao,
                enabled: true,
            },
        ));
        reg.register(BlitProvider::new(
            BlitPassDescriptor {
                effect: PassEffect::Bloom,
                display_name: "Bloom".into(),
                shader_name: "Hidden/Bloom".into(),
            },
            MockPass {
                effect: PassEffect::Bloom,
                enabled: true,
            },
        ));
        let names: Vec<String> = reg
            .providers()
            .map(|p| p.descriptor().display_name)
            .collect();
        assert_eq!(names, vec!["SSAO".to_string(), "Bloom".to_string()]);
    }

    #[test]
    fn get_provider_mut_works() {
        let mut reg = PostFxPassRegistry::new();
        reg.register(BlitProvider::new(
            BlitPassDescriptor {
                effect: PassEffect::Bloom,
                display_name: "Bloom".into(),
                shader_name: "Hidden/Bloom".into(),
            },
            MockPass {
                effect: PassEffect::Bloom,
                enabled: true,
            },
        ));
        let p = reg.get_provider_mut(PassEffect::Bloom).unwrap();
        p.pass_mut().set_enabled(false);
        assert!(!p.is_enabled());
    }
}
