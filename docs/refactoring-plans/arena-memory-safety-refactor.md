# Arena Memory Safety Refactoring Plan

**Date:** March 31, 2026  
**Component:** `src/arena.rs`  
**Priority:** CRITICAL  
**Status:** Planning

---

## Executive Summary

The `Arena<T>` allocator contains several critical memory safety vulnerabilities that can lead to:
- **Use-after-free** bugs (accessing deallocated memory)
- **Double-free** errors (freeing the same slot twice)
- **Memory corruption** (writing to wrong memory locations)
- **Panics** in production (unchecked bounds)
- **Stale data leaks** (returning garbage from unoccupied slots)

Additionally, there is no mechanism to detect **stale handles** after slot reuse, allowing application code to accidentally access newly-allocated data thinking it's the old data.

This plan outlines a **two-tier refactoring strategy**:
1. **Tier 1: Fix Core Memory Safety** - Eliminate the 7 identified vulnerabilities (phases 1-6)
2. **Tier 2: Add Generational Handles** - Prevent use-after-free at the application level via generation counters (phases 7-8)

This produces a **production-ready arena** that is both memory-safe and resilient to stale handle bugs.

---

## Problems Analysis

### P1: Integer Truncation on Free List (CRITICAL)

**Location:** `dealloc()`, line 71  
**Severity:** CRITICAL - Use-After-Free / Double-Free

```rust
#[allow(clippy::cast_possible_truncation)]
self.free.push(index as u32);
```

**Problem:**
- Indices are `usize` but stored as `u32` with explicit truncation
- On 64-bit systems, indices > 4GB are silently corrupted
- When `alloc()` pops and reconstructs the index, it refers to wrong memory
- Can cause double-free, use-after-free, and data corruption

**Impact:**
- Systems with large allocations (>2^32 elements) corrupt heap
- Use becomes unsafe despite type system
- Silent failures make debugging extremely difficult

---

### P2: Missing Bounds Check in `set_occupied()` (CRITICAL)

**Location:** `set_occupied()`, lines 116-125  
**Severity:** CRITICAL - Panic in Production

```rust
fn set_occupied(&mut self, index: usize, value: bool) {
    let word = index / 64;
    let bit = index % 64;
    if value {
        self.occupied[word] |= 1u64 << bit;  // PANICS if word >= occupied.len()
    } else {
        self.occupied[word] &= !(1u64 << bit);
    }
}
```

**Problem:**
- When reusing freed slots in `alloc()`, `set_occupied()` is called before validating bounds
- If free list contains corrupted index, or if `occupied` vec is too short, indexing panics
- No panic recovery possible; terminates application

**Impact:**
- Unexpected crashes in production
- Combined with P1, creates guaranteed panic scenarios
- Violates Rust safety guarantees for bounds

---

### P3: Free List Can Reference Non-Existent Slots (CRITICAL)

**Location:** `alloc()`, lines 56-58  
**Severity:** CRITICAL - Out-of-Bounds Access

```rust
let index = if let Some(idx) = self.free.pop() {
    let i = idx as usize;
    self.slots[i] = value;  // PANICS if i >= slots.len()
    i
} else {
    // ...
};
```

**Problem:**
- Free list validity is assumed but never validated
- If P1 corruption occurs, free list contains invalid indices > `slots.len()`
- Attempting to reuse freed slot triggers out-of-bounds panic
- Created by interaction of P1 (truncation) + missing validation

**Impact:**
- Cascading failures from truncation
- No recovery mechanism exists

---

### P4: Debug Assertions Only (MODERATE)

**Location:** `get()`, `get_mut()`, `dealloc()`, lines 84-98  
**Severity:** MODERATE - Release Build Memory Unsafety

```rust
pub fn get(&self, index: usize) -> &T {
    debug_assert!(
        self.is_occupied(index),
        "Arena::get: slot {index} is not occupied"
    );
    &self.slots[index]
}
```

**Problem:**
- Safety checks only active in debug builds
- Release builds skip assertions, allowing access to unoccupied slots
- Returns stale/garbage data
- Violates type system contract: occupied slots should always be valid

**Impact:**
- Silent memory corruption in production (debug assertions stripped)
- Logic errors from garbage data go unnoticed
- Security risk if stale data contains sensitive information

---

### P5: Free List Invariant Not Validated (MODERATE)

**Location:** Entire struct  
**Severity:** MODERATE - Silent Data Corruption

**Problem:**
- Invariant: "All indices in `free` are marked as unoccupied in `occupied`"
- No mechanism validates this is maintained
- Bugs in implementation can silently violate invariant
- No way to detect or recover from corruption
- Cloning doesn't verify state correctness

**Impact:**
- Hard-to-diagnose bugs
- State can be corrupted without obvious error messages
- No audit trail or validation tool

---

### P6: Clone Propagates Corrupted State (MODERATE)

**Location:** `Clone` impl, lines 138-147  
**Severity:** MODERATE - Corruption Replication

```rust
impl<T: Default + Clone> Clone for Arena<T> {
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            occupied: self.occupied.clone(),
            free: self.free.clone(),
            count: self.count,
        }
    }
}
```

**Problem:**
- If source arena is corrupted (truncated indices, invalid state), clone replicates corruption
- No validation during clone
- Spreads bugs across multiple instances
- Combined with P1, creates multiple corrupted arenas from one

**Impact:**
- Corruption multiplies across cloned instances
- Makes debugging harder (multiple broken copies)
- Validation failures cascade

---

### P7: Possible Out-of-Bounds in `iter_occupied()` (MINOR)

**Location:** `iter_occupied()`, lines 110-115  
**Severity:** MINOR - Inconsistent State

```rust
pub fn iter_occupied(&self) -> impl Iterator<Item = (usize, &T)> + '_ {
    (0..self.slots.len())
        .filter(move |&i| self.is_occupied(i))
        .map(move |i| (i, &self.slots[i]))
}
```

**Problem:**
- Assumes `occupied` bitmap is at least as large as `slots`
- If corruption causes `occupied` to be too short, `is_occupied()` returns false for all
- Silently skips valid entries instead of panicking
- No explicit validation of bitmap size

**Impact:**
- Iterator may return incomplete or no results
- Silent data loss difficult to diagnose

---

## Refactoring Strategy

### Phase 1: Use `usize` Consistently (CRITICAL)
**Goal:** Eliminate truncation vulnerability  
**Breaking Change:** Yes (free list type changes)

1. Change `free: Vec<u32>` → `free: Vec<usize>`
2. Update `dealloc()` to push `usize` directly
3. Update `alloc()` to handle `usize` from free list
4. Add compile-time check: `#[cfg(target_pointer_width = "64")]` for large allocations if needed
5. Update all related tests

**Rationale:** Eliminates the entire class of truncation bugs. No silent corruption possible.

### Phase 2: Add Bounds Validation (CRITICAL)
**Goal:** Prevent panics and validate assumptions  
**Breaking Change:** No

1. Add validation in `set_occupied()`:
   ```rust
   fn set_occupied(&mut self, index: usize, value: bool) {
       let word = index / 64;
       let bit = index % 64;
       if word >= self.occupied.len() {
           panic!("Arena::set_occupied: word index {word} out of bounds");
       }
       // ...
   }
   ```

2. Add validation in `alloc()` when reusing slots:
   ```rust
   let index = if let Some(idx) = self.free.pop() {
       if idx >= self.slots.len() {
           panic!("Arena::alloc: free slot index {idx} out of bounds");
       }
       let i = idx;
       self.slots[i] = value;
       i
   } else {
       // ...
   };
   ```

3. Add validation in `clone()` to detect corrupted state

**Rationale:** Converts silent failures into catchable panics. Makes bugs visible immediately.

### Phase 3: Runtime Safety Checks (MODERATE)
**Goal:** Guarantee safety in release builds  
**Breaking Change:** No

1. Replace `debug_assert!` with `assert!` in:
   - `get()` - line 84
   - `get_mut()` - line 93
   - `dealloc()` - line 68

2. Or create a Feature-Gated Option:
   ```rust
   #[cfg(feature = "arena-unchecked")]
   pub fn get(&self, index: usize) -> &T {
       &self.slots[index]  // Fast path for trusted code
   }
   
   #[cfg(not(feature = "arena-unchecked"))]
   pub fn get(&self, index: usize) -> &T {
       assert!(self.is_occupied(index), ...);
       &self.slots[index]
   }
   ```

**Rationale:** 
- Prevents release-build memory unsafety
- `assert!` is typically optimized away in release for inline functions
- Feature flag allows performance-critical code to opt-out

### Phase 4: Add Invariant Validation Method (MODERATE)
**Goal:** Enable debugging and detection of corruption  
**Breaking Change:** No (adds method)

```rust
#[cfg(test)]
impl<T: Default> Arena<T> {
    /// Validates all internal invariants.
    /// Returns Err with description of first violation found.
    pub fn validate_invariants(&self) -> Result<(), String> {
        // Check 1: count matches occupied slots
        let occupied_count = (0..self.slots.len())
            .filter(|&i| self.is_occupied(i))
            .count();
        if occupied_count != self.count as usize {
            return Err(format!(
                "count mismatch: {} reported, {} occupied slots",
                self.count, occupied_count
            ));
        }

        // Check 2: free list indices are valid and unoccupied
        for &idx in &self.free {
            if idx >= self.slots.len() {
                return Err(format!(
                    "free list contains out-of-bounds index: {}",
                    idx
                ));
            }
            if self.is_occupied(idx) {
                return Err(format!(
                    "free list contains occupied slot: {}",
                    idx
                ));
            }
        }

        // Check 3: no duplicates in free list
        let mut seen = std::collections::HashSet::new();
        for &idx in &self.free {
            if !seen.insert(idx) {
                return Err(format!("free list contains duplicate index: {}", idx));
            }
        }

        Ok(())
    }
}
```

**Rationale:** Allows tests and debug builds to detect invariant violations early.

### Phase 5: Update Clone Implementation (MODERATE)
**Goal:** Validate before cloning  
**Breaking Change:** No

```rust
impl<T: Default + Clone> Clone for Arena<T> {
    fn clone(&self) -> Self {
        #[cfg(test)]
        if let Err(e) = self.validate_invariants() {
            panic!("Arena::clone: source arena has invalid invariants: {}", e);
        }

        Self {
            slots: self.slots.clone(),
            occupied: self.occupied.clone(),
            free: self.free.clone(),
            count: self.count,
        }
    }
}
```

**Rationale:** Prevents cloning of corrupted arenas in test/debug scenarios.

### Phase 6: Add Comprehensive Tests (MODERATE)
**Goal:** Exercise edge cases and invariant checks  
**Breaking Change:** No

New tests to add:
1. **Bounds tests:**
   - Allocating and reusing many slots (test wrapping)
   - Large allocations (test 64-bit index handling)

2. **Invariant tests:**
   - Validate after each operation
   - Test after clone
   - Stress test with many alloc/dealloc cycles

3. **Safety tests:**
   - Attempt double-deallocate (should panic/fail gracefully)
   - Attempt get on deallocated slot (should panic)
   - Reuse patterns with many freed slots

**Rationale:** Catches regressions and edge cases.

### Phase 7: Add Generational Handle Support (MODERATE)
**Goal:** Enable application-level use-after-free detection  
**Breaking Change:** Yes (API change to `alloc()` return type)

Reference: See `docs/refactoring-plans/arena-comparison-analysis.md` for GitHub design rationale.

1. Add `generations` field to struct:
   ```rust
   pub struct Arena<T: Default> {
       slots: Vec<T>,
       occupied: Vec<u64>,
       free: Vec<usize>,
       count: u32,
       generations: Vec<u32>,  // ← New field
   }
   ```

2. Update `alloc()` signature and implementation:
   ```rust
   pub fn alloc(&mut self, value: T) -> (usize, u32) {
       let index = if let Some(idx) = self.free.pop() {
           // ... existing code ...
       } else {
           // ... existing code ...
           if i >= self.generations.len() {
               self.generations.resize(i + 1, 0);
           }
       };
       self.set_occupied(index, true);
       self.count += 1;
       let generation = self.generations[index];  // ← Capture current generation
       (index, generation)  // ← Return both
   }
   ```

3. Update `dealloc()` to increment generation:
   ```rust
   pub fn dealloc(&mut self, index: usize) -> T {
       debug_assert!(self.is_occupied(index), ...);
       self.set_occupied(index, false);
       self.free.push(index);
       self.count -= 1;
       // Increment generation (wrapping is acceptable)
       self.generations[index] = self.generations[index].wrapping_add(1);
       std::mem::take(&mut self.slots[index])
   }
   ```

4. Add `generation()` query method:
   ```rust
   #[must_use]
   #[inline]
   pub fn generation(&self, index: usize) -> u32 {
       self.generations.get(index).copied().unwrap_or(0)
   }
   ```

5. Update `initialize()` for new fields in cloning/default paths

**Rationale:** 
- Generational handles allow clients to validate: `if arena.generation(idx) == stored_gen { /* handle valid */ }`
- Generation increments on dealloc, invalidating stale handles
- Prevents use-after-free bugs at the application level
- Design validated by GitHub reference implementation

### Phase 8: Update Tests for Generational Handles (MODERATE)
**Goal:** Test generation tracking and validation  
**Breaking Change:** No (additive tests)

New tests to add:
1. **Generation lifecycle:**
   - Allocation returns generation 0 (for new slots)
   - Generation increments on dealloc
   - Multiple alloc/dealloc cycles increment properly
   - Wrapping works correctly (generation u32::MAX → 0)

2. **Handle validation:**
   - Valid handle has matching generation
   - Stale handle (old generation) is detected
   - Different slots can have different generations

3. **API interactions:**
   - Update return type signature expectations
   - Test (index, generation) tuple in all scenarios
   - Validate generation API with edge cases

**Rationale:** Ensures generational handle feature is reliable and well-tested.

---

## Implementation Phases

### Phase 1: Preparation
- [ ] Create feature flags in `Cargo.toml`:
  - `arena-unchecked` - for performance-critical paths
  - `arena-validate` - adds invariant checking

### Phase 2: Critical Fixes (Implement Together)
- [ ] Change `free: Vec<u32>` → `free: Vec<usize>`
- [ ] Update `dealloc()` to use `usize`
- [ ] Add bounds checking in `set_occupied()`
- [ ] Add bounds checking in `alloc()` free slot handling
- [ ] Update tests for new types
- [ ] Run full test suite: `cargo test --all-features`

### Phase 3: Safety Improvements
- [ ] Replace `debug_assert!` with conditional `assert!` or feature-gated
- [ ] Add `validate_invariants()` method
- [ ] Update `clone()` to validate
- [ ] Add all new tests
- [ ] Run linter: `cargo clippy --all-features -- -D warnings`

### Phase 4: Documentation
- [ ] Update struct-level documentation with invariants
- [ ] Document `validate_invariants()` use cases
- [ ] Add safety notes to public methods
- [ ] Document feature flags

### Phase 5: Regression Testing (Memory Safety)
- [ ] Run entire test suite: `cargo test --all-features`
- [ ] Run with address sanitizer: `RUSTFLAGS=-Zsanitizer=address cargo test --all-features`
- [ ] Run with memory sanitizer: `RUSTFLAGS=-Zsanitizer=memory cargo test --all-features`
- [ ] Benchmark performance: `cargo bench --bench depth`
- [ ] Document baseline metrics for comparison

### Phase 6: Generational Handles Implementation
- [ ] Add `generations: Vec<u32>` field to struct
- [ ] Initialize generations in all code paths (alloc, dealloc, clone, default)
- [ ] Update `alloc()` to return `(usize, u32)` tuple with generation
- [ ] Update `dealloc()` to increment generation (wrapping_add)
- [ ] Add `generation(index: usize) -> u32` query method
- [ ] Update Clone impl to handle generations
- [ ] Update Debug impl if needed
- [ ] Update documentation with usage examples

### Phase 7: Generational Validation Tests
- [ ] Port all existing tests to handle new `(usize, u32)` return types from `alloc()`
- [ ] Add generation lifecycle tests (increment on dealloc, wrapping)
- [ ] Add handle validation scenario tests (valid vs stale)
- [ ] Add multi-cycle tests (alloc/dealloc/realloc with generation checks)
- [ ] Add edge case tests (generation overflow at u32::MAX)
- [ ] Run full test suite: `cargo test --all-features`

### Phase 8: Final Validation & Documentation
- [ ] Update struct documentation with generation explanation
- [ ] Add example code showing handle validation pattern
- [ ] Create migration guide for API break change
- [ ] Run all sanitizers: `RUSTFLAGS=-Zsanitizer=address,memory cargo test --all-features`
- [ ] Final benchmark comparison with main branch
- [ ] Create CHANGELOG entry with migration notes
- [ ] **Code Coverage Analysis:**
  - [ ] Generate coverage report: `cargo tarpaulin --out Html --exclude-files tests/`
  - [ ] Identify uncovered code paths (esp. error conditions, edge cases)
  - [ ] Add tests to cover identified gaps:
    - [ ] Cover all error branches in `set_occupied()`, `alloc()`, `dealloc()`
    - [ ] Cover generation wrapping edge cases (u32::MAX transitions)
    - [ ] Cover all bitmap word boundary conditions
    - [ ] Test interaction of free list and occupied bitmap under stress
  - [ ] Target minimum coverage: **95%** for critical paths
  - [ ] Document known untested paths (if any) and rationale
  - [ ] Run coverage again after adding new tests and verify target met

---

## Risk Assessment

| Phase | Risk | Mitigation | Effort |
|-------|------|-----------|--------|
| 1 | Type change breaks users | Semantic versioning (major bump) | 2h |
| 2 | Panics in production? | Bounds checks prevent corruption | 3h |
| 3 | Performance impact | Feature flag for unchecked paths | 2h |
| 4 | Test coverage gaps | Add comprehensive stress tests | 4h |
| 5 | Sanitizer build failures | Fix issues incrementally | 2h |
| 6 | API incompatibility | Breaking change requires major version | 2h |
| 7 | Generation overflow? | Wrapping behavior is acceptable | 3h |
| 8 | Coverage gaps remain | Iterative testing to close gaps | 3h |
| **Total** | **Comprehensive approach** | **Complete validation** | **~21h** |

---

## Success Criteria

- [ ] All 7 identified memory safety problems eliminated
- [ ] No type truncations anywhere
- [ ] All bounds checks in place
- [ ] All debug_assert → assert (or feature-gated)
- [ ] `validate_invariants()` passes for all tests
- [ ] Generational handles working correctly
  - [ ] `alloc()` returns (usize, u32) with correct generation
  - [ ] `dealloc()` increments generation
  - [ ] Stale handles are rejected when generation mismatches
- [ ] `cargo test --all-features` passes 100%
- [ ] `cargo clippy --all-features` passes with no warnings
- [ ] Address sanitizer: no errors
- [ ] Memory sanitizer: no errors
- [ ] **Code Coverage Goals:**
  - [ ] Minimum 95% line coverage achieved
  - [ ] All critical paths covered (memory safety operations)
  - [ ] All error conditions tested
  - [ ] Edge cases covered (generation wrapping, bitmap boundaries, etc.)
  - [ ] Coverage report generated and documented
- [ ] Documentation updated with examples
- [ ] Migration guide provided for users
- [ ] Benchmark shows <5% performance impact (with generations)

---

## Backward Compatibility Notes

### Breaking Changes
- `free` field changes from `Vec<u32>` to `Vec<usize>` (Phase 2)
- `alloc()` return type changes from `usize` to `(usize, u32)` (Phase 6)
- If any external code accesses `free` directly or relies on `alloc()` return type, will break compilation

### Non-Breaking Changes
- Public API methods remain the same (get, get_mut, is_occupied, iter_occupied, etc.)
- Feature flags allow opt-out of safety checks
- New `generation()` method is additive to API
- Panic behavior may change but is not guaranteed anyway

### Migration Path
1. **Phase 1-5:** Bump to semver minor (safety improvements, same API)
2. **Phase 6-8:** Bump to semver major (API breaking change for generational handles)
3. Provide clear migration guide with examples
4. Support old version in parallel briefly if needed

---

## Comparison with Existing Reference

This plan improves upon the GitHub reference implementation (`da2ce7/torrust-index:b358f6ab37...`) by:
1. **Fixing all memory safety issues** that GitHub reference also contains
2. **Retaining comprehensive test coverage** (current: 27 tests vs GitHub: 0 tests)
3. **Adding validation infrastructure** (invariant checking, sanitizer validation)
4. **Including generational handles** (from GitHub) but fixing their underlying vulnerabilities
5. **Providing clear migration path** and documentation

See `docs/refactoring-plans/arena-comparison-analysis.md` for detailed comparison.

---

## Next Steps

1. **Review & Approval:** Get team consensus on two-tier approach
2. **Spike (Phase 1-2):** Implement memory safety fixes to validate feasibility
3. **Full Implementation:** Roll out all 8 phases systematically
4. **Continuous Testing:** Validation with sanitizers after each phase
5. **Documentation:** Update all references and provide migration guide
6. **Release:** Major version bump with comprehensive changelog
7. **Monitoring:** Track adoption and gather feedback from users
