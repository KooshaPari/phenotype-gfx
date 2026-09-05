# Summary

[Introduction](./README.md)

# Architecture

- [Voxel Kernel](./voxel.md)
  - [Chunk Storage](./voxel.md#chunk-storage)
  - [Greedy Meshing](./voxel.md#greedy-meshing)
  - [Level of Detail](./voxel.md#level-of-detail)
  - [SIMD Acceleration](./voxel.md#simd-acceleration)
- [Post-Processing](./postfx.md)
  - [SSAO](./postfx.md#ssao-screen-space-ambient-occlusion)
  - [Bloom](./postfx.md#bloom)
  - [ACES Tonemapping](./postfx.md#aces-tonemapping)
  - [SSGI](./postfx.md#ssgi-screen-space-global-illumination)
  - [Vignette](./postfx.md#vignette)
  - [Chromatic Aberration](./postfx.md#chromatic-aberration)
  - [LUT](./postfx.md#lut-color-grading)
- [Foreign Function Interface](./ffi.md)
  - [C-ABI Exports](./ffi.md#c-abi-exports)
  - [cbindgen Headers](./ffi.md#cbindgen-headers)
  - [Unity C# Wrapper](./ffi.md#unity-c-wrapper)
- [Unity Integration](./unity.md)
  - [PhenotypeGfx.cs Lifecycle](./unity.md#phenotypegfxcs-lifecycle)
  - [NUnit Test Suite](./unity.md#nunit-test-suite)

# Operations

- [Development](./development.md)
  - [Build](./development.md#build)
  - [Test](./development.md#test)
  - [Run Examples](./development.md#run-examples)
  - [Cargo Features](./development.md#cargo-features)

---

[phenotype-gfx on GitHub](https://github.com/KooshaPari/phenotype-gfx)
