# Refactoring Plan: Clean up `src/graph/core.rs`

## Status: Draft — awaiting review

---

## Motivation

`src/graph/core.rs` was created by inlining `legacy_promote` and `rebalance` from their original
free-function homes.  The inlining was mechanical and correct, but several local improvements were
not part of that structural refactor.  This plan addresses five categories of quality:

- **Simpler** — remove dead assignments and clarify diverging control flow
- **Cleaner** — eliminate full module-path repetition and config duplication
- **More readable** — collapse repeated boilerplate into test helpers; rename ambiguous identifiers
- **More testable** — separate white-box from black-box tests; mark untestable paths
- **More sustainable** — add a tech-debt marker so `legacy_promote` is not silently promoted to
  permanent API

Each item below is self-contained and can land as a single commit.

---

## Item 1: Remove the dead `c_evictable = false` assignment (simpler)

### Problem

Inside `legacy_promote`, three evictability values are computed before updating the grandparent's
`has_evictable` field:

```rust
let c_evictable = false;
let p_evictable = self.vtree.nodes.node_has_evictable(p);
let u_evictable = self.vtree.nodes.node_has_evictable(u_id);
...
*has_evictable = c_evictable || p_evictable || u_evictable;
```

`c_evictable` is always `false`.  The assignment was written for visual symmetry but hides an
invariant: `c` is being demoted and must never be evictable at this point.  A reader has to confirm
the constant is never overwritten before trusting the short-circuit evaluation.

### Fix

Remove the variable.  State the invariant as a comment instead:

```rust
// c is being demoted — it is never evictable at this point
let p_evictable = self.vtree.nodes.node_has_evictable(p);
let u_evictable = self.vtree.nodes.node_has_evictable(u_id);
...
*has_evictable = p_evictable || u_evictable;
```

### Files touched

- `src/graph/core.rs` — `legacy_promote` body

### Verification

`./scripts/verify.sh` must pass with no change to logic or test outcomes.

---

## Item 2: Clarify `handle_iteration_limit` callsite control flow (simpler)

### Problem

The current callsite combines the over-budget guard and the limit function in a single `if`
condition:

```rust
if iterations > max_iterations
    && handle_iteration_limit(
        &self.vtree.nodes,
        &self.vtree.violations,
        iterations,
        max_iterations,
        resolved,
        c,
    )
{
    break;
}
```

`handle_iteration_limit` either diverges (`panic!` in debug) or returns `true` (release).  The
`&&` short-circuit means a reader must know the return type semantics to understand why `break` is
only reached in release builds.  The function signature — `-> bool` — does not communicate the
diverging debug path.

### Fix

Split into two statements so the release-only `break` is visually separate:

```rust
if iterations > max_iterations {
    // In debug builds handle_iteration_limit diverges via panic!;
    // in release it logs and signals break.
    if handle_iteration_limit(
        &self.vtree.nodes,
        &self.vtree.violations,
        iterations,
        max_iterations,
        resolved,
        c,
    ) {
        break;
    }
}
```

### Files touched

- `src/graph/core.rs` — `rebalance` body

### Verification

`./scripts/verify.sh` — no logic change; the existing `handle_iteration_limit_panics_in_debug_mode`
test still exercises this path.

---

## Item 3: Local `use` for `audit_violations` (cleaner)

### Problem

`rebalance` references `crate::diagnostics::diagnostic::audit_violations` three times via its full
path:

```rust
crate::diagnostics::diagnostic::audit_violations(&self.vtree.nodes, &self.vtree.violations, "PRE-REBALANCE");
// ...
crate::diagnostics::diagnostic::audit_violations(&self.vtree.nodes, &self.vtree.violations, "POST-RESOLVE");
// ...
crate::diagnostics::diagnostic::audit_violations(&self.vtree.nodes, &self.vtree.violations, "RESIDUAL");
```

The repetition is line-length pressure and obscures the three-phase structure of the loop.

### Fix

Add a function-level `use` at the top of `rebalance`:

```rust
pub(crate) fn rebalance(&mut self) -> Vec<GNodeId> {
    use crate::diagnostics::diagnostic::audit_violations;
    // ...
    audit_violations(&self.vtree.nodes, &self.vtree.violations, "PRE-REBALANCE");
```

### Files touched

- `src/graph/core.rs` — `rebalance` method

### Verification

`./scripts/verify.sh`.

---

## Item 4: Consolidate duplicate config in test helpers (cleaner)

### Problem

The `tests` module defines two fixture functions whose config structs share identical field values
but are written out separately:

```rust
fn make_config() -> Config<u32> {
    Config {
        split_threshold: 2,
        structural: StructuralConfig {
            depth_create: 3,
            depth_evict: 5,
            budget: None,
            alpha_relax: 0.5,
            bounded_eviction: false,
        },
    }
}

fn make_graph() -> GvGraph<u8, u64, 8> {
    GvGraph::new(Config {
        split_threshold: 2,             // ← duplicated values
        structural: StructuralConfig {
            depth_create: 3,
            depth_evict: 5,
            budget: None,
            alpha_relax: 0.5,
            bounded_eviction: false,
        },
    })
}
```

If the test fixture config changes, both must be updated in sync.  The different `V` types (`u32`
vs `u64`) are the only real difference between the two graphs.

### Fix

Make `make_graph` derive its config from `make_config`, casting where necessary:

```rust
fn make_graph() -> GvGraph<u8, u64, 8> {
    let cfg = make_config();
    GvGraph::new(Config {
        split_threshold: cfg.split_threshold.into(),
        structural: cfg.structural,
    })
}
```

Alternatively, if `Config<V>` does not implement `From` across `V` types, define a separate
`fn make_config_u64() -> Config<u64>` that calls a shared `fn base_structural() -> StructuralConfig`
so the structural fields are defined exactly once:

```rust
fn base_structural() -> StructuralConfig {
    StructuralConfig {
        depth_create: 3,
        depth_evict: 5,
        budget: None,
        alpha_relax: 0.5,
        bounded_eviction: false,
    }
}

fn make_config() -> Config<u32> {
    Config { split_threshold: 2, structural: base_structural() }
}

fn make_graph() -> GvGraph<u8, u64, 8> {
    GvGraph::new(Config { split_threshold: 2, structural: base_structural() })
}
```

Choose the approach that compiles without extra trait bounds.

### Files touched

- `src/graph/core.rs` — `tests` module

### Verification

`./scripts/verify.sh` — all tests must still pass with identical behaviour.

---

## Item 5: Rename `g` → `grandparent` in `legacy_promote` (more readable)

### Problem

`legacy_promote` uses three local variables to navigate the V-tree ancestry chain:

```rust
let p = ...;  // VNodeId — parent of c
let g = ...;  // VNodeId — grandparent of c (parent of p)
let gnode_id = ...;  // GNodeId — G-node backing c
```

`g` (a `VNodeId`) and `gnode_id` (a `GNodeId`) sit in the same scope.  In a dual-tree codebase
where `g` is conventionally short for "G-tree node", this naming inverts the convention.  The
tracing span worsens the confusion: both `g = g.index()` and `gnode = gnode_id.index()` appear side
by side.

### Fix

Rename `g` to `vg` throughout `legacy_promote`:

```rust
let vg = self
    .vtree
    .nodes
    .get(p.index())
    .parent()
    .expect("legacy_promote: p must have a grandparent");

let _span = tracing::debug_span!(
    "legacy_promote",
    c = %rebalance::Nd(&self.vtree.nodes, c),
    p = p.index(),
    vg = vg.index(),
    gnode = gnode_id.index(),
)
.entered();
// ... all subsequent uses of g → vg
```

### Files touched

- `src/graph/core.rs` — `legacy_promote` method body and span

### Verification

`./scripts/verify.sh`.

---

## Item 6: Add a tech-debt marker on `legacy_promote` (more sustainable)

### Problem

`legacy_promote` is flagged as "legacy" in its name but carries no context explaining why it exists,
what it is a stepping stone toward, or when it should be removed.  Without a marker it risks
becoming a permanent part of the API.

### Fix

Add a `TODO` comment on the method and, optionally, a `#[deprecated]` attribute:

```rust
/// Promotes entry `c` from a semi-internal G-node into the grandparent level of
/// the V-tree, creating the missing G-child that `c` previously occupied.
///
/// # TODO
///
/// This function is a legacy adaptation path used during the transition away from
/// the old borrow-split resolve model.  Once `resolve_path_b` is refactored to
/// operate directly on `&mut GvCore`, this function should be inlined or removed.
/// Tracked in: <issue or PR reference>.
pub(crate) fn legacy_promote(&mut self, c: VNodeId) -> GNodeId {
```

If a tracking issue exists, reference it.  If not, open one and link it before landing this item.

### Files touched

- `src/graph/core.rs` — `legacy_promote` doc comment

### Verification

`./scripts/clippy-strict.sh` — the added doc comment must not introduce any lint warnings.

---

## Item 7: Separate white-box and black-box tests in `rebalance_fn` (more testable)

### Problem

The `rebalance_fn` submodule mixes two test styles that have different stability guarantees:

**Black-box (public API):**
- `total_sum_invariant_is_maintained_after_many_observations`
- `node_count_is_at_least_one_after_many_observations`
- `rebalance_skips_destroyed_queue_nodes`
- `rebalance_skips_already_resolved_nodes`

These rely only on `GvGraph`'s public surface and should survive any internal restructuring.

**White-box (internal API):**
- `resolve_returns_none_when_node_has_no_parent` — calls `rebalance::resolve` directly
- `handle_iteration_limit_panics_in_debug_mode` — calls `super::super::handle_iteration_limit`
  directly

These couple the tests to the module-private implementation; they will break if `resolve` or
`handle_iteration_limit` are moved, renamed, or inlined.

### Fix

Split into two named inner modules with a comment explaining the distinction:

```rust
mod rebalance_fn {
    use super::*;

    // Black-box tests: exercise GvGraph's public API only.
    // These should survive any internal restructuring.
    mod public_api {
        use super::*;

        #[test]
        fn total_sum_invariant_is_maintained_after_many_observations() { ... }

        #[test]
        fn node_count_is_at_least_one_after_many_observations() { ... }

        #[test]
        fn rebalance_skips_destroyed_queue_nodes() { ... }

        #[test]
        fn rebalance_skips_already_resolved_nodes() { ... }
    }

    // White-box tests: exercise internal helpers directly.
    // These are coupled to the current implementation; update them when refactoring internals.
    mod internals {
        use super::*;
        use crate::graph::algorithm::rebalance::resolve;

        #[test]
        fn resolve_returns_none_when_node_has_no_parent() { ... }

        #[test]
        #[should_panic(expected = "rebalance: exceeded")]
        fn handle_iteration_limit_panics_in_debug_mode() { ... }
    }
}
```

Additionally, add a comment to the `handle_iteration_limit_panics_in_debug_mode` test noting that
the release-build `true` return path has no automated coverage (it can only be exercised by
intentionally making the rebalance loop cycle, which is not practical in unit tests):

```rust
// Note: the release-build path (returns true, signals break) is not covered by
// automated tests — exercising it requires a cycle in violation resolution.
```

### Files touched

- `src/graph/core.rs` — `tests::rebalance_fn` submodule

### Verification

`./scripts/verify.sh` — all 584 tests must still pass.

---

## Implementation order

| Priority | Item | Rationale |
|----------|------|-----------|
| 1 | Item 3 — local `use audit_violations` | Smallest diff, purely cleanup, no logic risk |
| 2 | Item 1 — remove `c_evictable` | One-line logic change; makes invariant explicit |
| 3 | Item 4 — consolidate test config | Eliminates silent config drift in tests |
| 4 | Item 7 — split white/black-box tests | Structural only; no logic change |
| 5 | Item 5 — rename `g` → `vg` | Touches more lines but zero logic risk |
| 6 | Item 2 — clarify callsite control flow | Pure readability; verify the test still runs |
| 7 | Item 6 — tech-debt marker | Requires opening or referencing a tracking issue |

Each item can be a single focused commit following conventional commit format, e.g.:

```
refactor(core): remove dead c_evictable assignment in legacy_promote
refactor(core): add local use for audit_violations in rebalance
test(core): consolidate duplicate config in test fixtures
test(core): separate white-box and black-box tests in rebalance_fn
refactor(core): rename g to vg in legacy_promote for clarity
refactor(core): split handle_iteration_limit guard into two statements
docs(core): add TODO marker on legacy_promote tech-debt
```

---

## Progress tracker

| Item | Status |
|------|--------|
| 1. Remove dead `c_evictable` assignment | ⬜ Not started |
| 2. Clarify `handle_iteration_limit` callsite | ⬜ Not started |
| 3. Local `use audit_violations` | ⬜ Not started |
| 4. Consolidate test config | ⬜ Not started |
| 5. Rename `g` → `vg` in `legacy_promote` | ⬜ Not started |
| 6. Tech-debt marker on `legacy_promote` | ⬜ Not started |
| 7. Split white-box / black-box tests | ⬜ Not started |
