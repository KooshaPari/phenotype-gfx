// StreamingManagerTests.cs — Validates LRU streaming semantics.

using System;
using NUnit.Framework;

namespace PhenotypeGfx.Tests
{
    [TestFixture]
    [Category("streaming")]
    public sealed class StreamingManagerTests
    {
        [SetUp]
        public void SetUp() => MockNative.ResetAll();

        [Test]
        public void Create_NewManager_StartsEmpty()
        {
            using var sm = new StreamingManager();
            Assert.That(sm.LoadedCount, Is.EqualTo(0u));
        }

        [Test]
        public void Load_NewChunk_ReturnsPositiveId()
        {
            using var sm = new StreamingManager();
            uint id = sm.Load(1, 2, 3);
            Assert.That(id, Is.GreaterThan(0u));
            Assert.That(sm.LoadedCount, Is.EqualTo(1u));
        }

        [Test]
        public void Load_SameChunkTwice_FirstSucceedsSecondReturnsZero()
        {
            using var sm = new StreamingManager();
            uint first  = sm.Load(1, 2, 3);
            uint second = sm.Load(1, 2, 3);
            Assert.That(first,  Is.GreaterThan(0u));
            Assert.That(second, Is.EqualTo(0u),
                "loading an already-present chunk must return 0 (matches Rust contract)");
            Assert.That(sm.LoadedCount, Is.EqualTo(1u));
        }

        [Test]
        public void Unload_PresentChunk_ReturnsTrueAndDecrementsCount()
        {
            using var sm = new StreamingManager();
            sm.Load(5, 5, 5);
            Assert.That(sm.Unload(5, 5, 5), Is.True);
            Assert.That(sm.LoadedCount, Is.EqualTo(0u));
        }

        [Test]
        public void Unload_AbsentChunk_ReturnsFalse()
        {
            using var sm = new StreamingManager();
            Assert.That(sm.Unload(42, 42, 42), Is.False);
            Assert.That(sm.LoadedCount, Is.EqualTo(0u));
        }

        [Test]
        public void EvictOldest_RemovesFirstLoadedChunk()
        {
            using var sm = new StreamingManager();
            sm.Load(0, 0, 0);
            sm.Load(1, 0, 0);
            sm.Load(2, 0, 0);
            Assert.That(sm.LoadedCount, Is.EqualTo(3u));

            Assert.That(sm.EvictOldest(), Is.True);
            Assert.That(sm.LoadedCount, Is.EqualTo(2u));

            // The first-loaded chunk (0,0,0) was evicted — reloading must succeed.
            uint id = sm.Load(0, 0, 0);
            Assert.That(id, Is.GreaterThan(0u));
        }

        [Test]
        public void EvictOldest_EmptySet_ReturnsFalse()
        {
            using var sm = new StreamingManager();
            Assert.That(sm.EvictOldest(), Is.False);
        }

        [Test]
        public void Dispose_StopsFurtherUse()
        {
            var sm = new StreamingManager();
            sm.Dispose();
            Assert.Throws<ObjectDisposedException>(() => sm.Load(0, 0, 0));
            Assert.Throws<ObjectDisposedException>(() => sm.Unload(0, 0, 0));
            Assert.Throws<ObjectDisposedException>(() => sm.EvictOldest());
            Assert.Throws<ObjectDisposedException>(() => { var _ = sm.LoadedCount; });
        }

        [Test]
        public void Dispose_CalledTwice_IsIdempotent()
        {
            var sm = new StreamingManager();
            sm.Dispose();
            sm.Dispose();
            Assert.Pass();
        }
    }
}
