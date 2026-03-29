# Integration Roadmap: When & How to Apply Patterns

Tactical guide for deciding when refactoring patterns should be applied within the accidental complexity phases.

---

## 🎯 Core Principle

**Architecture First, Optimization Second**

1. **Accidental Complexity Plan (Phases 1-5)** – Establishes correct ownership & structure
2. **Refactoring Patterns (Selective)** – Optimizes parameters *within* that structure
3. **Never optimize before the architecture decision is made**

---

## 📊 Decision Tree

```
START: Ready to begin refactoring?
  ├─ NO → Read PREREQUISITES.md
  └─ YES ↓

PHASE 0: Prerequisites & Baseline Complete?
  ├─ NO → Complete Phase 0
  └─ YES ↓

PHASE 1: API Normalization Complete?
  ├─ NO → Execute Phase 1
  │       (convert free functions to methods)
  │
  └─ YES ↓
             Ready for DECISION POINT #1 ↓
             
DECISION POINT #1: Apply Parameter Patterns?
  │
  ├─ Question: "Do we need parameter optimization?"
  ├─ Measure: Avg params > 3.5 in key functions?
  ├─ Measure: Cognitive complexity too high?
  │
  ├─ YES (ROI > 30%) ↓
  │   Apply Refactoring Patterns (Phase 3):
  │   ├─ Pattern 1: Tell-Don't-Ask (already done in Phase 1!)
  │   └─ Choose up to 2 patterns:
  │       ├─ Pattern 2: Escalation Context (if 5+ param functions exist)
  │       ├─ Pattern 3: Coordinate Range (if 40+ lo/hi pairs exist)
  │       └─ Pattern 4: Violation Queue (if 15+ mut Vec functions exist)
  │   
  │   → Execute patterns with full test gates
  │   → Return to PHASE 2
  │
  └─ NO (ROI ≤ 30%) ↓
      Skip patterns, proceed to Phase 2
      
PHASE 2: Abstractions Complete?
  ├─ NO → Execute Phase 2
  └─ YES ↓

PHASE 4: Reorganization Complete?
  ├─ NO → Execute Phase 4
  └─ YES ↓

PHASE 5: Stabilization Complete?
  ├─ NO → Execute Phase 5
  └─ YES → ✅ DONE!
```

---

## 💡 Decision Point #1: Should We Apply Patterns?

### When PHASE 1 (API Normalization) is Complete

**Before Phase 2 (Abstractions), evaluate:**

#### Measure: Function Parameter Complexity
```bash
# In src/graph/algorithm/:
cargo expand --lib 2>/dev/null | grep -E "fn \w+\([^)]{60,}" | wc -l
# Result: How many functions have 50+ chars of parameters?
# Target: 0 (ideally)
```

#### Measure: Coordination Parameters
```bash
# Count (lo, hi) pairs still scattered:
grep -r "query_lo.*query_hi\|_lo.*_hi" src/graph/algorithm/ | wc -l
# Result: ~40+ means Pattern 3 has ROI
# Result: < 5 means skip Pattern 3
```

#### Measure: Mutation Centralization
```bash
# Count free functions still taking &mut Vec:
grep -r "fn.*&mut Vec" src/graph/algorithm/ | wc -l
# Result: 15+ means Pattern 4 has ROI
# Result: < 5 means skip Pattern 4
```

#### Measure: Context Grouping
```bash
# Check escalation functions:
grep -r "fn.*escalate.*(" src/graph/algorithm/rebalance/ | wc -l
# If all have 5+ params → Pattern 2 has ROI
# If all reduced to 3- params → skip Pattern 2
```

### Decision Criteria

| Scenario | Action |
|----------|--------|
| After Phase 1, avg params **unchanged** (5+) | ✅ Apply patterns (ROI high) |
| After Phase 1, avg params **reduced** to 3-4 | ⚠️ Measure ROI before deciding |
| After Phase 1, avg params **already** < 3 | ❌ Skip patterns (ROI low) |
| Phase 1 left 40+ `lo/hi` pairs | ✅ Apply Pattern 3 alone |
| Phase 1 left 15+ `&mut Vec` functions | ✅ Apply Pattern 4 alone |
| Phase 1 resolved all issues | ❌ Skip to Phase 2 |

---

## 🔄 Pattern Application Within Phase 1

### Special Case: Tell-Don't-Ask (Pattern 1)

**Pattern 1 = Core of Phase 1 API Normalization**

When you execute **P1.2 (Split helper method conversion):**
- You are literally implementing Pattern 1
- This is mandatory, not optional
- `bootstrap_split` → `GvGraph::bootstrap_split()` ✓

**No separate decision needed** – it's part of Phase 1.

---

## 🔄 Pattern Application Between Phases

### Recommended Sequence If Patterns Chosen

```
Phase 1 Complete ✓
  ↓
DECISION POINT #1: Measure & Decide
  ↓
Phase 2: Abstractions (Run parallel to patterns if time permits)
  ↓
During Phase 2 + Pattern Application (Optional):
  
If choosing patterns:
  ├─ Pattern 2: Escalation Context (1-2 days, low risk)
  ├─ Pattern 3: Coordinate Range (1-2 days, low risk)
  └─ Pattern 4: Violation Queue (2-3 days, medium risk)
  
Phase 2 resumes after patterns complete
  ↓
Phase 4: Reorganization
  ↓
Phase 5: Stabilization
```

---

## ⚠️ When NOT to Apply Patterns

### Red Flags: Skip Patterns Entirely

1. **Architecture Not Clear** – If Phase 1 left questions, resolve first
2. **Test Coverage < 90%** – Patterns need high confidence
3. **Time Pressure** – Patterns are nice-to-have, not critical path
4. **Fundamental Issues Found** – If Phase 1 revealed design problems, address those first
5. **ROI Unclear** – Measure first, decide second

### When to Defer Patterns

- Team capacity is low
- Other priorities emerged
- Test suite flaky after Phase 1
- Documentation needed before refactoring

→ Move patterns to "Phase 3b: Future Optimization" and note in PROGRESS.md

---

## 🎯 Pattern ROI Calculation

For each pattern, measure **Return on Investment**:

```
ROI = (Benefit / Effort) × 100

Pattern 2 (Escalation Context):
  Benefit: Reduce 18 param slots → 9 (-50%)
  Effort: 2-3 days
  ROI = (50% / 3 days) = 16.7% per day ✅ HIGH

Pattern 3 (Coordinate Range):
  Benefit: Eliminate 40 lo/hi pair bugs
  Effort: 1-2 days
  ROI = (Type Safety + consistency / 2) = HIGH ✅

Pattern 4 (Violation Queue):
  Benefit: Reduce 15+ functions from 4→2 params
  Effort: 3-4 days  
  ROI = (30% / 4 days) = 7.5% per day ⚠️ MEDIUM

Threshold: ROI > 10% per day = Recommended ✓
```

---

## 📋 Phase-by-Phase Integration

### After Phase 1 ✓

**Checkpoint:** API boundaries clear?
- ✓ YES → Continue to DECISION POINT #1 (evaluate patterns)
- ✗ NO → Fix Phase 1 issues first

**If patterns chosen:**
- ✓ Update PROGRESS.md with "Pattern Phase: Weeks 5-6"
- ✓ Add pattern sub-tasks to Phase 2 checklist
- ✓ Run full test suite after each pattern

---

### After Phase 2 ✓ (or pattern application)

**Checkpoint:** Abstractions in place?
- ✓ YES → Continue to Phase 4
- ✗ NO → Review Phase 2 for incomplete work

---

### After Phase 4 ✓

**Checkpoint:** Modules reorganized?
- ✓ YES → Continue to Phase 5 (final stabilization)
- ✗ NO → Address reorganization issues

---

### After Phase 5 ✓

**DONE!** All refactoring complete. Patterns are optional bonus, not required.

---

## 🚨 Stop & Escalate Conditions

If **any** of these occur during pattern application:

1. **Test Failure** that persists > 2 hours
   → Stop pattern, revert changes, document issue
   → Decide: Fix issue or skip pattern?

2. **Performance Regression** > 5%
   → Stop pattern, investigate
   → If unfixable: Skip pattern, note decision

3. **Merge Conflict** that's hard to resolve
   → Pause, sync with Phase 2 work
   → Resume after conflict clear

4. **Architecture Inconsistency** discovered
   → Stop pattern immediately
   → Return to phases 1-2 to fix foundation

→ See [ROLLBACK.md](ROLLBACK.md) for full recovery procedures

---

## 📝 Decision Log Template

**Keep track of why you chose/skipped patterns:**

```markdown
## DECISION POINT #1: Pattern Evaluation
**Date:** 2026-04-09 (hypothetical Phase 1 end)
**Evaluator:** [NAME]

### Measurements
- Avg params in hotspots: 4.2 (unchanged from Phase 1)
- lo/hi pair occurrences: 38 (still significant)
- &mut Vec functions: 12 (some resolved in Phase 1)
- CC avg: 7.2 (slightly improved)

### Decision
✅ Apply Patterns 2 & 3 (high ROI measured)
❌ Skip Pattern 4 (ROI too low, resolved by architecture changes)

### Rationale
- Pattern 2 reduces params by 50% → high value
- Pattern 3 eliminates data-invalid states → safety benefit  
- Pattern 4 redundant after Phase 1 & 2 → skip

### Implementation Plan
- Week 5: Apply Pattern 2 (Escalation Context)
- Week 5-6: Apply Pattern 3 (Coordinate Range)
- Week 6-7: Integration testing
```

---

## 🔗 Reference

- [PREREQUISITES.md](PREREQUISITES.md) – Blockers & sequencing
- [ROLLBACK.md](ROLLBACK.md) – Stop conditions & recovery
- [PROGRESS.md](PROGRESS.md) – Master tracker
- [accidental-complexity-plan/](accidental-complexity-plan/) – Phases 1-5
- [refactoring-patterns/](refactoring-patterns/) – Detailed pattern docs

---

**TL;DR:**
1. Complete Phase 1 (API Normalization) ✓
2. Measure parameter complexity
3. If improvement needed & ROI > 30%: Apply patterns
4. Otherwise: Proceed to Phase 2
5. Patterns are optional tactical optimization within architecture foundation

