//! Demonstrates chunk streaming over a mock TCP connection.
//!
//! This example simulates a client-server architecture where a client
//! requests specific voxel chunks by their coordinates, and the server
//! responds with the chunk data. It highlights the use of `ChunkCoord`
//! for addressing and `VoxelWorld` as the backend storage.

use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use phenotype_gfx::voxel::chunk::{Chunk, CHUNK_EDGE};
use phenotype_gfx::voxel::coord::{ChunkCoord, WorldCoord, FIXED_SCALE};
use phenotype_gfx::voxel::world::VoxelWorld;

/// Simple request message: [cx, cy, cz] as i32 big-endian.
type ChunkRequest = [i32; 3];

/// Mock server that holds the voxel world and serves chunk data.
struct VoxelServer {
    world: VoxelWorld<u8>,
}

impl VoxelServer {
    fn new() -> Self {
        let mut world = VoxelWorld::new(FIXED_SCALE);
        // Populate a small 3x3x3 grid of chunks with dummy data
        for x in 0..3 {
            for y in 0..3 {
                for z in 0..3 {
                    world.write(
                        WorldCoord {
                            x: x * FIXED_SCALE * CHUNK_EDGE as i64,
                            y: y * FIXED_SCALE * CHUNK_EDGE as i64,
                            z: z * FIXED_SCALE * CHUNK_EDGE as i64,
                        },
                        42,
                    );
                }
            }
        }
        Self { world }
    }

    /// Handle a single client connection.
    fn handle_client(&self, mut stream: TcpStream) -> Result<()> {
        let mut buf = [0u8; 12]; // 3 x i32
        stream.read_exact(&mut buf).context("Failed to read request")?;

        let cx = i32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let cy = i32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let cz = i32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);

        let coord = ChunkCoord { cx, cy, cz };
        println!("Server: Received request for chunk {:?}", coord);

        if let Some(chunk) = self.world.chunk(coord) {
            // In a real protocol, we'd send the full voxel payload.
            // Here we just send the first 16 bytes as a mock payload.
            let payload = &chunk.voxels[..16.min(chunk.voxels.len())];
            stream.write_all(payload).context("Failed to send payload")?;
        } else {
            // Send zero-filled response to indicate missing chunk
            stream.write_all(&[0u8; 16]).context("Failed to send error")?;
        }

        Ok(())
    }
}

fn run_server(addr: &str) -> Result<()> {
    let listener = TcpListener::bind(addr).context("Failed to bind server")?;
    let server = VoxelServer::new();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                server.handle_client(stream)?;
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }
    Ok(())
}

fn run_client(addr: &str) -> Result<()> {
    let mut stream = TcpStream::connect(addr).context("Failed to connect to server")?;
    
    let request: ChunkRequest = [0, 0, 0]; // Request origin chunk
    let bytes: Vec<u8> = request.iter().flat_map(|x| x.to_be_bytes()).collect();
    
    stream.write_all(&bytes).context("Failed to send request")?;
    
    let mut response = [0u8; 16];
    stream.read_exact(&mut response).context("Failed to read response")?;
    
    println!("Client: Received {} bytes of voxel data", response.len());
    Ok(())
}

fn main() -> Result<()> {
    let addr = "127.0.0.1:7878";
    
    // Start server in a background thread
    let server_handle = thread::spawn(move || run_server(addr).unwrap());
    
    // Give server a moment to start
    thread::sleep(std::time::Duration::from_millis(100));
    
    // Run client
    run_client(addr)?;
    
    // For demo purposes, we just let the process finish
    Ok(())
}
