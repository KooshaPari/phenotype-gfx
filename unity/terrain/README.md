<!-- AI-DD-META:START -->
<!-- This repository is planned, maintained, and managed by AI Agents only. -->
<!-- Slop issues are expected and intentionally present as part of an HITL-less -->
<!-- /minimized AI-DD metaproject of learning, refining, and building brute-force -->
<!-- training for both agents and the human operator. -->
![Downloads](https://img.shields.io/github/downloads/KooshaPari/phenotype-terrain/total?style=flat-square&label=downloads&color=blue)
![GitHub release](https://img.shields.io/github/v/release/KooshaPari/phenotype-terrain?style=flat-square&label=release)
![License](https://img.shields.io/github/license/KooshaPari/phenotype-terrain?style=flat-square)
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
<!-- ci-refresh: 2026-06-11T09:26:41Z -->
