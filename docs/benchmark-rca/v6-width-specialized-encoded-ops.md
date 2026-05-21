# V6 Width-Specialized Encoded Operations

## Purpose

Document the width-specialized encoded comparison pass.

This PRD introduced a scalar width-dispatch layer for encoded key comparisons and used it in LFTJ leapfrog key ordering/search. It did not add ARM NEON yet. It explicitly did not add x86 SIMD.

## Artifacts

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-width-specialized-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-width-specialized-job-10k.json
```

Baseline artifacts:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-lftj-mechanics-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-lftj-mechanics-job-10k.json
```

## Implementation Summary

Added internal width dispatch:

```text
EncodedWidth::W1
EncodedWidth::W8
EncodedWidth::W16
```

Added scalar comparison helpers:

```text
compare_encoded_bytes
compare_encoded_ref
compare_encoded_ref_owned
```

LFTJ `LeapfrogState` now uses borrowed encoded references for some key comparisons instead of always materializing owned keys.

No x86 SIMD was introduced.

No ARM NEON implementation was added in this PRD because scalar width dispatch did not expose enough improvement to justify jumping straight to NEON before further iterator/layout work.

## Benchmark Delta

### Non-JOB

| Query | Before us | After us | Delta | LFTJ next | LFTJ seek | LFTJ key reads |
|---|---:|---:|---:|---:|---:|---:|
| sailors/red_boat_sailors | 4969 | 5102 | +3% | 34153 | 17491 | 105789 |
| sailors/high_rating_red_boats | 4376 | 4429 | +1% | 34153 | 17493 | 105793 |
| joinstress/triangle_count | 10278 | 10239 | 0% | 90000 | 119995 | 589992 |
| tpch/revenue_by_customer_range | 2872 | 3001 | +4% | 20000 | 4000 | 40002 |
| tpch/supplier_nation_orders | 2383 | 2527 | +6% | 18577 | 7143 | 50013 |

### JOB

| Query | Before us | After us | Delta | Static proof us |
|---|---:|---:|---:|---:|
| job_q09_voice_us_actor | 892 | 939 | +5% | 868 |
| job_q16_character_title_us | 593 | 594 | 0% | 570 |
| job_q24_voice_keyword_actor | 616 | 617 | 0% | 590 |

## Target Results

Hard gates:

- non-JOB gates: pass
- JOB 10k gates: pass

Optimization targets:

- q16/q24 static proof target 10% improvement: missed; observed neutral
- two non-JOB LFTJ hot queries target 10% improvement: missed; observed neutral/noisy
- direct kernel no-regression target: pass at gate level

## Interpretation

Scalar width dispatch alone is not the current bottleneck.

The measurements show that after PRD 03 and PRD 05, the remaining LFTJ cost is not substantially improved by replacing generic byte comparisons with scalar width-specific comparisons in the limited locations touched here.

Likely reasons:

- remaining overhead is still iterator control flow and key-read volume
- owned key construction still exists where `seek` requires an owned max key
- sort/dedup/output changes moved more time than comparison scalarization
- NEON would need wider batch-oriented loops, not isolated single-key comparisons

## Recommendation

Keep the scalar width-dispatch helper as a clean foundation, but do not expand SIMD yet.

Before ARM NEON work, PRD 07 should determine whether query image/trie memory layout can expose contiguous arrays that make NEON worthwhile.

Future ARM NEON work should target batch scans/intersections, not one-key-at-a-time comparisons.

## SIMD Policy Confirmation

Active Rust code was checked for x86 SIMD references:

```text
std::arch::x86
std::arch::x86_64
is_x86_feature_detected
_mm_
avx2
avx512
sse2
sse4
```

No matches were found.

## Compatibility Statement

No backwards compatibility. No migrations. No x86 SIMD. No decoded comparisons were introduced into hot encoded paths.
