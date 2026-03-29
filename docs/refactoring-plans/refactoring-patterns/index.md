# Refactoring Patterns – Documentation Index (Current State)

This directory tracks pattern-driven complexity reduction in the algorithm layer.

This page is intentionally status-first: it reflects what is already done in code and what is still worth implementing.

## Status Snapshot

| Pattern | Status | Evidence in Code | Notes |
|---|---|---|---|
| Pattern 1: Tell-Don't-Ask for split operations | Completed | `src/graph/algorithm/split.rs` | `bootstrap_split` and `catalytic_split` are methods on `GvGraph` |
| Pattern 2: Escalation context grouping | Completed | `src/graph/algorithm/rebalance/context.rs`, `src/graph/algorithm/rebalance/resolve.rs` | `EscalationContext` and `VTreeMutContext` in active use |
| Pattern 3: Coordinate range type | Completed | `src/spatial/range.rs`, `src/graph/algorithm/query/range_sum.rs`, `src/graph/algorithm/query/contour.rs` | `CoordinateRange<C>` replaced raw lo/hi pairs in query path |
| Pattern 4: Violation queue ownership | In Progress | `src/graph/algorithm/violation_push.rs` | Remaining major gap: raw `&mut Vec<VNodeId>` still passed widely |
| Pattern 5: Plateau context flattening | Deferred | `src/graph/algorithm/plateau/dynamic_tracker/` | Recent helper-phase decomposition reduced complexity; broader signature redesign postponed |

## What Changed Since Original Plan

- The original docs assumed a greenfield 6-7 week rollout.
- In reality, Patterns 1-3 are already integrated.
- Current planning should focus on finishing Pattern 4 safely and treating Pattern 5 as optional/ROI-based.

## Reading Order

1. [refactoring-patterns-quick-reference.md](refactoring-patterns-quick-reference.md)
2. [refactoring-patterns-analysis.md](refactoring-patterns-analysis.md)
3. [refactoring-patterns-execution-checklist.md](refactoring-patterns-execution-checklist.md)
4. [refactoring-patterns-metrics.md](refactoring-patterns-metrics.md)
5. [refactoring-patterns-roadmap.md](refactoring-patterns-roadmap.md)
6. [function-signature-inventory.md](function-signature-inventory.md)

## Current Execution Focus

1. Complete Pattern 4 with a thin `ViolationQueue` wrapper while preserving behavior.
2. Migrate highest-churn call sites first (`rebalance/resolve.rs`, split helpers).
3. Keep free-function compatibility during migration; remove only after all call sites are converted.
4. Re-run full validation gates after each small slice.

## Validation Gates

- `cargo check --all-features`
- Targeted tests for touched area
- `cargo test --all-features`

## Red Flags

- Any behavior change in violation propagation ordering.
- Any increase in residual violations after rebalance.
- Any complexity regression in already-stabilized plateau helpers.

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

