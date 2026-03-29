# Prerequisites & Sequencing: What Must Happen First

Critical blockers and mandatory sequencing for the refactoring initiative.

---

## ⚠️ Absolute Blockers (Must Be True Before Starting)

### 1. ✅ All Tests Currently Pass
```bash
cargo test --lib --all
```
**Requirement:** Exit code 0, no flaky tests  
**Why:** We're changing code architecture. Baseline must be stable.  
**If failing:** Fix tests first, then start refactoring.

### 2. ✅ Test Coverage > 90%
```bash
cargo tarpaulin --lib --timeout 300
```
**Requirement:** Overall coverage >= 90%  
**Why:** Refactoring is risky. High coverage catches regressions.  
**If failing:** Add tests before refactoring, target 95%+.

### 3. ✅ Git History Clean
```bash
git status  # Should be: working tree clean
git log --oneline -5  # Recent commits should be stable
```
**Requirement:** No uncommitted changes, no unfinished branches  
**Why:** We'll create clean branches for each phase.  
**If failing:** Commit/stash everything, align with team.

### 4. ✅ Team Alignment on Plan
- [ ] All stakeholders read PREREQUISITES.md
- [ ] All stakeholders read INTEGRATION.md  
- [ ] Team agrees on timeline & effort
- [ ] Clear owner/responsible assigned per phase

**Why:** Refactoring requires discipline. Half-committed effort fails.  
**If blocked:** Schedule alignment meeting before starting.

### 5. ✅ Code Review Process Defined
```
Requirement: Before any PR merges, team must:
  ├─ Review complete (at least 1 approval)
  ├─ All tests passing (Gate A + B + C)
  ├─ No performance regression
  └─ Documentation updated
```

**Why:** Without code review, bad changes slip through.  
**If unclear:** Define & document before starting.

---

## 🚫 Mandatory Sequencing: Order Matters!

### ❌ WRONG: Starting Multiple Phases in Parallel

```
DON'T DO THIS:
Phase 1 (API Norm) + Phase 2 (Abstractions) at same time
↓
Merge conflicts, unclear ownership, tests confusing
↓
FAILURE
```

### ✅ RIGHT: Linear Sequencing

```
Phase 0 (Baseline) ✓
  ↓ [GATE C PASS]
Phase 1 (API Norm) ✓
  ↓ [GATE C PASS + DECISION POINT #1]
Phase 2 (Abstractions) ✓ [optional patterns in parallel]
  ↓ [GATE C PASS]
Phase 4 (Reorganization) ✓
  ↓ [GATE C PASS]
Phase 5 (Stabilization) ✓
  ↓
DONE!
```

---

## 🔗 Dependency Chain (Critical!)

### Dependency: Phase 1 MUST Complete Before Any Other Phase

```
Phase 0: Prerequisites
  └─ GATE C PASS ✓
    └─ Phase 1: API Normalization (CRITICAL)
         └─ GATE C PASS ✓ ← BLOCKER for everything below
           ├─ Phase 2: Abstractions
           ├─ Phase 4: Reorganization  
           └─ (Optional) Patterns: Parameter Optimization
                └─ Feeds back into Phase 2
```

**Why Phase 1 is critical:**
- Establishes clear ownership boundaries
- Makes free function decisions
- Enables other phases to build on firm foundation
- Cannot reorganize (Phase 4) until ownership is clear

**⚠️ You CANNOT skip or parallelize Phase 1.**

---

## 📋 Pre-Refactoring Checklist

Run this before starting Phase 0:

```bash
#!/bin/bash
echo "=== PRE-REFACTORING VERIFICATION ==="

# 1. Test baseline
echo "1. Running full test suite..."
cargo test --lib --all 2>&1 | tail -5
if [ ${PIPESTATUS[0]} -ne 0 ]; then
    echo "❌ BLOCKER: Tests failing. Fix before proceeding."
    exit 1
fi
echo "✅ Tests pass"

# 2. Code clarity
echo "2. Checking for uncommitted changes..."
if [ -n "$(git status --porcelain)" ]; then
    echo "❌ BLOCKER: Uncommitted changes. Commit/stash first."
    exit 1
fi
echo "✅ Clean working tree"

# 3. Coverage
echo "3. Estimating test coverage..."
echo "   (Run 'cargo tarpaulin --lib' separately if needed)"

# 4. Performance baseline
echo "4. Recording performance baseline..."
cargo bench --bench depth 2>&1 | grep "time:" | head -3

echo ""
echo "=== PRE-CHECKS COMPLETE ==="
echo "Ready to begin Phase 0: Prerequisites & Baseline"
```

**Run this before starting:**
```bash
chmod +x pre-refactoring-check.sh
./pre-refactoring-check.sh
```

---

## 🎯 Phase 0: Exact Prerequisites (Must Complete)

Before any refactoring begins, you must complete Phase 0:

### P0.1: Establish Metrics Baseline
```bash
# Test baseline
cargo test --lib --all > metrics/test-baseline.log

# Coverage baseline
cargo tarpaulin --lib --timeout 300 --out Html

# Performance baseline (if benchmarked)
cargo bench --bench depth > metrics/bench-baseline.log

# Code analysis baseline
cargo expand --lib 2>&1 | grep -E "fn " | wc -l > metrics/function-count.txt
```

**Deliverable:** `metrics/baseline-*.log` files in repo

### P0.2: Document Current State
```markdown
# docs/refactoring-plans/BASELINE.md (to create)

## Metrics as of 2026-03-29

- Test coverage: 92%
- Function count: 342
- Avg cyclomatic complexity: 7.2
- Functions with CC > 15: 8
- Performance (depth bench): X ms ± Y

## Blockers/Issues Found
- [List known issues that refactoring might expose]

## Team Alignment
- [Document who approved plan, timeline expectations]
```

### P0.3: Setup Git Structure
```bash
# Create main refactoring branch
git checkout -b refactor/master-2026-03

# Tag baseline
git tag baseline/2026-03-29

# Create phase branches (will use these as phases progress)
# git branch refactor/phase-1-api-norm
# git branch refactor/phase-2-abstractions
# etc.

# Document in PROGRESS.md
```

**Deliverable:** Main branch created, baseline tagged

### P0.4: Team Brief
```
Required: Before Phase 1 starts, all team members must:
  ☐ Read PREREQUISITES.md (this file)
  ☐ Read INTEGRATION.md (decision points)
  ☐ Read ROLLBACK.md (stop conditions)
  ☐ Understand Phase 1 goals: accidental-complexity-plan/phase-1-api-normalization.md
  ☐ Confirm availability & ownership assignments
  ☐ Ask questions in team meeting
```

**Deliverable:** All checks complete, team aligned

---

## ❌ Common Failure Modes (Avoid These!)

### Failure #1: Starting Without Baseline
```
❌ "Let's just start refactoring, we'll measure later"
  └─ Result: Can't prove you improved anything
  └─ Result: Regression goes undetected
  └─ Result: Wasted effort
```

**Prevention:** Complete Phase 0 checklist fully.

### Failure #2: Running Phases in Parallel
```
❌ "Let's do Phase 1 & 2 simultaneously, faster"
  └─ Result: Merge conflicts
  └─ Result: Both phases half-done
  └─ Result: Progress stalled 3 weeks
```

**Prevention:** Enforce linear sequence. Phase 1 must Gate C before Phase 2 starts.

### Failure #3: Skipping Phase 1
```
❌ "Phase 1 is boring ownership stuff, let's go straight to patterns"
  └─ Result: Patterns applied to wrong functions
  └─ Result: Optimization in wrong places
  └─ Result: Wasted effort on Phase 3
```

**Prevention:** Phase 1 is MANDATORY. Everything depends on it.

### Failure #4: Not Testing After Each Step
```
❌ "We'll test after we finish the whole phase"
  └─ Result: 20 commits accumulated before test failure found
  └─ Result: Can't identify which commit broke it
  └─ Result: Rollback loses 3 days of work
```

**Prevention:** Gate C after every P1.X, P2.X, etc. step.

### Failure #5: Wrong Owner Assignment
```
❌ "Assigned to engineer, but they're also on another project"
  └─ Result: Phase abandoned mid-way
  └─ Result: Branch becomes stale, hard to rebase
  └─ Result: After 2 weeks, doesn't merge cleanly
```

**Prevention:** Clear single owner per phase, no split assignments.

---

## 🟢 Green Lights Before Each Phase

### Before Phase 1 Starts

- ✅ Phase 0 complete (baseline established, team briefed)
- ✅ All tests passing (`Gate C` from Phase 0)
- ✅ Phase 1 owner clearly assigned
- ✅ Phase 1 branch created: `refactor/phase-1-api-norm`
- ✅ Team understands Phase 1 goals
- ✅ PROGRESS.md updated with owner & planned dates

### Before Phase 2 Starts

- ✅ Phase 1 complete (PR merged, `Gate C` passing)
- ✅ Phase 1 PR reviewed & approved
- ✅ DECISION POINT #1 evaluated (pattern decision made)
- ✅ Phase 2 owner assigned
- ✅ Phase 2 branch created: `refactor/phase-2-abstractions`

### Before Phase 4 Starts

- ✅ Phase 1 & 2 complete
- ✅ If patterns chosen: Pattern phase complete
- ✅ Phase 4 owner assigned
- ✅ Phase 4 owner reviewed Phase 1 & 2 outcomes

### Before Phase 5 Starts

- ✅ Phases 1, 2, 4 complete & merged
- ✅ All tests passing
- ✅ No regressions in metrics
- ✅ Phase 5 owner ready for final stabilization

---

## 🛑 Show-Stoppers (Abort If Any Happen)

If **any** of these occur during refactoring, STOP immediately and evaluate:

1. **Two consecutive `Gate C` failures** in same phase
   → Issue: Phase is ill-designed or regressions too deep
   → Action: STOP, document issue, decide: fix or pivot?

2. **Performance regression > 5%** and not fixable
   → Issue: Pattern or phase made things worse
   → Action: STOP, revert phase, pivot to different approach

3. **Test coverage drops below 85%**
   → Issue: Refactoring exposed lack of coverage
   → Action: STOP, add tests before continuing

4. **More than 2 weeks in one phase with no commits merged**
   → Issue: Phase is stuck, complexity grew
   → Action: STOP, replan phase with smaller steps

5. **Team consensus lost** (people disagree on approach)
   → Issue: Plan is misaligned
   → Action: STOP, regroup, decide on direction

→ **See [ROLLBACK.md](ROLLBACK.md) for recovery procedures**

---

## 📞 Decision: Go or No-Go?

### Before Starting Phase 0, Answer These:

1. **Do you have baseline tests passing?**
   - YES → ✅ OK
   - NO → ❌ DO NOT PROCEED – Fix tests first

2. **Is test coverage > 90%?**
   - YES → ✅ OK
   - NO → ⚠️ RISKY – Add tests, increase to 95%+

3. **Is the team aligned on the whole plan?**
   - YES → ✅ OK
   - PARTIAL → ❌ DO NOT PROCEED – Align team first
   - NO → ❌ DO NOT PROCEED – Get buy-in from stakeholders

4. **Do you have clear owners for each phase?**
   - YES → ✅ OK
   - NO → ❌ DO NOT PROCEED – Assign owners

5. **Can you commit to linear sequencing?**
   - YES → ✅ OK (no parallel phases)
   - NO → ⚠️ PLAN WILL FAIL – Restructure team/timeline

6. **Do you have 8-10 weeks available?**
   - YES → ✅ OK
   - LESS → ⚠️ REDUCE SCOPE – Skip patterns, do only Phases 1-2
   - NO → ❌ POSTPONE – Start later when capacity frees

---

## 🚀 Final Gate: Ready to Begin?

```
If ALL of these are TRUE, you're ready:

☐ Tests passing (Gate C)
☐ Coverage > 90%
☐ Git clean
☐ Team briefed & aligned
☐ Owners assigned per phase
☐ Code review process defined
☐ Phase 0 checklist prepared
☐ This doc read & understood by all
☐ INTEGRATION.md read & understood by all
☐ ROLLBACK.md read & understood by all

If ANY are FALSE: DO NOT START - Fix first.

If ALL are TRUE: BEGIN PHASE 0 ✅
```

---

## 📝 Checklist to Copy & Paste

```markdown
# Refactoring Initiative: Pre-Launch Checklist

- [ ] All tests passing (`cargo test --lib --all`)
- [ ] Coverage baseline measured (target: >90%)
- [ ] Git working tree clean
- [ ] Team fully read PREREQUISITES.md
- [ ] Team fully read INTEGRATION.md
- [ ] Team fully read ROLLBACK.md
- [ ] Phase 1 goals understood by all
- [ ] Owner assigned to Phase 0
- [ ] Owner assigned to Phase 1
- [ ] Code review process documented
- [ ] Timeline agreed (8-10 weeks)
- [ ] Main refactoring branch created
- [ ] Baseline tagged: baseline/2026-03-29
- [ ] Team meeting conducted: all questions answered
- [ ] PROGRESS.md initialized

SIGN-OFF:
- Technical Lead: _________________ Date: _______
- Project Manager: ________________ Date: _______
- Team Representative: ____________ Date: _______

GO/NO-GO: _________ (READY or NOT READY)
```

---

## 🔗 Reference

- [PROGRESS.md](PROGRESS.md) – Master tracker
- [INTEGRATION.md](INTEGRATION.md) – Decision points
- [ROLLBACK.md](ROLLBACK.md) – Stop conditions & recovery
- [accidental-complexity-plan/phase-1-api-normalization.md](accidental-complexity-plan/phase-1-api-normalization.md) – Phase 1 details

---

**Remember:** Rush a refactoring → spend months debugging. Go slow, test constantly, stay aligned.

