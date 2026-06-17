# ADR-002: Fold phenotype-postfx into phenotype-gfx (superset-merge)

Date: 2026-06-16
Status: Proposed

## Context

phenotype-postfx is a C# UPM package (PostStack.cs + 5 HLSL shaders). Per org convention
(superset-merge), we fold rather than drop when consolidating microlibs.

## Decision

Copy all sources into phenotype-gfx under src/postfx/ (C#/HLSL archived at
docs/postfx/archive/). Future target = @phenotype/gfx-postfx npm/TS sidecar package
(separate extraction, Task #2 Step 3).

## Consequences

- phenotype-postfx repo remains intact until Task #6 post-freeze archive review.
- No files deleted from either repo (anti-wipe gate enforced).
- Consumers (Civis, WSM3D) are NOT touched.
