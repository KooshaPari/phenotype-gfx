# UPSTREAM — Lessons Learned from WSM3D

This document captures non-obvious pitfalls discovered during the
WorldSphereMod3D (WSM3D) project that any engine consuming `phenotype-voxel`
must understand. These are not theoretical; each item bit us in production.

---

## GPU Instancing Must Be Explicitly Enabled on Materials

**Root cause recorded in WSM3D:** `MeshInstanceBatcher` and
`MeshInstanceBatcherBRG` submit `Graphics.DrawMeshInstanced` / BRG draw
calls at runtime. Unity does **not** automatically enable instancing on a
material just because you call a batched draw API — you must set
`material.enableInstancing = true` explicitly after creating or loading
the material.

**Observed failure mode:** Actors and buildings rendered correctly at
count=1 but silently degraded to one draw-call-per-mesh once the batch
size exceeded 1. There was no error in the console; the only symptom was
a GPU frame-time spike and profiler showing thousands of individual
`DrawMesh` calls instead of a handful of `DrawMeshInstanced` calls.

**Fix:** Always set `enableInstancing = true` on every material that
will be submitted through an instanced batcher:

```csharp
// C# / Unity example (WSM3D pattern)
var mat = new Material(Shader.Find("Universal Render Pipeline/Lit"));
mat.enableInstancing = true;   // REQUIRED — not the default
```

```rust
// Rust / Bevy example equivalent
// StandardMaterial does not have enableInstancing; the Bevy BatchedUniformBuffer
// path is always-on for StandardMaterial. For custom materials implementing
// AsBindGroup you must add #[instanced] to the material's derive macro or
// implement GpuMesh instancing manually. Failing to do so produces the same
// silent per-draw-call fallback.
```

**Engine-specific notes for phenotype-voxel consumers:**

| Engine | Where to enable |
|--------|----------------|
| Unity (URP/HDRP) | `Material.enableInstancing = true` |
| Bevy | `#[instanced]` on custom `AsBindGroup` impl; built-in `StandardMaterial` is fine |
| Godot 4 | `GeometryInstance3D.use_multi_mesh` or a `MultiMeshInstance3D` node |

---

## BatchRendererGroup (BRG) Paths Can Silently Disable Instancing

**Root cause recorded in WSM3D:** The `MeshInstanceBatcherBRG` path
(Unity's `BatchRendererGroup` API, used for DOTS-style rendering) has an
additional gate: `!UseBRG` (a `SavedSettings` flag) silently killed
`enableInstancing` on the standard material path when BRG was off. The
standard batcher took over, but because `enableInstancing` was never set
on the fallback material, instancing was entirely absent.

**Observed failure mode:** BRG worked; toggling `UseBRG=false` for
debugging made every mesh draw non-instanced. The flag looked like it
only controlled the BRG code path, but it also gatekept instancing on
the fallback path.

**Pattern to follow in any renderer that has two draw-call back-ends
(BRG/standard, GPU/CPU, etc.):**

1. Enable instancing at material-creation time, unconditionally, before
   any rendering flag is evaluated.
2. The BRG / GPU path may add *additional* instancing optimizations, but
   the base material must already be instancing-capable.
3. Add a startup assertion or CI test that verifies `enableInstancing`
   is set on every material that goes through an instanced batcher.

---

## OpaqueVertexColor Shader Upgrade Race

**Root cause recorded in WSM3D:** The `OpaqueVertexColor` shader
(a custom URP shader used for voxel meshes that bake color into
`mesh.colors`) must be assigned to the material **after** the
`AssetBundle` containing it has fully loaded. A timing race during
`Mod.OnLoad` caused the material to be created with `Standard` shader
first and `OpaqueVertexColor` applied on the next frame. The `Standard`
shader does not read vertex colors, so the mesh rendered solid black for
one frame — and more critically, `enableInstancing` set on the `Standard`
material was **not** preserved after the shader swap.

**Fix:** Upgrade the shader and set `enableInstancing` in the same
synchronous block, after the bundle's `LoadAsset<Shader>` call completes.
Never split these two operations across frames or callbacks.

---

## Summary Checklist

When wiring up a new renderer that consumes `phenotype-voxel` meshes:

- [ ] Set `enableInstancing = true` (or engine equivalent) on every
      material that will be batched.
- [ ] Do **not** rely on a BRG / GPU-instancing back-end to implicitly
      enable instancing on the fallback material path.
- [ ] Assign the correct shader and set material flags **atomically**
      (same frame / same synchronous block), never split across callbacks.
- [ ] Add a smoke-test that renders > 1 instance and verifies the
      draw-call count in the profiler or a test harness.
