//! Demonstrates how to integrate phenotype-gfx with a Bevy-like ECS pattern.
//!
//! This example shows how to wrap `VoxelWorld` as a Resource and create
//! a query system for loading chunks based on a player's position.

use phenotype_gfx::voxel::coord::{to_chunk_coord, ChunkCoord, WorldCoord, FIXED_SCALE};
use phenotype_gfx::voxel::world::VoxelWorld;

/// A mock Resource that holds the voxel world state.
struct VoxelResource {
    world: VoxelWorld<u8>,
}

impl Default for VoxelResource {
    fn default() -> Self {
        Self {
            world: VoxelWorld::new(64),
        }
    }
}

/// A mock Component representing the player's position.
struct Transform {
    position: WorldCoord,
}

/// A mock System that queries the VoxelResource and Transform.
/// It identifies which chunks should be loaded/unloaded.
fn chunk_loader_system(resource: &VoxelResource, player: &Transform) {
    // 1. Determine the player's current chunk
    let player_chunk = to_chunk_coord(
        player.position,
        FIXED_SCALE,
        16, // CHUNK_EDGE
    );

    println!(
        "Query: Player is in chunk ({}, {}, {})",
        player_chunk.cx, player_chunk.cy, player_chunk.cz
    );

    // 2. Query the world for chunks in a 3x3 radius around the player
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                let target = ChunkCoord {
                    cx: player_chunk.cx + dx,
                    cy: player_chunk.cy + dy,
                    cz: player_chunk.cz + dz,
                };

                // 3. Check if the chunk exists in the world
                match resource.world.chunk(target) {
                    Some(chunk) => {
                        // In a real ECS, we might check if a Mesh component needs updating
                        println!(
                            "  [Update] Chunk {:?} is loaded ({} voxels)",
                            target,
                            chunk.voxels.len()
                        );
                    }
                    None => {
                        // 4. If not, we might trigger a generation or network request
                        println!("  [Load]   Requesting chunk {:?}", target);
                    }
                }
            }
        }
    }
}

fn main() {
    let mut resource = VoxelResource::default();

    // Populate a few chunks for the demo
    resource.world.write(WorldCoord { x: 0, y: 0, z: 0 }, 1);
    resource.world.write(
        WorldCoord {
            x: 16 * FIXED_SCALE,
            y: 0,
            z: 0,
        },
        1,
    );
    resource.world.drain_dirty(); // Clear events

    let player = Transform {
        position: WorldCoord {
            x: 5 * FIXED_SCALE,
            y: 0,
            z: 0,
        },
    };

    println!("--- ECS Integration Demo ---");
    chunk_loader_system(&resource, &player);
}
