# phenotype-water

Phenotype-org shared package providing a Gerstner-wave water system for Unity (net48).

Designed to be consumed by any Phenotype mod that needs animated water surfaces --
ocean, lake, and river rendering with camera-aware LOD.

## Components

- **GerstnerWaveBank** -- parameterised wave bank evaluated per-vertex for surface displacement.
- **FluidMesh** -- procedural grid mesh driven by the wave bank.
- **WaterLod** -- camera-distance LOD controller for vertex density and wave eval frequency.

## Usage

Reference `phenotype-water.csproj` as a sibling project dependency from your mod's `.csproj`.

## License

See org-level LICENSE.
