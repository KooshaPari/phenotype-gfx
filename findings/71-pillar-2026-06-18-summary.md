# 71-Pillar Audit Refresh — 1-Page Summary (2026-06-18)

**Date:** 2026-06-18
**Author:** parent-claude (Track T3 weekly refresh, v8 DAG)
**Focal repo:** `KooshaPari/phenotype-gfx` (post-L5-114 absorption)
**Source repos:** 4 SUPERSEDED (phenotype-voxel, phenotype-terrain, phenotype-water, phenotype-postfx)

---

## Headline Numbers

| Metric | Value | Notes |
|---|---:|---|
| **71-pillar sum (phenotype-gfx)** | **87 / 213** | 40.8% of max |
| **Per-domain %** | AX 63.9, Perf 52.4, Quality 50.0, DX 33.3, UX 41.7, Sec 10.0, Obs&Ops 37.5, Doc&SSOT 40.0, Gov&Sust 33.3 | see scorecard §3 |
| **vs 2026-06-17 fleet avg** | +19.3 pp | different denominator (10-rep vs 1-rep); on weighted-source-avg basis = +1.8 pp |
| **Tests pass** | 311 / 314 (98.7%) | up from ~200 across 4 source repos |
| **LOC** | 13,409 in single Rust core | down 29% from ~18,900 split (dedup) |
| **Hexagonal ports** | 21 in single tree | down 12% from ~24 (dedup) |
| **Adopted ADRs** | 1 (ADR-004) | foundational |
| **Findings** | 5 (1,202 lines) | Block-C audits from 4 source repos |

---

## Top 5 Strengths (SOTA, 3/3)

1. **L6 — Hexagonal port/adapter discipline** (100%) — 21 traits in `ports/` dirs across 4 modules; `adapters/` for concrete impls; mock adapters for tests. Direct result of L5-114 absorption.
2. **L11 — Extensibility** (100%) — Bevy adapter optional; mock adapters demonstrate the contract; FFI edges planned.
3. **L15 — Concurrency safety & races** (100%) — sync-only = no data races; `#![forbid(unsafe_code)]` is SOTA for memory safety.
4. **L24 — Type system & error handling** (100%) — thiserror in every module; `Clone + PartialEq + Eq` for testability; `Result<T, Error>` chains throughout.
5. **L25 — Memory safety** (100%) — `#![forbid(unsafe_code)]` (better than `deny(unsafe_op_in_unsafe_fn)`); `bytemuck` for safe POD transmutation.

## Top 5 Weaknesses (Absent, 0/3)

1. **L48 — Threat model & attack surface** (0%) — no STRIDE doc. Graphics kernels are a known attack surface (shader injection, memory corruption via FFI); needs an ADR.
2. **L51 — Audit log & compliance** (0%) — no audit layer. For a graphics kernel exposed to FFI, an audit layer would help.
3. **L54 — Dependency policy** (0%) — no `deny.toml`. 13 direct deps with no license/RUSTSEC/bans check.
4. **L55 — Security ops** (0%) — no `gitleaks`, no `cargo-audit`, no `dependabot`, no `SECURITY.md`.
5. **L22 — Linting & static analysis** (0%) — no `clippy` in CI. **Easiest win — 1 line in `.github/workflows/ci.yml`.**
6. **L23 — Formatting & style consistency** (0%) — no `rustfmt` in CI. **Also 1 line.**

## Top 5 Pillars to Improve Next (Highest ROI)

> ROI = (current / max) gap × ease of fix. Each item below can be done in 1-2 hours and lifts 1-3 pillars by 1-2 points.

| # | Action | Pillars lifted | Effort | Pillar-by-pillar impact |
|---|---|---|---|---:|
| **1** | Add `deny.toml` from `phenotype-ops/templates/deny.toml` + add `cargo-deny` workflow | L47, L54, L69 | 30 min | +3 |
| **2** | Add `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` to CI | L22, L23 | 15 min | +2 |
| **3** | Write `AGENTS.md` at repo root (copy template from `pheno-config`) | L39, L64, L66, L8 | 30 min | +4 |
| **4** | Add `gitleaks` + `cargo-audit` + `SECURITY.md` + `security.txt` | L48, L51, L55 | 1 hr | +3 |
| **5** | Add `tracing` + `tracing-subscriber` + `pheno-otel` OTLP exporter per ADR-012 | L33, L56, L57, L58 | 2 hr | +4 |

**Total effort:** ~5 hours of work to lift ~16 pillars by 1-2 points each = **+18-32 points on the 213-point scale = +8-15 pp**.

**This is the recommended v8 DAG Track T13 (post-L5-114 hardening):** the L5-114 absorption gave us a structurally clean phenotype-gfx; the next step is to fill the 5 systemic Security + Observability gaps that the absorption did not address.

---

## Factory AI Agent Readiness (per ADR-026)

**Current Level estimate:** 2 (Documented) → 0.6 (below 80% threshold for Level 2)
**Target:** 3 (Standardized) by 2026-09-17

| Level | Threshold | Status | Why |
|---|---|---|---|
| L1 Functional | 80% | **PASS** | cargo check ✓; 311 tests ✓; consumable as a crate ✓ |
| L2 Documented | 80% | **PARTIAL (~60%)** | cargo doc + 5 findings + 1 ADR; no AGENTS.md; no llms.txt; outdated README |
| L3 Standardized | 80% | **FAIL (~30%)** | no clippy/rustfmt/editorconfig/pre-commit |
| L4 Optimized | 80% | **FAIL (~40%)** | thiserror + forbid(unsafe_code) + criterion + perf_regression_guards; no tracing/OTel/metrics/SLO |
| L5 Autonomous | 80% | **FAIL (0%)** | no self-improving CI |

**Org-level (per ADR-026 §10 rule):** `floor(avg(L1..L5))` = `floor(0.46)` = **Level 0 (Pre-Functional)** for phenotype-gfx.

**Quick wins to reach Level 3 (Standardized):**
- L2 → 3: AGENTS.md (5 min) + llms.txt (10 min) + update README (15 min) + update VERSION.toml (5 min)
- L3 → 3: clippy in CI (1 line) + rustfmt in CI (1 line) + .editorconfig (2 min) + deny.toml (10 min) + gitleaks (10 min) + cargo-audit (10 min) + SECURITY.md (10 min)

**Quick wins to reach Level 4 (Optimized):** add `tracing` + `tracing-subscriber` + `pheno-otel` OTLP exporter per ADR-012; add SLO.md with P50/P95/P99 for hot paths; add regression alerts in CI.

**Both frameworks matter:** 71-pillar = breadth (9 domains × 71 pillars); Factory AI = depth (5 levels × 9 pillars). See `findings/71-pillar-2026-06-18.md` §11 for the full crosswalk.

---

## Future Pillars (Schema-2.0 Review, 2026-09-17)

5 candidate pillars recorded for the quarterly schema review:

| # | Candidate | Why | Evidence |
|---|---|---|---|
| L72 | FFI surface | The single-core + thin FFI edges pattern (ADR-004) is the integration seam. A dedicated pillar would score the FFI layer. | `src/lib.rs:39-41` planned `c_api` + `wasm`; `Cargo.toml:15` `cdylib`; 9 HLSL + 2 SHIM_README in `unity/` |
| L73 | Graphics / GPU correctness | Graphics kernels have correctness concerns distinct from general-purpose Rust (shader compiler bugs, GPU memory alignment, determinism, NaN/Inf). | `coord.rs:18-20` fixed-point world coords; `mod.rs:14-24` determinism contract |
| L74 | Asset pipeline / schema versioning | SCHEMA_VERSION + is_supported_schema_version is a critical pattern for any library that persists to disk. | `voxel/mod.rs:79-94`; water/postfx schemas are NOT versioned (gap) |
| L75 | Reactive / streaming behaviour | `streaming.rs:1-12683` is the largest module in the repo with no pillar coverage. | `streaming.rs` ring-based chunk lifecycle + eviction |
| L76 | Polyglot consumer parity | The 4-repo absorption left consumer-side artifacts (HLSL + SHIM_READMEs). A dedicated pillar would track parity. | `unity/postfx-shaders/` (9 HLSL), `unity/terrain/SHIM_README.md`, `unity/water/SHIM_README.md` |

**Schema-2.0 review trigger:** 2026-09-17 (90 days from 2026-06-17). These 5 candidates should be on the agenda.

---

## Action Items for v8 DAG Track T13 (post-L5-114 hardening)

| # | Action | Owner | Effort | Deadline |
|---|---|---|---|---|
| 1 | Add `deny.toml` + cargo-deny workflow | worklog-schema circle | 30 min | 2026-06-23 |
| 2 | Add `cargo clippy` + `cargo fmt --check` to CI | worklog-schema circle | 15 min | 2026-06-23 |
| 3 | Write `AGENTS.md` at root + update README + update VERSION.toml | worklog-schema circle | 1 hr | 2026-06-23 |
| 4 | Add `gitleaks` + `cargo-audit` + `SECURITY.md` + `security.txt` | worklog-schema circle | 1 hr | 2026-06-25 |
| 5 | Add `tracing` + `tracing-subscriber` + `pheno-otel` OTLP per ADR-012 | pheno-tracing circle | 2 hr | 2026-06-30 |
| 6 | Add `llms.txt` at root per Answer.AI spec | worklog-schema circle | 30 min | 2026-06-25 |
| 7 | Add `SLO.md` with P50/P95/P99 for hot paths | worklog-schema circle | 1 hr | 2026-06-30 |
| 8 | Open L5-115 "post-L5-114 hardening" DAG track in v8 plan | parent-claude | 30 min | 2026-06-19 |

---

## Cross-References

- `findings/71-pillar-2026-06-18.md` — full scorecard
- `findings/71-pillar-2026-06-18-delta.md` — diff vs 2026-06-17
- `findings/71-pillar-2026-06-17-schema.md` — canonical schema
- `findings/71-pillar-2026-06-17.md` — previous scorecard
- `findings/2026-06-18-L5-114-4-repo-retirement.md` — L5-114 audit
- `findings/2026-06-18-sister-repo-block-c-summary.md` — Block-C verdict
- `phenotype-gfx/docs/adr/ADR-004-single-core-ffi-edges.md` — architectural foundation
- `AGENTS.md` § "71-pillar audit" — fleet-wide SSOT
- `AGENTS.md` § "Factory AI Agent Readiness" — external standard (ADR-026)

**Next refresh:** 2026-06-23 09:00 PDT
**Next schema review:** 2026-09-17

---

**End of summary. 1 page. Total: 87/213 = 40.8%. Factory AI Level 2 (Documented) → 3 (Standardized) target. 5 systemic gaps (L22/L23/L48/L51/L54/L55) + 1 missing meta-bundle (AGENTS.md) = 5-hour fix.**
