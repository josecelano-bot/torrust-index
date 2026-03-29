# Rollback & Recovery: Stop Conditions & Procedures

Guide to recognizing when to stop refactoring and procedures to recover.

---

## 🛑 Red Flags: When to STOP Immediately

### 🔴 RED FLAG #1: Two Consecutive `Gate C` Failures

**Symptom:** Full test suite (`cargo test --all`) fails in same phase, persists after fix attempts

**Example:**
```
Commit A: Make change → Gate C FAIL ❌
Fix attempt → Gate C FAIL ❌
Revert & retry → Gate C FAIL ❌
```

**Why it matters:** Either the phase is ill-designed OR there's a hidden complexity/coupling issue

**Recovery Procedure:**

```bash
# Step 1: Determine severity
git log --oneline -5  # See recent changes
cargo test --lib     # Run targeted tests
cargo test --all     # Run full suite

# Step 2: Review last 3 commits for patterns
git diff HEAD~3 HEAD -- src/graph/algorithm/ | head -100

# Step 3: Decision time
# Option A: Fix (if issue is clear)
#   - Identify root cause
#   - Make targeted fix
#   - Gate C must pass
#   - Document why it failed
#
# Option B: Revert (if issue is unclear)
#   - git revert HEAD~2..HEAD (last 3 commits)
#   - Gate C must pass after revert
#   - Schedule team meeting to review approach
#   - Restart with clearer plan
#
# Option C: Abort Phase (if issue is systemic)
#   - git checkout refactor/master-2026-03  # Reset to branch point
#   - Document findings in ROLLBACK_LOG.md
#   - Escalate to tech lead

# Step 4: Update PROGRESS.md
# Mark as ❌ BLOCKED, add entry to blockers section
```

**Decision Threshold:**
- If fix is obvious (< 1 hour) → Fix it
- If fix is unclear (> 1 hour research) → Revert + replano
- If systemic issue → Abort phase, escalate

---

### 🔴 RED FLAG #2: Performance Regression > 5%

**Symptom:** Benchmark shows significant slowdown

```bash
# Before refactoring
cargo bench --bench depth
# Result: 42.5ms ± 0.3ms

# After refactoring
cargo bench --bench depth
# Result: 45.3ms ± 0.4ms
# Δ = +2.8ms = +6.6% ❌ OVER THRESHOLD (5%)
```

**Why it matters:** Refactoring should not make code slower (or very marginally slower)

**Recovery Procedure:**

```bash
# Step 1: Confirm regression is real
cargo bench --bench depth  # Run 3 times, average result

# Step 2: Profile to find cause
cargo flamegraph --bench depth

# Step 3: Investigate recent changes
git diff HEAD~5 HEAD -- src/graph/algorithm/ | grep -A5 -B5 "inefficient pattern"

# Step 4: Decision time
# Option A: Optimize (if cause obvious)
#   - Fix inefficiency (e.g., unnecessary clone, suboptimal algorithm)
#   - Verify regression gone
#   - Gate C must pass
#
# Option B: Revert (if optimization unclear)
#   - git revert HEAD  # Or revert last N commits
#   - Verify regression gone
#   - Document: "Reverted because perf regression unexplained"
#   - Approach: Different refactoring strategy
#
# Option C: Accept (only if tradeoff justified)
#   - Document: "2% perf loss acceptable for X architectural benefit"
#   - Requires tech lead approval
#   - Add note to PR

# Step 5: Benchmark again to confirm fix
cargo bench --bench depth
```

**Decision Threshold:**
- Regression < 2% → Accept (within noise margin)
- Regression 2-5% → Investigate, but not show-stopper
- Regression > 5% → STOP, must fix or revert

---

### 🔴 RED FLAG #3: Test Coverage Drops Below 85%

**Symptom:** Refactoring exposed lack of test coverage

```bash
# Before: 92% coverage
cargo tarpaulin --lib

# After: 83% coverage ❌ DROPPED BELOW 85%
```

**Why it matters:** Low coverage means regression risk, refactoring instability

**Recovery Procedure:**

```bash
# Step 1: Measure exact coverage
cargo tarpaulin --lib --out Html

# Step 2: Identify what's uncovered
# Look at HTMLreport, find new uncovered lines

# Step 3: Add tests for uncovered code
# This is the right thing to do anyway
cargo test --lib

# Step 4: Retry coverage
cargo tarpaulin --lib

# Step 5: Must get back to >90% before continuing
# If can't: STOP phase, add tests first, resume later
```

**Decision Threshold:**
- Coverage > 90% → Continue
- Coverage 85-90% → Add tests (don't continue)
- Coverage < 85% → ABORT phase, add tests first

---

### 🔴 RED FLAG #4: Stuck in Same Phase for > 2 Weeks

**Symptom:** One phase accumulates commits without merging

```bash
git log --oneline refactor/phase-1-api-norm | head -20
# Result: 20+ commits, none merged to main

# Time check:
git log -1 --date=short --format=%ad refactor/phase-1-api-norm
# Result: 18 days old

# Status: PHASE BLOCKED FOR 18 DAYS ❌
```

**Why it matters:** Stale branches compound merge conflicts, lose context, signal design issue

**Recovery Procedure:**

```bash
# Step 1: Assess status
git log --oneline refactor/phase-1-api-norm | wc -l
git diff main..refactor/phase-1-api-norm -- src/graph/ | wc -l

# Step 2: Identify why it's stuck
# Post-mortem questions:
#   - Were small steps being merged? Or one giant commit?
#   - Are there test failures blocking merge?
#   - Did requirements change?
#   - Is the phase too ambitious?

# Step 3: Decide: Continue or Abandon?

# Option A: Salvage phase (if 80% done)
#   - Checkpoint current state: git tag checkpoint/phase-1-attempt-1
#   - Revert to smaller, simpler version
#   - Merge what you have
#   - Follow up in Phase 2 if needed
#
# Option B: Abandon phase (if 50% or less done)
#   - Don't merge the branch
#   - Revert to main: git checkout main
#   - Document lessons learned
#   - Plan simpler approach
#   - Start fresh with new branch

# Step 4: Team meeting
# Discuss: Why did this phase stall?
# Adjust: Smaller steps? Different approach? More resources?
```

**Decision Threshold:**
- Phase completion visible (4+ PRs merged) → Continue
- Phase stuck (0 PRs merged > 2 weeks) → DECISION MEETING required

---

### 🔴 RED FLAG #5: Team Consensus Lost

**Symptom:** Team disagreement on approach, code review comments increasing

```
PR Review Comments Pattern:
Commit 1: 5 comments
Commit 2: 8 comments
Commit 3: 12 comments
Commit 4: 18 comments ← Spiral!

Team Slack:
"I'm not sure we should do this way"
"But the PR says we're doing X"
"Wait, I thought we agreed on Y"
```

**Why it matters:** Misalignment kills refactoring. Half-hearted effort compounds problems

**Recovery Procedure:**

```bash
# Step 1: Call team meeting
# Agenda:
#   - What is the actual goal? (agreement check)
#   - What approach is best? (option A vs B vs C)
#   - Who disagrees & why?
#   - What decision makes 80%+ team OK?

# Step 2: Make a call
# Options:
#   A) Pivot direction (most agree on alt approach)
#   B) Pause & research (revisit in 1 week)
#   C) Split: do both approaches in parallel phases
#   D) Revert & simplify phase scope

# Step 3: Document decision
# File: docs/refactoring-plans/DECISION_LOG.md
# Entry:
#   Date: 2026-04-X
#   Issue: [Description]
#   Options considered: [A, B, C, D]
#   Decision: [X chosen because...]
#   Dissenters: [None / Names & why]

# Step 4: Retry phase with clear direction
```

**Decision Threshold:**
- Consensus (80%+ team) → Continue
- Disagreement (>30% team) → STOP, align first

---

### 🟡 YELLOW FLAG: Performance Regression 2-5%

**Not immediately stop-worthy, but requires investigation**

```bash
# Regression: 42ms → 43.1ms = +2.6%
```

Check:
- Is it a real regression or measurement noise?
- Run benchmark 3 times, check consistency
- If consistent: Investigate cause (may be acceptable tradeoff)
- Document in PROGRESS.md why acceptable

---

### 🟡 YELLOW FLAG: One `Gate B` Test Failing

**Not immediate stop, but concerning**

```bash
# Commit applies pattern change
# Gate B (targeted tests): FAIL ❌
# Gate C (full tests): PASS ✅
```

**Action:**
- Review Gate B tests: Did they need updating?
- Fix targeted tests
- Re-run Gate C
- If Gate C still passes: Commit & document
- If Gate C fails on retry: Escalate to RED FLAG #1

---

## 🔄 Recovery Procedures

### Recovery Option A: Quick Fix (< 1 hour)

For simple, isolated issues:

```bash
# Symptom: Test failure is obvious/fixable
# E.g., test was missing, assertion too strict, etc.

Step 1: Understand why test failed
  cargo test --lib FAILING_TEST -- --nocapture

Step 2: Fix the issue
  [Edit code]
  
Step 3: Re-run test
  cargo test --lib FAILING_TEST

Step 4: Run full gates
  cargo check --all-features        # Gate A
  cargo test --lib [FOCUSED_AREA]   # Gate B
  cargo test --lib --all            # Gate C

Step 5: Commit
  git add [files]
  git commit -m "fix: [brief reason why test was failing]"
```

---

### Recovery Option B: Smallstep Revert (1-4 hours)

For changes that are wrong direction but partially understood:

```bash
# Symptom: Phase change broke something, but fix unclear

Step 1: Identify problem commit(s)
  git log --oneline -10
  # Let's say commits ABC, DEF, GHI are problematic

Step 2: Revert specific commits
  git revert ABC        # Revert in reverse order
  git revert DEF
  git revert GHI
  
  # OR revert range:
  git revert ABC^..GHI  # Revert ABC through GHI

Step 3: Verify revert
  cargo test --lib --all  # Gate C must pass

Step 4: Document
  Write in PR description why these were reverted
  
Step 5: Re-approach
  Plan simpler, more incremental version
  Remove the commits that were reverted
  Restart with smaller steps
```

---

### Recovery Option C: Phase Reset (4-8 hours)

For phases that accumulated too much without clear progress:

```bash
# Symptom: Phase branch is 15+ commits old, unclear state

Step 1: Identify split point
  git merge-base main refactor/phase-1-api-norm
  # Result: commit ABC123

Step 2: Review work on branch
  git diff ABC123..refactor/phase-1-api-norm | head -500

Step 3: Decide: Keep or discard?
  Option A: This is valuable, reset to simpler version
  Option B: Start over with new approach

# Option A: Simplify
  git checkout -b refactor/phase-1-api-norm-v2
  git reset --hard refactor/phase-1-api-norm
  # [Manually remove some commits in interactive rebase]
  git rebase -i ABC123
  # Mark early commits as 'pick', later ones as 'drop'
  cargo test --all  # Verify still works
  
# Option B: Start fresh
  git branch -D refactor/phase-1-api-norm
  git checkout -b refactor/phase-1-api-norm-v2 main
  # Start fresh with simpler plan

Step 4: Communicate
  Message to team: "Phase 1 being simplified, restarting with clearer steps"
```

---

### Recovery Option D: Abort & Escalate (2-3 hours)

For systemic issues needing higher-level decision:

```bash
# Symptom: Issue is not fixable at engineer level

Step 1: Document the issue
  Create: docs/refactoring-plans/INCIDENT_[DATE].md
  Include:
    - What happened
    - What went wrong
    - Why it's not easily fixable
    - What options exist
    - Recommendation

Step 2: Notify stakeholders
  Email tech lead & project manager:
  "Phase 1 Refactoring: Unable to continue, escalation needed"
  Attach: INCIDENT_[DATE].md

Step 3: Mark as blocked
  Update PROGRESS.md:
    Status: ❌ BLOCKED
    Blocker: [Issue description]
    Expected resolution: [Depends on leadership decision]

Step 4: Stack rank options for decision maker
  Option A: [Description] - Effort: X days, Outcome: Y
  Option B: [Description] - Effort: X days, Outcome: Y
  Option C: [Description] - Effort: X days, Outcome: Y
  
  Recommendation: [Option X because...]

Step 5: Wait for decision
  Tech lead chooses option
  Proceed based on decision

Step 6: Document decision
  Add to DECISION_LOG.md
  Adjust timeline & scope in PROGRESS.md
```

---

## 📋 Recovery Actions Checklist

When any red flag appears, follow this checklist:

```
☐ 1. STOP committing (pause the phase)
☐ 2. Document the issue (what happened, when, why concerning?)
☐ 3. Assess severity
     ☐ Can fix in < 1 hour? → Quick Fix (Option A)
     ☐ Need to revert some commits? → Smallstep Revert (Option B)
     ☐ Need to reset whole phase? → Phase Reset (Option C)
     ☐ Systemic issue? → Abort & Escalate (Option D)
☐ 4. Execute recovery option
☐ 5. Verify Gates A+B+C pass after recovery
☐ 6. Update PROGRESS.md with status
☐ 7. Notify team (Slack/email/meeting)
☐ 8. Adjust timeline in PROGRESS.md
☐ 9. Decision: Resume or pause?
☐ 10. If resuming: Confirm with tech lead before continuing
```

---

## 📝 Incident Log Template

Create this file if you need to document a recovery:

```markdown
# Incident Log: [DATE] - [BRIEF TITLE]

**Date:** 2026-04-09  
**Phase:** Phase 1 - API Normalization  
**Severity:** 🔴 RED / 🟡 YELLOW  
**Status:** RESOLVED / PENDING

## What Happened

[Describe the issue]

## Impact

- Tests affected: [count]
- Performance: [% change]
- Other: [blocked team members, etc.]

## Root Cause

[Why did this happen?]

## Recovery Action

[What was done to fix it?]

## Timeline

- 14:00 Issue detected
- 14:15 Diagnosed as [root cause]
- 14:45 Recovery started
- 15:30 Tests passing again
- Duration: 1.5 hours

## What We Learned

[What should be different next time?]

## Prevention

[Process or check to prevent recurrence]
```

---

## 🔗 Decision Tree for Recovery

```
RED FLAG DETECTED
  ↓
Is it a test failure? → Yes → Could be simple fix
  ↓ NO
    
Is it a performance regression? → Yes → Profile & decide
  ↓ NO

Is it coverage drop? → Yes → Add tests (don't continue)
  ↓ NO

Is phase stuck 2+ weeks? → Yes → Team meeting needed
  ↓ NO

Is team disagreeing? → Yes → Align first
  ↓ NO

UNKNOWN ISSUE → Escalate to tech lead
```

---

## 🚀 When to Resume After Recovery

**After any red flag recovery, ask:**

1. ✅ Are ALL test gates passing?
   - Gate A: `cargo check --all-features`
   - Gate B: Targeted tests
   - Gate C: Full test suite

2. ✅ Have we updated PROGRESS.md?
   - Status: Clear what happened
   - Owner: Same or different?
   - Timeline: Adjusted if needed?

3. ✅ Has tech lead reviewed recovery?
   - Decision: OK to proceed?
   - Or pause until something else changes?

4. ✅ Are we confident it won't repeat?
   - What changed to prevent recurrence?
   - Is it documented?

**Only AFTER all 4 are ✅, resume work.**

---

## 📞 Reference

- [PROGRESS.md](PROGRESS.md) – Update when incidents occur
- [PREREQUISITES.md](PREREQUISITES.md) – Go/no-go decision gates
- [INTEGRATION.md](INTEGRATION.md) – Decision points during phases
- [accidental-complexity-plan/](accidental-complexity-plan/) – Phase details

---

**Remember:** "Stop fast, recover fast, learn systematically."  
Better to pause and replan than to push through broken code.

