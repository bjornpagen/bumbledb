# Retiring the disposable proposal folder

This checklist prepares and records retirement of `final-solution/` and
root `PROMPT.md`. **Retirement happens before final candidate capture
and qualification**, not after a green battery.

The earlier “review → checks → retire → commit” sequence is expressly
withdrawn: retirement changes qualified inputs. Do not restore that
order.

L21 transfers contracts here. The coordinator executes deletion and the
sole final commit. This page does not authorize package publication or
live-tenant mutation.

## What must already live here (L21, now)

| Content | Permanent home |
| --- | --- |
| Semantic contract | [semantics.md](semantics.md) |
| Public API contract | [api.md](api.md) |
| Performance/storage contract | [performance.md](performance.md) |
| 68 audit + 78 prior-review + 220 child obligations | [behavioral-obligations.md](behavioral-obligations.md), [obligation-inventory.json](obligation-inventory.json) |
| D01–D29, G00–G16, runner order, evidence identity | [release-gates.md](release-gates.md) |
| Remaining qualification checklist | [qualification-checklist.md](qualification-checklist.md) |
| Lean/bridge correspondence (permanent, stays under `lean/**`) | [semantics.md](semantics.md) wire/proof boundary; sources: `lean/Bumbledb/Bridge.lean`, `lean/proof-bridge-ledger.md`, `lean/correspondence.md`, `scripts/lean.sh`, `scripts/spec-census.sh` |
| Evidence/checker discipline | `scripts/release-results.mjs`, [release-results.schema.json](release-results.schema.json) |

No executable tool may scrape deleted Markdown to invent inventory after
retirement. `obligation-inventory.json` `generatedFrom` lists only
`docs/reference/**`.

## Coordinator deletion barrier

Delete **only** after the transfer above is in tree and the coordinator
has verified no executable/doc link still depends on the packet:

- `final-solution/` (entire folder)
- root `PROMPT.md`

Preserve `audit/` permanently. Preserve packet `STATUS.md` recovery
copies in external coordinator state if resumption needs them.

Then follow [qualification-checklist.md](qualification-checklist.md):
capture the post-retirement candidate, run required cells, populate
evidence from actual results, commit the tree that matches that
candidate.

## Honest unqualified handoff

Missing S3 credentials, Graviton hardware, or other required
backend/target prerequisites remain **NotRun/unqualified**. Required
cells do not accept `NotApplicable` waivers for the advertised 1.0
envelope. Report the exact missing prerequisite instead of faking
Passed evidence.
