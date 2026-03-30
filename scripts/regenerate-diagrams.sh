#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DOCS_DIR="${REPO_ROOT}/docs"

if ! command -v plantuml &>/dev/null; then
    echo "ERROR: 'plantuml' is not installed or not in PATH." >&2
    echo "       Install it first, then re-run this script." >&2
    exit 1
fi

# Render .puml files to SVG
plantuml -Djava.awt.headless=true -tsvg "$DOCS_DIR"/*.puml

# Post-process: responsive sizing, correct aspect ratio, white background
for f in "$DOCS_DIR"/*.svg; do
    sed -i \
        -e 's/ width="[0-9]*px"//g' \
        -e 's/ height="[0-9]*px"//g' \
        -e 's/style="width:[0-9]*px;height:[0-9]*px;"/style="max-width:100%;height:auto;background:#ffffff;"/' \
        -e 's/preserveAspectRatio="none"/preserveAspectRatio="xMidYMid meet"/' \
        "$f"
    echo "Generated: $f"
done
