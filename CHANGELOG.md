# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-08-19

### Added
- **SBOM Generation**: Integrated CycloneDX and SPDX SBOM generation into the CI workflow.
- **CI Multi-OS Matrix**: Extended the CI pipeline to run across multiple operating systems for broader compatibility.
- **Governance**: Added comprehensive governance files including `CLAUDE.md`, `CODEOWNERS`, `SECURITY.md`, `DESIGN.md`, and a `Taskfile.yml` for AgilePlus governance.
- **Security**: Added dual-license support and integrated Infisical for secret management.
- **Tooling**: Added `pre-commit` hooks and `renovate.json` for automated dependency management.
- **Trunk Integration**: Configured `trunk.yaml` for linting and formatting.

### Changed
- **CI Modernization**: Updated `trunk-io/trunk-action` to `v1.3.1` to resolve workflow failures.
- **Workflow Stability**: Updated CI workflows to use stable lint/test gate names for better consistency.

### Fixed
- **Deep Audit**: Resolved doc-test issues, updated `MaterialPalette` to return `Result`, and removed deprecated Bevy annotations.
- **Performance Regression Guards**: Addressed potential performance regressions through clippy cleanup and targeted fixes.
- **Trunk Configuration**: Migrated `trunk.yaml` to v0.1 format and removed invalid configuration keys.
- **Trunk Formatting**: Repaired `trunk check` formatting failures to ensure CI stability.

### Security
- **Crossbeam CVE**: Patched `crossbeam-epoch` to address a known vulnerability (CVE fix).

## [0.1.0] - 2026-06-18

### Added
- **Unified Graphics Kernel**: Initial release of the core graphics kernel.
- **Voxel System**: Implemented core voxel data structures and meshing.
- **LOD & Streaming**: Added Level of Detail (LOD) and streaming support for large-scale worlds.
- **Water System**: Implemented water simulation and rendering.
- **Terrain Engine**: Added terrain generation and rendering capabilities.
- **Postfx**: Implemented a basic post-processing effects pipeline.
- **Bevy Adapter**: Added optional Bevy adapter for ECS integration.
- **Observability**: Integrated structured tracing and a metrics facade.
- **Performance Benchmarks**: Added initial Criterion benchmarks for voxelizer and meshing operations.
