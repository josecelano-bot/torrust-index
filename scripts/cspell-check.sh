#!/usr/bin/env bash
set -euo pipefail

# Run cspell spell-checking across the project.
# Requires cspell to be installed (e.g. npm install --prefix ~/.local cspell).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

CSPELL="${CSPELL:-cspell}"

if ! command -v "${CSPELL}" &>/dev/null; then
  # Fall back to the user-local npm installation
  LOCAL_CSPELL="${HOME}/.local/node_modules/.bin/cspell"
  if [[ -x "${LOCAL_CSPELL}" ]]; then
    CSPELL="${LOCAL_CSPELL}"
  else
    echo "error: cspell not found. Install it with: npm install --prefix ~/.local cspell" >&2
    exit 1
  fi
fi

cd "${REPO_ROOT}"

exec "${CSPELL}" --no-progress "$@"
