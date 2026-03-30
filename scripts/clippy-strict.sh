#!/usr/bin/env bash
set -euo pipefail

# Run Clippy in strict mode for all targets/features in this crate/workspace.
# Extra cargo args can be passed after "--" to filter targets/packages if needed.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

cargo clippy \
  --workspace \
  --all-targets \
  --all-features \
  "$@" \
  -- \
  -D warnings \
  -D clippy::all \
  -D clippy::pedantic \
  -D clippy::nursery
