# Post-Processing

The post-processing pipeline lives in `src/postfx/`. It was ported from
C# + HLSL `phenotype-postfx` (L5-112, 2026-06-18) into the single Rust core
per ADR-004. The C# code is now a thin P/Invoke shim under `unity/postfx/`;
the HLSL shaders remain in `unity/postfx-shaders/` for the C# edge. All
real logic lives here in Rust.

## The seven passes

The stack exposes **seven independent post-processing passes**. Each is a
configuration struct plus a descriptor that the engine binds to a render pass
at runtime.

| # | Pass                  | Config type              | Source                                |
| - | --------------------- | ------------------------ | ------------------------------------- |
| 1 | SSAO                  | `SsaoConfig`             | `src/postfx/ssao_pass.rs`             |
| 2 | Bloom                 | `BloomConfig`            | `src/postfx/bloom_pass.rs`            |
| 3 | ACES Tonemapping      | `AcesConfig`             | `src/postfx/aces_pass.rs`             |
| 4 | SSGI                  | `SsgiConfig`             | `src/postfx/ssgi_pass.rs`             |
| 5 | Vignette              | `VignetteConfig`         | `src/postfx/vignette_pass.rs`         |
| 6 | Chromatic Aberration  | `ChromaticConfig`        | `src/postfx/chromatic_pass.rs`        |
| 7 | LUT Color Grading     | `LutConfig`              | `src/postfx/lut_pass.rs`              |

> **Upstream:** <https://github.com/KooshaPari/phenotype-postfx>

## PostStack

`PostStack` (`src/postfx/post_stack.rs`) is the central driver for the
post-processing stack. Engine-agnostic — the Rust side holds the
configuration, the validated availability flags, and the pass registry; the
C# edge binds the registry to a `MonoBehaviour` at runtime and dispatches
`on_render` for each pass in render order.

```rust,no_run
use phenotype_gfx::postfx::{PostStack, PostStackConfig, DEFAULT_POSTFX_STACK};

let stack = PostStack::new(PostStackConfig::default());
let descriptors = stack.describe_passes();
```

`describe_passes()` returns `Vec<PassDescriptor>` so that the editor /
driver can build an inspector / dispatcher without referring to the concrete
`BloomPass` / `SsaoPass` types.

## `PostStackConfig`

`PostStackConfig` (`post_stack.rs:31`) is the serialisable configuration for
the entire stack — one struct, no engine references:

```rust,no_run
use phenotype_gfx::postfx::PostStackConfig;
use phenotype_gfx::postfx::ports::post_fx_pass::PassQuality;

let cfg = PostStackConfig {
    enable_ssao: true,
    enable_ssgi: false,
    enable_bloom: true,
    enable_aces: true,
    enable_vignette: true,
    enable_chromatic_aberration: true,
    enable_lut: true,
    quality: PassQuality::High,
    ssao_samples: 16,
    ssao_radius: 0.5,
    ssao_bias: 0.025,
    ssao_intensity: 1.0,
    ssgi_samples: 32,
    ssgi_radius: 1.0,
    ssgi_intensity: 1.0,
    exposure: 1.0,
    vignette_intensity: 0.4,
    vignette_smoothness: 0.5,
    vignette_roundness: 1.0,
    vignette_center: [0.5, 0.5],
    chromatic_aberration_intensity: 0.005,
};
```

> **Default preset:** `DEFAULT_POSTFX_STACK` (`post_stack.rs:DEFAULT_POSTFX_STACK`)

## Pass-by-pass reference

### SSAO (Screen-Space Ambient Occlusion)

Configures ambient-occlusion darkening based on screen-space depth comparison.

| Field          | Type   | Purpose                                          |
| -------------- | ------ | ------------------------------------------------ |
| `samples`      | `u32`  | Number of sample kernel taps                     |
| `radius`       | `f32`  | World-space sampling radius                      |
| `bias`         | `f32`  | Depth bias to prevent self-occlusion             |
| `intensity`    | `f32`  | Final AO multiplier                              |

> **Source:** `src/postfx/ssao_pass.rs:SsaoConfig`

### Bloom

Extracts bright regions above a luminance threshold and blurs them back over
the scene to produce a glow.

| Field            | Type   | Purpose                                  |
| ---------------- | ------ | ---------------------------------------- |
| `threshold`      | `f32`  | Luminance threshold for bright pass      |
| `intensity`      | `f32`  | Final bloom multiplier                   |
| `kernel_size`    | `u32`  | Blur kernel radius                       |
| `downsample`     | `u32`  | Pyramid level for the bright-pass input  |

> **Source:** `src/postfx/bloom_pass.rs:BloomConfig`

### ACES Tonemapping

Maps HDR linear colour to display-referred output through the ACES filmic
curve, with an exposure multiplier.

| Field       | Type   | Purpose                                       |
| ----------- | ------ | --------------------------------------------- |
| `exposure`  | `f32`  | Pre-curve exposure multiplier (stop-based)    |

> **Source:** `src/postfx/aces_pass.rs:AcesConfig`

### SSGI (Screen-Space Global Illumination)

One-bounce indirect illumination estimated from screen-space radiance and
depth, used to fill the gap left by SSAO in interior scenes.

| Field       | Type   | Purpose                                  |
| ----------- | ------ | ---------------------------------------- |
| `samples`   | `u32`  | Number of rays per pixel                 |
| `radius`    | `f32`  | World-space radius for ray marching      |
| `intensity` | `f32`  | Indirect-light multiplier                |

> **Source:** `src/postfx/ssgi_pass.rs:SsgiConfig`

### Vignette

Darkens screen edges to focus attention on the centre.

| Field         | Type        | Purpose                                       |
| ------------- | ----------- | --------------------------------------------- |
| `intensity`   | `f32`       | Maximum darkening at the corners              |
| `smoothness`  | `f32`       | Edge-softness of the falloff                  |
| `roundness`   | `f32`       | Round vs. square falloff shape                |
| `center`      | `[f32; 2]`  | Normalized screen-space centre                |

> **Source:** `src/postfx/vignette_pass.rs:VignetteConfig`

### Chromatic Aberration

Splits the red / green / blue channels at the lens edges to simulate a
real-world lens defect.

| Field       | Type   | Purpose                                          |
| ----------- | ------ | ------------------------------------------------ |
| `intensity` | `f32`  | Per-channel offset at the edges of the frame     |

> **Source:** `src/postfx/chromatic_pass.rs:ChromaticConfig`

### LUT Color Grading

Applies a 3D lookup-table colour transform — typically baked from
photographic reference stills — for final image shaping.

| Field            | Type      | Purpose                                       |
| ---------------- | --------- | --------------------------------------------- |
| `lut_data`       | `LutData` | LUT pixel payload + format                    |
| `intensity`      | `f32`     | Blend between original and graded colour      |
| `format`         | `LutFormat` | `R8G8B8`, `R16G16B16`, `R16G16B16A16`       |

> **Source:** `src/postfx/lut_pass.rs:LutConfig`, `src/postfx/ports/lut_pipeline.rs`

## Pass descriptor model

Every pass is described uniformly by a `PassDescriptor` so the editor and
driver code never depends on the concrete pass types:

```rust,no_run
use phenotype_gfx::postfx::ports::post_fx_pass::{
    PassDescriptor, PassEffect, PassQuality, PostFxContext, PostFxPass,
};
```

- **`PassDescriptor`** — engine-neutral metadata: `name`, `effect`,
  `quality`, `enabled`, `order`.
- **`PostFxPass`** — the trait a pass implements to expose its
  `descriptor()`, `enabled()`, and `execute(context)`.
- **`PostFxContext`** — engine-agnostic context (input / output targets,
  camera, depth, normal buffers).

## Pass registry

`PostFxPassRegistry` (`post_fx_pass_registry.rs:PostFxPassRegistry`) holds the
ordered set of passes the engine should dispatch per frame. The C# edge binds
this registry to a `MonoBehaviour` and walks it in `render_order` to produce
the final image.

## Hexagonal ports

`src/postfx/ports/` provides the trait surfaces used to keep the engine edge
thin:

- **`material_registry`** — `PostFxMaterialRegistry`,
  `InMemoryPostFxMaterialRegistry`, `RecordingPostFxMaterialRegistry`,
  `PostFxMaterialInfo`, `PostFxMaterialKind`.
- **`serialization`** — `PostFxSerializationPort`,
  `JsonFilePostFxSerialization`, `PostFxStackSnapshot`.
- **`shader_availability`** — `PostFxShaderAvailability`,
  `DefaultPostFxShaderAvailability`.
- **`lut_pipeline`** — `LutData`, `LutFormat`.
- **`urp_render_graph`** — `BrpToUrpAdapter`, `PostFxUrpContext`,
  `PostFxUrpPass`.
- **`post_fx_pass`** — `PassDescriptor`, `PassEffect`, `PassQuality`,
  `PostFxContext`, `PostFxPass`.

## Engine-agnostic rendering types

`src/postfx/rendering.rs` provides engine-agnostic render-target + material
handle types used by the C# edge. `PostFxMaterial` is a `#[deprecated]`
pass-through kept for source-compat with the original C# `PostStack.cs`.

## HLSL shader constants

`src/postfx/shaders.rs` preserves HLSL shader constants verbatim from
upstream `phenotype-postfx/Runtime/Shaders/*.shader`. The actual HLSL source
lives in `unity/postfx-shaders/` and is shipped alongside the C# edge so
URP can include it via `Shader.Find`.
