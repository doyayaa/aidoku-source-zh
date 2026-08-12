#!/bin/bash
# Build every source crate under sources/ and pack each into its own package.aix.
# Usage: ./build.sh  (run from the repo root)
set -e

for src in sources/*/; do
    name=$(basename "$src")
    echo "=== Building $name ($src) ==="
    (cd "$src" && cargo +nightly build --release)
    python pack.py "$src"
done

echo "Done. Outputs:"
for src in sources/*/package.aix; do
    echo "  $src"
done
