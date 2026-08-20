# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Shader system expansion (terrain, lighting)
- Example demos (terrain, water, postfx, full scene)
- CI coverage reporting with cargo-tarpaulin
- CI MSRV validation workflow
- deny.toml for cargo-deny
- CHANGELOG.md
- Streaming module tests (68 tests)
- Observability module tests (17 tests)
- Cross-module integration tests (3 tests)
- CODEOWNERS and Taskfile.yml governance files

### Fixed
- Trunk Check workflow failures
- VERSION.toml alignment
- Deprecated Bevy material annotations
- Broken doc-tests in voxel module
- MaterialPalette::add() error handling
- crossbeam-epoch vulnerability (0.9.18 -> 0.9.20)
- 56 clippy warnings
- cargo fmt inconsistencies

## [0.2.0] - 2026-08-19

### Added
- **CI & Tooling**: Added `deny.toml` for `cargo-deny` CI compliance and `cargo-audit` security scanning to the CI and release pipeline.
- **Testing**: Added comprehensive streaming module tests (previously 0% coverage) and observability module tests.
- **Security**: Added SBOM generation workflow (CycloneDX + SPDX) and integrated `SECURITY.md`.
- **Governance**: Added `CODEOWNERS`, `Taskfile.yml`, `CLAUDE.md`, `DESIGN.md`, and Infisical integration for secret management.
- **Trunk Integration**: Configured `trunk.yaml` for linting and formatting.

### Changed
- **Configuration**: Aligned `VERSION.toml` with `Cargo.toml` (0.2.0) and scoped `CODEOWNERS` with directory-level review paths.
- **Workflow Stability**: Updated `trunk-io/trunk-action` to `v1.3.1` and migrated CI to use stable lint/test gate names.
- **CI Multi-OS Matrix**: Extended the CI pipeline to run across multiple operating systems for broader compatibility.

### Fixed
- **Deep Audit**: Resolved doc-test issues, updated `MaterialPalette` to return `Result`, and removed deprecated Bevy annotations.
- **Trunk Integration**: Migrated `trunk.yaml` to v0.1 format and removed invalid configuration keys to resolve CI failures.
- **Maintenance**: Addressed 56 clippy warnings, crossbeam CVE fix, and formatting regressions.

### Security
- **Vulnerability Remediation**: Patched `crossbeam-epoch` to address a known vulnerability (CVE fix) and implemented automated security auditing via `cargo-audit`.

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
