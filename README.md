# phenotype-terrain

## State

Progress: `[██░░░░░░░░] 20%` — shared terrain mesh infrastructure (scaffold).

_Updated 2026-06-08 — audit pass._

[![CI](https://github.com/KooshaPari/phenotype-terrain/actions/workflows/dotnet-build.yml/badge.svg)](https://github.com/KooshaPari/phenotype-terrain/actions/workflows/dotnet-build.yml)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

Shared terrain mesh infrastructure for Phenotype-org mods targeting Unity/WorldBox.

This package extracts reusable terrain concerns — height-field storage, chunk mesh
generation, and LOD management — out of individual mod codebases so they can be
consumed as a sibling project reference. It follows the same hexagonal polyrepo
pattern as the other `phenotype-*` packages under the Phenotype org (e.g. the
in-repo sibling `phenotype-water` shares the same Unity/`net48`/`$(WorldBoxManaged)`
contract layout).

## Structure

| File | Responsibility |
|---|---|
| `src/HeightField.cs` | Per-tile elevation data and world-space Y queries |
| `src/ChunkMeshBuilder.cs` | Unity Mesh generation from height-field chunks |
| `src/TerrainLod.cs` | Camera-distance LOD selection for terrain chunks |

## Build

Requires the `WorldBoxManaged` MSBuild property pointing at the WorldBox
`Managed/` directory (same as WorldSphereMod's `Directory.Build.props`):

```powershell
$env:WorldBoxManaged = "C:/Program Files (x86)/Steam/steamapps/common/worldbox/worldbox_Data/Managed"
dotnet build phenotype-terrain.csproj -c Release
```

## Consuming from another mod

Add a `<ProjectReference>` in the consuming `.csproj`:

```xml
<ProjectReference Include="../phenotype-terrain/phenotype-terrain.csproj" />
```

## License

MIT — see [`LICENSE`](./LICENSE).

## Description

Shared terrain mesh infrastructure for Phenotype-org mods targeting Unity / WorldBox — height-field storage, chunk mesh generation, and camera-distance LOD selection, packaged as a sibling-project reference.

## Install

Reference the project from a consuming `.csproj`:

```xml
<ProjectReference Include="../phenotype-terrain/phenotype-terrain.csproj" />
```

Set the `WorldBoxManaged` MSBuild property before building (see below).

## Usage

Build: `$env:WorldBoxManaged = "..."; dotnet build phenotype-terrain.csproj -c Release`. Use `HeightField` for elevation queries, `ChunkMeshBuilder` for Unity `Mesh` generation, `TerrainLod` for camera-distance selection.

## Contributing

PRs welcome. See `CONTRIBUTING.md`. Keep the `net48` / `$(WorldBoxManaged)` contract stable so sibling mods don't break.
<!-- ci-refresh: 2026-06-10T07:21:45Z -->
