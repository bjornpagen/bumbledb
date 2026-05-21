# V6 Query Image And Trie Layout Decision

## Purpose

Document the PRD 07 decision on query image and trie memory layout.

No layout change was made in this PRD.

## Artifacts

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-layout-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-layout-job-10k.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-layout-job-static-focused.json
```

Relevant prior artifacts:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-profiles/allocation-hotset-job-10k.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-width-specialized-job-10k.json
```

## Validation Results

- `cargo fmt --all --check`: pass
- `cargo check --workspace --all-targets --all-features`: pass
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass
- `cargo test --workspace --all-features`: pass
- `cargo check --manifest-path fuzz/Cargo.toml`: pass
- non-JOB gates: pass
- JOB 10k gates: pass

## Focused Static Proof / Query Image Results

| Query | BDB us | Image us | Static proof us | Query image encoded bytes | Gate |
|---|---:|---:|---:|---:|---|
| job_q16_character_title_us | 603 | 20275 | 611 | 0 | pass |
| job_q24_voice_keyword_actor | 645 | 20658 | 591 | 0 | pass |
| job_q33_linked_series_companies | 59 | 8073 | 0 | 0 | pass |

The focused artifact is a streaming JOB dataset run, so eager query image stats are not populated in the top-level benchmark counters. Phase timing still shows query image time is visible for static proof queries.

## Decision

Decision: defer query image/trie memory layout changes.

Rationale:

- v6 PRD 03 and PRD 05 delivered clear query-side wins without changing query image layout.
- Width-specialized scalar comparisons were neutral, suggesting single-key comparison mechanics are not enough to justify a large layout rewrite yet.
- Query image allocations are large for JOB static/direct paths, but JOB gates already pass and query-side timings remain strong.
- Current evidence does not isolate query image/trie cache locality as the next highest-leverage bottleneck.
- A layout rewrite would be invasive and should be driven by hardware counters or focused query-image allocation profiles, not trace inference alone.

## What Would Justify Revisiting Layout

Reopen layout work if one of these becomes true:

- hardware counters show L1/L2/cache misses in query image or trie iteration dominate hot queries
- PRD 08 or future ingest work shows durable segment/image layout causes write/read amplification
- static proof q16/q24 becomes a bottleneck after other work
- LFTJ build/trie traversal remains dominant after iterator and ARM NEON batch work

## Recommended Future Layout Direction

If revisited, prioritize:

1. Width-homogeneous column arrays.
2. Contiguous access-path key slabs.
3. Sorted trie level arrays with packed child/row ranges.
4. Alignment for ARM NEON batch scans.

Do not implement dual old/new layouts. No backwards compatibility or migrations.

## Compatibility Statement

No backwards compatibility. No migrations. No layout changes in this PRD.
