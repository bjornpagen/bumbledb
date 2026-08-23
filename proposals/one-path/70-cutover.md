# 70 — Cutover readiness

The pass ends where the consumer begins. Everything here leaves the
primer-spec cutover one command away from each thing it needs, and nothing
here publishes anything — releases are owner ceremony.

## Packaging (prepared, unpublished)

- The engine SDK moves to **0.17.1** in the lockstep (main == platform ==
  napi crate == engine == C ABI version fields; the C ABI GENERATION stays
  4 — nothing in the ABI moved). The 0.17.1 payload is exactly the
  `internalBlake3` exposure the TS driver's store consumes (one-path/30).
- ts-log stays 0.17.0 with peerDependency `^0.17.1`, correct the moment
  the SDK publishes.
- ts/PUBLISHING.md gains the 0.17.1 entry in the runbook's voice: the one
  export, its consumer (the ts-log store's etags), the unchanged ABI, and
  the note that 0.17.1 exists to unblock `@bjornpagen/bumbledb-log`'s
  first publish. The publish command sequence is the standing one; the
  owner runs it when ready.
- ts-log's own first-publish steps are written into the same runbook
  (platformless package, no napi half, `pnpm publish --no-git-checks`
  after the SDK lands) — one command away, never run by an agent.

## S3, only if the network is open this session

If crates.io and npm are reachable when the rollout runs: Lane S builds
`S3Store` over `object_store` (dual storage-class targets, the retry/
GET-verify law already in store.rs) and the TS aws4fetch store per 40/70,
runs the credential-gated smoke if credentials exist (loud-skip otherwise),
and checks 90's boxes B and C for real. If the registries are still
blocked, the boxes stay honestly unchecked and this section is the
receipt's one-line reason. Nothing else in the pass depends on this lane
either way.

## The handoff note (the pass's last artifact)

One file is NOT written — the house buried standing prose. The handoff is
the FINAL COMMIT MESSAGE, in the house voice, carrying:

- What changed, at the ruling level (one loss path; one store protocol;
  the knob funerals; the doc set's new shape with 15 gone).
- **The deletion tally, itemized** — the pass's headline metric.
- What remains open, each with its owner and trigger: S3 stores (network),
  the quantitative-algebra reopen trigger (one-path/10), TS-native
  checkpoint duty (one-path/30), the SDK 0.17.1 publish (owner ceremony).
- The cutover sentence: primer-spec's parallel scope loops start on
  deployment case 5 — openReplica/openWriter over fsStore per the 70
  recipe — with nothing in their path unbuilt.

## Primer-spec readiness checklist (what case 5 actually touches, verified last)

- The 70 local-fleet recipe compiles and runs against the unified store
  as written in the doc (the doc is the spec; the recipe is its test).
- The TS multi-process lane is green (one-path/30) — the property the
  consumer's whole deployment shape rests on.
- The double-mint K-conflict story lands the winner's row via the one
  path (existing e2e test, re-asserted post-cut).
- ErrContention's payload carries real determinants from the violation
  (one-path/10) — the consumer's repair loop reads it.
- The rotated-LMDB-dir sweep is in (one-path/40) — scope-loop processes
  restart often; their caches must not hoard.
