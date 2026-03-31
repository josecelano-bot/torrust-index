# AGENTS.md

Guidance for AI coding agents working on **torrust-mudlark** — a φ-Bounded Geometric-Value Graph library for adaptive streaming spatial density estimation over 1D coordinate spaces.

---

## Project overview

`torrust-mudlark` is a pure-Rust library (crate name `torrust-mudlark`) that answers:
_"where is activity concentrated right now?"_

It maintains an adaptive histogram over a fixed 1D domain, splitting high-activity sub-ranges at finer resolution guided by the golden ratio φ, while quiet regions stay coarse. Temporal decay supports sliding-window tracking.

Internally it uses a **dual-tree** design:

- **G-tree** — geometric skeleton; recursively partitions the coordinate space.
- **V-tree** — value tree; accumulates observations and drives rebalancing.

Core operations: `observe(coord, delta)`, `decay(root, attenuation, q)`, `sample(rng)`, `extract()`.

Primary public type: `GvGraph<C, V, N>` where `C` = coordinate type, `V` = value type, `N` = address-space depth (domain `[0, 2^N − 1]`).

---

## Rust version and edition

- **Edition**: 2024
- **MSRV**: 1.85

---

## Setup after cloning

Run once after cloning to install the git hooks:

```bash
./scripts/install-hooks.sh
```

This installs a `pre-commit` hook that runs `./scripts/verify.sh` before every commit, blocking commits that fail formatting, Clippy, tests, or spell-check.

---

## Build commands

```bash
# Build (debug)
cargo build

# Build all features
cargo build --all-features

# Build in release mode
cargo build --release
```

---

## Test commands

```bash
# Full test suite (unit + integration + snapshot tests)
cargo test

# All features
cargo test --all-features

# Single test by name pattern
cargo test <test_name_pattern>

# Run a specific example
cargo run --example ip_range_ban_detection

# Benchmarks (compiles in release mode)
cargo bench --bench depth
```

Tests live in:
- `src/` — inline unit tests (`mod tests { ... }`)
- `tests/integration.rs` — integration tests
- `tests/snapshot_tests.rs` — snapshot tests; fixtures stored in `tests/snapshots/`

---

## Linting and formatting

Always pass these before finishing a task:

```bash
# Strict Clippy (mirrors CI)
./scripts/clippy-strict.sh

# Spell-check (requires cspell: npm install --prefix ~/.local cspell)
./scripts/cspell-check.sh

# Standard formatter
cargo fmt --all
```

`clippy-strict.sh` runs:

```
cargo clippy --workspace --all-targets --all-features \
  -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery
```

If a new word causes a spell-check failure, add it to `project-words.txt`.

---

## Code style

- **`unsafe_code` is forbidden** (`#![forbid(unsafe_code)]` in `lib.rs`). Never introduce unsafe.
- **`unused = deny`** — every new symbol must be used, or removed. Helper APIs that are not yet called will fail CI.
- All standard Clippy lint groups (`all`, `pedantic`, `nursery`, `correctness`, `complexity`, `perf`, `style`, `suspicious`) are denied.
- `pub(crate)` is preferred over `pub` for internal modules; use `pub` only when exposing items in the public API.
- When extracting a method into a submodule that is called by a sibling submodule, visibility must be `pub(super)`, not `pub`.
- Do not add doc-comments, type annotations, or error handling beyond what is required by the task.
- Feature flags: `dynamic-contour-tracking` and `rand` are on by default; `serde`, `arena-unchecked`, and `arena-validate` are opt-in.

---

## Module layout

```
src/
  arena.rs          — memory arena for efficient allocations
  handle.rs         — handle types (GNodeId) for identifying nodes
  lib.rs            — crate root; public API re-exports
  diagnostics/      — invariant checking and diagnostic tools (pub)
  graph/            — GvGraph, GNodeChildren, Config, StructuralConfig
  nodes/            — G-node and V-node implementations
  spatial/          — coordinate space, contour ranges, plateau, Pewei
  traits/           — core trait definitions (Coordinate, Observation, …)
  tree/             — G-tree and V-tree data structures
```

All modules except `diagnostics` are `pub(crate)`. Only re-exported items in `lib.rs` are part of the public API.

---

## Testing instructions

- Run `cargo test` before every commit. All tests must pass.
- Run `./scripts/clippy-strict.sh` before every commit. Zero warnings allowed.
- Snapshot tests regenerate fixtures automatically; if a snapshot changes intentionally, review and commit the updated file in `tests/snapshots/`.
- When adding a new invariant or diagnostic, add coverage tests alongside it (see `src/diagnostics/` for the pattern).
- Do not disable or `#[ignore]` tests without an explicit comment explaining why.

---

## Verification cadence

Run the all-in-one verification script after every non-trivial change and **always before committing**:

```bash
./scripts/verify.sh
```

This single command runs, in order:

1. `cargo fmt --all -- --check` — formatting
2. `./scripts/clippy-strict.sh` — strict Clippy
3. `cargo test --all-features` — full test suite
4. `./scripts/cspell-check.sh` — spell-check

Fix failures as they appear rather than letting them pile up. If only one step needs re-running, call the individual script directly (see *Linting and formatting* above).

---

## Commit signing

All commits **must be GPG-signed**. The repository's git configuration already specifies which key to use; signing is automatic when the key passphrase is loaded by the agent.

If `git commit` fails with a GPG error (passphrase not cached), ask the user to unlock the key:

```
Please run: gpx-connect  (or enter your GPG passphrase when prompted)
```

Do **not** add `--no-gpg-sign` or otherwise bypass signing.

---

## Commit conventions

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<optional scope>): <short summary>

<optional body — bullet points per sub-change>
```

Common types: `feat`, `fix`, `refactor`, `chore`, `docs`, `test`, `perf`.

Example (from this repo):
```
chore: fix clippy-strict and cspell errors

clippy-strict:
- redundant_pub_crate: pub(crate) -> pub in gtree/mod.rs, vtree/mod.rs
- missing_const_for_fn: add const to GNodeTree::count() and VNodeTree::new()

cspell:
- project-words.txt: add uninitialised, gchild, gvgraph
```

---

## Documentation

- [docs/architecture.md](docs/architecture.md) — component and call-flow diagrams
- [docs/design-model.md](docs/design-model.md) — data-model rationale
- [docs/complexity-analysis.md](docs/complexity-analysis.md) — complexity analysis and metrics
- [docs/snapshot-tests.md](docs/snapshot-tests.md) — snapshot test strategy
- [docs/use-cases/](docs/use-cases/) — design rationale per example
- [docs/refactoring-plans/](docs/refactoring-plans/) — ongoing refactoring plans

---

## Feature flags reference

| Flag                       | Default | Description                                             |
| -------------------------- | ------- | ------------------------------------------------------- |
| `dynamic-contour-tracking` | ✓       | Maintain contour ranges for efficient range queries     |
| `rand`                     | ✓       | Implement `WeightedSampler` via `rand_core::RngCore`    |
| `serde`                    | ✗       | `Serialize`/`Deserialize` for `GvGraph`, `Pewei`, etc.  |
| `arena-unchecked`          | ✗       | Skip safety assertions in hot arena allocation code     |
| `arena-validate`           | ✗       | Enable invariant validation in checked builds and tests |

---

## Security

- `unsafe_code` is forbidden unconditionally.
- Do not introduce new dependencies without discussion.
- License: AGPL-3.0-only.
