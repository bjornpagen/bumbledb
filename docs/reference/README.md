# docs/reference — the permanent documentation home

These pages are the permanent home the `final-solution/` proposal
contracts move into when the proposal retires (chapter 61/64: the
proposal is retired only after the selected work is complete and
qualified; retirement first moves the binding contracts and the complete
gate/audit inventory here, updates `scripts/release-results.mjs` to read
these paths, and only then removes the folder). Until that retirement,
**`final-solution/` remains normative** and these pages are skeletons plus
already-stable operational content — they never fork the contract.

## Pages

| Page | Content now | Receives at retirement |
| --- | --- | --- |
| [architecture.md](architecture.md) | The shipped product shape: crates, packages, one native runtime, log/AWS boundaries | final-solution 00–02, 10–13, 20–22, 30–31 (normative semantics) |
| [packaging.md](packaging.md) | Artifact roster, immutable staging, exact pins, handshake | final-solution 32 + C12 (formats/artifacts) with the F3-frozen physical choices |
| [deployment.md](deployment.md) | Supported targets, floors, envelope (PENDING F3 numbers), unsupported runtimes, cutover runbook | final-solution 33 deployment/envelope evidence + APP-04/05/06/07 records |
| [operations-runbook.md](operations-runbook.md) | Backup/restore/admin/erase procedures over the shipped admin API | final-solution 21/22 retention/recovery contracts + OPS gate evidence |
| [apple-silicon-performance.md](apple-silicon-performance.md) | Preexisting measured notes (historical) | chapter 40/41 measured decisions (P14's F3 reports) |

The obligation inventory (chapter 50 audit rows, chapter 70's 17 parents /
220 child families) moves here as a machine-readable inventory when the
release checker is re-pointed; until then the checker reads
`final-solution/50-*.md` and `final-solution/70-*.md` and this directory
must NOT carry a competing copy.

`audit/` is preserved permanently and is never subsumed by these pages.
