# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `PostStack` BRP post-processing component with deterministic SSAO -> SSGI -> Bloom -> ACES tonemap -> LUT color grading chain via `OnRenderImage`.
- `ShaderVariantCollection` (`Runtime/phenotype-postfx-variants.shadervariants`) to prevent build-time shader stripping; reference it from AssetBundles or `PreloadedAssets` to keep all post-FX shaders alive.
- Runtime shader-variant validation on `PostStack` initialization that fails fast with a clear error if a required shader is stripped.
- Vignette post-FX effect with shader variant validation and NUnit tests.
- Chromatic aberration post-FX effect with per-channel UV offset fringe.
- `IPostFxPass` interface for composable, per-effect post-FX passes with `Render(PostFxContext)` contract.
- `PostFxPassRegistry` with Composio-style provider decoupling -- passes register themselves via `IPostFxPassProvider` and the registry resolves them by type.
- `PostFxPassProviderAdapter` bridge for hexagonal architecture integration between the registry and the Unity-specific runtime.
- `BloomPass` with 4-pass threshold -> blur H -> blur V -> composite pipeline, implementing `IPostFxPass`.
- `SSAOPass` with 8-tap rotated kernel generation, screen-space AO sampling, and unit tests.
- Hexagonal port refactor (T21) -- `IPostFxPass` contract for `BloomPass`, `SSAOPass`, and registry adapters.
- T27 URP 17 + T40 HDR LUT hexagonal port specifications (spec-only skeletons).
- R2 batch specs/skeletons (T23-T39) -- governance, clippy, proptests, benchmarks, OTel, adapters, UF tests, hooks, MDX, sqlx, wasm, and hexagonal port specs.
- BenchmarkDotNet benchmark suite (`tests/Benchmarks/`) for `PostFxPass` performance characterization with `BenchmarkSwitcher` CLI.
- NUnit test projects with UnityEngine stubs (`tests/PostStackSourceTests.csproj`, `tests/PostStackVariantTests/PostStackVariantTests.csproj`) and source compilation.
- Comprehensive C# XML documentation comments for all public runtime types (`PostStack`, `PostFxPassRegistry`, `BloomPass`, `SSAOPass`, `IPostFxPass`, and supporting types).
- Standard org governance files: `LICENSE` (MIT), `CODEOWNERS`, `SECURITY.md`, `CONTRIBUTING.md` (Phenotype-org standard), `FUNDING.yml`.
- `README.md` with usage examples, shader reference table (7 shaders), install instructions, and shader stripping prevention guidance.
- `Taskfile.yml` with SSOT recipes for build, lint, test, and validate.
- Dependabot configuration (`.github/dependabot.yml`) for cargo, nuget, and github-actions ecosystems with PR limit 5.
- GitHub Actions CI workflow (`.github/workflows/ci.yml`) with `dotnet test`, `ubuntu-24.04` runner, 10-minute timeout, SHA-pinned third-party actions, and concurrency control.
- `.github/ISSUE_TEMPLATE` (`bug_report.md`, `feature_request.md`) and pull request template.
- `NuGet.config` for package source configuration.

### Changed

- Extracted shared `PostFxPass` abstraction to deduplicate per-effect boilerplate across `BloomPass`, `SSAOPass`, and future passes.
- Complete hexagonal port refactor of the post-FX stack -- all passes now implement `IPostFxPass` and the registry resolves them via the provider contract.
- Replaced `justfile` with `Taskfile.yml` as the org-standard task runner.
- Replaced `grep -oP` with `python3` regex in the `validate` task for macOS compatibility.
- Standardized CI runner to `ubuntu-24.04` (pinned, not floating `ubuntu-latest`) with `timeout-minutes: 10`.
- Bound `PassRegistry` owner in the constructor to prevent null-owner edge cases.
- Removed hardcoded Windows `WorldBoxManaged` path in `.csproj` in favor of portable `$(WorldBoxManaged)` HintPath.
- Merged Phase 0 docs/readme-hygiene fixes and org-files branch with corrected LICENSE copyright attribution.

### Fixed

- Benchmark CLI argument handling via `BenchmarkSwitcher` instead of hardcoded `BenchmarkRunner`.
- `PostFxPassRegistry` test failures after provider decoupling -- corrected mock expectations and adapter wiring.
- README shader count (7 actual shaders, not 5).
- `validate` task `grep -oP` portability issue on macOS.
- `package.json` validation and shader existence check in `validate` task.

### Security

- SHA-pinned all third-party GitHub Actions to immutable commit hashes.
- Added `permissions: contents: read` to CI workflows.
- Added `concurrency` groups to cancel stale workflow runs.
