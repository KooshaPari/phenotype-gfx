<!-- AI-DD-META:START -->
<!-- This repository is planned, maintained, and managed by AI Agents only. -->
<!-- Slop issues are expected and intentionally present as part of an HITL-less -->
<!-- /minimized AI-DD metaproject of learning, refining, and building brute-force -->
<!-- training for both agents and the human operator. -->
![Downloads](https://img.shields.io/github/downloads/KooshaPari/phenotype-voxel/total?style=flat-square&label=downloads&color=blue)
![GitHub release](https://img.shields.io/github/v/release/KooshaPari/phenotype-voxel?style=flat-square&label=release)
![License](https://img.shields.io/github/license/KooshaPari/phenotype-voxel?style=flat-square)
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
## Work State

| Field | Value |
|---|---|
| Last commit | 2026-06-08 |
| Open issues | 0 |
| Open PRs | 3 |
| Focus | Adaptive SVO + dense 16³ leaf voxel substrate for Phenotype-org games |

Progress: ████████░░ 80%

# phenotype-voxel

## State

Progress: `[██░░░░░░░░] 20%` — adaptive voxel substrate (pre-MVP scaffold).

_Updated 2026-06-08 — audit pass._

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

**Adaptive voxel substrate for Phenotype-org games.**

Sparse voxel octree (SVO) for coarse / far-from-camera space + dense 16³ leaf chunks
for near-camera detail. Every write produces a deterministic `DirtyChunkEvent` so
consumers (Civis, WorldSphereMod3D, …) can rebuild meshes in a replay-safe order.

## Status

Pre-MVP scaffold. Public types are stable enough to wire against; the real storage
+ meshing implementations land in follow-up PRs (P-V1 in Civis).

## Design references

- Civis 3D PRD addendum: `Civis/docs/roadmap/civis-3d-extension.md`
- Adaptive voxel ADR: `Civis/docs/adr/ADR-005-adaptive-voxel.md`
- Voxel-kernel design notes: `~/.claude/plans/civis-3d-scratch/phenotype-voxel-design.md`

## Determinism contract

- World coordinates are **fixed-point `i64` at `10^6` scale**. No `f32`/`f64` crosses
  the public API.
- Dirty events are ordered by `(chunk_id, write_seq)`. Internal collections never
  leak iteration ordering through the public surface.
- `VoxelScaleMultiplier` is a first-class semantic with default `8.0` (WSM3D-lineage
  visible-default). LOD selection composes with it through `lod::select_lod` so
  consumers cannot accidentally desynchronise.

## Consumers

- **Civis** (Rust): native path / git dependency.
- **WorldSphereMod3D** (C# / Unity): consumes via a C ABI generated through
  `ffi-core` / `cbindgen` (lands in a later PR).

## Modules

| Module | Purpose |
|---|---|
| `chunk` | Dense 16³ leaf chunks + `ChunkId` + borrowed `ChunkView`. |
| `coord` | Fixed-point world coords + chunk-grid coord conversion. |
| `delta` | Deterministic dirty-event queue (`WriteSeq`, `DirtyChunkEvent`). |
| `lod` | `LodLevel`, `LodPolicy`, `select_lod`, `VoxelScaleMultiplier`. |
| `material` | Engine-neutral material palette. |
| `mesh` | Mesh-neutral vertex/index buffers + `Mesher` trait. |
| `octree` | Sparse voxel octree skeleton. |

## License

Dual-licensed at the consumer's option:

- MIT — `LICENSE-MIT`
- Apache 2.0 — `LICENSE-APACHE`

## Description

Adaptive voxel substrate for Phenotype-org games — sparse voxel octree (SVO) for coarse / far space plus dense 16³ leaf chunks for near space. Deterministic `DirtyChunkEvent` ordering keeps consumers (Civis, WorldSphereMod3D, …) replay-safe.

## Install

Reference from another Rust crate: `phenotype-voxel = { path = "../phenotype-voxel" }`. Unity / C# consumers wire via the C ABI generated through `ffi-core` / `cbindgen` (lands in a follow-up PR).

## Usage

World coordinates are fixed-point `i64` at `10^6` scale — `coord::WorldFixed`. Writes go through `chunk::ChunkStore` and emit ordered `delta::DirtyChunkEvent`s. LOD composes with `VoxelScaleMultiplier` via `lod::select_lod`.

## Contributing

PRs welcome. See `CONTRIBUTING.md`. All public types must keep the determinism contract: no `f32`/`f64` crosses the API, dirty events ordered by `(chunk_id, write_seq)`, and no leaking of internal collection ordering.
