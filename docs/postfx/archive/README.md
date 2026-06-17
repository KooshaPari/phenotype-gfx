# phenotype-postfx

Reusable BRP post-processing stack for Unity. Deterministic SSAO → SSGI → Bloom → ACES tonemap → LUT color grading chain via `OnRenderImage`.

## Usage

1. Add `PostStack` component to your main camera
2. Toggle passes via the inspector: `EnableSSAO`, `EnableSSGI`, `EnableBloom`, `EnableACES`, `EnableLUT`
3. Assign a 256x16 LUT strip texture to `LutTexture` for color grading

```csharp
var cam = Camera.main;
var stack = cam.gameObject.AddComponent<Phenotype.PostFx.PostStack>();
stack.EnableSSAO = true;
stack.EnableBloom = true;
stack.EnableACES = true;
```

## Shaders

| Shader | Purpose |
|--------|---------|
| `BrpACES.shader` | Filmic ACES tonemap curve |
| `BrpBloom.shader` | 4-pass bloom (threshold → blur H → blur V → composite) |
| `ScreenSpaceAO.shader` | 8-tap rotated kernel SSAO |
| `ColorGradingLUT.shader` | 32-slice strip LUT lookup |

## Preventing Shader Stripping

Unity's build-time shader stripper removes variants it cannot statically
prove are reachable. If you ship phenotype-postfx inside an AssetBundle,
include `Runtime/phenotype-postfx-variants.shadervariants` in your
AssetBundle build to keep all five post-FX shaders alive:

```
# In your AssetBundle manifest / addressables group:
Runtime/phenotype-postfx-variants.shadervariants
```

Alternatively, reference the SVC from a `PreloadedAssets` entry or from a
`GraphicsSettings` warmup list. Without it, one or more passes (most often
`ScreenSpaceGI` and `BrpBloom`) will silently fall back to the error
magenta shader at runtime.

## Requirements

- Unity 2021.3+ (Built-In Render Pipeline)
- Camera with `DepthTextureMode.Depth` (auto-enabled by PostStack)

## Origin

Extracted from [WorldSphereMod3D](https://github.com/KooshaPari/WorldSphereMod) WSM3DPostStack.
