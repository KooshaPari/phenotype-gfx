// MeshBufferTests.cs — Validates MeshBuffer ownership + Unity Mesh upload.

using System;
using NUnit.Framework;
using UnityEngine;

namespace PhenotypeGfx.Tests
{
    [TestFixture]
    [Category("mesh")]
    public sealed class MeshBufferTests
    {
        private VoxelWorld _world;
        private MeshBuffer _mesh;

        [SetUp]
        public void SetUp()
        {
            MockNative.ResetAll();
            _world = new VoxelWorld();
            // Place a single solid voxel at the origin so we get a deterministic
            // 8-vertex / 36-index cube mesh.
            _world.Set(0, 0, 0, 1);
            _mesh = _world.BuildMesh(0, 0, 0);
        }

        [TearDown]
        public void TearDown()
        {
            _mesh?.Dispose();
            _world?.Dispose();
        }

        [Test]
        public void VertexCount_ForSingleSolidVoxel_IsEight()
        {
            Assert.That(_mesh, Is.Not.Null);
            Assert.That(_mesh.VertexCount, Is.EqualTo(8u));
        }

        [Test]
        public void IndexCount_ForSingleSolidVoxel_IsThirtySix()
        {
            Assert.That(_mesh, Is.Not.Null);
            Assert.That(_mesh.IndexCount, Is.EqualTo(36u));
        }

        [Test]
        public void GetVertices_ReturnsEightVertexRecords()
        {
            var verts = _mesh.GetVertices();
            Assert.That(verts.Length, Is.EqualTo(8));
            // Each vertex must carry a non-zero material slot (we wrote 1).
            for (int i = 0; i < verts.Length; i++)
                Assert.That(verts[i].material, Is.EqualTo(1));
        }

        [Test]
        public void GetIndices_ReturnsThirtySixIndices()
        {
            var idx = _mesh.GetIndices();
            Assert.That(idx.Length, Is.EqualTo(36));
            // Indices must reference vertices in [0, 8).
            for (int i = 0; i < idx.Length; i++)
                Assert.That(idx[i], Is.LessThan(8u));
        }

        [Test]
        public void FillUnityMesh_PopulatesArraysAndCallsClearOnce()
        {
            var mesh = new Mesh();
            _mesh.FillUnityMesh(mesh);

            Assert.That(mesh.ClearCallCount, Is.EqualTo(1),
                "FillUnityMesh must invoke Mesh.Clear exactly once before populating arrays");
            Assert.That(mesh.vertices.Length, Is.EqualTo(8));
            Assert.That(mesh.normals.Length, Is.EqualTo(8));
            Assert.That(mesh.uv.Length, Is.EqualTo(8));
            Assert.That(mesh.triangles.Length, Is.EqualTo(36));
        }

        [Test]
        public void FillUnityMesh_ThrowsOnNullMesh()
        {
            Assert.Throws<ArgumentNullException>(() => _mesh.FillUnityMesh(null));
        }

        [Test]
        public void Dispose_StopsFurtherUse()
        {
            var mesh = _world.BuildMesh(0, 0, 0);
            mesh.Dispose();
            Assert.Throws<ObjectDisposedException>(() => mesh.GetVertices());
            Assert.Throws<ObjectDisposedException>(() => mesh.GetIndices());
            Assert.Throws<ObjectDisposedException>(() => mesh.FillUnityMesh(new Mesh()));
        }

        [Test]
        public void Dispose_CalledTwice_IsIdempotent()
        {
            var mesh = _world.BuildMesh(0, 0, 0);
            mesh.Dispose();
            mesh.Dispose();
            Assert.Pass();
        }
    }
}
