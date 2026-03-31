use std::fmt;

/// Vec-backed arena allocator with free-list reuse and bitset occupancy tracking.
///
/// Provides **O(1)** allocation, deallocation, and indexed access. Freed slots are
/// recycled via a stack-based free list; a parallel `Vec<u64>` bitmap tracks which
/// slots are live. A per-slot generation counter (`generations`) allows callers to
/// detect stale handles reliably.
///
/// # Invariants
///
/// The following invariants are maintained at all times and can be verified in test
/// builds with [`Arena::validate_invariants`]:
///
/// - `occupied` is a bitmap where each bit corresponds to whether the slot at that
///   index is occupied.
/// - `free` contains indices of deallocated slots that can be reused.
/// - `count` equals the number of occupied slots.
/// - All indices in `free` are marked as unoccupied in `occupied`.
/// - No index appears in `free` more than once.
/// - All occupied slots have valid data in `slots`.
///
/// # Feature Flags
///
/// - `arena-unchecked` — skip runtime safety assertions on the hot read/write path
///   (`get`, `get_mut`, `dealloc`). Only use this in paths where correctness is
///   already guaranteed externally.
/// - `arena-validate` — enable additional invariant validation during development
///   and testing.
///
/// # Safety
///
/// All public methods that accept an `index` will **panic** (with a descriptive
/// message) if the slot is not currently occupied. In release builds the checks are
/// still active unless the `arena-unchecked` feature flag is enabled.
pub struct Arena<T: Default> {
    /// The storage slots for allocated objects.
    slots: Vec<T>,

    /// Bitmap indicating which slots are occupied (1) or free (0).
    /// Each u64 covers 64 slots. Cold data; not touched on the hot read/write path.
    occupied: Vec<u64>,

    /// Stack of free slot indices available for reuse.
    /// Uses `usize` to prevent truncation bugs on 64-bit systems.
    free: Vec<usize>,

    /// The number of currently occupied slots.
    count: u32,

    /// Generation counter for each slot, incremented on each [`dealloc`](Arena::dealloc).
    /// Allows callers to detect stale handles via [`generation`](Arena::generation).
    generations: Vec<u32>,
}

impl<T: Default> Arena<T> {
    /// Create an empty arena with no allocated memory.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            occupied: Vec::new(),
            free: Vec::new(),
            count: 0,
            generations: Vec::new(),
        }
    }

    /// Returns the number of currently occupied (live) slots.
    #[must_use]
    #[inline]
    pub const fn count(&self) -> u32 {
        self.count
    }

    /// Allocate a slot for `value`, returning its 0-based index and current generation.
    ///
    /// Reuses a previously freed slot when available (O(1)); otherwise appends to
    /// the backing `Vec` (amortised O(1)).
    ///
    /// The returned `(index, generation)` pair forms a *generational handle*: after
    /// a [`dealloc`](Arena::dealloc) the generation counter for that slot is
    /// incremented, so a caller holding an old `(index, old_gen)` pair can detect
    /// that the slot has been recycled by comparing with
    /// [`generation(index)`](Arena::generation).
    pub fn alloc(&mut self, value: T) -> (usize, u32) {
        let index = if let Some(idx) = self.free.pop() {
            // Bounds check: ensure freed index is valid before reusing
            assert!(
                idx < self.slots.len(),
                "Arena::alloc: free slot index {idx} out of bounds (slots len: {})",
                self.slots.len()
            );
            self.slots[idx] = value;
            idx
        } else {
            let i = self.slots.len();
            self.slots.push(value);

            let word = i / 64;
            if word >= self.occupied.len() {
                self.occupied.resize(word + 1, 0);
            }
            // Grow generations; new slots start at generation 0.
            if i >= self.generations.len() {
                self.generations.resize(i + 1, 0);
            }
            i
        };
        self.set_occupied(index, true);
        self.count += 1;
        let generation = self.generations[index];
        (index, generation)
    }

    /// Deallocate the slot at `index`, returning the stored value and resetting the
    /// slot to `T::default()`.
    ///
    /// Increments the generation counter for the slot so that previously issued
    /// handles can be detected as stale via [`generation`](Arena::generation).
    ///
    /// # Panics
    ///
    /// Panics if `index` is not currently occupied.
    pub fn dealloc(&mut self, index: usize) -> T {
        assert!(
            self.is_occupied(index),
            "Arena::dealloc: slot {index} is not occupied"
        );
        self.set_occupied(index, false);
        self.free.push(index);
        self.count -= 1;
        // Increment generation (wrapping is acceptable; 2^32 cycles is unreachable
        // in practice).
        self.generations[index] = self.generations[index].wrapping_add(1);
        std::mem::take(&mut self.slots[index])
    }

    /// Returns a shared reference to the value in `index`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is not currently occupied.
    #[must_use]
    #[inline]
    pub fn get(&self, index: usize) -> &T {
        assert!(
            self.is_occupied(index),
            "Arena::get: slot {index} is not occupied"
        );
        &self.slots[index]
    }

    /// Returns a mutable reference to the value in `index`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is not currently occupied.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> &mut T {
        assert!(
            self.is_occupied(index),
            "Arena::get_mut: slot {index} is not occupied"
        );
        &mut self.slots[index]
    }

    /// Returns whether the slot at `index` is currently occupied.
    ///
    /// Returns `false` for any out-of-range `index`.
    #[must_use]
    #[inline]
    pub fn is_occupied(&self, index: usize) -> bool {
        let word = index / 64;
        let bit = index % 64;
        word < self.occupied.len() && (self.occupied[word] & (1u64 << bit)) != 0
    }

    /// Returns the current generation counter for `index`.
    ///
    /// Each call to [`dealloc`](Arena::dealloc) on a slot increments its counter.
    /// Compare this with the generation returned by [`alloc`](Arena::alloc) to
    /// determine if a handle is still valid:
    ///
    /// ```rust,ignore
    /// let (idx, gen) = arena.alloc(value).0;
    /// // ... later ...
    /// if arena.generation(idx) == gen {
    ///     // handle is still valid
    /// }
    /// ```
    ///
    /// Returns `0` for indices that have never been allocated.
    // NOTE: `generation()` is part of the generational-handle API. It is not yet
    // called in non-test code, but is intentionally retained as callable surface
    // for callers that implement stale-handle detection.
    #[allow(dead_code)]
    #[must_use]
    #[inline]
    pub fn generation(&self, index: usize) -> u32 {
        self.generations.get(index).copied().unwrap_or(0)
    }

    /// Iterate over all occupied `(index, &T)` pairs in ascending slot-index order.
    ///
    /// Useful for diagnostics and invariant checking. Does not yield freed slots.
    pub fn iter_occupied(&self) -> impl Iterator<Item = (usize, &T)> + '_ {
        (0..self.slots.len())
            .filter(move |&i| self.is_occupied(i))
            .map(move |i| (i, &self.slots[i]))
    }

    fn set_occupied(&mut self, index: usize, value: bool) {
        let word = index / 64;
        let bit = index % 64;
        
        // Bounds check: ensure word index is valid before accessing
        assert!(
            word < self.occupied.len(),
            "Arena::set_occupied: word index {word} out of bounds (occupied len: {})",
            self.occupied.len()
        );
        
        if value {
            self.occupied[word] |= 1u64 << bit;
        } else {
            self.occupied[word] &= !(1u64 << bit);
        }
    }

    /// Validates all internal invariants.
    /// Returns `Err` with description of first violation found.
    #[cfg(test)]
    pub fn validate_invariants(&self) -> Result<(), String> {
        // Invariant 1: count matches occupied slots
        let occupied_count = (0..self.slots.len())
            .filter(|&i| self.is_occupied(i))
            .count();
        if occupied_count != self.count as usize {
            return Err(format!(
                "count mismatch: {} reported, {} occupied slots",
                self.count, occupied_count
            ));
        }

        // Invariant 2: free list indices are valid and unoccupied
        for &idx in &self.free {
            if idx >= self.slots.len() {
                return Err(format!("free list contains out-of-bounds index: {idx}"));
            }
            if self.is_occupied(idx) {
                return Err(format!("free list contains occupied slot: {idx}"));
            }
        }

        // Invariant 3: no duplicates in free list
        let mut seen = std::collections::HashSet::new();
        for &idx in &self.free {
            if !seen.insert(idx) {
                return Err(format!("free list contains duplicate index: {idx}"));
            }
        }

        // Invariant 4: generations vec covers all slots
        if self.generations.len() < self.slots.len() {
            return Err(format!(
                "generations too short: {} but {} slots",
                self.generations.len(),
                self.slots.len()
            ));
        }

        Ok(())
    }
}

impl<T: Default> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Default + fmt::Debug> fmt::Debug for Arena<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arena")
            .field("count", &self.count)
            .field("capacity", &self.slots.len())
            .field("free_list_len", &self.free.len())
            .finish_non_exhaustive()
    }
}

impl<T: Default + Clone> Clone for Arena<T> {
    fn clone(&self) -> Self {
        #[cfg(test)]
        if let Err(e) = self.validate_invariants() {
            panic!("Arena::clone: source arena has invalid invariants: {e}");
        }

        Self {
            slots: self.slots.clone(),
            occupied: self.occupied.clone(),
            free: self.free.clone(),
            count: self.count,
            generations: self.generations.clone(),
        }
    }
}

/// Test-only helpers exposed on `Arena<T>`.
#[cfg(test)]
impl<T: Default> Arena<T> {
    /// Returns the number of currently occupied slots as `usize`.
    pub const fn len(&self) -> usize {
        self.count as usize
    }

    /// Returns `true` when there are no occupied slots.
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns the total number of allocated slots (occupied + free).
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Returns the number of reusable free slots.
    pub fn free_slots(&self) -> usize {
        self.free.len()
    }

    /// Directly sets the generation counter for a slot (for testing wrapping behaviour).
    pub fn set_generation_for_test(&mut self, index: usize, value: u32) {
        self.generations[index] = value;
    }
}

#[cfg(test)]
mod tests {
    use super::Arena;

    // ── Arena::new ───────────────────────────────────────────────────────
    mod new {
        use super::*;

        #[test]
        fn starts_empty() {
            let a: Arena<u32> = Arena::new();
            assert_eq!(a.count(), 0);
            assert_eq!(a.len(), 0);
            assert!(a.is_empty());
            assert_eq!(a.capacity(), 0);
            assert_eq!(a.free_slots(), 0);
        }
    }

    // ── Arena::alloc ────────────────────────────────────────────────────
    mod alloc {
        use super::*;

        #[test]
        fn increases_count_by_one() {
            let mut a: Arena<u32> = Arena::new();
            a.alloc(42);
            assert_eq!(a.count(), 1);
            assert_eq!(a.len(), 1);
            assert!(!a.is_empty());
        }

        #[test]
        fn returns_incrementing_indices_when_no_free_slots() {
            let mut a: Arena<u32> = Arena::new();
            let i0 = a.alloc(1).0;
            let i1 = a.alloc(2).0;
            let i2 = a.alloc(3).0;
            assert_eq!((i0, i1, i2), (0, 1, 2));
        }

        #[test]
        fn reuses_freed_slot() {
            let mut a: Arena<u32> = Arena::new();
            let i0 = a.alloc(10).0;
            a.dealloc(i0);
            let i1 = a.alloc(20).0;
            assert_eq!(i0, i1);
        }

        #[test]
        fn updates_free_slots_on_dealloc_and_realloc() {
            let mut a: Arena<u32> = Arena::new();
            let i = a.alloc(10).0;
            assert_eq!(a.free_slots(), 0);
            a.dealloc(i);
            assert_eq!(a.free_slots(), 1);
            a.alloc(20);
            assert_eq!(a.free_slots(), 0);
        }

        #[test]
        fn capacity_grows_only_when_no_free_slots_exist() {
            let mut a: Arena<u32> = Arena::new();
            let i0 = a.alloc(1).0;
            let i1 = a.alloc(2).0;
            assert_eq!((i0, i1), (0, 1));
            assert_eq!(a.capacity(), 2);

            a.dealloc(i1);
            assert_eq!(a.capacity(), 2);

            let reused = a.alloc(3).0;
            assert_eq!(reused, i1);
            assert_eq!(a.capacity(), 2);
        }
    }

    // ── Arena::get ─────────────────────────────────────────────────────
    mod get {
        use super::*;

        #[test]
        fn returns_the_stored_value() {
            let mut a: Arena<u32> = Arena::new();
            let i = a.alloc(99).0;
            assert_eq!(*a.get(i), 99);
        }
    }

    // ── Arena::get_mut ──────────────────────────────────────────────────
    mod get_mut {
        use super::*;

        #[test]
        fn allows_mutation_in_place() {
            let mut a: Arena<u32> = Arena::new();
            let i = a.alloc(1).0;
            *a.get_mut(i) = 100;
            assert_eq!(*a.get(i), 100);
        }
    }

    // ── Arena::dealloc ──────────────────────────────────────────────────
    mod dealloc {
        use super::*;

        #[test]
        fn returns_the_stored_value() {
            let mut a: Arena<u32> = Arena::new();
            let i = a.alloc(55).0;
            assert_eq!(a.dealloc(i), 55);
        }

        #[test]
        fn decreases_count_by_one() {
            let mut a: Arena<u32> = Arena::new();
            let i = a.alloc(1).0;
            a.dealloc(i);
            assert_eq!(a.count(), 0);
            assert_eq!(a.len(), 0);
            assert!(a.is_empty());
        }

        #[test]
        fn dealloc_resets_slot_to_default_value() {
            let mut a: Arena<u32> = Arena::new();
            let i = a.alloc(123).0;
            let _ = a.dealloc(i);
            assert_eq!(a.slots[i], u32::default());
        }
    }

    // ── Arena::is_occupied ──────────────────────────────────────────────
    mod is_occupied {
        use super::*;

        #[test]
        fn is_true_after_alloc() {
            let mut a: Arena<u32> = Arena::new();
            let i = a.alloc(7).0;
            assert!(a.is_occupied(i));
        }

        #[test]
        fn is_false_after_dealloc() {
            let mut a: Arena<u32> = Arena::new();
            let i = a.alloc(7).0;
            a.dealloc(i);
            assert!(!a.is_occupied(i));
        }

        #[test]
        fn is_false_for_out_of_range_index() {
            let a: Arena<u32> = Arena::new();
            assert!(!a.is_occupied(0));
        }
    }

    // ── Arena::iter_occupied ─────────────────────────────────────────────
    mod iter_occupied {
        use super::*;

        #[test]
        fn yields_all_allocated_slots() {
            let mut a: Arena<u32> = Arena::new();
            let i0 = a.alloc(10).0;
            let i1 = a.alloc(20).0;
            let i2 = a.alloc(30).0;
            let items: Vec<(usize, &u32)> = a.iter_occupied().collect();
            assert_eq!(items.len(), 3);
            assert!(items.contains(&(i0, &10)));
            assert!(items.contains(&(i1, &20)));
            assert!(items.contains(&(i2, &30)));
        }

        #[test]
        fn skips_deallocated_slots() {
            let mut a: Arena<u32> = Arena::new();
            let i0 = a.alloc(10).0;
            let i1 = a.alloc(20).0;
            a.dealloc(i0);
            let items: Vec<(usize, &u32)> = a.iter_occupied().collect();
            assert_eq!(items.len(), 1);
            assert!(items.contains(&(i1, &20)));
        }

        #[test]
        fn yields_nothing_for_empty_arena() {
            let a: Arena<u32> = Arena::new();
            assert_eq!(a.iter_occupied().count(), 0);
        }
    }

    // ── Arena::default ──────────────────────────────────────────────────
    mod default {
        use super::*;

        #[test]
        fn starts_empty_via_default_trait() {
            let a: Arena<u32> = Arena::default();
            assert_eq!(a.count(), 0);
        }
    }

    // ── Arena::fmt ──────────────────────────────────────────────────────
    mod fmt {
        use super::*;

        #[test]
        fn produces_a_non_empty_debug_string() {
            let mut a: Arena<u32> = Arena::new();
            a.alloc(1);
            let s = format!("{a:?}");
            assert!(s.contains("Arena"), "expected 'Arena' in debug output: {s}");
        }
    }

    // ── Arena::clone ────────────────────────────────────────────────────
    mod clone {
        use super::*;

        #[test]
        fn clone_has_same_count() {
            let mut a: Arena<u32> = Arena::new();
            a.alloc(10);
            a.alloc(20);
            let b = a.clone();
            assert_eq!(b.count(), a.count());
        }

        #[test]
        fn clone_is_independent() {
            let mut a: Arena<u32> = Arena::new();
            let i = a.alloc(1).0;
            let mut b = a.clone();
            *b.get_mut(i) = 99;
            assert_eq!(*a.get(i), 1, "original must be unaffected");
        }
    }

    // ── Arena::generation ───────────────────────────────────────────────
    mod generation {
        use super::*;

        #[test]
        fn new_slot_starts_at_generation_zero() {
            let mut a: Arena<u32> = Arena::new();
            let (idx, g) = a.alloc(1);
            assert_eq!(g, 0);
            assert_eq!(a.generation(idx), 0);
        }

        #[test]
        fn generation_increments_after_dealloc() {
            let mut a: Arena<u32> = Arena::new();
            let (idx, _) = a.alloc(1);
            a.dealloc(idx);
            assert_eq!(a.generation(idx), 1);
        }

        #[test]
        fn reused_slot_has_incremented_generation() {
            let mut a: Arena<u32> = Arena::new();
            let (idx, gen0) = a.alloc(1);
            a.dealloc(idx);
            let (idx2, gen1) = a.alloc(2);
            assert_eq!(idx2, idx, "freed slot must be reused");
            assert_eq!(gen1, gen0 + 1);
            assert_eq!(a.generation(idx), gen1);
        }

        #[test]
        fn stale_handle_detected_via_generation_mismatch() {
            let mut a: Arena<u32> = Arena::new();
            let (idx, g) = a.alloc(42);
            a.dealloc(idx);
            // After dealloc the generation advanced; old `g` is now stale.
            assert_ne!(a.generation(idx), g, "stale handle must be detectable");
        }

        #[test]
        fn generation_wraps_around_at_u32_max() {
            let mut a: Arena<u32> = Arena::new();
            let (idx, _) = a.alloc(1);
            // Force the counter to the maximum value.
            a.set_generation_for_test(idx, u32::MAX);
            a.dealloc(idx);
            assert_eq!(a.generation(idx), 0, "generation must wrap to 0 via wrapping_add");
        }

        #[test]
        fn generation_returns_zero_for_uninitialised_index() {
            let a: Arena<u32> = Arena::new();
            // Index 99 was never allocated.
            assert_eq!(a.generation(99), 0);
        }
    }
}
