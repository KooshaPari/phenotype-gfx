# CLAUDE.md — phenotype-gfx

Extends parent governance. See:
- Global baseline: `~/.claude/CLAUDE.md`
- AgilePlus canonical: `https://github.com/KooshaPari/AgilePlus`

## Project Overview

- **Name:** phenotype-gfx
- **Description:** Polyglot graphics SDK — Rust core + Unity bindings for voxel terrain, water, post-FX
- **Language Stack:** Rust (edition 2021), C# (Unity), GLSL
- **Key Areas:** `src/`, `crates/`, `bindings/`, `unity/`, `benches/`, `examples/`, `spec/`, `docs/`
- **Status:** Active

## Repository Layout

- `src/` — Rust core library (voxel rendering, terrain, water)
- `crates/` — Sub-crates (if any)
- `bindings/` — Language bindings (C#, FFI)
- `unity/` — Unity package integration
- `benches/` — Rust benchmarks (criterion)
- `examples/` — Usage examples
- `spec/` — Specifications
- `docs/` — Documentation and ADRs
- `.github/workflows/` — CI/CD

## Quality Checks

From this repository root:

```bash
# Rust checks
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt -- --check

# Benchmarks
cargo bench

# Pre-commit
pre-commit run --all-files

# Trunk
trunk check
```

## Worktree & Git Discipline

- Feature work uses feature branches off `main`
- Canonical repo stays on `main` except during explicit merge operations
- All feature branches are temporary; integrate via pull request or squash commit

## Related Documents

- `README.md` — project overview
- `AGENTS.md` — agent-facing guidance
- `CONTRIBUTING.md` — contribution guidelines
- `spec/` — specifications

---

For CI, scripting language hierarchy, and other policies, see the canonical sources listed above.
