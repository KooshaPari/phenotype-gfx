---
title: "Threat Model"
version: 0.1.0
lastUpdated: 2026-06-16
---

# Threat Model

> **Source of truth:** phenotype-postfx (Reusable BRP post-processing stack for Unity: SSAO, SSGI, Bloom, ACES, LUT)
> **Scope:** Unity BRP post-processing scripts, shader source, build pipeline, distribution

## Assets

1. **Post-processing C# scripts (`Assets/Scripts/PostFX/`, etc.)** — Unity scripts that drive the post-processing stack. If mutable, an attacker can ship a script that exfiltrates player data or game state.
2. **Shader source (`*.shader`)** — HLSL/ShaderLab source for SSAO, SSGI, Bloom. If mutable, can ship a shader that runs arbitrary code on the GPU (with side-channel risks) or reveals framebuffer content.
3. **LUT assets (`*.cube`)** — Color-grading LUTs. If mutable, can ship a LUT that biases the visual output (e.g., to obscure in-game UI elements).
4. **Build pipeline** — Unity build process. If mutable, can inject backdoors into the built game binary.
5. **Distribution package (`PhenotypePostFX.unitypackage`)** — Distributed to Unity asset consumers. If mutable in transit, can drop malicious content into consumer projects.

## Threats (STRIDE)

| Category | Threat | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| **Spoofing** | An adversary publishes a `phenotype-postfx` package under a similar name (e.g., `PhenotypePostFX` vs `phenotype-postfx`) and a Unity developer imports the wrong package. | Low | Critical | Releases are signed (cosign, keyless). The README documents the canonical Unity Asset Store URL and the canonical import path. |
| **Tampering** | A C# script is modified in a release to exfiltrate player data via a hidden HTTP request. | Low | Critical | The `Assets/Scripts/` directory is reviewed in PRs. CI runs a `codeql` Unity scan. The package is signed; Unity's `AssetDatabase` validates the signature on import. |
| **Repudiation** | A contributor pushes a script change and later denies it. | Low | Medium | All commits are signed (gitsign, keyless). Releases are tagged. The git history is the audit trail. |
| **Information Disclosure** | A shader includes a hidden render pass that captures the framebuffer and sends it to a remote server. | Low | High | All `.shader` files are reviewed in PRs. The Unity build process strips custom render passes that don't match a known allowlist. |
| **Denial of Service** | A maliciously-large LUT or shader file (1GB) causes Unity to OOM at import time. | Medium | Low | The Unity Asset Postprocessor enforces `max-asset-size=100MB`. Assets over the limit are rejected at import. |
| **Elevation of Privilege** | A C# script uses reflection to access private Unity APIs or modify other assets at runtime. | Low | High | The CI runs `codeql` with a Unity-specific ruleset. Reflection on Unity private APIs is flagged. The package documents the public API surface; deviations are reviewed. |

## Residual Risk and Revision Cadence

The most material residual risk is **shader compromise** — a malicious shader can run arbitrary GPU code (with side-channel risks) or render hidden content. The strongest available mitigation is the codeql Unity scan + PR review, but these do not catch a deliberately obfuscated shader. The next highest residual is **distribution package tampering** — if a Unity Asset Store mirror is compromised, every consumer of the package is affected. This threat model should be revised quarterly (February, May, August, November) or whenever a new post-processing effect is added, a new shader is integrated, or the Unity version target changes. The revision trigger is any PR that adds a new shader, a new C# script, or a new render pass.
