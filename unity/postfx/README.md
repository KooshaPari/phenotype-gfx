<!-- AI-DD-META:START -->
<!-- This repository is planned, maintained, and managed by AI Agents only. -->
<!-- Slop issues are expected and intentionally present as part of an HITL-less -->
<!-- /minimized AI-DD metaproject of learning, refining, and building brute-force -->
<!-- training for both agents and the human operator. -->
![Downloads](https://img.shields.io/github/downloads/KooshaPari/phenotype-postfx/total?style=flat-square&label=downloads&color=blue)
![GitHub release](https://img.shields.io/github/v/release/KooshaPari/phenotype-postfx?style=flat-square&label=release)
![License](https://img.shields.io/github/license/KooshaPari/phenotype-postfx?style=flat-square)
![AI-Slop](https://img.shields.io/badge/AI--DD-Slop%20Expected-orange?style=flat-square)
![AI-Only-Maintained](https://img.shields.io/badge/Planned%20%26%20Maintained%20by-AI%20Agents%20Only-red?style=flat-square)
![HITL-less](https://img.shields.io/badge/HITL--less%20AI--DD-metaproject-yellow?style=flat-square)

> ⚠️ **AI-Agent-Only Repository**
>
> This repo is **planned, maintained, and managed exclusively by AI Agents**.
> Slop issues, rough edges, and AI artifacts are **expected and intentionally
> present** as part of an **HITL-less / minimized AI-DD** metaproject focused
> on learning, refining, and brute-force training both the agents and the
> human operator. Bug reports and contributions are still welcome, but please
> expect AI-generated code, comments, and documentation throughout.
<!-- AI-DD-META:END -->
# phenotype-postfx

## State

Progress: `[███████░░░] 70%` — post-FX stack extracted from WSM3D, registry refactor merged.

_Updated 2026-06-08 — audit pass._

[![CI](https://github.com/KooshaPari/phenotype-postfx/actions/workflows/ci.yml/badge.svg)](https://github.com/KooshaPari/phenotype-postfx/actions)
[![License](https://img.shields.io/github/license/KooshaPari/phenotype-postfx)](LICENSE)

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
| `ScreenSpaceGI.shader` | Screen-space global illumination (raymarched) |
| `ColorGradingLUT.shader` | 32-slice strip LUT lookup |
| `ChromaticAberration.shader` | Per-channel UV offset fringe effect |
| `Vignette.shader` | Radial darkening mask |

## Preventing Shader Stripping

Unity's build-time shader stripper removes variants it cannot statically
prove are reachable. If you ship phenotype-postfx inside an AssetBundle,
include `Runtime/phenotype-postfx-variants.shadervariants` in your
AssetBundle build to keep all post-FX shaders alive:

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

## Description

Reusable BRP post-processing stack for Unity, extracted from WorldSphereMod3D. Deterministic SSAO/SSGI/Bloom/ACES/LUT chain via `OnRenderImage` with a composable registry.

## Install

Drop `Runtime/` into your Unity Assets folder (Built-In Render Pipeline, 2021.3+). Add the `PostStack` component to your main camera.

## Contributing

PRs welcome. See `CONTRIBUTING.md` (Phenotype-org standard). New passes go through `PostFxPassRegistry`; follow the existing `IPostFxPass` contract.

## License

MIT — see [`LICENSE`](./LICENSE).
<!-- ci-refresh: 2026-06-10T07:21:52Z -->
