// MaterialPaletteTests.cs — Validates MaterialPalette lifecycle + lookup.

using System;
using NUnit.Framework;

namespace PhenotypeGfx.Tests
{
    [TestFixture]
    [Category("material")]
    public sealed class MaterialPaletteTests
    {
        [SetUp]
        public void SetUp() => MockNative.ResetAll();

        [Test]
        public void Create_NewPalette_StartsEmpty()
        {
            using var palette = new MaterialPalette();
            Assert.That(palette.Count, Is.EqualTo(0u));
        }

        [Test]
        public void Add_FirstMaterial_ReturnsIdZero()
        {
            using var palette = new MaterialPalette();
            ushort id = palette.Add("stone", 5.0f);
            Assert.That(id, Is.EqualTo(0));
            Assert.That(palette.Count, Is.EqualTo(1u));
        }

        [Test]
        public void Add_MultipleMaterials_ReturnsSequentialIds()
        {
            using var palette = new MaterialPalette();
            ushort stone = palette.Add("stone", 5.0f);
            ushort wood  = palette.Add("wood",  1.0f);
            ushort iron  = palette.Add("iron", 10.0f);

            Assert.That(stone, Is.EqualTo(0));
            Assert.That(wood,  Is.EqualTo(1));
            Assert.That(iron,  Is.EqualTo(2));
            Assert.That(palette.Count, Is.EqualTo(3u));
        }

        [Test]
        public void TryGetHardness_RoundTripsForKnownId()
        {
            using var palette = new MaterialPalette();
            ushort id = palette.Add("diamond", 9.5f);
            Assert.That(palette.TryGetHardness(id, out float h), Is.True);
            Assert.That(h, Is.EqualTo(9.5f).Within(1e-5));
        }

        [Test]
        public void TryGetHardness_ReturnsFalseForUnknownId()
        {
            using var palette = new MaterialPalette();
            palette.Add("stone", 1.0f);
            Assert.That(palette.TryGetHardness(999, out float h), Is.False);
            Assert.That(h, Is.EqualTo(0f));
        }

        [Test]
        public void Dispose_StopsFurtherUse()
        {
            var palette = new MaterialPalette();
            palette.Dispose();
            Assert.Throws<ObjectDisposedException>(() => palette.Add("x", 1.0f));
            Assert.Throws<ObjectDisposedException>(() => palette.TryGetHardness(0, out _));
            Assert.Throws<ObjectDisposedException>(() => { var _ = palette.Count; });
        }

        [Test]
        public void Dispose_CalledTwice_IsIdempotent()
        {
            var palette = new MaterialPalette();
            palette.Dispose();
            palette.Dispose();
            Assert.Pass();
        }
    }
}
