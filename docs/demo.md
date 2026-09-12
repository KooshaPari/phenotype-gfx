# Phenotype GFX — Demo Walkthrough

> **Version:** pinned to v0.1.0 (`phenotype-gfx`)
> **Last updated:** 2026-08-29
> **Scope:** the seven `postfx` passes shipped by the Rust core, with one
> executable example per pass and the visual signature you should see.

This is the hands-on companion to [`postfx.md`](./postfx.md). It is intended
for graphics engineers integrating `phenotype-gfx` into a custom renderer,
for QA verifying the visual output, and for demo videographers who want a
known-good walkthrough to record.

Every example below is **idempotent** — running the same code twice produces
the same `describe_passes()` output and the same shader constants.

---

## 1. Pre-requisites

### 1.1 Hardware

| Component | Minimum                       | Recommended                  |
| --------- | ----------------------------- | ---------------------------- |
| CPU       | x86_64 4-core                 | x86_64 8-core + AVX2         |
| RAM       | 8 GiB                         | 16 GiB                       |
| GPU       | wgpu-capable (any backend)    | Discrete NVIDIA RTX series   |
| Disk      | 4 GiB free                    | 8 GiB SSD                    |

### 1.2 Toolchain

```bash
rustup component add rustfmt clippy
cargo install cbindgen            # for the C header regeneration step
```

The repo pins `rust-version` in `Cargo.toml`; anything at or above the
manifest's MSRV on `stable` resolves cleanly.

### 1.3 Optional substrate packages

| Substrate | Why                                                            |
| --------- | -------------------------------------------------------------- |
| Unity 2022.3 LTS+ | Required for the `unity/` C# edge tests                |
| Godot 4.3+         | Required for the `godot-ref` smoke tests                |
| Unreal Engine 5.4+ | Required for the `unreal-show` source build             |

You can run the pure-Rust demos without any of these — the `postfx` module
is engine-agnostic.

---

## 2. Build the workspace

```bash
cargo check --workspace                              # fastest sanity check
cargo build --release -p phenotype-gfx               # release artifacts
cargo test  -p phenotype-gfx postfx::               # all postfx unit tests
```

### 2.1 What you get

```text
target/release/libphenotype_gfx.rlib       # Rust core
target/release/libphenotype_gfx.so        # shared (Linux)
target/release/phenotype_gfx.dll          # shared (Windows)
target/release/phenotype_gfx.dylib        # shared (macOS)
```

If `cbindgen` is installed:

```bash
cargo build --release -p phenotype-gfx --features cbindgen
# C header emitted to target/phenotype_gfx.h
```

---

## 3. Demo 0 — list every pass

The single most useful smoke test: enumerate the seven passes with their
shader names, costs, and defaults.

```rust,no_run
use phenotype_gfx::postfx::{PostStack, PostStackConfig, DEFAULT_POSTFX_STACK};

fn main() {
    let stack = PostStack::new(PostStackConfig::default());
    for d in stack.describe_passes() {
        println!(
            "{:<18} shader={:<32} enabled={:<5} cost={:.2}",
            format!("{:?}", d.effect),
            d.shader_name,
            d.default_enabled,
            d.cost
        );
    }
    assert_eq!(stack.describe_passes().len(), 7, "expected 7 PostFX passes");
    let _ = DEFAULT_POSTFX_STACK;
}
```

**Expected output (abridged):**

```text
Ssao                shader=Hidden/Phenotype/SSAOPass          enabled=true   cost=0.25
Bloom               shader=Hidden/Phenotype/BloomPass         enabled=true   cost=0.35
Aces                shader=Hidden/Phenotype/ACESPass          enabled=true   cost=0.10
Ssgi                shader=Hidden/Phenotype/SSGIPass          enabled=false  cost=0.30
Vignette            shader=Hidden/WSM3D/Vignette              enabled=true   cost=0.10
ChromaticAberration shader=Hidden/WSM3D/ChromaticAberration   enabled=true   cost=0.05
Lut                 shader=WSM3D/ColorGradingLUT              enabled=true   cost=0.10
```

---

## 4. The seven PostFX passes

For each pass below you get:

1. **Config struct** — the Rust-side knobs you can tweak.
2. **Shader invocation** — the exact `PassDescriptor::shader_name` plus the
   keyword you must `#pragma multi_compile` for.
3. **Minimal example** — a copy-paste-runnable snippet.
4. **Expected visual effect** — what the rendered frame should look like.

### 4.1 SSAO — Screen-Space Ambient Occlusion

| Field         | Type   | Default |
| ------------- | ------ | ------- |
| `is_enabled`  | `bool` | `true`  |
| `radius`      | `f32`  | `0.5`   |
| `intensity`   | `f32`  | `1.2`   |
| `bias`        | `f32`  | `0.04`  |
| `kernel_size` | `u32`  | `8`     |

```rust,no_run
use phenotype_gfx::postfx::SsaoConfig;

let ssao = SsaoConfig {
    is_enabled: true,
    radius: 0.5,
    intensity: 1.4,
    bias: 0.04,
    kernel_size: 16,
};
```

**Shader invocation**

```hlsl
Shader.Find("Hidden/Phenotype/SSAOPass");
#pragma multi_compile _ SSAOPASS
```

The kernel samples the depth buffer at offsets drawn from a deterministic
LCG seeded with `1337` (matching the C# `new System.Random(1337)` upstream),
which keeps the AO pattern byte-identical run-to-run.

**Expected visual effect:** creases and concave corners darken proportionally
to `intensity`. With `kernel_size = 16` and `radius = 0.5`, half-meter gaps
between voxel columns visibly deepen. Self-occlusion under flat ground is
prevented by the `bias = 0.04` floor.

---

### 4.2 Bloom — multi-pass bright extract + blur

| Field         | Type   | Default |
| ------------- | ------ | ------- |
| `is_enabled`  | `bool` | `true`  |
| `threshold`   | `f32`  | `0.8`   |
| `intensity`   | `f32`  | `0.5`   |
| `iterations`  | `u32`  | `2`     |

```rust,no_run
use phenotype_gfx::postfx::BloomConfig;

let bloom = BloomConfig {
    is_enabled: true,
    threshold: 0.85,
    intensity: 0.6,
    iterations: 3,
};
```

**Shader invocation**

```hlsl
Shader.Find("Hidden/Phenotype/BloomPass");
#pragma multi_compile BLOOM_LOW BLOOM_MEDIUM BLOOM_HIGH BLOOM_ULTRA
// quality → keyword mapping:
//   Low    → BLOOM_LOW     (KERNEL_SIZE = 2)
//   Medium → BLOOM_MEDIUM  (KERNEL_SIZE = 5)
//   High   → BLOOM_HIGH    (KERNEL_SIZE = 7)
//   Ultra  → BLOOM_ULTRA   (KERNEL_SIZE = 9)
```

**Expected visual effect:** pixels with luminance above `threshold` (the
bright-pass uses the BT.709 weights `0.2126 / 0.7152 / 0.0722`) bleed
outward through a separable Gaussian blur (horizontal then vertical pass
per iteration). With `iterations = 3`, glow halos extend roughly 24 px at
1080p and stack additively in screen-space.

---

### 4.3 ACES Filmic Tonemapping

| Field        | Type   | Default |
| ------------ | ------ | ------- |
| `is_enabled` | `bool` | `true`  |
| `exposure`   | `f32`  | `1.0`   |
| `gamma`      | `f32`  | `2.2`   |

```rust,no_run
use phenotype_gfx::postfx::AcesConfig;

let aces = AcesConfig {
    is_enabled: true,
    exposure: 1.05,
    gamma: 2.2,
};
```

**Shader invocation**

```hlsl
Shader.Find("Hidden/Phenotype/ACESPass");
#pragma multi_compile _ ACES
```

The Rust core's `AcesConfig::aces_filmic` uses Krzysztof Narkowicz's
constants (`a = 2.51, b = 0.03, c = 2.43, d = 0.59, e = 0.14`).

**Expected visual effect:** HDR linear input collapses into a filmic
display-referred curve. Highlights roll off smoothly instead of clipping,
and mid-tones retain contrast. Set `exposure = 1.5` to over-expose a noon
shot by one stop without burning the highlights.

---

### 4.4 SSGI — Screen-Space Global Illumination

| Field        | Type   | Default  |
| ------------ | ------ | -------- |
| `is_enabled` | `bool` | `false`  |
| `samples`    | `u32`  | `12`     |
| `radius`     | `f32`  | `1.8`    |
| `intensity`  | `f32`  | `0.45`   |

```rust,no_run
use phenotype_gfx::postfx::SsgiConfig;

let ssgi = SsgiConfig {
    is_enabled: true,
    samples: 24,
    radius: 2.0,
    intensity: 0.6,
};
```

**Shader invocation**

```hlsl
Shader.Find("Hidden/Phenotype/SSGIPass");
#pragma multi_compile _ SSGIPASS
```

Sample directions come from a golden-ratio low-discrepancy sequence
(`phi = π * (1 + √5)`), so increasing `samples` covers the hemisphere
evenly without clustering.

**Expected visual effect:** interior cavities fill with a soft coloured
bounce proportional to `radius` and `intensity`. The pass is **off by
default** because it doubles fragment cost; turn it on when the camera is
inside a built structure.

---

### 4.5 Vignette

| Field         | Type        | Default      |
| ------------- | ----------- | ------------ |
| `is_enabled`  | `bool`      | `true`       |
| `center`      | `[f32; 2]`  | `[0.5, 0.5]` |
| `intensity`   | `f32`       | `0.45`       |
| `smoothness`  | `f32`       | `0.6`        |
| `roundness`   | `f32`       | `1.0`        |

```rust,no_run
use phenotype_gfx::postfx::VignetteConfig;

let vignette = VignetteConfig {
    is_enabled: true,
    center: [0.5, 0.5],
    intensity: 0.55,
    smoothness: 0.7,
    roundness: 1.2,
};
```

**Shader invocation**

```hlsl
Shader.Find("Hidden/WSM3D/Vignette");
// no keyword — single-pass shader
```

The falloff uses `1 - smoothstep(inner, 1, dist)`, where `inner = saturate(1 - smoothness)`.

**Expected visual effect:** the corners darken towards `1 - intensity` at
the frame edges. `roundness > 1` squashes the vignette horizontally, useful
for ultra-wide aspect ratios.

---

### 4.6 Chromatic Aberration

| Field        | Type   | Default |
| ------------ | ------ | ------- |
| `is_enabled` | `bool` | `true`  |
| `intensity`  | `f32`  | `0.15`  |

```rust,no_run
use phenotype_gfx::postfx::ChromaticConfig;

let chromatic = ChromaticConfig {
    is_enabled: true,
    intensity: 0.20,
};
```

**Shader invocation**

```hlsl
Shader.Find("Hidden/WSM3D/ChromaticAberration");
// no keyword — single-pass shader
```

The shader computes a per-channel offset along the radial direction with
`shift = _Intensity * dist * 0.03`, then samples R, G, B independently.

**Expected visual effect:** red is offset outward and blue inward along
the radial axis, simulating a real-world lens defect. The displacement is
linear in distance from the screen centre, so corners exhibit the strongest
fringing. Keep `intensity ≤ 0.20` for a tasteful look.

---

### 4.7 LUT Color Grading

| Field         | Type       | Default                  |
| ------------- | ---------- | ------------------------ |
| `is_enabled`  | `bool`     | `true`                   |
| `lut_data`    | `LutData`  | identity LUT (passthrough) |
| `intensity`   | `f32`      | `1.0`                    |
| `format`      | `LutFormat`| `R8G8B8`                 |

```rust,no_run
use phenotype_gfx::postfx::{LutConfig, ports::lut_pipeline::{LutData, LutFormat}};

let lut = LutConfig {
    is_enabled: true,
    lut_data: LutData::identity_32(),
    intensity: 0.9,
    format: LutFormat::R8G8B8,
};
```

**Shader invocation**

```hlsl
Shader.Find("WSM3D/ColorGradingLUT");
// LUT shipped as a 32-slice horizontal strip (W = 32 * 32, H = 32)
// bound to sampler2D _LUT_Tex2D
```

The shader does both `_Exposure` and `_Saturation` pre-emphasis before the
LUT lookup. Lookup is bi-linear between adjacent blue slices for smoothness.

**Expected visual effect:** the colour curve is resampled through the LUT.
With `intensity = 1.0` the result is fully graded; `intensity = 0.5` blends
halfway with the original image. Use `R16G16B16A16` for film-style LUTs that
need >8-bit per channel to avoid banding in the shadows.

---

## 5. Putting it all together — `DEFAULT_POSTFX_STACK`

The single recommended way to wire the seven passes in your engine:

```rust,no_run
use phenotype_gfx::postfx::{PostStack, PostStackConfig, DEFAULT_POSTFX_STACK};

fn configure_postfx() -> PostStack {
    // Start from the in-tree defaults (SSAO + Bloom + ACES + Vignette +
    // Chromatic + LUT on; SSGI off because it's expensive).
    let mut cfg: PostStackConfig = DEFAULT_POSTFX_STACK.clone();

    // Turn SSGI on for the dungeon-cam, leave the surface-cam alone.
    cfg.enable_ssgi = true;
    cfg.ssgi_samples = 24;

    // Push bloom a little for the neon-lit night scenes.
    cfg.bloom.intensity = 0.75;

    // Warm LUT for sunset.
    cfg.lut.intensity = 0.85;

    PostStack::new(cfg)
}
```

Then in your render-graph code, ask the stack which passes are present and
dispatch them in `render_order`:

```rust,no_run
use phenotype_gfx::postfx::PostStack;

let stack = configure_postfx();
for descriptor in stack.describe_passes() {
    if !descriptor.default_enabled { continue; }
    println!("dispatch {:?} via shader {}", descriptor.effect, descriptor.shader_name);
}
```

---

## 6. End-to-end visual checklist

When recording a demo reel, the seven signatures to look for in the final
frame are:

| Pass               | Look for                                                                                 |
| ------------------ | ---------------------------------------------------------------------------------------- |
| SSAO               | Crease shadows where two walls meet at a right angle                                     |
| Bloom              | Soft glow halos around emissive surfaces (campfires, neon, sun)                          |
| ACES               | No clipped highlights; sky / sun read as photographic, not flat                          |
| SSGI               | Soft coloured bounce in enclosed spaces (only visible when `enable_ssgi = true`)         |
| Vignette           | Corners ~25–50 % darker than the centre, falloff smooth                                  |
| Chromatic Aberration | Red / blue fringing at the extreme corners of the frame                                |
| LUT                | Overall colour cast matching the chosen grading LUT (warm sunset, cool night, etc.)      |

A correct stack shows **all seven** in a single frame. If any single pass
is missing, the most common cause is the `PassQuality::Off` keyword being
set globally — the `PostFxShaderAvailability` port will skip every pass
with `default_enabled = false` in that case.

---

## 7. Troubleshooting

| Symptom                                         | Likely cause                                         |
| ----------------------------------------------- | ---------------------------------------------------- |
| Pass missing from `describe_passes()`          | `PassQuality::Off` in the global registry             |
| SSAO looks noisy / flickering                   | `kernel_size < 8` or `bias < 0.02`                   |
| Bloom looks blocky                              | `iterations < 2`, or the bright-pass target is 8-bit |
| ACES looks washed out                           | `exposure > 1.5` or `gamma != 2.2`                   |
| SSGI produces fireflies                         | `samples < 16` — raise to ≥ 24 for stable output     |
| Vignette is a hard mask                         | `smoothness < 0.2` — raise to ≥ 0.5                  |
| Chromatic aberration invisible                  | `intensity < 0.05` — typical sweet spot is 0.10–0.20 |
| LUT banding in shadows                          | `format = R8G8B8` — switch to `R16G16B16A16`         |

For deeper integration questions, see [`postfx.md`](./postfx.md) and
[`development.md`](./development.md).
