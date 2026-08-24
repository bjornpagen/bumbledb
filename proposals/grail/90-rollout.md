# 90 — Rollout

Self-contained dispatch. This set is senior to the numbered docs for
the pass's duration; Lane X reconciles them at the end. This file is
the only grail file lanes edit (receipts below). Ground rules as
always: the house laws whole; never weaken a red test; deletions
counted and named; commit per deliverable in the house voice; agents
never publish (the ceremony below is owner-run). NETWORK: lanes N
(crate fetch), S, and E require registry/AWS access — this pass IS the
network-enabled session the receipts have waited for; where the
sandbox blocks, the lane prepares the exact command and the owner runs
it.

## Lanes and order

```
Lane B (beauty: deep read + renames + types) ──┐
Lane N (SDK 0.17.2: internalDescriptor,      ──┼──► Lane S (stores: S3Store, aws4fetch,
        roster widening)                       │        memStore, gated smokes)
Lane C (CI: amazonlinux law, both workflows) ──┘            │
                                                Lane D (duty binary) ──┐
                                                                       ▼
                                            ★ OWNER CEREMONY: publish 0.17.2 (three
                                              packages, linux artifact from Lane C's
                                              run) and ts-log 0.18.0 ★
                                                                       │
                                                Lane E (Alchemy example, deployed
                                                        FROM THE REGISTRY)
                                                                       │
                                                Lane X (doc amendments, receipts,
                                                        battery, DELETE proposals/grail/)
```

The ceremony sits BEFORE Lane E on purpose: the example installs the
published packages like any consumer, so the deploy smoke is also the
end-to-end registry proof — loader roster, pack-time pins, peer ranges,
all exercised the way a stranger would.

- **Lane B** — 10 whole. Opens with the full deep read of both
  drivers; every finding fixed or recorded. The descriptor collapse
  waits on Lane N's export (disjoint files otherwise; B and N run in
  parallel). ts-log manifest to 0.18.0.
- **Lane N** — 20's SDK half: `internalDescriptor` in ts/crate + the
  hidden ts/src export; the two-platform roster widening with its
  tests; lockstep to 0.17.2 prepared unpublished.
- **Lane C** — 40 whole: the new `bumbledb-log.yml` AND the
  amazonlinux-law containerization of ci.yml's existing linux legs.
  Lands early so every later push exercises it; the linux-arm64
  artifacts come from its runs.
- **Lane S** — 30 whole plus 20's Rust `S3Store`: written against the
  renamed surface; `memStore`; both gated smokes on a real bucket; the
  interop lane's s3 variant.
- **Lane D** — 20's duty binary, both modes; FsStore-backed tests
  in-repo; the s3 target exercised by the gated smoke.
- **Ceremony** — owner-run, from PUBLISHING.md's 0.17.2 entry: download
  Lane C's linux artifacts, place, verify the tarball proof, publish
  platform packages then main then ts-log 0.18.0, tag.
- **Lane E** — 50 whole: the example directory, the Alchemy program,
  one real deploy with the owner present — registry install, commit
  and read through the function URL from a Vercel host, the duty event
  fired by the schedule, cold-start and commit latencies recorded in
  the example README.
- **Lane X** — amend the numbered docs (40-object-store gains memStore
  and both shipped S3 stores with boxes B and C closed; 70 gains the
  Lambda recipe pointer; PUBLISHING already carries 0.17.2 from the
  ceremony); re-issue proposals/90 receipts; run the whole battery
  (both suites, check.sh, lean.sh, census, both workflows green on the
  tip); report the deletion tally; `git rm -r proposals/grail/` in the
  closing commit whose message is the handoff.

## Acceptance checklist (receipts land here)

- [ ] B: deep read complete; renames landed whole (0.18.0); descriptor
      authority collapsed onto internalDescriptor; parse-don't-validate
      closures (StoreKey and the brand sweep); writer.rs split;
      findings ledger in the receipt
- [ ] N: internalDescriptor shipped; roster = {darwin-arm64,
      linux-arm64} with widened pins and tests; 0.17.2 lockstep
      prepared unpublished
- [ ] C: bumbledb-log.yml green on both jobs with artifacts attached;
      ci.yml's linux legs moved into the amazonlinux:2023 container;
      no Ubuntu userspace builds or tests anything, anywhere
- [ ] S: S3Store + aws4fetch store + memStore landed; boxes B and C
      closed with the gated smokes run against a real bucket
- [ ] D: duty binary, --once and resident modes, tested over FsStore,
      smoked over s3
- [ ] CEREMONY: 0.17.2 (main + two platforms) and ts-log 0.18.0
      published and tagged by the owner
- [ ] E: example deployed once for real from the registry; the
      function URL called from a Vercel host; duty fired by schedule;
      cold-start and commit latencies recorded
- [ ] X: numbered docs amended; receipts re-issued; battery whole;
      proposals/grail/ deleted; handoff written

## Lane C receipt

YAML landed; GitHub-hosted runners were not executed from this
machine, so the C checkbox stays unchecked until a real run is green
with artifacts attached.

Landing hashes:

- `16ad4b1572c7a0c510f7ca57e6f15afc6e723a05` — `bumbledb-log.yml`
  (linux-arm64 on `ubuntu-24.04-arm` + `amazonlinux:2023`; darwin
  owns only the bumbledb-log crate and the ts-log suite; artifacts
  `bumbledb.linux-arm64.node` and `bumbledb-log-duty`; S3 smoke
  loud-skips unless `BUMBLEDB_S3_SMOKE_BUCKET`, `AWS_ACCESS_KEY_ID`,
  and `AWS_SECRET_ACCESS_KEY` are set)
- `af87477368a2abf47b3db43e93c2d67f82773f8d` — the law applied
  whole: ci.yml check/sdk linux cells and c-abi.yml's linux cell
  keep `ubuntu-latest` as the host kernel and run every build/test
  step inside `amazonlinux:2023`

Deletion tally (4):

1. An Ubuntu userspace as a place that could have built the
   linux-arm64 artifacts (unrepresentable in bumbledb-log.yml)
2. The check lane's Ubuntu userspace
3. The sdk lane's Ubuntu userspace
4. The c-abi lane's Ubuntu userspace

What moved into amazonlinux:2023: ci.yml `check` linux, ci.yml
`sdk` linux, c-abi.yml linux, and the new linux-arm64 job. lean
and miri stay macos-latest (already not Ubuntu). darwin jobs stay
macos-latest.

S3 smoke gate: both bumbledb-log.yml jobs; loud skip naming the
missing variables; the crate's current `s3_smoke` test still
refuses when credentials are present (Lane S wires the store).

Un-containerizable linux legs: none. Darwin cannot be Amazon
Linux and is not Ubuntu.

Deviations and unverified packages (docker was not available on
this machine; the dnf map is from AWS AL2023 docs + reasonable
names, not a live `dnf` probe):

- Path filters on bumbledb-log.yml add `crates/bumbledb/**`,
  `crates/bumbledb-theory/**`, and `rust-toolchain.toml` so the
  deploy artifacts rebuild when their Rust inputs move. 40 named
  only crates/bumbledb-log, ts-log, ts, and the workflow itself.
- c-abi.yml is containerized even though 40 names only ci.yml,
  because "no Ubuntu userspace anywhere" is the acceptance line.
- linux-arm64 uses dnf `nodejs22` as 40 writes; ts/ and ts-log
  `engines` pin `node >=24`; the sdk linux cell keeps
  `actions/setup-node` at 24 inside the container so that battery
  does not change language. AL2023 also ships `nodejs24` per AWS
  docs — unused here.
- `crates/bumbledb-log/src/bin/duty.rs` is not in the tree (Lane
  D). The linux-arm64 job will fail at `cargo build --bin duty`
  until that binary lands. Not skipped.
- dnf package map used: `gcc`, `gcc-c++`, `make`, `binutils`,
  `git`, `tar`, `gzip`, `xz`, `unzip`, `python3` (check.sh
  `flame.py` selftest, stdlib-only), `sed`, `gawk` (check.sh
  `filtered_test`), `diffutils` (c-abi cbindgen diff),
  `findutils`, `ca-certificates`, `curl`, `nodejs22`,
  `nodejs22-npm`, then `alternatives --set node /usr/bin/node-22`.
  Unverified against a live image: `nodejs22-npm`, `binutils`,
  `xz`, `unzip`, `sed`, `gawk`, `diffutils`, `findutils`, the
  `alternatives` path, and whether `nodejs22` ships `corepack`
  (40 asserts it does; the step is `corepack enable pnpm`).
- rustup is installed in-container via rustup.rs (`--default-toolchain
  none`); not a dnf package. Network required on the runner.
