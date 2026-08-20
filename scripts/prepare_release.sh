#!/bin/bash
set -euo pipefail

echo "=== phenotype-gfx Release Preparation ==="
echo ""

# 1. Verify clean state
echo "1. Checking git status..."
if [ -n "$(git status --porcelain)" ]; then
    echo "ERROR: Working directory not clean"
    exit 1
fi
echo "   OK"

# 2. Verify version alignment
echo "2. Verifying version alignment..."
CARGO_VER=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
TOML_VER=$(grep '^version' VERSION.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
if [ "$CARGO_VER" != "$TOML_VER" ]; then
    echo "ERROR: Version mismatch (Cargo.toml=$CARGO_VER, VERSION.toml=$TOML_VER)"
    exit 1
fi
echo "   Version: $CARGO_VER"

# 3. Run checks
echo "3. Running cargo check..."
cargo check --workspace
echo "   OK"

echo "4. Running cargo test..."
cargo test --workspace
echo "   OK"

echo "5. Running cargo clippy..."
cargo clippy --workspace -- -D warnings 2>/dev/null || echo "   Warnings found (non-blocking)"

echo "6. Running cargo fmt check..."
cargo fmt --check 2>/dev/null || echo "   Formatting issues found"

echo ""
echo "=== Release preparation complete ==="
echo "To create a release:"
echo "  1. Update CHANGELOG.md with version number"
echo "  2. git tag v$CARGO_VER"
echo "  3. git push origin main --tags"
echo "  4. GitHub Actions will create the release"
