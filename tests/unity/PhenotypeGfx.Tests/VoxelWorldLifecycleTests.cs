// VoxelWorldLifecycleTests.cs — End-to-end FFI lifecycle for VoxelWorld.
//
// Exercises the exact path the Unity example takes:
//   create → set voxels in a pattern → build mesh → assert vertices > 0 → dispose.
// Plus the negative paths (post-dispose, empty-chunk mesh, double dispose).

using System;
using NUnit.Framework;
using UnityEngine;

namespace PhenotypeGfx.Tests
{
    [TestFixture]
    [Category("voxel")]
    public sealed class VoxelWorldLifecycleTests
    {
        [SetUp]
        public void SetUp() => MockNative.ResetAll();

        // ----------------------------------------------------------------
        // Happy path — matches unity/PhenotypeGfx.Example.cs:Start()
        // ----------------------------------------------------------------

        [Test]
        public void Create_NewWorld_HandleIsNonZero()
        {
            using var world = new VoxelWorld();
            Assert.That(world.Handle, Is.GreaterThan(0L));
        }

        [Test]
        public void Create_WithDefaultSpan_UsesFixedScale()
        {
            using var world = new VoxelWorld();
            // Default ctor uses FixedScale = 1_000_000 — we cannot inspect the
            // span from outside, but we can verify the default does not throw
            // and yields a usable world.
            world.Set(0, 0, 0, 1);
            Assert.That(world.ChunkCount, Is.EqualTo(1u));
        }

        [Test]
        public void Lifecycle_CreateSetMeshDestroy_ProducesNonEmptyMesh()
        {
            // 1) Create
            using var world = new VoxelWorld(MockNative.FixedScale);
            Assert.That(world.ChunkCount, Is.EqualTo(0u), "fresh world has no chunks");

            // 2) Set voxels in a 4×4×4 alternating pattern
            int blockSize = 4;
            for (int x = 0; x < blockSize; x++)
            for (int y = 0; y < blockSize; y++)
            for (int z = 0; z < blockSize; z++)
            {
                byte val = (byte)(((x + y + z) % 2 == 0) ? 1 : 2);
                world.Set(x * MockNative.FixedScale,
                          y * MockNative.FixedScale,
                          z * MockNative.FixedScale,
                          val);
            }

            // All 64 voxels fit in chunk (0,0,0).
            Assert.That(world.ChunkCount, Is.EqualTo(1u));

            // 3) Round-trip read
            byte readBack = world.Get(0, 0, 0);
            Assert.That(readBack, Is.EqualTo(1));

            // 4) Build mesh for chunk (0,0,0)
            using var mesh = world.BuildMesh(0, 0, 0);
            Assert.That(mesh, Is.Not.Null, "expected non-null mesh for a solid chunk");
            Assert.That(mesh.VertexCount, Is.GreaterThan(0u),
                "mesh vertex count must be > 0 — this is the core assertion from the task spec");
            // 64 solid voxels × 8 vertices per cube = 512 vertices
            Assert.That(mesh.VertexCount, Is.EqualTo(512u));
            Assert.That(mesh.IndexCount, Is.EqualTo(64u * 36u),
                "expected 36 indices per cube face mesh");

            // 5) Dispose is implicit via `using` — but verify we can also
            //    dispose twice without crashing.
            mesh.Dispose();
            world.Dispose();
        }

        [Test]
        public void BuildMesh_EmptyChunk_ReturnsNull()
        {
            using var world = new VoxelWorld();
            var mesh = world.BuildMesh(0, 0, 0);
            Assert.That(mesh, Is.Null,
                "BuildMesh on an unmapped chunk must return null to match the Rust contract");
        }

        [Test]
        public void Get_UnmappedCoordinate_ReturnsZero()
        {
            using var world = new VoxelWorld();
            // Far away from any write — must return 0 (air) per the Rust impl.
            Assert.That(world.Get(999_000_000L, 0, 0), Is.EqualTo(0));
        }

        [Test]
        public void ChunkCount_IncrementsAcrossDistinctChunks()
        {
            using var world = new VoxelWorld();
            world.Set(0, 0, 0, 1);
            Assert.That(world.ChunkCount, Is.EqualTo(1u));
            // 16 m × FixedScale = 16_000_000 µm — write to the next chunk over.
            world.Set(17 * MockNative.FixedScale, 0, 0, 2);
            Assert.That(world.ChunkCount, Is.EqualTo(2u));
        }

        // ----------------------------------------------------------------
        // Dispose / ownership paths
        // ----------------------------------------------------------------

        [Test]
        public void Dispose_StopsFurtherUse()
        {
            var world = new VoxelWorld();
            world.Dispose();
            Assert.Throws<ObjectDisposedException>(() => world.Set(0, 0, 0, 1));
            Assert.Throws<ObjectDisposedException>(() => world.Get(0, 0, 0));
            Assert.Throws<ObjectDisposedException>(() => { var _ = world.ChunkCount; });
            Assert.Throws<ObjectDisposedException>(() => world.BuildMesh(0, 0, 0));
        }

        [Test]
        public void Dispose_CalledTwice_IsIdempotent()
        {
            var world = new VoxelWorld();
            world.Dispose();
            world.Dispose(); // must not throw / double-free
            Assert.Pass();
        }

        [Test]
        public void SetUnity_ConvertsMetresToFixedPoint()
        {
            using var world = new VoxelWorld();
            world.SetUnity(new Vector3(2, 3, 4), 7);
            // Round-trip via GetUnity should hit the same voxel.
            Assert.That(world.GetUnity(new Vector3(2, 3, 4)), Is.EqualTo(7));
            // A neighbouring voxel at the same integer coordinate must still be 0.
            Assert.That(world.GetUnity(new Vector3(2, 3, 5)), Is.EqualTo(0));
        }
    }
}
