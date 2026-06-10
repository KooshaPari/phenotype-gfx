# Per-OS Icon Spec — phenotype-voxel

**Status:** PROPOSED (no assets generated yet)
**Date:** 2026-06-04
**Author:** L1 worker (apps registry audit)

## Why

phenotype-voxel is a library crate. Per-OS desktop icons are NOT in scope.
But the repo still ships a docs site + may ship a web viewer — both need a
favicon + social card icon. This spec is minimal: 1 SVG + 1 favicon set.

## Base vector

Hand-authored SVG at `assets/brand/logo.svg` — a voxel-cube silhouette viewed
from a 3/4 isometric angle with a single interior lit cell.

## Variants

| Surface  | File                                  | Size          | Notes |
|----------|---------------------------------------|---------------|-------|
| Docs     | `docs/.vitepress/public/favicon.ico`  | 16/32/48      | Browser tab |
| Docs     | `docs/.vitepress/public/logo.svg`     | vector        | Navbar + OG card |
| Social   | `assets/brand/social-512.png`         | 512x512       | OG/Twitter card |
| Crates   | (none — `cargo` doesn't display a logo) | —          | n/a |

## Material language

phenotype-voxel inherits the Phenotype keycap palette (teal #7ebab5 +
midnight #090a0c, Montserrat type). Voxel = isometric cube silhouette,
NOT a flat glyph.

## AI-DD + renderers

- AI-CODED (hand-authored SVG).
- Raster: resvg → ImageMagick → Pillow fallback.
- Favicon ICO: ImageMagick or Pillow.

## Out of scope

- Any desktop OS icon (lib doesn't ship a binary by default).
- A separate per-OS variant (1 SVG + 1 PNG + 1 ICO covers all surfaces).

## Open questions

- Does phenotype-voxel ever ship a standalone `voxel-viewer` binary that would warrant full per-OS desktop icons? (decides whether to expand the spec later.)
