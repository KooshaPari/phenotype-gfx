# phenotype-voxel Justfile
#
# After 2026-06-11, this justfile is a thin shell that re-exports the shared
# `phenotype.just` library (defined in just/phenotype.just). The 9 most
# common recipes (default, build, test, lint, fmt, audit, unused, ci, docs)
# are now defined once in the library and parameterized over the build
# system.
#
# Stack-specific recipes (e.g. `clean`, `dev`) stay in this file.
#
# To upgrade: pull the latest phenotype.just from the central repo, or
# vendor it as a git submodule.

import "just/phenotype.just"

# Stack-specific: clean build artifacts.
clean:
    cargo clean

# Watch mode: rebuild on change.
dev:
    cargo watch -x build -x test || echo "cargo-watch not installed; run: cargo install cargo-watch"

