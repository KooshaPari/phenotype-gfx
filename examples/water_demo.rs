//! Water rendering example: surface mesh, wave animation, and flow direction.
//!
//! Demonstrates the `phenotype-gfx` water subsystem: Gerstner wave bank,
//! surface displacement, and analytic normal calculation.

use anyhow::Result;
use phenotype_gfx::water::gerstner_wave_bank::{GerstnerWave, GerstnerWaveBank};

fn main() -> Result<()> {
    println!("=== Water Demo ===");

    // 1. Create a Gerstner wave bank with custom waves
    let mut bank = GerstnerWaveBank::new();
    bank.add(GerstnerWave::new(0.5, 20.0, 0.6, (1.0, 0.3), 3.0));
    bank.add(GerstnerWave::new(0.2, 10.0, 0.4, (-0.5, 1.0), 2.0));
    bank.add(GerstnerWave::new(0.1, 5.0, 0.2, (0.7, -0.7), 1.5));

    println!("Created wave bank with {} waves", bank.len());

    // 2. Sample displacement at different points and times
    let points = [
        glam::Vec2::new(0.0, 0.0),
        glam::Vec2::new(10.0, 5.0),
        glam::Vec2::new(-5.0, 10.0),
    ];
    let times = [0.0, 1.0, 2.0];

    println!("\nWave Displacement (dx, dy, dz):");
    for time in &times {
        println!("  t={time:.1}:");
        for pt in &points {
            let disp = bank.sample_displacement(*pt, *time);
            println!(
                "    [{pt:?}] -> ({:.3}, {:.3}, {:.3})",
                disp.x, disp.y, disp.z
            );
        }
    }

    // 3. Sample analytic normals
    println!("\nSurface Normals:");
    for time in &times {
        let normal = bank.sample_normal(glam::Vec2::ZERO, *time);
        println!(
            "  t={time:.1}: ({:.3}, {:.3}, {:.3})",
            normal.x, normal.y, normal.z
        );
    }

    // 4. Flow direction configuration
    let flow_dir = glam::Vec2::new(0.8, 0.2).normalize();
    println!("\nFlow direction: ({:.3}, {:.3})", flow_dir.x, flow_dir.y);

    // 5. Use a preset bank
    let ocean = GerstnerWaveBank::create_ocean_preset();
    let ocean_disp = ocean.sample_displacement(glam::Vec2::new(50.0, 50.0), 5.0);
    println!(
        "\nOcean preset displacement at (50,50) t=5.0: ({:.3}, {:.3}, {:.3})",
        ocean_disp.x, ocean_disp.y, ocean_disp.z
    );

    println!("\nWater demo completed successfully.");
    Ok(())
}
