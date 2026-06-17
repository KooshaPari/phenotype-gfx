<!-- AI-DD-META:START -->
<!-- This repository is planned, maintained, and managed by AI Agents only. -->
<!-- Slop issues are expected and intentionally present as part of an HITL-less -->
<!-- /minimized AI-DD metaproject of learning, refining, and building brute-force -->
<!-- training for both agents and the human operator. -->
![Downloads](https://img.shields.io/github/downloads/KooshaPari/phenotype-water/total?style=flat-square&label=downloads&color=blue)
![GitHub release](https://img.shields.io/github/v/release/KooshaPari/phenotype-water?style=flat-square&label=release)
![License](https://img.shields.io/github/license/KooshaPari/phenotype-water?style=flat-square)
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
<!-- ci-refresh: 2026-06-10T07:21:47Z -->
