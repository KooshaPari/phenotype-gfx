// T35: phenotype-postfx Unity Test Framework (UTF) EditMode tests.
// Covers the BRP postfx chain public API surface.
using NUnit.Framework;
using UnityEngine;
using Phenotype.PostFx;

public class PostStackEditTests
{
    [Test] public void PostFxEffect_HasSevenEffects()
    {
        var names = System.Enum.GetNames(typeof(PostFxEffect));
        Assert.AreEqual(7, names.Length);
    }

    [Test] public void SSAO_IsFirstEffect() => Assert.AreEqual(0, (int)PostFxEffect.SSAO);
    [Test] public void SSGI_IsSecondEffect() => Assert.AreEqual(1, (int)PostFxEffect.SSGI);
    [Test] public void Bloom_IsThirdEffect() => Assert.AreEqual(2, (int)PostFxEffect.Bloom);
    [Test] public void ACES_IsFourthEffect() => Assert.AreEqual(3, (int)PostFxEffect.ACES);
    [Test] public void Vignette_IsFifthEffect() => Assert.AreEqual(4, (int)PostFxEffect.Vignette);
    [Test] public void ChromaticAberration_IsSixthEffect() => Assert.AreEqual(5, (int)PostFxEffect.ChromaticAberration);
    [Test] public void LUT_IsLastEffect() => Assert.AreEqual(6, (int)PostFxEffect.LUT);

    [Test] public void PostStack_HasPassRegistry() => Assert.IsNotNull(new PostStack().PassRegistry);

    [Test] public void PostStack_HasSetAvailabilityProvider()
    {
        var stack = new PostStack();
        Assert.IsNotNull(typeof(PostStack).GetMethod("SetAvailabilityProvider"));
    }

    [Test] public void PostStack_HasValidateShaderVariants()
    {
        Assert.IsNotNull(typeof(PostStack).GetMethod("ValidateShaderVariants",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance));
    }

    [Test] public void PostStack_HasEnableVignette() => Assert.IsNotNull(typeof(PostStack).GetField("EnableVignette"));
    [Test] public void PostStack_HasEnableChromaticAberration() => Assert.IsNotNull(typeof(PostStack).GetField("EnableChromaticAberration"));

    [Test] public void PassRegistry_CreateDefault_HasProviders()
    {
        var reg = PostFxPassRegistry.CreateDefault();
        int count = 0;
        foreach (var _ in reg.Providers) count++;
        Assert.Greater(count, 0);
    }
}
