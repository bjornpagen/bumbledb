# docs/reference — permanent documentation home

These pages are the durable home for release contracts, operational
runbooks, and the machine-readable obligation/evidence inventory.
Executable tooling reads only this directory. The disposable
`final-solution/` packet is not an input to checkers, batteries or
inventory once the coordinator retires it.

## Release contracts

| Artifact | Role |
| --- | --- |
| [semantics.md](semantics.md) | Canonical denotation, F64/intervals, wire/proof boundary (Lean/bridge scope + independent oracles) |
| [api.md](api.md) | Public Rust/TS/log vocabulary, ownership, packed-consumer expectations |
| [performance.md](performance.md) | Meaning home: representation, hashing, bound L20 13-cell plan (`appperf::plan`) |
| [behavioral-obligations.md](behavioral-obligations.md) | 68 audit IDs, 78 prior-review IDs, 220 child families |
| [release-gates.md](release-gates.md) | G00–G16, D01–D29, runner order, evidence identity |
| [qualification-checklist.md](qualification-checklist.md) | Post-retirement candidate capture and exact commands |
| [final-solution-retirement.md](final-solution-retirement.md) | Transfer-then-retire-then-qualify order (not checks-then-retire) |
| [obligation-inventory.json](obligation-inventory.json) | Machine roster: 68 + 17 + 220 + 78 + 29 + required cells |
| [release-results.schema.json](release-results.schema.json) | Format v2 index schema |
| [release-results.json](release-results.json) | Populated only from real final qualification; absent file fails closed |

Actual run reports, tarball digests, and platform/backend logs stay in
CI/release artifacts. They are referenced by hash/path from the small
index above, not checked into the repository as an exhaust tree.

Qualification identity uses three digests:

1. **`candidateSourceDigest`** — SHA-256 of the deterministic
   tracked+untracked source inventory, framing path/kind/mode/payload
   (`node scripts/release-results.mjs --candidate-digest`).
2. **`specificationRevision`** — SHA-256 of the canonical obligation
   inventory content.
3. **Artifact/report SHA-256** — exact built outputs bound in each
   evidence row.

The index excludes itself from the candidate source preimage. Optional
`sourceRevision` records the final commit object name in handoff
metadata only; it is not required before the single integrated commit.

Checker usage:

```sh
node scripts/release-results.mjs --inventory
node scripts/release-results.mjs --candidate-digest
node scripts/release-results.mjs --specification-revision
node scripts/release-results.mjs --write-native-provenance
node scripts/release-results.mjs --verify-native-provenance
node scripts/release-results.mjs pre-promotion [manifest.json] [candidate-digest]
```

A successful `scripts/battery.sh` exit is not all-platform qualification.

## Product and operations pages

| Page | Content |
| --- | --- |
| [architecture.md](architecture.md) | Shipped product shape: crates, packages, one native runtime, log/AWS boundaries |
| [packaging.md](packaging.md) | Artifact roster, immutable staging, exact pins |
| [deployment.md](deployment.md) | Supported targets, floors, deployment runbook |
| [operations-runbook.md](operations-runbook.md) | Backup/restore/admin/erase over the shipped admin API |
| [apple-silicon-performance.md](apple-silicon-performance.md) | Historical measured notes (pre-1.0 attribution preserved) |

`audit/` is preserved permanently and is never subsumed by these pages.
