# TODO — the plan of record

## Open

- **The 1.0.0 close (owner-gated, explicitly deferred 2026-07-18)** — R2 of
  `docs/structural-1.0.0/`: crate version `1.0.0` + the annotated `v1.0.0`
  tag. Owner ceremony only; no agent bumps, tags, or publishes.
- **PR #10 (incremental images + the 32 GiB ceiling)** — complete on its
  branch, gates green, measured (copy-on-append 2.54× on the cold lineage
  family; the mask fork refuted by the decider twin; durable 32 GiB /
  ephemeral 4 GiB ceiling split). Held open by owner order; merge is the
  owner's call.
- **Optional, unscheduled:** a fresh one-rev seven-run bench session would
  restore min-of-3 durable sampling and re-clean `mandate_overlap` (excluded
  from the current pin as contaminated-in-both). The current README numbers
  are fully derivable from the committed artifacts and need nothing.
- **Release-flow note (recurs every version):** the version-bump commit
  pins the exact platform optional-dep before the package exists, so the
  CI sdk lane's `--frozen-lockfile` fails between bump and publish. The
  post-publish step is a lockfile regeneration commit
  (`cd ts && pnpm install --no-frozen-lockfile`).

## Everything else: shipped

`@bjornpagen/bumbledb@0.4.0` (+ `-darwin-arm64@0.4.0`) is published and
tagged `v0.4.0` — the host-idiom SDK on the law-typed 0.3.0 core; primer is
cut over and merged (PR #85). **The bench pin is healed (2026-07-19):** the
README's read-family numbers (18.7× durable over clean min-of-2 with
`mandate_overlap` excluded-and-counted, 18.4× ephemeral over all 22, ALL-WIN
in both) derive from the committed `bench-out/` artifacts at one rev
(`adac4010`, 2026-07-16), charts regenerated from the same; the orphaned
mixed-rev run1 is deleted; the tails sentence names its one honest exception
(`meets_chain` p99). The shipped packets live at their tags. History lives
in git; this document is not an archive.
