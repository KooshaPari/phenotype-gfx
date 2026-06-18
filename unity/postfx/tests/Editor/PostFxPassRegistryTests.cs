// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! Unit tests for the PostFxPassRegistry and IPostFxPass interface.
//! These tests run in any NUnit-compatible test runner (NUnit 3.13+).

using NUnit.Framework;
using UnityEngine;
using Phenotype.PostFx;

namespace Phenotype.PostFx.Tests
{
    [TestFixture]
    public class PostFxPassRegistryTests
    {
        // Test 1: empty registry has no providers
        [Test]
        public void EmptyRegistry_HasNoProviders()
        {
            var reg = new PostFxPassRegistry();
            int count = 0;
            foreach (var _ in reg.Providers) count++;
            Assert.AreEqual(0, count);
        }

        // Test 2: CreateDefault returns a registry with built-in providers
        [Test]
        public void CreateDefault_HasProviders()
        {
            var reg = PostFxPassRegistry.CreateDefault();
            int count = 0;
            foreach (var _ in reg.Providers) count++;
            Assert.Greater(count, 0);
        }

        // Test 3: IPostFxPass interface exists with Init, Render, Dispose
        [Test]
        public void IPostFxPass_HasInitRenderDispose()
        {
            var pass = new MockPostFxPass();
            pass.Init(null!);
            Assert.IsTrue(pass.InitCalled);

            var src = new RenderTexture { width = 64, height = 64 };
            var dst = new RenderTexture { width = 64, height = 64 };
            pass.Render(src, dst);
            Assert.IsTrue(pass.RenderCalled);

            pass.Dispose();
            Assert.IsTrue(pass.DisposeCalled);
        }

        // Test 4: BlitPassProvider implements IPostFxPass
        [Test]
        public void BlitPassProvider_ImplementsIPostFxPass()
        {
            Assert.IsTrue(typeof(IPostFxPass).IsAssignableFrom(typeof(BlitPassProvider)));
        }

        // Test 5: BloomPassProvider implements IPostFxPass
        [Test]
        public void BloomPassProvider_ImplementsIPostFxPass()
        {
            Assert.IsTrue(typeof(IPostFxPass).IsAssignableFrom(typeof(BloomPassProvider)));
        }

        // Test 6: Init sets owner on provider
        [Test]
        public void Init_SetsOwner()
        {
            var stack = new PostStack();
            var pass = new MockPostFxPass();
            pass.Init(stack);
            Assert.AreSame(stack, pass.Owner);
        }

        // Test 7: Dispose is called on all providers when registry is disposed
        [Test]
        public void Dispose_CallsDisposeOnProviders()
        {
            var reg = new PostFxPassRegistry();
            var pass = new MockPostFxPass();
            reg.Register(new MockPassProvider(pass));
            reg.Dispose();
            Assert.IsTrue(pass.DisposeCalled);
        }

        // Test 8: GetProvider returns null for unregistered effect
        [Test]
        public void GetProvider_Unregistered_ReturnsNull()
        {
            var reg = new PostFxPassRegistry();
            Assert.IsNull(reg.GetProvider(PostFxEffect.SSAO));
        }

        // Test 9: Register adds provider
        [Test]
        public void Register_AddsProvider()
        {
            var reg = new PostFxPassRegistry();
            var provider = new MockPassProvider(new MockPostFxPass());
            reg.Register(provider);
            Assert.AreSame(provider, reg.GetProvider(PostFxEffect.Bloom));
        }

        // Test 10: Unregister removes provider
        [Test]
        public void Unregister_RemovesProvider()
        {
            var reg = new PostFxPassRegistry();
            var provider = new MockPassProvider(new MockPostFxPass());
            reg.Register(provider);
            reg.Unregister(PostFxEffect.Bloom);
            Assert.IsNull(reg.GetProvider(PostFxEffect.Bloom));
        }
    }

    /// <summary>Mock IPostFxPass for testing.</summary>
    internal sealed class MockPostFxPass : IPostFxPass
    {
        public PostStack? Owner { get; private set; }
        public bool InitCalled { get; private set; }
        public bool RenderCalled { get; private set; }
        public bool DisposeCalled { get; private set; }

        public void Init(PostStack owner)
        {
            Owner = owner;
            InitCalled = true;
        }

        public bool Render(RenderTexture src, RenderTexture dst)
        {
            RenderCalled = true;
            return true;
        }

        public void Dispose()
        {
            DisposeCalled = true;
        }
    }

    /// <summary>Mock IPostFxPassProvider for testing.</summary>
    internal sealed class MockPassProvider : IPostFxPassProvider
    {
        private readonly IPostFxPass _pass;

        public PostFxEffect Effect => PostFxEffect.Bloom;
        public string DisplayName => "Mock";
        public Material? Material => null;

        public MockPassProvider(IPostFxPass pass) => _pass = pass;

        public void Init(PostStack owner) => _pass.Init(owner);
        public bool Render(RenderTexture src, RenderTexture dst) => _pass.Render(src, dst);
        public void Dispose() => _pass.Dispose();
        public bool IsEnabled(PostStack owner) => true;
        public bool IsSupported(PostStack owner) => true;
        public void ApplyParams(PostStack owner) { }
    }
}
