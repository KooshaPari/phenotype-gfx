using BenchmarkDotNet.Attributes;
using BenchmarkDotNet.Jobs;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace Phenotype.PostFx.Benchmarks;

[MemoryDiagnoser]
[SimpleJob(RuntimeMoniker.Net90, launchCount: 1, warmupCount: 3, iterationCount: 5)]
public class PostStackBenchmarks
{
    // ------------------------------------------------------------------
    // Shared state
    // ------------------------------------------------------------------
    private PostStack _stack = null!;
    private PostFxPassRegistry _registry = null!;
    private RenderTexture _src = null!;
    private RenderTexture _dst = null!;
    private readonly Dictionary<PostFxEffect, bool> _allAvailable = new()
    {
        [PostFxEffect.SSAO] = true,
        [PostFxEffect.SSGI] = true,
        [PostFxEffect.Bloom] = true,
        [PostFxEffect.ACES] = true,
        [PostFxEffect.Vignette] = true,
        [PostFxEffect.ChromaticAberration] = true,
        [PostFxEffect.LUT] = true,
    };
    private readonly Dictionary<PostFxEffect, bool> _noneAvailable = new()
    {
        [PostFxEffect.SSAO] = false,
        [PostFxEffect.SSGI] = false,
        [PostFxEffect.Bloom] = false,
        [PostFxEffect.ACES] = false,
        [PostFxEffect.Vignette] = false,
        [PostFxEffect.ChromaticAberration] = false,
        [PostFxEffect.LUT] = false,
    };

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------
    private static Material CreateStubMaterial()
    {
        var shader = new Shader();
        return new Material(shader);
    }

    private static void SetMaterialProperties(Material mat)
    {
        mat.SetInt("_SampleCount", 12);
        mat.SetFloat("_Radius", 2.0f);
        mat.SetFloat("_Bias", 0.0012f);
        mat.SetFloat("_Intensity", 1.0f);
        mat.SetVector("_Center", new Vector4(0.5f, 0.5f, 0f, 0f));
        mat.SetFloat("_Smoothness", 0.6f);
        mat.SetFloat("_Roundness", 1.0f);
    }

    private static PostStack CreateStackWithMaterials()
    {
        var stack = new PostStack();
        var mat1 = CreateStubMaterial();
        var mat2 = CreateStubMaterial();
        var mat3 = CreateStubMaterial();
        var mat4 = CreateStubMaterial();
        var mat5 = CreateStubMaterial();
        var mat6 = CreateStubMaterial();
        var mat7 = CreateStubMaterial();

        SetMaterialProperties(mat1);
        SetMaterialProperties(mat2);
        SetMaterialProperties(mat3);
        SetMaterialProperties(mat4);
        SetMaterialProperties(mat5);
        SetMaterialProperties(mat6);
        SetMaterialProperties(mat7);

        stack.SetAvailabilityProvider(new MockAvailabilityProvider(new Dictionary<PostFxEffect, bool>
        {
            [PostFxEffect.SSAO] = true,
            [PostFxEffect.SSGI] = true,
            [PostFxEffect.Bloom] = true,
            [PostFxEffect.ACES] = true,
            [PostFxEffect.Vignette] = true,
            [PostFxEffect.ChromaticAberration] = true,
            [PostFxEffect.LUT] = true,
        }));

        // Inject materials via reflection so we don't rely on Resources.Load
        var fieldNames = new[]
        {
            ("_ssaoMat", mat1),
            ("_ssgiMat", mat2),
            ("_bloomMat", mat3),
            ("_acesMat", mat4),
            ("_vignetteMat", mat5),
            ("_chromaticAberrationMat", mat6),
            ("_lutMat", mat7),
        };

        foreach (var (name, mat) in fieldNames)
        {
            var field = typeof(PostStack).GetField(name,
                System.Reflection.BindingFlags.NonPublic |
                System.Reflection.BindingFlags.Instance)!;
            field.SetValue(stack, mat);
        }

        stack.EnableSSAO = true;
        stack.EnableSSGI = true;
        stack.EnableBloom = true;
        stack.EnableACES = true;
        stack.EnableVignette = true;
        stack.EnableChromaticAberration = true;
        stack.EnableLUT = true;

        return stack;
    }

    // ------------------------------------------------------------------
    // Benchmark 1: Material creation time
    // ------------------------------------------------------------------
    [Benchmark(Description = "Material creation + property setup")]
    public Material MaterialCreation()
    {
        var mat = CreateStubMaterial();
        SetMaterialProperties(mat);
        return mat;
    }

    [Benchmark(Description = "Full material batch creation (7 effects)")]
    public List<Material> MaterialBatchCreation()
    {
        var materials = new List<Material>(7);
        for (int i = 0; i < 7; i++)
        {
            var mat = CreateStubMaterial();
            SetMaterialProperties(mat);
            materials.Add(mat);
        }
        return materials;
    }

    // ------------------------------------------------------------------
    // Benchmark 2: Blit pass execution time
    // ------------------------------------------------------------------
    [GlobalSetup(Target = nameof(BlitPassSingle))]
    public void SetupBlitPass()
    {
        _stack = CreateStackWithMaterials();
        _stack.ValidateShaderVariants();
        _registry = PostFxPassRegistry.CreateDefault();
        _registry.Init(_stack);
        _src = new RenderTexture { width = 1920, height = 1080 };
        _dst = new RenderTexture { width = 1920, height = 1080 };
    }

    [GlobalCleanup(Target = nameof(BlitPassSingle))]
    public void CleanupBlitPass()
    {
        _src = null!;
        _dst = null!;
        _stack = null!;
        _registry = null!;
    }

    [Benchmark(Description = "Single blit pass execution")]
    public bool BlitPassSingle()
    {
        var provider = _registry.GetProvider(PostFxEffect.ACES);
        if (provider == null) return false;
        provider.ApplyParams(_stack);
        return provider.Render(_src, _dst);
    }

    [Benchmark(Description = "Registry pass enumeration (all active)")]
    public int RegistryEnumerateAllActive()
    {
        int count = 0;
        foreach (var provider in _registry.Providers)
        {
            if (provider.IsEnabled(_stack) && provider.IsSupported(_stack) && provider.Material != null)
                count++;
        }
        return count;
    }

    [Benchmark(Description = "HasAnyActivePass check")]
    public bool HasAnyActivePass()
    {
        return _registry.HasAnyActivePass(_stack);
    }

    [Benchmark(Description = "Bloom multi-pass execution")]
    public bool BloomMultiPass()
    {
        var provider = _registry.GetProvider(PostFxEffect.Bloom);
        if (provider == null) return false;
        return provider.Render(_src, _dst);
    }

    // ------------------------------------------------------------------
    // Benchmark 3: Shader variant switching overhead
    // ------------------------------------------------------------------
    [GlobalSetup(Target = nameof(ShaderVariantValidationAllSupported))]
    public void SetupVariantValidation()
    {
        _stack = CreateStackWithMaterials();
    }

    [GlobalCleanup(Target = nameof(ShaderVariantValidationAllSupported))]
    public void CleanupVariantValidation()
    {
        _stack = null!;
    }

    [Benchmark(Description = "ValidateShaderVariants (all supported)")]
    public void ShaderVariantValidationAllSupported()
    {
        _stack.SetAvailabilityProvider(new MockAvailabilityProvider(_allAvailable));
        _stack.ValidateShaderVariants();
    }

    [Benchmark(Description = "ValidateShaderVariants (none supported)")]
    public void ShaderVariantValidationNoneSupported()
    {
        _stack.SetAvailabilityProvider(new MockAvailabilityProvider(_noneAvailable));
        _stack.ValidateShaderVariants();
    }

    [Benchmark(Description = "Availability provider switching + re-validation")]
    public void AvailabilityProviderSwitching()
    {
        _stack.SetAvailabilityProvider(new MockAvailabilityProvider(_allAvailable));
        _stack.ValidateShaderVariants();
        _stack.SetAvailabilityProvider(new MockAvailabilityProvider(_noneAvailable));
        _stack.ValidateShaderVariants();
    }

    [Benchmark(Description = "Pass registry re-init owner")]
    public void RegistryRebindOwner()
    {
        var registry = PostFxPassRegistry.CreateDefault();
        registry.Init(_stack);
    }
}

// ------------------------------------------------------------------
// Mock availability provider (same pattern as tests)
// ------------------------------------------------------------------
internal sealed class MockAvailabilityProvider : IShaderAvailabilityProvider
{
    readonly Dictionary<PostFxEffect, bool> _map;

    public MockAvailabilityProvider(Dictionary<PostFxEffect, bool> map)
        => _map = map;

    public bool IsAvailable(PostFxEffect effect)
        => _map.TryGetValue(effect, out bool v) && v;
}
