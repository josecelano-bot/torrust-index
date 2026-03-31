#!/usr/bin/env bash
set -euo pipefail

# Install git hooks for this repository.
# Run this once after cloning.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
HOOKS_DIR="${REPO_ROOT}/.git/hooks"

install_hook() {
    local name="$1"
    local target="${HOOKS_DIR}/${name}"

    if [[ -e "${target}" && ! -L "${target}" ]]; then
        echo "warning: ${name} hook already exists and is not a symlink — skipping (back it up manually if needed)"
        return
    fi

    ln -sf "${REPO_ROOT}/scripts/${name}" "${target}"
    chmod +x "${target}"
    echo "  installed: .git/hooks/${name} -> scripts/${name}"
}

echo "==> Installing git hooks …"
install_hook "pre-commit"
echo "Done."
