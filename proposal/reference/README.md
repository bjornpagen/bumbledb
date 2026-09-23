# Historical proposal and research

**Reference only. Read the [active proposal](../README.md) for the current scope.**

This directory restores authored documents from the preserved
`codex/event-algebra` branch at
[`79c46299a71edd7a8c0fd9e48bef5369bdec2548`](https://github.com/bjornpagen/bumbledb/tree/79c46299a71edd7a8c0fd9e48bef5369bdec2548).
The restored files are unedited originals. Their implementation claims, open
goals, research decisions, milestone lists, and instructions describe the old
branch, not the active design or the current engine.

The [restoration manifest](restoration.json) records each original and restored
path, byte length, and SHA-256. Its scope is authored proposal/research Markdown,
historical implementation notes, and small research/source manifests. It
restores 543 files, totaling 11,115,148 bytes before this index and the manifest.

Runtime code, Rust laboratory programs, Lean sources/builds, PDFs, large result
archives, generated proof snapshots, logs, and binaries were not restored.
Some original relative links therefore point to missing files. The immutable
source commit retains the full tracked tree; follow it when inspecting a
particular omitted artifact. Historical result claims have not been rerun or
requalified on this branch.

## Useful starting points

- [Original proposal](proposal/proposal.md) and
  [research index](proposal/research/README.md).
- [Research handoff](proposal/RESEARCH-HANDOFF.md) and
  [representation report](proposal/experiments/event-repr-lab/REPORT.md).
- [Finite pair-signature calculus](proposal/experiments/event-repr-lab/SIGNATURE-CALCULUS.md):
  reusable readouts and the limits of relation-table composition.
- [Event storage semantics](proposal/event-storage.md): empty rows, coverage,
  and why interval uniqueness needs an extra premise for Event.
- [Representation literature](proposal/research/representation-search.md):
  canonicalization cost and why no representation is compact for every set.
- [Saved Typesafe primitive documentation](proposal/research/event-algebra/typesafe-primitives.md):
  historical source material, not a fresh reading of the current service.

Keep findings that justify the small database type. Do not treat restoring
research as authorization to restore its implementation or surrounding systems.
