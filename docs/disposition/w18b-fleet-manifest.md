# W18b — phenotype-gfx fleet manifest closeout

**Date:** 2026-06-19  
**Wave:** W18b-G (pheno fleet) + L5-114 (gfx sister-repo supersession)  
**Owner:** phenotype-gfx  
**Status:** **COMPLETE**

## Summary

- **W18b pheno gate:** manifest scan on `main` — no `KooshaPari/pheno` or `phenoShared` git pins in `Cargo.toml` / `go.mod`.
- **Gfx sister-repo retirement:** `VERSION.toml` and Unity water CI no longer reference archived `phenotype-terrain` / `phenotype-voxel` repos.
- **Fleet compat:** `crates/phenotype-voxel` shim re-exports `phenotype_gfx::voxel` for Civis and other consumers.

## Applied changes

| Area | Before | After |
|------|--------|-------|
| `VERSION.toml` module upstream comments | `KooshaPari/phenotype-voxel` etc. | in-repo `src/voxel`, `unity/terrain`, `unity/water` |
| `unity/water` CI | checkout `KooshaPari/phenotype-terrain` | in-monorepo `../terrain` project reference |
| `crates/phenotype-voxel` | (missing) | compat shim for archived crate name |

## Verification

```bash
cargo check -p phenotype-voxel
cargo test -p phenotype-gfx
```

## Related

- [phenotype-gfx#10](https://github.com/KooshaPari/phenotype-gfx/pull/10) — sister-repo absorption
- [phenotype-registry chokepoints](../../phenotype-registry/registry/chokepoints.json) — `phenotype-gfx` W18b row
