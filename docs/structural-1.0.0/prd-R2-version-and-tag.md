# PRD-R2 — Version 1.0.0 + annotated tag

Wave 3 · Repo: bumbledb · depends on: R1 (README trued) + owner approval · OWNER CEREMONY

## Objective

Cut the `1.0.0` release marker on the engine repo: bump the workspace version and
create the annotated `v1.0.0` tag. **This is owner ceremony** — the packet
PREPARES everything to the tag boundary; the owner makes the release decision and
pushes the tag. 1.0.0 is the owner's call, never a gate's.

## Context

- The exit criterion (the owner's own): grep the repo for a known defect, a
  measured-but-unclaimed win, an unexplained behavior, or an unresolved OPEN-ledger
  row — and find nothing. A/S1–S5 (Wave 1) close the SDK + the last engine
  semantic; C1/C2 (Wave 2) settle the measurement-owned candidates; R1 trues the
  numbers. R2 is the marker on the tree where all of that is true.
- The current release marker is `v0.1.0` (the first published waypoint). `1.0.0`
  is the real release.

## Work (an agent PREPARES 1–3; the owner does 4)

1. **Bump the workspace version** in the root `Cargo.toml` to `1.0.0`; update
   `Cargo.lock`; confirm `cargo build` is clean at the new version.
2. **Confirm the release floor** with a final grep/audit: no `TODO`/`FIXME`/`BUG`
   markers in engine source; the OPEN ledger in `70-api.md` all resolved
   (fixed/refuted/fired/declined); `scripts/check.sh` + `scripts/lean.sh` green;
   R1's charts + README current; the SDK (Wave 1) green. Produce the audit as the
   tag's evidence.
3. **Prepare the annotated tag message** (the release notes: what 1.0.0 is — the
   set-semantic Free Join engine, the two-oracle discipline, the Lean spec, the
   structural TS SDK; research-grade, Apple-Silicon-tuned; the honest scope) as a
   drafted `git tag -a v1.0.0` command + message for the owner to run.
4. **The owner pushes the tag** — `git tag -a v1.0.0 …` + `git push origin v1.0.0`
   + the GitHub release. The release ceremony is the owner's; no agent creates or
   pushes the `v1.0.0` tag.

## Technical direction

- Do NOT push the tag from an agent (frozen ruling: the owner tags). The version
  bump commit MAY be pushed to `main` by an agent (it is code, gate-green), but the
  TAG is the owner's hand.
- Version lockstep: the SDK's release version (R3) corresponds to this engine tag.
  Do not bump the SDK version here (R3 owns it, and the owner decides 0.2.0-vs-1.0.0
  for the SDK separately).
- If the final audit finds ANY unresolved item, R2 STOPS and reports it — 1.0.0 is
  not tagged over a known gap.

## Passing criteria

- Root `Cargo.toml` at `1.0.0`, `Cargo.lock` updated, `cargo build` clean; the bump
  commit gate-green (`check.sh` + `lean.sh`) and pushed.
- The release-floor audit is clean and recorded (the tag's evidence).
- The annotated-tag command + message are drafted and handed to the owner; the
  agent does NOT create/push `v1.0.0`.
- Reported to the owner as "ready to tag" with the audit and the drafted command.
