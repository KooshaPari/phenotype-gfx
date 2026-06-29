# Contributing to phenotype-gfx

Thank you for your interest in contributing to the Phenotype GFX SDK!

## Getting Started

1. **Clone the repo:**
   ```bash
   git clone https://github.com/KooshaPari/phenotype-gfx.git
   cd phenotype-gfx
   ```

2. **Build:**
   ```bash
   cargo check --all-targets
   ```

3. **Run tests:**
   ```bash
   cargo test --all-targets
   ```

4. **Format & lint:**
   ```bash
   cargo fmt --check --all
   cargo clippy --all-targets
   ```

## Code Style

- **Formatting:** Use `cargo fmt` (rustfmt) before committing.
- **Linting:** Ensure `cargo clippy` produces no warnings.
- **Rust edition:** 2021.
- **Naming:** `snake_case` for functions/variables, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- **Error handling:** Use `thiserror` for structured error enums with `Display` and `Error` impls.
- **Comments:** Document public items with doc comments (`///`). Use `//!` for module-level docs.

## Pull Request Process

1. **Branch naming:** `<type>/<topic>` in kebab-case, e.g. `fix/memory-leak`, `feat/voxel-lod`.
2. **Conventional commits:** Use conventional commit messages (e.g. `feat:`, `fix:`, `docs:`, `chore:`).
3. **Regression guards:** Add tests for new functionality or bugfixes. Use the existing regression guard pattern in `tests/perf_regression_guards.rs` for behavioral invariants.
4. **CI passes:** Ensure `cargo check --all-targets` and `cargo test --all-targets` pass.
5. **Feature flag:** If your change affects optional features, verify `cargo check --features bevy` still passes.
6. **PR description:** Describe what changed, why, and any downstream impact on `unity/` subpackages.

## Testing

- **Unit tests:** Inline with `#[cfg(test)] mod tests { ... }` inside each module.
- **Integration tests:** In `tests/` for cross-module behavior and regression guards.
- **Property-based tests:** Use `proptest` (dev-dependency) for randomized inputs.
- **Benchmarks:** In `benches/` using Criterion. Run with `cargo bench`.

## Security

If you discover a security vulnerability, please follow the disclosure policy in the relevant Unity subpackage (`unity/*/SECURITY.md`) rather than opening a public issue.

## License

This project is licensed under MIT OR Apache-2.0 (dual-licensed). By contributing, you agree that your contributions will be licensed under these terms.
