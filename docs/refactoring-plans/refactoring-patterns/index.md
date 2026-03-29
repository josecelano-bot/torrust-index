# Refactoring Patterns – Complete Documentation Index

This directory (`docs/refactoring-plans/refactoring-patterns/`) contains a comprehensive guide for reducing parameter complexity in the torrust-index algorithm codebase.

> 🔗 **Part of unified refactoring initiative.** Start here: [../index.md](../index.md)
> 
> **When to use patterns:** Read [../INTEGRATION.md](../INTEGRATION.md) — patterns apply ONLY after Accidental Complexity Phase 1 is complete.

---

## 📚 Document Overview

### Quick Navigation

| Document | Purpose | Audience | Time to Read |
|----------|---------|----------|--------------|
| [quick-reference.md](refactoring-patterns-quick-reference.md) | 1-page overview, prioritization matrix | Everyone | 5 min |
| [analysis.md](refactoring-patterns-analysis.md) | Deep technical analysis with Rust examples | Engineers | 30 min |
| [execution-checklist.md](refactoring-patterns-execution-checklist.md) | Step-by-step implementation for each pattern | Refactoring lead | 45 min |
| [metrics.md](refactoring-patterns-metrics.md) | Before/after measurement & validation | QA/Devops | 20 min |
| [roadmap.md](refactoring-patterns-roadmap.md) | 6-phase implementation timeline | Project manager | 30 min |
| [function-signature-inventory.md](function-signature-inventory.md) | API surface analysis + refactoring opportunities | Everyone | 15 min |

---

## 🎯 Key Patterns (At-a-Glance)

### Pattern 1: Tell-Don't-Ask (Ownership)
**Problem:** Free functions take `&mut GvGraph` → tight coupling
**Solution:** Move to `GvGraph` methods
**Impact:** 2 functions, ~3 call sites each
**Effort:** 2 days

### Pattern 2: Escalation Context (Parameter Grouping)
**Problem:** Escalation functions have 5-7 parameters across 3 functions
**Solution:** Create `EscalationContext` struct to group related parameters
**Impact:** Reduce average from 6 params → 3 params per function (-50%)
**Effort:** 4 days + testing

### Pattern 3: Coordinate Range (Type Safety)
**Problem:** `(query_lo, query_hi)` pairs scattered across 40+ call sites
**Solution:** Create `CoordinateRange` type
**Impact:** Type safety + eliminate invalid states (-100% lo/hi pair bugs)
**Effort:** 3 days (mostly refactoring call sites)

### Pattern 4: Violation Queue (Mutation Centralization)
**Problem:** 15+ functions take `&mut Vec<VNodeId>` parameter
**Solution:** Create `ViolationQueue` struct centralizing all mutation
**Impact:** Reduce 4 params → 2 params, clearer control flow
**Effort:** 5 days + integration testing

---

## 🚀 Getting Started

### For Decision Makers: Read in This Order
1. [quick-reference.md](refactoring-patterns-quick-reference.md) (5 min)
2. [roadmap.md](refactoring-patterns-roadmap.md#phase-overview) sections (10 min)
3. [function-signature-inventory.md](function-signature-inventory.md) sections (5 min)
3. Prioritization matrix in quick-reference (2 min)

**Decision:** Which patterns to implement? Check roadmap phases.

### For Engineers Implementing: Read in This Order
1. [quick-reference.md](refactoring-patterns-quick-reference.md) (5 min) – Context
2. [analysis.md](refactoring-patterns-analysis.md) (30 min) – Deep dive on chosen pattern
3. [execution-checklist.md](refactoring-patterns-execution-checklist.md) (15 min) – Your specific pattern
4. [roadmap.md](refactoring-patterns-roadmap.md#phase-n) (10 min) – Your phase details
5. [metrics.md](refactoring-patterns-metrics.md) (5 min) – Success validation
6. [function-signature-inventory.md](function-signature-inventory.md) – Identify opportunities

**Action:** Pick one pattern → follow execution checklist → validate with metrics.

### For QA/Testing: Read This
1. [metrics.md](refactoring-patterns-metrics.md) (20 min)
2. [execution-checklist.md](refactoring-patterns-execution-checklist.md#validation) (15 min)

**Task:** Write baseline measurements, execute validation steps after refactoring.

---

## 📊 Recommended Reading Path

```
Start Here
    ↓
Quick Reference (5 min)
    ↓
Consult: Function Signature Inventory
    ↓
Choose: You are...
    ├─ Making go/no-go decision → Roadmap Overview
    ├─ Implementing right now → Analysis + Checklist + Inventory
    ├─ Testing afterward → Metrics
    └─ Managing the project → Roadmap (all phases)
    ↓
Deep Dive Document (15-45 min)
    ↓
Action Items + Checklist
    ↓
Measure + Validate
    ↓
Complete!
```

---

## 🔍 Target Hotspots & Priority

### Priority 1: Highest Impact (Start Here)
**Pattern 2: Escalation Context**
- Current: 3 functions × (5-7 params) = 18 total parameter slots
- Target: 3 functions × 3 params = 9 total parameter slots
- ROI: -50% parameter count
- Effort: 4 days
- Risk: Medium (central to rebalance logic)

### Priority 2: Type Safety Wins (Next)
**Pattern 3: Coordinate Range**
- Current: 40+ call sites with lo/hi pair logic
- Target: 1 type enforces invariants
- ROI: Eliminate invalid state bugs
- Effort: 3 days
- Risk: Low (mostly search-replace)

### Priority 3: Control Flow Clarity (Then)
**Pattern 4: Violation Queue**
- Current: 15+ functions each taking violations vector
- Target: Centralized queue state
- ROI: -40% parameter count across affected functions
- Effort: 5 days
- Risk: Medium (affects mutation patterns)

### Priority 4: Clean Architecture (Last)
**Pattern 1: Tell-Don't-Ask**
- Current: 2 free functions taking &mut graphs
- Target: Methods on owning structs
- ROI: Architectural improvement
- Effort: 2 days
- Risk: Low (few call sites)

---

## 📈 Expected Improvements

### Before Starting:
```
Average function params (hotspots): 4.2
Function params range: 1-7
Tell-Don't-Ask violations: 2
Parameter pairs (lo/hi): 40+ occurrences
Total complexity score (approx): 8.5/10
```

### After All Patterns:
```
Average function params (hotspots): 2.5-2.8 ✓ (-40%)
Function params range: 1-4 ✓
Tell-Don't-Ask violations: 0 ✓
Parameter pairs: 0 (~eliminated) ✓
Total complexity score (approx): 5.0/10 ✓ (-40%)
```

### Validation:
- ✅ All tests pass (same suite)
- ✅ Coverage maintained > 95%
- ✅ Performance: ±2% vs. baseline
- ✅ Build time: ±5% vs. baseline

---

## 🛠️ Implementation Timeline

### Week 1-2: Foundation
- Test baseline
- Create branches
- Map current violations

### Week 2-3: Phase 1 - Tell-Don't-Ask
- bootstrap_split → method
- catalytic_split → method
- 3 tests, validation

### Week 3-4: Phase 2 - Escalation Context
- Create EscalationContext struct
- Refactor 3 escalation functions
- Integration testing

### Week 4-5: Phase 3 - Coordinate Range
- Create CoordinateRange type
- Update decompose_basis, range_sum_inner
- 40+ call sites

### Week 5-6: Phase 4 - Violation Queue
- Create ViolationQueue
- Consolidate 15+ functions
- Mutation pattern testing

### Week 6-7: Integration & Cleanup
- E2E tests
- Remove deprecated code
- Performance validation

**Total: 6-7 weeks** (can be parallelized)

---

## ✅ Checklist Before You Start

- [ ] Read: Quick Reference (5 min)
- [ ] Read: Analysis for your chosen pattern (30 min)
- [ ] Understand: Each pattern's goal & trade-offs
- [ ] Have: ~6-7 weeks available (or phase separately)
- [ ] Setup: Git branch practice, testing discipline
- [ ] Team: Code review process established
- [ ] Tooling: `cargo test`, `cargo bench`, metrics collection ready

---

## 🚨 Red Flags (Stop & Re-Plan)

❌ Cyclomatic Complexity increases by >10%
❌ Test coverage drops below 90%
❌ New compiler warnings appear
❌ Performance regression > 5%
❌ Parameter counts increase in any hotspot

**If any red flag:** Pause refactoring, review approach, adjust plan.

---

## 📝 Document Guides

### 0. Function Signature Inventory
**Use this to:**
- Get a complete listing of all module, type, and function signatures
- Identify API surface complexity and opportunities
- Find functions with suspicious signatures (too many params, unclear ownership)
- Spot candidates for pattern application
- Track which functions are best targets for optimization

**Length:** ~20 pages (reference manual)

**When to use:** Before starting refactoring to understand what you're working with.

### 1. Quick Reference
**Use this to:**
- Understand what each pattern does in 1 minute
- See the prioritization matrix
- Get code snippets showing before/after
- Decide which patterns to implement

**Length:** ~4 pages (quick read)

### 2. Analysis Document
**Use this to:**
- Understand why each pattern works
- See detailed Rust code examples
- Understand trade-offs & constraints
- Learn the underlying principles

**Length:** ~20 pages (deep read)

**Bonus:** Includes a section on "Why These Patterns Work" with software engineering theory.

### 3. Execution Checklist
**Use this to:**
- Follow step-by-step for your specific pattern
- Verify you didn't miss anything
- Know exactly what code to write
- Have validation steps ready

**Length:** ~30 pages (reference manual)

**Bonus:** Includes code templates for most patterns.

### 4. Metrics & Measurement
**Use this to:**
- Establish baseline before refactoring
- Know what to measure after each phase
- Detect regressions early
- Prove ROI of refactoring

**Length:** ~15 pages (measurement guide)

**Bonus:** Includes bash scripts and Python tools for automation.

### 5. Roadmap
**Use this to:**
- Understand 6-phase implementation
- See dependencies between phases
- Manage timeline & risk
- Plan rollback strategy

**Length:** ~25 pages (project plan)

**Bonus:** Includes success metrics, risk mitigation, and ongoing maintenance guide.

---

## 🔗 Files Location & Navigation

All documents are in `docs/refactoring-plans/` with two main sections:

```
docs/refactoring-plans/
├── accidental-complexity-plan/         ← Architectural foundation (phases 1-4)
│   ├── index.md                        ← Start here for overall strategy
│   ├── phase-1-api-normalization.md
│   ├── phase-2-abstractions.md
│   ├── phase-3-reorganization.md
│   └── phase-4-stabilization.md
│
└── refactoring-patterns/               ← Tactical optimization patterns
    ├── index.md                        ← Navigation (this file)
    ├── function-signature-inventory.md ← API surface analysis (reference)
    ├── refactoring-patterns-quick-reference.md
    ├── refactoring-patterns-analysis.md
    ├── refactoring-patterns-execution-checklist.md
    ├── refactoring-patterns-metrics.md
    └── refactoring-patterns-roadmap.md
```

**Strategy:** Follow Accidental Complexity Plan first (architectural decisions), then apply Refactoring Patterns within those boundaries (tactical optimization).

---

## 💡 Pro Tips

### Tip 1: Start With Quick Reference
Don't jump to analysis—quick reference gives you framework first.

### Tip 2: One Pattern Per Branch
Each pattern = separate git branch. Easier to review, test, rollback.

### Tip 3: Measure Before & After
Document metrics before starting, measure after each phase.

### Tip 4: Run Tests Constantly
After every change, run full test suite. No surprises.

### Tip 5: Keep Deprecated Wrappers
Until you're confident, keep old function as wrapper to new method. Easier fallback.

### Tip 6: Code Review Each Phase
Each pattern should have code review + testing before merge.

---

## ❓ FAQ

**Q: Do I need to implement all 4 patterns?**
A: No! Pick the 1-2 with highest ROI for your codebase. Patterns are independent (mostly).

**Q: Can I parallelize these?**
A: Partially. Patterns 1 & 3 are independent. Pattern 2 & 4 have some integration. See roadmap.

**Q: How much time do I need?**
A: Per pattern: 2-5 days. All 4: 6-7 weeks. Can be spread across multiple sprints.

**Q: What if performance regresses?**
A: First: profile. Context types are usually zero-cost when optimized. If not: reconsider pattern approach.

**Q: How do I know if I did it right?**
A: Check metrics document. Parameter count reduced, tests pass, performance maintained.

**Q: What if tests break?**
A: Pause. Review your changes. The logic should be identical—only organization changed.

---

## 🎓 Learning Resources (References)

### Rust Best Practices Cited:
- "Tell Don't Ask" principle from Object-Oriented Design
- "Parameter Object" pattern from Refactoring by Martin Fowler
- Rust's ownership model enables safe mutation patterns
- Type-driven development (making invalid states unrepresentable)

### Further Reading:
- Rust Book: Ownership & Borrowing
- Design Patterns in Rust community resources
- Martin Fowler's Refactoring (book)

---

## 🎯 One More Thing

These patterns aren't just about code cleanliness—they're about **making the intent of the code clearer**:

- **Tell-Don't-Ask:** "This operation belongs to GvGraph"
- **Context:** "These 5 things are always used together"
- **CoordinateRange:** "This is a valid coordinate range, not arbitrary pair"
- **ViolationQueue:** "Violations are collected, then processed"

When the code is clearer, it's easier to maintain, test, and extend.

---

## 📞 Questions?

Refer to the specific sub-document:
- **"What do I implement first?"** → Quick Reference + Roadmap
- **"How do I implement X?"** → Analysis + Execution Checklist
- **"Did I succeed?"** → Metrics document  
- **"How long will this take?"** → Roadmap

---

**Last updated:** 2025-01-10
**Status:** Ready for Implementation
**Recommended Start:** Quick Reference (5 min read)

