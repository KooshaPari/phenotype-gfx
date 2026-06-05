# phenotype-water

Shared Unity water package for Phenotype-org mods targeting Unity / WorldBox. Provides a Gerstner-wave water system with `GerstnerWaveBank`, `FluidMesh`, and `WaterLod` for camera-distance LOD. Designed for net48 Unity builds.

## Stack

| Layer | Technology |
|-------|------------|
| Language | C# |
| Build | `dotnet build phenotype-water.slnx` (Microsoft.NET.Sdk) |
| Target | net48 (Unity-friendly runtime surface) |
| Consumers | Sibling project references in Unity mods |

## Key Commands

```bash
dotnet build phenotype-water.slnx
dotnet test    # when tests exist
```

Unity consumers add the project as a sibling project reference; rebuild after public surface changes.

## Key Files

- `phenotype-water.slnx` — Solution file
- `phenotype-water.csproj` — Project file (Microsoft.NET.Sdk, net48)
- `src/GerstnerWaveBank.cs` — Parameterised wave bank with per-vertex surface displacement
- `src/FluidMesh.cs` — Procedural grid mesh driven by the wave bank
- `src/WaterLod.cs` — Camera-distance LOD controller for vertex density and wave eval frequency
- `README.md` — Usage and structure documentation
- `AGENTS.md` — Local agent governance (canonical for working conventions)
- `CONTRIBUTING.md` — Contributor guide (AgilePlus mandate, branch conventions, PR expectations)
- `SECURITY.md` — Vulnerability disclosure path
- `.editorconfig`, `.gitattributes` — formatting / line-ending rules (Unity `.meta` files use `unityyamlmerge`)

## Reference

- **Local source of truth for agent behavior:** `AGENTS.md`
- **Global Phenotype rules:** `~/.claude/CLAUDE.md` or `/Users/kooshapari/CodeProjects/Phenotype/repos/CLAUDE.md`
- **AgilePlus work tracking:** `cd /repos/AgilePlus && agileplus <command>` (required for non-trivial work per the CONTRIBUTING mandate)
- **Sibling shared packages:** `phenotype-voxel`, `phenotype-terrain` (sibling Phenotype mod packages)
