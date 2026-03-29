# Alternatives to torrust-mudlark

This document compares practical alternatives for streaming hotspot detection and
adaptive frequency estimation over a 1D keyspace.

The goal is not to find a universal replacement, but to clarify when another
approach may fit better depending on constraints (accuracy, memory, latency,
mergeability, and implementation complexity).

## Problem framing

`torrust-mudlark` is strongest when you need all of the following at once:

- online updates from a stream,
- bounded memory behavior,
- temporal decay (recent activity matters more),
- weighted sampling from current activity,
- and adaptive spatial granularity in 1D.

Most alternatives optimize a subset of these requirements.

## Comparison summary

| Approach | Strengths | Limitations | Best fit |
| --- | --- | --- | --- |
| Count-Min Sketch (+ optional heavy-hitter layer) | Very fast updates, tiny memory, simple scaling | Overestimation and hash collisions, no native spatial adaptivity | High-throughput approximate counting |
| Space-Saving / Misra-Gries | Strong top-k heavy hitter tracking with bounded memory | Focused on keys, not ranges or spatial structure | "Who are the top offenders?" style workloads |
| Fixed-bucket histogram (+ decay) | Easiest to implement and reason about | Resolution is static; either coarse or expensive | Stable domains with known useful bucket granularity |
| Segment tree / Fenwick tree | Efficient prefix/range queries with deterministic behavior | Less adaptive; can be memory-heavy at high resolution | Deterministic range-sum systems |
| Quantile / frequency sketches (DataSketches family) | Mature ecosystem, merge-friendly streaming analytics | Often not directly aligned with adaptive 1D hotspot partitioning | Distributed analytics pipelines and telemetry stacks |

## Detailed alternatives

### 1) Count-Min Sketch (CMS)

Core idea:
- Hash each key into multiple rows and increment counters.

Why teams pick it:
- Extremely low memory footprint.
- O(1) updates and queries.
- Easy to shard and merge.

What you give up:
- Frequency estimates are biased high due to collisions.
- No direct representation of coordinate neighborhoods.
- Native weighted sampling over coordinate ranges is not inherent.

Use CMS when:
- You need approximate frequency under very high throughput,
- and key-level estimates are more important than adaptive range detail.

### 2) Space-Saving / Misra-Gries heavy hitters

Core idea:
- Keep a bounded set of candidate keys and counters to track likely top-k items.

Why teams pick it:
- Strong practical signal for top offenders.
- Bounded memory independent of stream length.

What you give up:
- Not a full density estimator.
- Range queries are external to the algorithm.
- Weighted sampling by geometric locality is not native.

Use this when:
- You primarily need top keys or abuse sources,
- and spatial locality is secondary.

### 3) Fixed-bucket histogram with decay

Core idea:
- Predefine bins over the domain and apply decayed counts per bin.

Why teams pick it:
- Simple implementation and operational predictability.
- Easy debugging and clear interpretation.

What you give up:
- Fixed resolution misses local hotspots unless bucket count is high.
- Increasing resolution raises memory and update costs.

Use this when:
- The domain is stable and expected hotspot scale is known,
- and simplicity is valued over adaptive precision.

### 4) Segment tree / Fenwick tree variants

Core idea:
- Maintain hierarchical or prefix-sum structures for efficient range aggregations.

Why teams pick it:
- Strong query performance for range sums.
- Deterministic behavior and predictable complexity.

What you give up:
- Not inherently adaptive to activity concentration.
- Dynamic splitting/eviction logic must be added separately.

Use this when:
- Range query performance and determinism are primary requirements,
- and adaptive model behavior is not the core value.

### 5) DataSketches-style streaming sketches

Core idea:
- Use established sketch families for approximate frequency, quantiles,
  set operations, and mergeable summaries.

Why teams pick it:
- Battle-tested design and distributed-system friendliness.
- Well understood error/space tradeoffs.

What you give up:
- May require composition of multiple sketches to emulate adaptive spatial
  hotspot detection with decay and weighted sampling.

Use this when:
- You already run sketch-based analytics,
- and interoperability/merge semantics are top priorities.

## Decision guide

Choose `torrust-mudlark` when:
- you need adaptive 1D hotspot localization,
- temporal decay is a first-class requirement,
- and sampling from current density is part of the workflow.

Choose an alternative when:
- you only need top-k keys (Space-Saving / Misra-Gries),
- you only need approximate per-key counts at extreme scale (CMS),
- you need maximum operational simplicity (fixed histogram),
- or you need deterministic range queries with static resolution assumptions
  (segment/Fenwick structures).

## Hybrid strategy

A practical production pattern is hybridization:

- Use CMS or Space-Saving as a coarse front filter for candidate hotspots.
- Feed selected regions/keys into `torrust-mudlark` for adaptive spatial
  refinement and sampling.

This reduces global memory pressure while preserving fine-grained behavior
where it matters.

## Closing note

There is no strict winner across all environments.
The right choice depends on which error mode is acceptable:

- approximation error,
- spatial under-resolution,
- delayed adaptation,
- or operational complexity.

If your primary question is "where is activity concentrated right now in a
streaming 1D domain, and how should I sample from it?", `torrust-mudlark`
remains a strong fit.