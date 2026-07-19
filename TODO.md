# TODO — the plan of record

## Open

- **The bench repin (T1 — the one debt).** The README's numbers are TORN:
  the committed `bench-out/run1/report.json` (chart input) is a 2026-07-16
  run at rev `adac4010` while `run1/report.md` is 2026-07-18 at `5f531e17`,
  and run2/run3/eph1–3 are all the old rev — so the published 21×/20.9×
  geomeans mix two binaries (void under the measurement law; honest
  recompute from committed data: 19.4× durable / 18.1× ephemeral), the
  "tails an order of magnitude inside SQLite's" sentence is false for
  `meets_chain` (ours p99 ≈1.35 ms vs ≈135 µs), and eph1 carries five
  contaminated read blocks. The repair (owner-scheduled, idle machine
  ONLY): release build at one rev → `bumbledb-bench gen` + `verify`
  (2,862-case stamp) → 3 durable + 3 ephemeral + `scenarios` through
  `scripts/measure.sh` → force-add matching `report.{json,md}` for EVERY
  run dir → `scripts/bench_viz.py` regenerates the five `assets/*.svg` →
  re-true every README number from the committed JSONs and fix the tails
  sentence. Full spec preserved in git history:
  `docs/hardening-0.3.0/prd-T1-bench-repin.md` at tag `v0.3.0`.
- **The 1.0.0 close (owner-gated, explicitly deferred 2026-07-18)** — R2 of
  `docs/structural-1.0.0/`: crate version `1.0.0` + the annotated `v1.0.0`
  tag. Owner ceremony only; no agent bumps, tags, or publishes.
- **Release-flow note (recurs every version):** the version-bump commit
  pins the exact platform optional-dep before the package exists, so the
  CI sdk lane's `--frozen-lockfile` fails between bump and publish. The
  post-publish step is a lockfile regeneration commit
  (`cd ts && pnpm install --no-frozen-lockfile`) — budget for it.
- Housekeeping when convenient: the shipped worktrees
  (`.claude/worktrees/hardening-030`, primer's `bumbledb-030`) can be
  removed; their branches are merged.

## Everything else: shipped

`@bjornpagen/bumbledb@0.3.0` (+ `-darwin-arm64@0.3.0`) is published and
tagged `v0.3.0` — the law-typed SDK: `.as` deleted, `schema()` computes
every domain from the statement list (`ts/src/law.ts`), ψ on both surfaces,
`vars()`/free comparisons/`Kind.match`/3-arg `closed`; the macro notation
untouched and newly checked (M5 newtype coherence); per-recipe TS↔Rust
fingerprint goldens; the notation⇄IR conformance corpus; the CI sdk lane.
Primer is cut over and merged (PR #78). The hardening-0.3.0 packet is
deleted per house convention; it lives at tag `v0.3.0`. History lives in
git; this document is not an archive.
