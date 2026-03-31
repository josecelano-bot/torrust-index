#!/usr/bin/env bash
set -euo pipefail

# Run all verification checks: format, Clippy, tests, spell-check.
# Run this after every non-trivial change and always before committing.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

echo "==> Formatting …"
cargo fmt --all -- --check

echo "==> Clippy (strict) …"
"${SCRIPT_DIR}/clippy-strict.sh"

echo "==> Tests …"
cargo test --all-features

echo "==> Spell-check …"
"${SCRIPT_DIR}/cspell-check.sh"

echo ""
echo "All checks passed."
