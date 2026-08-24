# 90 — Rollout

Self-contained dispatch. This set is senior to the numbered docs for
the pass's duration; the DOC work inside Lane X reconciles them at the
end. This file is the only grail file lanes edit (receipts below).
Ground rules as always: the house laws whole; never weaken a red test;
deletions counted and named; commit per deliverable in the house voice;
no publishes (owner ceremony). NETWORK: lanes S, N (crate fetch), and E
require registry/AWS access — this pass IS the network-enabled session
the receipts have waited for; where the sandbox blocks, the lane
prepares the exact command and the owner runs it.

## Lanes and order

```
Lane B (beauty: deep read + renames + types, both drivers) ──┐
Lane N (SDK 0.17.2: internalDescriptor + roster widening)  ──┼─► Lane S (stores: S3Store,
Lane C (CI workflow, amazonlinux container)                ──┘      aws4fetch, memStore, smokes)
                                                                        │
                                              Lane D (duty bin) ────────┤
                                                                        ▼
                                              Lane E (alchemy example, smoke deploy)
                                                                        │
                                              Lane X (docs, receipts, publish prep,
                                                      DELETE proposals/grail/)
```

- **Lane B** — 10 whole. Opens with the full deep read of both
  drivers; every finding fixed or recorded. The descriptor collapse
  waits on Lane N's export (disjoint files otherwise; B and N run in
  parallel). ts-log manifest to 0.18.0.
- **Lane N** — 20's SDK half: `internalDescriptor` in ts/crate + the
  hidden ts/src export; the two-platform roster widening with its
  tests; lockstep to 0.17.2 prepared unpublished.
- **Lane C** — 40 whole; the workflow lands early so every later lane's
  push exercises it.
- **Lane S** — 30 whole plus 20's Rust S3Store: written against the
  renamed surface; memStore; both gated smokes; the interop lane's
  s3 variant.
- **Lane D** — 20's duty binary, both modes; FsStore-backed tests
  in-repo; s3 wiring exercised by the gated smoke.
- **Lane E** — 50 whole: the example directory, the Alchemy program,
  one real deploy smoke with the owner present (commit, read, duty
  fire, cold-start numbers recorded in the example README).
- **Lane X** — amend the numbered docs (40-object-store gains memStore
  and the shipped S3 stores with boxes B and C closed; 70 gains the
  Lambda recipe pointer; PUBLISHING gains 0.17.2 and ts-log 0.18.0),
  re-issue proposals/90 receipts, verify the whole battery (both
  suites, check.sh, lean.sh, census, CI green on the tip), report the
  deletion tally, and `git rm -r proposals/grail/` in the closing
  commit whose message is the handoff.

## Acceptance checklist (receipts land here)

- [ ] B: deep read complete; renames landed whole (0.18.0); descriptor
      authority collapsed onto internalDescriptor; parse-don't-validate
      closures (StoreKey and the brand sweep); writer.rs split;
      findings ledger in the receipt
- [ ] N: internalDescriptor shipped; roster = {darwin-arm64,
      linux-arm64} with widened pins and tests; 0.17.2 lockstep
      prepared unpublished
- [ ] C: bumbledb-log.yml green on both jobs; artifacts attached;
      AL2023 container the only linux userspace
- [ ] S: S3Store + aws4fetch store + memStore landed; boxes B and C
      closed with the gated smokes run against a real bucket
- [ ] D: duty binary, --once and resident modes, tested over FsStore,
      smoked over s3
- [ ] E: example deployed once for real; UF (function URL) called from
      a Vercel host; duty fired by schedule; cold-start and commit
      latencies recorded
- [ ] X: numbered docs amended; receipts re-issued; battery whole;
      publish prep one command away; proposals/grail/ deleted; handoff
      written
