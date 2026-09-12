// ObservabilityTests.cs — Validates the static observability counters.

using NUnit.Framework;

namespace PhenotypeGfx.Tests
{
    [TestFixture]
    [Category("observability")]
    public sealed class ObservabilityTests
    {
        [SetUp]
        public void SetUp() => MockNative.ResetAll();

        [Test]
        public void Init_CalledTwice_DoesNotThrow()
        {
            Observability.Init();
            Observability.Init(); // idempotent — must not throw
            Assert.Pass();
        }

        [Test]
        public void CounterInc_IncrementsGlobalCounter()
        {
            uint before = MockNative.ObsCounter;
            Observability.CounterInc(10);
            Observability.CounterInc(5);
            Assert.That(MockNative.ObsCounter, Is.EqualTo(before + 15u));
        }

        [Test]
        public void GaugeSet_OverwritesCurrentValue()
        {
            Observability.GaugeSet(42);
            Assert.That(MockNative.ObsGauge, Is.EqualTo(42u));
            Observability.GaugeSet(999);
            Assert.That(MockNative.ObsGauge, Is.EqualTo(999u));
        }

        [Test]
        public void CounterInc_WithZero_IsNoOp()
        {
            uint before = MockNative.ObsCounter;
            Observability.CounterInc(0);
            Assert.That(MockNative.ObsCounter, Is.EqualTo(before));
        }
    }
}
