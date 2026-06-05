# phenotype-water

Phenotype-org shared package providing a Gerstner-wave water system for Unity (net48).

Designed to be consumed by any Phenotype mod that needs animated water surfaces --
ocean, lake, and river rendering with camera-aware LOD.

## Components

- **GerstnerWaveBank** — parameterised wave bank evaluated per-vertex for surface displacement.
- **FluidMesh** — procedural grid mesh driven by the wave bank.
- **WaterLod** — camera-distance LOD controller for vertex density and wave eval frequency.

## Build

Requires the `WorldBoxManaged` MSBuild property pointing at the WorldBox
`Managed/` directory (same as WorldSphereMod's `Directory.Build.props`):

```powershell
$env:WorldBoxManaged = "C:/Program Files (x86)/Steam/steamapps/common/worldbox/worldbox_Data/Managed"
dotnet build phenotype-water.slnx -c Release
```

The test project (`tests/phenotype-water.tests.csproj`) is wired up to the
xunit runner; once the Unity reference is resolvable locally you can run:

```powershell
dotnet test tests/phenotype-water.tests.csproj -c Release
```

## Consuming from another mod

Add a `<ProjectReference>` to the **library** (not the test project) in the
consuming `.csproj`:

```xml
<ProjectReference Include="../phenotype-water/phenotype-water.csproj" />
```

The test project is excluded from the library's compile glob and is not
intended to be taken as a dependency by consumers.

## License

MIT, consistent with the Phenotype org default. A `LICENSE` file has not
been committed to this repository yet — until one lands here, the
package is "All rights reserved" by default under copyright law. The
package owner should commit an MIT `LICENSE` file at the repo root
before the first public release, and consumers should treat the
absence of the file as a red flag.
