# Phase 8 — API & Traits Clarity

## Goal

Make the public interface of `GvGraph` and the trait layer clean, consistent,
and easy to implement correctly, by:

- Ensuring every trait method has a precise doc comment stating its contract.
- Reducing the cognitive distance between a trait's promise and its default
  implementations.
- Improving the spatial type layer (`spatial/`) readability without changing
  semantics.
- Ensuring `graph/traits.rs` is readable as the single source-of-truth for the
  public extension points.

## Files targeted

- `src/graph/traits.rs`
- `src/traits/coordinate.rs`
- `src/traits/accumulator.rs`
- `src/traits/observation.rs`
- `src/traits/attenuatable.rs`
- `src/traits/proratable.rs`
- `src/traits/weighable.rs`
- `src/traits/inspectable.rs`
- `src/spatial/pewei.rs`
- `src/spatial/contour_range.rs`
- `src/spatial/plateau.rs`
- `src/spatial/plateau_basis.rs`

## Complexity baseline (2026-03-28)

### File summary

| CC  | Cognitive | SLOC | File                          |
| --- | --------- | ---- | ----------------------------- |
|  65 |   0       |  510 | `traits/coordinate.rs`        |
|  59 |  15       |  781 | `spatial/pewei.rs`            |
|  32 |   0       |  185 | `graph/traits.rs`             |
|  32 |   6       |  198 | `spatial/plateau.rs`          |
|  29 |   4       |  190 | `spatial/plateau_basis.rs`    |
|  27 |   0       |  151 | `traits/accumulator.rs`       |
|  23 |   0       |  151 | `traits/observation.rs`       |
|  23 |   2       |  138 | `traits/proratable.rs`        |
|  21 |   4       |   96 | `traits/attenuatable.rs`      |
|  19 |   0       |  113 | `traits/inspectable.rs`       |
|  19 |   1       |  101 | `traits/rng.rs`               |
|  15 |   8       |  157 | `spatial/contour_range.rs`    |
|  14 |   0       |  143 | `spatial/view.rs`             |

> **Note:** Cognitive scores of 0 in trait/coordinate files indicate that the CC
> comes entirely from `match` arms in `#[derive]`-like impls, not from nested
> logic.  The work here is about documentation and contract clarity, not
> structural decomposition.

## Target (post-Phase 8)

| Metric                              | Before | Target                                         |
| ----------------------------------- | ------ | ---------------------------------------------- |
| `cargo doc` warnings for `traits/`  | ≥ 1    | 0                                              |
| `cargo doc` warnings for `spatial/` | ≥ 1    | 0                                              |
| `graph/traits.rs` doc completeness  | partial | Every trait item has a `///` contract comment  |
| `coordinate.rs` SLOC / complexity   | 510    | No change (docs only)                          |

---

## Background

The traits layer is the extension point for users of `torrust-mudlark`.  A user
who wants to use the crate with a custom coordinate type must implement
`Coordinate`, `SpatialRead`, `SpatialWrite`, and several smaller traits.  If
any trait method has an unclear contract, the user will implement it incorrectly.

`graph/traits.rs` has CC=32 and Cog=0 — meaning its complexity is entirely from
`match` arms in provided method bodies, not from nested logic.  The file is not
hard to read statically, but it lacks contract documentation.  This is the
highest-priority risk: an undocumented trait method is a time-bomb.

`spatial/pewei.rs` (CC=59, SLOC=781) is the most complex spatial file.  Most of
its complexity comes from coordinate arithmetic; the SLOC count suggests the file
could benefit from splitting into sub-modules.

---

## Tasks

### P8.1 — Document all trait methods in `graph/traits.rs`

**Status:** `[x]` done

**What to do:**

1. Read `src/graph/traits.rs` from top to bottom.
2. For every trait method, add a `/// <contract>` comment that answers:
   - What does this method do?
   - What is the caller's responsibility (invariants that must hold on entry)?
   - What does the implementation guarantee on exit?
3. For provided (default) implementations, add `/// # Default implementation`
   notes explaining what the default does and when it should be overridden.

**Acceptance:** `cargo doc --no-deps 2>&1 | grep "missing documentation"` shows
zero entries for `graph::traits`; `cargo test` passes.

---

### P8.2 — Document all trait methods in `traits/coordinate.rs`

**Status:** `[x]` done

**Depends on:** P8.1

**What to do:**

`Coordinate` is the most central trait.  Every method should have:
- A `/// <purpose>` line.
- A `/// # Panics` section if the method can panic (e.g. index out of bounds).
- A `/// # Examples` block for at least the three most-used methods.

**Acceptance:** `cargo doc --no-deps` produces zero warnings for
`traits::coordinate`; examples compile via `cargo test --doc`.

---

### P8.3 — Document `traits/accumulator.rs`, `observation.rs`, and related files

**Status:** `[x]` done

**Depends on:** P8.2

**What to do:**

Apply the same documentation pattern from P8.1 to:

- `traits/accumulator.rs`
- `traits/observation.rs`
- `traits/attenuatable.rs`
- `traits/proratable.rs`
- `traits/weighable.rs`
- `traits/inspectable.rs`
- `traits/rng.rs`

For each trait, add a module-level `//! <description>` doc comment explaining
the trait's role in the system and when a user would need to implement it.

**Acceptance:** `cargo doc --no-deps` produces zero warnings for all files;
`cargo test` passes.

---

### P8.4 — Split `spatial/pewei.rs` into focused submodules

**Status:** `[ ]` not started

**What to do:**

`pewei.rs` (781 SLOC) covers at least three distinct concerns:

1. The `Pewei` struct and its core arithmetic (weight calculation, depth gate).
2. The contour / plateau interaction logic.
3. Iterator / visitor helpers for scanning the Pewei structure.

Proposed split (without removing any public items):

```
src/spatial/pewei/
    mod.rs          ← re-exports all public items from sub-modules
    core.rs         ← Pewei struct + weight/depth arithmetic
    plateau.rs      ← contour/plateau interaction
    iter.rs         ← iterator helpers
```

Steps:
1. Create `src/spatial/pewei/` directory.
2. Move existing content into sub-modules, one group at a time.
3. Re-export everything from `mod.rs` to avoid breaking public API.
4. Run `cargo test` after each file move.

**Acceptance:** `pewei.rs` replaced by `pewei/` directory; all public items
accessible at the same path (`spatial::pewei::<Name>`); no CC change expected;
`cargo test` passes.

---

### P8.5 — Document `spatial/` public types

**Status:** `[ ]` not started

**Depends on:** P8.4

**What to do:**

Apply the same documentation pass from P8.1 to:

- `spatial/contour_range.rs`
- `spatial/plateau.rs`
- `spatial/plateau_basis.rs`
- `spatial/node.rs`
- `spatial/view.rs`

For each public struct/enum/trait, add:
- A struct-level `///` doc comment explaining what it represents in the domain.
- A `/// # Invariants` section listing any invariants the type maintains.

**Acceptance:** `cargo doc --no-deps` for `spatial` produces zero warnings;
`cargo test` passes.

---

### P8.6 — Add `#[must_use]` to pure query methods

**Status:** `[ ]` not started

**Depends on:** P8.5

**What to do:**

Any method that returns a value and has no side-effects should carry
`#[must_use]`.  Audit all `pub fn` items in `traits/` and `spatial/` and add
`#[must_use]` where appropriate.

Use `cargo clippy -- -W clippy::must_use_candidate` to surface candidates
automatically.  Review each suggestion; apply only those where ignoring the
return value is clearly a bug.

**Acceptance:** `cargo clippy -- -W clippy::must_use_candidate` produces ≤ 5
new suggestions (accounting for intentional exceptions); `cargo test` passes.
