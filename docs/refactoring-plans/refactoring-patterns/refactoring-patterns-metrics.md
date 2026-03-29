# Refactoring Patterns – Metrics & Measurement

Framework for measuring impact of refactoring patterns.

---

## Baseline Metrics (To Be Measured Before Refactoring)

### Metric 1: Average Function Parameter Count

**Query:**
```bash
# Extract all function signatures
cargo expand --lib | grep "fn " | head -100

# Or use rust-code-analysis:
rust-code-analysis-cli -o metrics-output/ src/graph/algorithm/
```

**Target Hotspots:**
```
escalate_contract_parent: 5 params ───→ target: 2
escalate_try_contract_grandparent: 7 params ───→ target: 2
escalate_skip_promote: 5 params ───→ target: 2
decompose_basis: 4 params ───→ target: 3
range_sum_inner: 4 params ───→ target: 3
```

**Baseline:**
- Average params (all functions): ~2.8
- Average params (algorithm functions): ~4.2
- Max params: 7 (escalate_try_contract_grandparent)

**Target:**
- Average params (all functions): ~2.5
- Average params (algorithm functions): ~2.8
- Max params: 3

---

### Metric 2: Tell-Don't-Ask Violations

**Query:**
```bash
# Count functions taking mutable self-like references
grep -r "fn.*(&mut.*[A-Z].*," src/graph/algorithm/ | grep -v "self" | wc -l
```

**Baseline Violations:**
- Free functions taking `&mut GvGraph`: 2
- Free functions taking `&mut Arena<GNode>`: 5+
- Free functions taking `&mut Arena<VNode>`: 10+
- Free functions taking `&mut Vec<VNodeId>`: 15+

**After Refactoring:**
- Free functions taking mutable complex types: < 5

---

### Metric 3: Redundant Parameter Passing

**Query:**
```bash
# Count parameter pairs that always appear together
# (query_lo, query_hi) pairs:
grep -r "query_lo.*query_hi\|query_hi.*query_lo" src/ | wc -l
```

**Baseline:**
- `(query_lo, query_hi)` pairs: ~40 occurrences
- `(parent_id, merged_id, heaviest_id)` triplets: ~12 occurrences
- `(vnodes, violations)` pairs: ~30+ occurrences

**After Refactoring:**
- Paired parameters: < 10 (only in constructors/builders)

---

### Metric 4: Lines of Code in Parameter Passing

**Query:**
```bash
# Estimate lines of code spent constructing/unpacking parameter groups
# This is complex, but look for lines with only parameter references

# Rough estimate: count assignments like "let lo = ...; let hi = ...;"
grep -r "let.*_lo\|let.*_hi" src/graph/algorithm/ | wc -l
grep -r "let.*p_id\|let.*child_id" src/graph/algorithm/ | wc -l
```

**Baseline:**
- Parameter construction lines: ~150-200
- Parameter unpacking lines: ~100-150

**Target:**
- Parameter construction lines: < 50
- Parameter unpacking lines: < 30

---

### Metric 5: Function Complexity (Cyclomatic Complexity)

**Tool:** `rust-code-analysis-cli`

```bash
rust-code-analysis-cli -o metrics-output/ src/graph/algorithm/
```

**Focus On:**
- `escalate_*` functions (CC should be stable, readability improves)
- `decompose_basis` (CC should improve slightly)
- `range_sum_inner` (CC should be stable)

**Baseline:**
```
escalate_contract_parent: CC=8, LOC=20
escalate_try_contract_grandparent: CC=12, LOC=25
escalate_skip_promote: CC=6, LOC=18
```

**Target:**
- CC unchanged (logic is same)
- LOC slightly reduced (fewer parameter operations)

---

### Metric 6: Test File Parameter Count

**Query:**
```bash
# Test call sites should be cleaner after refactoring
grep -r "graph\." tests/ | head -20
# Count how many params are passed to algorithmic functions
```

**Baseline:** Test call sites average 5-7 parameters
**Target:** Test call sites average 2-3 parameters

---

## Refactoring Impact Checklist

Use this to verify each refactoring reduces complexity:

### Pattern 1: Tell-Don't-Ask (Split Operations)
```
BEFORE:
- bootstrap_split(graph, gid)  [free function]
- 15 call sites: bootstrap_split(self, g_id)
- Parameter category: Taking complex struct + node ID

AFTER:
- GvGraph::bootstrap_split(gid)  [method]
- 15 call sites: self.bootstrap_split(g_id)
- Parameter category: Direct methods

Validity Check:
✓ Call sites reduced: 15 → 15 (same, but cleaner)
✓ Semantic coupling: Visible (GvGraph owns operation)
✓ Tests pass: cargo test --lib split
```

### Pattern 2: Escalation Context
```
BEFORE:
escalate_contract_parent(vnodes, p, heaviest, h_direct, violations): 5 params

AFTER:
escalate_contract_parent(tree, ctx): 2 params

Validity Check:
✓ Params: 5 → 2 (-60%)
✓ LOC constructions: 8 → 2 (-75%)
✓ CC: 8 → 8 (unchanged, logic same)
✓ Tests pass: cargo test --lib rebalance
```

### Pattern 3: Coordinate Range
```
BEFORE:
decompose_basis(gid, query_lo, query_hi, basis): 4 params
- 20+ function signatures with lo/hi pairs

AFTER:
decompose_basis(gid, range, basis): 3 params
- Pairs replaced by CoordinateRange

Validity Check:
✓ Avg params: 4.0 → 3.0 (direct calls)
✓ Compound cases (nested calls): 8 → 5
✓ Type safety: Pairs grouped in type
✓ Tests pass: cargo test --lib query
```

### Pattern 4: Violation Queue
```
BEFORE:
push_contraction_child_violations(vnodes, p, heaviest, violations): 4 params
push_side_effect_violations(vnodes, node, violations): 3 params
push_promoted_violations(vnodes, node, violations): 3 params
- All call sites pass violations: &mut Vec

AFTER:
queue.push_contraction_child(vnodes, p, heaviest)
queue.push_side_effects(vnodes, node)
queue.push_promoted(vnodes, node)
- All methods on queue struct

Validity Check:
✓ Parameter reduction: 3-4 → 2-3 per call
✓ Queue operations: Centralized
✓ Mutation semantics: Clearer (methods on queue owner)
✓ Tests pass: cargo test --lib violation
```

---

## Pre-Refactoring Measurement Script

Run this before making changes:

```bash
#!/bin/bash

echo "=== BASELINE METRICS ==="

echo "1. Function Parameter Count (hotspots)"
cargo expand --lib 2>/dev/null | grep -o "fn [a-zA-Z_]*([^)]*)" | \
  grep -E "escalate|decompose|range_sum" | head -10

echo ""
echo "2. Tell-Don't-Ask Violations"
echo "Free functions with &mut Graph/Arena/Vec:"
find src/graph/algorithm -name "*.rs" -exec grep -l "fn.*&mut" {} \; | wc -l

echo ""
echo "3. Coordinate Pair Occurrences"
grep -r "_lo.*_hi\|_hi.*_lo" src/graph/algorithm/ | wc -l

echo ""
echo "4. Complexity Analysis"
rust-code-analysis-cli -o metrics-output/ src/graph/algorithm/rebalance/resolve.rs 2>&1 | \
  grep -E "Cyclomatic|LOC" | head -5

echo ""
echo "5. Test Parameter Count"
grep -r "\.bootstrap_split\|\.catalytic_split\|escalate_" tests/ | wc -l

echo ""
echo "=== BASELINE COMPLETE ==="
```

---

## Post-Refactoring Measurement

After each pattern, run the same script and compare:

```bash
# Example output format:

PATTERN 1: Tell-Don't-Ask
Before: 2 free functions with &mut GvGraph
After:  0 free functions with &mut GvGraph
Status: ✓ PASS

PATTERN 2: Escalation Context
Before: 18 total params across 3 functions (avg 6/func)
After:  6 total params across 3 functions (avg 2/func)
Reduction: -67% ✓ PASS

PATTERN 3: Coordinate Range
Before: 40 lo/hi pair occurrences
After:  ~0 lo/hi pair occurrences (all CoordinateRange)
Reduction: -100% ✓ PASS

PATTERN 4: Violation Queue
Before: 15 functions with &mut Vec<VNodeId>
After:  3 free statefuncs with violations queues
Reduction: -80% ✓ PASS
```

---

## Continuous Monitoring

### SonarQube Metrics (If Available)

```
- Cognitive Complexity per function
- Parameter coupling
- Function size (LOC)
- Cyclomatic Complexity
```

### rust-code-analysis Tracking

```bash
# Run before and after
rust-code-analysis-cli -o metrics-before/ src/graph/algorithm/
# [make changes]
rust-code-analysis-cli -o metrics-after/ src/graph/algorithm/

# Compare JSON outputs
diff metrics-before/*.json metrics-after/*.json | grep -E "cc|loc|param"
```

### Custom Script for Parameter Analysis

```python
import re
import json

def extract_function_params(file_path):
    """Count parameters in each function"""
    with open(file_path, 'r') as f:
        content = f.read()
    
    # Match: fn name(param1: Type, param2: Type, ...)
    pattern = r'fn\s+(\w+)\s*\(([^)]*)\)'
    
    result = {}
    for match in re.finditer(pattern, content):
        func_name = match.group(1)
        params_str = match.group(2)
        param_count = len([p.strip() for p in params_str.split(',') if p.strip()])
        result[func_name] = param_count
    
    return result

# Before
before = extract_function_params('src/graph/algorithm/rebalance/resolve.rs.before')
# After
after = extract_function_params('src/graph/algorithm/rebalance/resolve.rs.after')

# Compare
for func in before:
    if func in after:
        delta = after[func] - before[func]
        print(f"{func}: {before[func]} → {after[func]} ({delta:+d})")
```

---

## Success Criteria

✅ Refactoring is successful if:

| Metric | Target | Measurement |
|--------|--------|-------------|
| Avg function params (hotspots) | -40% | Count & compare |
| Tell-Don't-Ask violations | < 5 | Search free functions |
| Parameter pair occurrences | -75% | grep lo/hi, lo/hi patterns |
| CC (complexity) | Unchanged | rust-code-analysis |
| Test coverage | >= 95% | cargo tarpaulin |
| All tests pass | 100% | cargo test |

---

## Red Flags (Stop & Review)

⚠️ Refactoring should be paused if:

1. **Cyclomatic Complexity increases** by > 10%
   - Fix: Simplify context structure or split function

2. **Test coverage drops** below 90%
   - Fix: Add tests before refactoring

3. **New compiler warnings**
   - Fix: Resolve before proceeding

4. **Performance regression** (if benchmarked)
   - Fix: Profile and optimize context structures

5. **Parameter counts increase** in any hotspot
   - Fix: Review context design

