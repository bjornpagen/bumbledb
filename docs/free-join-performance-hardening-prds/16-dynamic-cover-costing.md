# PRD 16: Dynamic Cover Costing

## Purpose

Improve dynamic cover selection so it follows the paper's principle of iterating the smallest key set without forcing expensive structures blindly.

## Current Problem

`key_count` can report offset length estimates that do not equal distinct key counts. This can make dynamic cover choice misleading, especially on skewed sources.

## Required Design

- Distinguish exact key count, estimated key count, and unknown key count.
- Use exact map length when already forced.
- Use filtered survivor length only as an estimate with explicit label.
- Add optional cheap distinct sampling or cached prefix stats when available.
- Trace every cover candidate with count kind and count value.
- Preserve deterministic tie-breaking.

## Required Policy

- Do not force a COLT solely to discover exact key count unless the cost model says it is cheaper than a bad cover choice.
- If a force-for-count decision is added, it must be traced as such.
- Cover choice must remain local to the node and current source state.

## Passing Criteria

- Tests prove deterministic tie-breaking.
- Tests prove exact map counts are preferred over larger estimates.
- Tests prove estimates are labeled as estimates in trace output.
- A skewed fixture shows dynamic cover can change after descending into a subtrie.
- JOB q09 exact output remains unchanged.
- Global acceptance from PRD 00 passes.
