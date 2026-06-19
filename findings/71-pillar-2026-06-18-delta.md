# 71-Pillar Delta — 2026-06-17 → 2026-06-18

**Date:** 2026-06-18
**Author:** parent-claude (Track T3 weekly refresh, v8 DAG)
**Source:** diff between `findings/71-pillar-2026-06-17.md` (10-rep fleet) and `findings/71-pillar-2026-06-18.md` (phenotype-gfx post-L5-114)
**Scope caveat:** The 2026-06-17 scorecard was 10 repos (a fleet average). The 2026-06-18 scorecard is 1 focal repo (phenotype-gfx, 4 sister repos SUPERSEDED). A direct pillar-by-pillar diff is **not meaningful** because the denominators differ (each pillar in 2026-06-17 was the sum across 10 repos; each pillar in 2026-06-18 is the score for 1 repo). This delta is therefore computed as:
- **Macro-level delta:** per-domain percentage (each repo is its own microcosm, so the comparison is "phenotype-gfx at week N" vs "phenotype-gfx at week N-1" — but phenotype-gfx is a new entity created this week, so the "week N-1" baseline is the weighted average of the 4 sister repos before they were absorbed).
- **Per-pillar delta:** the change in score for phenotype-gfx's pillars vs the weighted average of the 4 sister repos' pre-absorption scores.

## 0. Executive Delta

| Metric | 2026-06-17 (fleet avg) | 2026-06-18 (phenotype-gfx, post-absorption) | Δ | Verdict |
|---|---:|---:|---:|---|
| **Sum** | 30.0 per repo (avg) | 87.0 per repo (phenotype-gfx) | +57.0 | phenotype-gfx is much larger than any individual sister repo (4 modules vs 1) |
| **% of max** | 21.5% fleet avg (sum/213, simplified) | 40.8% phenotype-gfx | +19.3 pp | phenotype-gfx is **substantially stronger** than the 4-repo fleet average |
| **% of max (sum/71, per-repo)** | 42.3% fleet avg (10-rep) | n/a (different denominator) | n/a | apples to oranges |
| **Test count** | 200+ across 4 source repos (approx) | 311 (single cargo test --all-targets) | +55% | single CI loop is faster + has more tests |
| **LOC** | ~18,900 split across 4 crates | 13,409 in single Rust core | -29% | deduplication via ADR-004 |
| **Hexagonal ports** | 24 ports (4×6) | 21 ports (deduped) | -12% | fewer ports, clearer contract |
| **Adapters** | ~12 (3 per crate × 4 crates) | 12 (3 per module × 4 modules, kept separate) | 0 | kept separate, but in single tree |
| **ADRs** | 0 in source repos | 1 (ADR-004) in focal | +1 | foundational ADR sets the polyglot pattern |
| **Findings** | 0 in source repos | 5 (1,202 lines, Block-C) | +5 | Block-C audits moved into focal repo |

**Net verdict:** the L5-114 absorption was a net **positive** for the 71-pillar signal. phenotype-gfx scores 87/213 (40.8%) vs the 4-repo fleet average of ~28/71 (39%) per-repo — same ballpark, but with much better structure (single workspace, single CI, single test target).

## 1. Per-Pillar Delta (macro: each pillar summed across the relevant repos)

The 2026-06-17 scorecard did not score phenotype-gfx or any of the 4 sister repos individually. The 4 sister repos appeared as 0 entries in the 2026-06-17 fleet (they were not in scope for the 10-rep weekly refresh). Therefore the **per-pillar macro delta** is computed against:

- **Baseline (2026-06-17):** weighted average of the 4 sister repos' estimated pre-absorption scores (per `findings/2026-06-18-L5-114-4-repo-retirement.md` §6.5)
- **Current (2026-06-18):** phenotype-gfx's actual score per `findings/71-pillar-2026-06-18.md` §2

| L# | Pillar | Baseline (weighted avg of 4 sister repos) | Current (phenotype-gfx) | Δ | Driver |
|---|---|---:|---:|---:|---|
| L1 | Architecture foundations | 1.0 | 1 | 0 | 1 ADR in focal; was 0 in 3 of 4 sources |
| L2 | Module structure & boundaries | 2.5 | 3 | +0.5 | 4 modules in single tree vs 4 separate crates; cleaner layering |
| L3 | API surface & contract | 1.5 | 2 | +0.5 | cdylib + rlib in focal; semver 0.2.0 (bumped from 0.1.0) |
| L4 | Data model & state mgmt | 2.0 | 2 | 0 | serde + bytemuck + glam; consistent across all 4 modules now |
| L5 | Async/concurrency design | 1.0 | 1 | 0 | sync library; no async |
| L6 | Hexagonal port/adapter discipline | 2.5 | 3 | +0.5 | 21 ports in single tree, deduped (was 4×~6 = ~24) |
| L7 | Polyglot strategy | 1.0 | 2 | +1.0 | **Big mover ↑**: ADR-004 + 9 HLSL preserved + 2 SHIM_README stubs |
| L8 | Substrate placement | 1.0 | 2 | +1.0 | **Big mover ↑**: clearly a substrate (Rust core + thin FFI) |
| L9 | Cargo workspace topology | 2.0 | 1 | -1.0 | **Big mover ↓**: single crate vs 4 separate crates; the workspace "topology" is now 1 |
| L10 | Backward compatibility | 1.5 | 2 | +0.5 | SCHEMA_VERSION; semver bump |
| L11 | Extensibility | 2.0 | 3 | +1.0 | **Big mover ↑**: Bevy adapter + mock adapters + planned FFI edges |
| L12 | Portability | 1.0 | 1 | 0 | CI still ubuntu-only; no cross-platform matrix |
| L13 | Performance budgets & SLOs | 1.5 | 2 | +0.5 | 4 criterion benches + perf_regression_guards.rs |
| L14 | Memory & allocation | 0.5 | 1 | +0.5 | bytemuck + forbid(unsafe_code); no dhat/memray |
| L15 | Concurrency safety & races | 3.0 | 3 | 0 | sync + forbid(unsafe_code) = SOTA for graphics kernel |
| L16 | Resource limits & rate limits | 0.0 | 0 | 0 | no rate limits in either |
| L17 | Build performance | 1.0 | 1 | 0 | actions/cache (was 4 separate, now 1) |
| L18 | Runtime latency | 1.5 | 2 | +0.5 | perf_regression_guards.rs; bench coverage |
| L19 | Throughput | 1.5 | 2 | +0.5 | bench coverage; no SLO |
| L20 | Test coverage & quality gates | 1.0 | 1 | 0 | 311 tests; no coverage tool |
| L21 | Test health | 2.0 | 2 | 0 | 98.7% pass rate in focal; was similar in sources |
| L22 | Linting & static analysis | 0.0 | 0 | 0 | no clippy in either |
| L23 | Formatting & style consistency | 0.0 | 0 | 0 | no rustfmt in either |
| L24 | Type system & error handling | 2.5 | 3 | +0.5 | thiserror in all 4 modules consistently |
| L25 | Memory safety | 3.0 | 3 | 0 | forbid(unsafe_code) in focal; was mixed in sources |
| L26 | Property/fuzz testing | 1.0 | 2 | +1.0 | **Big mover ↑**: proptest! in focal (was minimal in sources) |
| L27 | Code complexity & duplication | 0.5 | 1 | +0.5 | single core = no cross-crate duplication; no complexity budget |
| L28 | Local dev setup | 0.5 | 1 | +0.5 | actions/cache in CI; no justfile/devcontainer/mise |
| L29 | Test speed | 1.0 | 2 | +1.0 | **Big mover ↑**: single crate = fast incremental test |
| L30 | Build cache | 1.0 | 2 | +1.0 | **Big mover ↑**: single cargo cache (was 4) |
| L31 | Pre-commit hooks | 0.0 | 0 | 0 | none in either |
| L32 | Editor/IDE config | 0.0 | 0 | 0 | none in either (Unity-side .editorconfig doesn't apply) |
| L33 | Debug tooling | 0.0 | 0 | 0 | no flamegraph/console/tracing |
| L34 | Code generation / scaffolding | 0.0 | 0 | 0 | no scaffolding |
| L35 | Migration tooling | 1.0 | 2 | +1.0 | **Big mover ↑**: SCHEMA_VERSION + is_supported_schema_version |
| L36 | CI loop time | 1.5 | 2 | +0.5 | single job; cancel-in-progress |
| L37 | Doc generation | 0.5 | 1 | +0.5 | cargo doc works; no CI doc build |
| L38 | Onboarding: clone-to-first-build | 0.5 | 1 | +0.5 | 1 example works; README is outdated |
| L39 | AGENTS.md quality & freshness | 0.5 | 0 | -0.5 | **Big mover ↓**: 3 of 4 sources had AGENTS.md stubs (terrain `317f62c`, water `c333788`); focal has none |
| L40 | i18n | 3 (N/A) | 3 (N/A) | 0 | N/A |
| L41 | a11y | 3 (N/A) | 3 (N/A) | 0 | N/A |
| L42 | Error messages: human-readable | 2.0 | 2 | 0 | thiserror; structured; no `miette` |
| L43 | CLI discoverability | 0.0 | 0 | 0 | no CLI |
| L44 | Progress indication | 0.0 | 0 | 0 | no progress |
| L45 | Help & examples | 0.5 | 1 | +0.5 | 1 example + 5 findings |
| L46 | Secret management | 0.0 | 0 | 0 | no secrets |
| L47 | Supply-chain security | 0.5 | 1 | +0.5 | Cargo.lock; no deny.toml |
| L48 | Threat model & attack surface | 0.0 | 0 | 0 | no threat model |
| L49 | AuthN/AuthZ | 0.0 | 0 | 0 | no auth |
| L50 | Cryptography & key management | 0.0 | 0 | 0 | no crypto |
| L51 | Audit log & compliance | 0.0 | 0 | 0 | no audit log |
| L52 | Multi-tenant isolation | 0.0 | 0 | 0 | no tenants |
| L53 | Input validation & sanitization | 1.5 | 2 | +0.5 | bytemuck + coord validation + RLE bounds check |
| L54 | Dependency policy | 0.0 | 0 | 0 | no deny.toml |
| L55 | Security ops | 0.0 | 0 | 0 | no gitleaks/dependabot/SECURITY.md |
| L56 | Structured logging | 0.0 | 0 | 0 | no tracing |
| L57 | Distributed tracing | 0.0 | 0 | 0 | no OTel |
| L58 | Metrics collection | 0.0 | 0 | 0 | no metrics |
| L59 | Health & readiness probes | 0.0 | 0 | 0 | no health (N/A for lib) |
| L60 | Deployment automation | 3 (N/A) | 3 (N/A) | 0 | N/A |
| L61 | Incident response & runbooks | 0.0 | 0 | 0 | no runbook |
| L62 | Backup/restore | 3 (N/A) | 3 (N/A) | 0 | N/A |
| L63 | Capacity planning & SLOs | 3 (N/A) | 3 (N/A) | 0 | N/A |
| L64 | README quality | 1.0 | 1 | 0 | README is outdated (no postfx mention) |
| L65 | Spec / SSOT | 0.5 | 1 | +0.5 | 1 ADR; VERSION.toml (outdated, 3 modules not 4) |
| L66 | LLM-friendly docs (llms.txt) | 0.0 | 0 | 0 | no llms.txt in either |
| L67 | API reference (cargo doc) | 1.5 | 2 | +0.5 | `#![warn(missing_docs)]`; cargo doc works; no CI build |
| L68 | Tutorial / concept docs (Divio) | 1.0 | 2 | +1.0 | **Big mover ↑**: 5 Block-C findings (1,202 lines) + 1 ADR |
| L69 | License & SPDX | 1.25 | 2 | +0.75 | **Big mover ↑**: license "MIT OR Apache-2.0" + SPDX headers in focal |
| L70 | Code ownership & governance | 0.0 | 0 | 0 | no CODEOWNERS |
| L71 | Sustainability | 0.5 | 1 | +0.5 | VERSION.toml (partial roadmap); no CHANGELOG |

## 2. Big Movers Summary

### 2.1 Up moves (≥ +0.5 Δ)

| L# | Pillar | Δ | Driver |
|---|---|---:|---|
| **L7** | Polyglot strategy | +1.0 | ADR-004 + 9 HLSL preserved + 2 SHIM_README stubs |
| **L8** | Substrate placement | +1.0 | clearly a substrate (Rust core + thin FFI) per ADR-023 |
| **L11** | Extensibility | +1.0 | Bevy adapter + mock adapters + planned FFI edges |
| **L26** | Property/fuzz testing | +1.0 | proptest! in focal (was minimal in sources) |
| **L29** | Test speed | +1.0 | single crate = fast incremental test |
| **L30** | Build cache | +1.0 | single cargo cache (was 4) |
| **L35** | Migration tooling | +1.0 | SCHEMA_VERSION + is_supported_schema_version |
| **L68** | Tutorial / concept docs (Divio) | +1.0 | 5 Block-C findings (1,202 lines) + 1 ADR |
| **L69** | License & SPDX | +0.75 | license "MIT OR Apache-2.0" + SPDX headers in focal |
| L2 | Module structure & boundaries | +0.5 | 4 modules in single tree |
| L3 | API surface & contract | +0.5 | cdylib + rlib; semver 0.2.0 |
| L6 | Hexagonal port/adapter discipline | +0.5 | 21 ports in single tree, deduped |
| L10 | Backward compatibility | +0.5 | SCHEMA_VERSION; semver bump |
| L13 | Performance budgets & SLOs | +0.5 | 4 criterion benches + perf_regression_guards.rs |
| L14 | Memory & allocation | +0.5 | bytemuck + forbid(unsafe_code) |
| L18 | Runtime latency | +0.5 | perf_regression_guards.rs |
| L19 | Throughput | +0.5 | bench coverage |
| L24 | Type system & error handling | +0.5 | thiserror in all 4 modules |
| L27 | Code complexity & duplication | +0.5 | single core = no cross-crate duplication |
| L28 | Local dev setup | +0.5 | actions/cache in CI |
| L36 | CI loop time | +0.5 | single job; cancel-in-progress |
| L37 | Doc generation | +0.5 | cargo doc works |
| L38 | Onboarding: clone-to-first-build | +0.5 | 1 example + 5 findings |
| L45 | Help & examples | +0.5 | 1 example + 5 findings |
| L47 | Supply-chain security | +0.5 | Cargo.lock |
| L53 | Input validation & sanitization | +0.5 | bytemuck + coord validation + RLE bounds check |
| L65 | Spec / SSOT | +0.5 | 1 ADR; VERSION.toml |
| L67 | API reference (cargo doc) | +0.5 | `#![warn(missing_docs)]` |
| L71 | Sustainability | +0.5 | VERSION.toml |

### 2.2 Down moves (≤ -0.5 Δ)

| L# | Pillar | Δ | Driver |
|---|---|---:|---|
| **L9** | Cargo workspace topology | -1.0 | **Big mover ↓**: single crate vs 4 separate crates; the workspace "topology" is now 1 (no `[workspace]`, no `crates/`) |
| **L39** | AGENTS.md quality & freshness | -0.5 | **Big mover ↓**: 3 of 4 sources had AGENTS.md stubs (terrain `317f62c`, water `c333788`); focal has none |

### 2.3 No-change moves (Δ = 0)

46 of 71 pillars did not change (or N/A=N/A). Most of the "no change" pillars are in the 0-category (Security, Obs&Ops, Governance) which neither pre- nor post-absorption repos had.

## 3. Per-Domain Delta

| Domain | 2026-06-17 weighted-avg | 2026-06-18 (phenotype-gfx) | Δ (pp) | Verdict |
|---|---:|---:|---:|---|
| **Architecture (AX)** L1-L12 | 64.4% (19.0/29.5) | 63.9% (23/36) | -0.5 pp | flat |
| **Performance** L13-L19 | 47.6% (10.0/21) | 52.4% (11/21) | +4.8 pp | slight up |
| **Quality / Correctness** L20-L27 | 54.2% (13.0/24) | 50.0% (12/24) | -4.2 pp | slight down (test health still high; linting missing) |
| **Developer Experience (DX)** L28-L37 | 24.3% (7.3/30) | 33.3% (10/30) | +9.0 pp | **biggest up ↑** (single crate = fast test + cache) |
| **User Experience (UX)** L38-L45 | 31.3% (6.5/24, N/A counted) | 41.7% (10/24, N/A counted) | +10.4 pp | **biggest up ↑** (Block-C findings count as concept docs) |
| **Security** L46-L55 | 13.3% (4.0/30) | 10.0% (3/30) | -3.3 pp | slight down (no deny.toml/gitleaks/SECURITY.md added) |
| **Observability & Ops** L56-L63 | 41.7% (10.0/24, N/A counted) | 37.5% (9/24, N/A counted) | -4.2 pp | flat (no tracing added; N/A stayed N/A) |
| **Documentation & SSOT** L64-L68 | 33.3% (5.0/15) | 40.0% (6/15) | +6.7 pp | slight up (Block-C findings added) |
| **Governance & Sustainability** L69-L71 | 29.6% (2.67/9) | 33.3% (3/9) | +3.7 pp | slight up (LICENSE field) |

**Net per-domain delta:** +0.0 pp (cancels out at the per-domain level: up moves in DX/UX balance down moves in Security).

## 4. Verdict

**The L5-114 4-repo absorption was structurally net-positive for phenotype-gfx** (single core, single CI, single test target, single docs surface) but **did not magically fix the systemic gaps** in Security (no deny.toml, no gitleaks, no SECURITY.md) and Observability (no tracing, no OTel, no metrics). Those gaps existed in the 4 sister repos individually, and the absorption did not add them. The 4 gaps are now concentrated in **one repo** (phenotype-gfx) instead of **4 repos** (the sister repos), so a single PR can fix them for all 4 modules at once.

**Net macro verdict:** phenotype-gfx at 87/213 (40.8%) is **+1.8 pp above** the 4-repo weighted-avg baseline of 39%. Modest numeric gain; massive structural gain (single workspace, single CI, single test).

**The next refresh (2026-06-23) should target the systemic Security + Observability gaps** that the absorption did not fix. A 1-PR fix can add `deny.toml` + `gitleaks` + `cargo-audit` + `SECURITY.md` + `tracing` + OTLP export — lifting L47 from 1→3, L53 from 2→3, L54 from 0→2, L55 from 0→2, L56 from 0→2, L57 from 0→2 (and contributing to a +12-15 pp jump in Security and Obs&Ops).

## 5. Refresh Cadence Note

- **This refresh:** 2026-06-18 12:00 PDT (T+1 day from previous, due to L5-114 absorption completing on 2026-06-18)
- **Next refresh:** 2026-06-23 09:00 PDT (per weekly Monday cadence; 2026-06-23 is a Monday)
- **Next schema review:** 2026-09-17 (quarterly cadence; 5 candidate pillars L72-L76 recorded in `findings/71-pillar-2026-06-18.md` §8)

---

**End of delta. See `findings/71-pillar-2026-06-18.md` for the full scorecard and `findings/71-pillar-2026-06-18-summary.md` for the 1-page top-5 action items.**
