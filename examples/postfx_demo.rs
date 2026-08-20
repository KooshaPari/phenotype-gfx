//! Post-processing example: bloom, color grading, and stack configuration.
//!
//! Demonstrates the `phenotype-gfx` postfx subsystem: configuration, shader
//! variant validation, and pass descriptors.

use anyhow::Result;
use phenotype_gfx::postfx::{
    BloomConfig, PostStack, PostStackConfig, SsaoConfig,
};
use phenotype_gfx::postfx::ports::post_fx_pass::PassQuality;
use phenotype_gfx::postfx::ports::shader_availability::DefaultPostFxShaderAvailability;

fn main() -> Result<()> {
    println!("=== PostFX Demo ===");

    // 1. Create a custom post-fx configuration
    let config = PostStackConfig {
        enable_bloom: true,
        enable_ssao: true,
        enable_aces: true,
        enable_vignette: true,
        enable_chromatic_aberration: false,
        enable_lut: true,
        quality: PassQuality::Ultra,
        exposure: 1.2,
        ..PostStackConfig::default()
    };

    // 2. Initialize the post-stack
    let mut stack = PostStack::new(config);
    println!("Post-stack initialized with {} samples SSAO", stack.config().ssao_samples);

    // 3. Validate shader availability
    let provider = DefaultPostFxShaderAvailability;
    stack.validate_shader_variants(&provider);
    
    println!("Shader Support:");
    println!("  SSAO: {}", stack.ssao_supported());
    println!("  Bloom: {}", stack.bloom_supported());
    println!("  ACES: {}", stack.aces_supported());
    println!("  Vignette: {}", stack.vignette_supported());
    println!("  LUT: {}", stack.lut_supported());

    // 4. Describe available passes
    let passes = PostStack::describe_passes();
    println!("\nAvailable Passes:");
    for pass in &passes {
        println!("  - {:?} (Description: {})", pass.effect, pass.description);
    }

    // 5. Bloom configuration
    let bloom = BloomConfig {
        is_enabled: true,
        ..BloomConfig::default()
    };
    println!("\nBloom Config: enabled={}", bloom.is_enabled);

    // 6. SSAO configuration
    let ssao = SsaoConfig {
        is_enabled: true,
        radius: 3.0,
        intensity: 1.5,
        ..SsaoConfig::default()
    };
    println!("SSAO Config: radius={:.1}, intensity={:.1}", ssao.radius, ssao.intensity);

    println!("\nPostFX demo completed successfully.");
    Ok(())
}
