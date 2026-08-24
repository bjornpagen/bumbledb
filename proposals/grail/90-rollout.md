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
Lane N (SDK 0.17.2: internalDescriptor,      ──┼──► Lane S (stores: S3Store, official
        roster widening)                       │        S3 client, memStore, gated smokes)
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
  renamed surface; official `@aws-sdk/client-s3` (owner killed
  aws4fetch); `memStore`; both gated smokes; the interop lane's s3
  variant.
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

- [x] B: deep read complete; renames landed whole (0.18.0); descriptor
      authority collapsed onto internalDescriptor; parse-don't-validate
      closures (StoreKey and the brand sweep); writer.rs split;
      findings ledger in the receipt — 3a128011, 21b333d4, dc5f7b6f,
      d051165b, 520403db, 205729f5; receipt below
- [x] N: internalDescriptor shipped; roster = {darwin-arm64,
      linux-arm64} with widened pins and tests; 0.17.2 lockstep
      prepared unpublished — 2ddbf4bb, 42e40ec2, 9c577672; receipt
      below
- [ ] C: bumbledb-log.yml green on both jobs with artifacts attached;
      ci.yml's linux legs moved into the amazonlinux:2023 container;
      no Ubuntu userspace builds or tests anything, anywhere
- [x] S: S3Store + official S3 client + memStore landed; Refresh kept
      with the census pin; Node floor 24; gated smokes loud-skipped on
      this machine (no credentials); receipt below
- [x] D: duty binary, --once and resident modes, tested over FsStore,
      smoked over s3 (loud-skip on this machine) — 53fb9e8b; receipt
      below
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

## Lane N receipt

internalDescriptor is exported. Lane B calls it:

```
import { internalDescriptor, lower } from "@bjornpagen/bumbledb"
const sealed = internalDescriptor(lower(theory))
```

Napi: `descriptor(spec)` (doc-hidden). TS wrapper: `internalDescriptor(spec: SchemaSpec): SealedDescriptor` from `@bjornpagen/bumbledb` / `#native.ts`, the internalBlake3 precedent (bridged, `@internal`, undocumented). Return shape: `{ relations, statements, fingerprint }` — relations are the engine Manifest relations (ids, sealed field order, closed extension rows with resolved axioms); statements are structured `StatementDescriptor`s in engine materialization order (fresh keys, closed auto-keys, then declared, `==` already split); fingerprint is the hex of the real engine digest, equal to `dbFingerprint` of a store created from the same spec. No store opens. Spec errors throw the schema family.

Landing hashes:

- `2ddbf4bb3a8fab4571a86035fc4efbd668b95937` — internalDescriptor
- `42e40ec271d55f09d79a103f4c2f5faa1ccfaa59` — roster {darwin-arm64, linux-arm64}
- `9c57767294cd7387d2cbe24523a09f2fb102a6df` — lockstep 0.17.2 unpublished

Deletion tally (5):

1. The duplicated Manifest relation-object walk (one helper now serves dbManifest and the sealed export)
2. The PUBLISH_PLATFORM singleton
3. The host-conditional foreign picker that treated linux-arm64 as unshipped
4. The 0.17.1 current-tree spellings
5. The two-package runbook that could not name linux-arm64

Suite at the lockstep commit: 403 green, `tsc --noEmit` clean, `biome check` clean. C ABI stays generation 4. linux-arm64 ships LICENSE + package.json (AL2023 baseline in the description); the `.node` is Lane C's artifact, not minted here.

Deviations:

- The lockstep gate compares engine workspace crates and bumbledb-c, so those version fields moved with the 0.17.1 precedent. ts-log's peer and crates/bumbledb-log were left for Lane B.
- `cargo update --offline` in the isolated napi and C lockfiles tried to move third-party crates; those lockfiles were restored and only the workspace package versions rewritten, matching the 0.17.1 lockfile shape.
- pnpm's pre-run dep check aborted without a TTY; the suite ran through `node scripts/build.ts`, `node --test`, `tsc --noEmit`, and `biome check` as the brief allows.

## Lane B receipt

Six landing hashes, both suites green after every one (ts-log 87;
rust `cargo fmt --check && cargo clippy --all-targets -- -D warnings
&& cargo test --manifest-path crates/bumbledb-log/Cargo.toml` whole).
Descriptor collapse is LANDED, not WAITING-ON-N — N's
`internalDescriptor` was already in `ts/src` (`2ddbf4bb`) when this
lane reached that item.

Landing hashes:

- `3a128011360b8d186607e49ccebd38e3d6be0394` — naming law: Log-prefix
  stutter dies (`LogValue`/`LogInterval`/`LogBatch`/`LogTheory`/
  `LogDescriptor`/`logValueOf`/`BatchOp`/`ChainPosition`/`PendingBatch`)
- `21b333d4e1ca884bbf2943e58db8ca8b07453b16` — parse-don't-validate:
  branded `StoreKey`/`Generation`/`Etag`/`Braid`; verbs take the proof
- `dc5f7b6f10a6519e6c63b85be8edac0d99a91eb3` — `writer.rs` (2072)
  becomes the `writer/` family (batch, discipline, drain, loss,
  pending, duty, open)
- `d051165b2db325bac5efb9842d591c4674118f79` — pre-one-path vocabulary
  dies (footprint, republish*, subsume*)
- `520403dbecad8543b1e7584cd10e23238efe9f1d` — ts-log 0.18.0, peer
  `^0.17.2`, README rewritten to the new names
- `205729f54c6c8e02f1bf017f2bb949b183ccfe3a` — descriptor collapse onto
  `internalDescriptor`

Deletion tally, itemized (25 named units across the six commits):

1–9. `LogValue`, `LogInterval`, `LogBatch`, `LogTheory`,
   `LogDescriptor`, `logValueOf`, `BatchOp`, `ChainPosition`,
   `PendingBatch`
10–14. `checkKey`; per-verb key re-validation; the codec's second
   `/^c([0-9a-f]{8})$/` braid grammar; primitive string keys on the
   five verbs; primitive etag/braid/generation on the TS public
   surfaces
15. The 2072-line `writer.rs` monolith (1 file)
16–18. `footprint`, `republish*`, `subsume*` (intersect, linger,
   `max_pending` were already unrepresentable)
19–21. The 0.17.0 manifest spelling; the `^0.17.1` peer; the
   README's Log-era silence about the brands
22–25. `fingerprintOf`; the string-axiom refusal; the
   `bumbledb-schema-v5` re-encoder; the intern-id gap that refusal
   papered over

Deep-read findings (fixed or recorded):

- Fixed: empty-prefix key assembly produced a leading slash on `""`
  (illegal); now matches Rust (omit the slash).
- Fixed: two braid parses (codec regex vs `braidHex`) collapsed onto
  `braid`/`braidHex`.
- Fixed: `checkKey` discarded the proof; `StoreKey` is parsed once.
- Fixed: Log-prefix stutter; one vocabulary across languages.
- Fixed: pre-one-path words on the owned surfaces and the numbered
  writer/conformance docs those surfaces spoke.
- Fixed: `writer.rs` monolith split along the real seams.
- Fixed: theory fingerprint mirror; `descriptorOf` is engine truth.
- Keep: TS `DecodedBatch` vs Rust `codec::Batch` — more precise noun
  wins; do not export two `Batch` types from `index.ts`.
- Keep: TS `Op.op` (`"insert" | "delete"`) vs Rust `Op.kind` —
  tagged-union idiom vs enum.
- Keep: Rust generation stays `u64` — a newtype that only wraps u64
  is a stutter; TS `Generation` is the bigint range proof.
- Keep: `Sweep.log_deleted: Vec<String>` — a display list after
  delete consumed the proof.
- Keep: `Etag` is wrap-only (HTTP vendors carry tags verbatim); no
  grammar.
- Keep: `replica.ts` (~796) unsplit. The seams (apply/catch-up,
  manifest, store lifecycle, pending recovery, open+handle) are real
  as function clusters but they share one `Core` and one gate;
  splitting would add import surface without making a state
  unrepresentable. The temporal-gate test reads `replica.ts` by
  filename.
- Keep: `assembleFromSpec` — the conformance corpus is not a theory
  (containments onto unkeyed projections, empty serial statements);
  the engine seal refuses it. Fixture-only, not exported from
  `index.ts`. Production `descriptorOf` never calls it.

Deviations from grail/10:

- `descriptor.ts` is 710 lines, not a thin file, because
  `assembleFromSpec` remains for corpus goldens. The deleted
  authority is the fingerprint mirror, not the fixture assembler.
- `replica.ts` reviewed and kept whole (split-only-if-seams-demand).
- `crates/bumbledb-log` crate version stays 0.17.0 — the mandated
  floor named ts-log 0.18.0, not the Rust crate.
- Numbered 40/70 were not amended for `StoreKey`: they did not show
  `get(key: string)`. 60 and 80 were amended where they still spoke
  republish/subsume as current vocabulary.

WAITING-ON-N: no. Collapse landed in `205729f5`.

Paths this lane changed: `ts-log/src/**`, `ts-log/test/**`,
`ts-log/package.json`, `ts-log/README.md`,
`crates/bumbledb-log/src/{store.rs,store/fs.rs,gc.rs,lease.rs,manifest.rs,tenants.rs,writer.rs→writer/**}`,
`crates/bumbledb-log/tests/**` (wrappers and call sites),
`crates/bumbledb-log/conformance/corpus/schemas.json`,
`crates/bumbledb-log/Cargo.lock` (path-dep 0.17.2 re-pin),
`proposals/60-writer.md`, `proposals/80-conformance.md`,
`proposals/grail/90-rollout.md` (this receipt). Did not touch `ts/`,
`.github/`, `duty.rs`, `examples/lambda/`, `lean/`.

## Lane S receipt

The three stores landed. aws4fetch shipped in `12be9118` and the
owner killed it; the official `@aws-sdk/client-s3` client is the
TS signer. Refresh is kept. Node is 24. The gated smokes loud-skip
on this machine.

Landing hashes:

- `f6c338e0` — Rust `S3Store` over `object_store` 0.14.1
- `12be9118` — TS aws4fetch store (owner-killed next)
- `7ada883d` — official client; aws4fetch and `objectUrl` die
- `bc7ef05b` — Refresh kept in both languages; census pin
- `06f767f2` — Node floor 24 in engines, runbook, and CI
- `44e69915` — multi-thread runtime; construct-outside-async
- `6de97425` — `memStore` / `MemStore`; fsync tax dies
- `ff097be2` — gated smokes + s3 interop read

aws4fetch: fully gone after `7ada883d`. `pnpm-lock.yaml` carries
`@aws-sdk/client-s3` only.

Refresh was deleted in the working tree only, never committed. The
keep commit is `bc7ef05b`. Both-language API is one credentials
sum: static keys (id, secret, optional session token) | refresh
callback. Rust: `S3Credentials::Static { … } | Refresh(Arc<dyn Fn()
-> io::Result<StaticKeys> + Send + Sync>)`, plus the
`RefreshProvider` field and the boxed future
`CredentialProvider` forces. TS: `StaticKeys | (() => StaticKeys |
Promise<StaticKeys>)` on the official client — not the SDK default
provider chain. Exemption site: `scripts/spec-census.sh`, three
exact lines in `crates/bumbledb-log/src/store/s3.rs`, reason
attached: caller-owned credential behavior at a foreign async-trait
boundary; cold path. Zero other log-driver dyns.

Engines / CI Node spellings (one answer: 24):

- `engines.node` is `>=24` in `ts/`, `ts/npm/darwin-arm64`,
  `ts/npm/linux-arm64`, and `ts-log`
- Runbook (`ts/PUBLISHING.md`) and `ts-log/README.md` state Node
  >=24 for the `.ts` test runner and build scripts
- `bumbledb-log.yml` arm job and both `ci.yml` linux cells install
  `nodejs24` / `nodejs24-npm` and `alternatives --set node
  /usr/bin/node-24`
- Darwin already `node-version: 24`
- `c-abi.yml` still installs `nodejs22` — Lane C's file, not taken
- E-prep (grail/50) must use the newest AL2023 Node runtime Lambda
  offers, which must be >=24 so engines admits it

Runtime ruling (F3): keep multi-thread. The writer's publisher
(`drain.rs` `spawn_publisher`) and duty thread call store verbs on
other OS threads; a current-thread runtime cannot drive two
`block_on` callers. `Builder::new_multi_thread().enable_all()`
names the choice. Construct `S3Store` outside an async context —
`Handle::try_current` at `new()` is a typed refusal. Reopen: a
consumer that only ever calls verbs from one thread and measures
the extra workers as cost.

Dep-weight ruling (F4): ACCEPT `object_store` + `tokio` as
unconditional crate deps. One way to build the crate; no feature
matrix. The embedded use case's TS consumers never compile this
crate. Reopen: an embedded Rust consumer that measures the build
cost.

Keep-ledger:

- Refresh KEPT; dyn is the honest spelling of caller-owned creds;
  exemption pinned with the reason above; reopen trigger is not
  needed for Refresh itself (it is in scope). dyn-for-caller-owned-
  credentials is the keep spelling.
- Multi-thread runtime + construct-outside-async (F3, above).
- `object_store` + tokio unconditional (F4, above).
- `BUMBLEDB_S3_SMOKE_REGION` defaults to `us-east-1` when unset so
  C's three-var gate still runs. Reopen: a smoke target whose
  region is not us-east-1 and whose env omits the region var.

Env contract (exact names):

- Required: `BUMBLEDB_S3_SMOKE_BUCKET`, `AWS_ACCESS_KEY_ID`,
  `AWS_SECRET_ACCESS_KEY`
- Optional: `BUMBLEDB_S3_SMOKE_REGION` (default `us-east-1`),
  `BUMBLEDB_S3_SMOKE_ENDPOINT`, `AWS_SESSION_TOKEN`

Smoke status: loud-skip on this machine (all three required vars
absent). Tests are named `s3_smoke*` so CI `cargo test … s3_smoke`
matches. They never fail without credentials. A live bucket was
not exercised here.

Close gates on this tree: `scripts/spec-census.sh` green (zero-dyn
exemption pinned: Error::source 3, credential refresh 3);
`scripts/check.sh` green; `scripts/lean.sh` green (277 conformance
cases, three-way comparator). A green suite with a red census is
a red tree — this close is green on all three.

memStore: landed. Single-process honesty stated where declared.
Third `Etag` producer (blake3 like `fsStore`; the brand is the
contract). Rust migrated 11 store-semantic / retry-law tests off
`FsStore` tempdirs. TypeScript migrated the five-verb semantics
plus replica-writer, recovery, and tenants. Multiprocess stays on
disk.

Deletion tally addendum (S):

1. aws4fetch (dep, import, every call)
2. `objectUrl` (the fetch-signer URL assembler)
3–5. `nodejs22` / `node-22` install spellings (bumbledb-log.yml
   arm + two ci.yml linux cells)
6. Implicit `Runtime::new()` (named as multi-thread)
7. The fsync tax on 22 test bodies that never touched a disk
8. The f11 pin that asserted credentials were absent and failed
   when they were present

Deviations:

- grail/30 named aws4fetch; the owner deleted that part.
- grail/30 named a refresh; the owner reversed a deletion and
  kept the arm.
- grail/50 still says Node 22 for Lambda — E-prep, not this lane;
  engines will refuse a 22 runtime.
- Live S3 smoke did not run (no credentials).
- `c-abi.yml` still spells nodejs22.

Blockers for D: duty execs `FsStore` in-repo and S3 via this
gate. The smoke env names above are the contract. Duty must
construct `S3Store` outside an async context. Refresh is in
scope if duty ever needs it. `c-abi.yml` nodejs22 is C's leftover
against the 24 floor.

Paths this lane changed: `crates/bumbledb-log/src/store.rs`,
`crates/bumbledb-log/src/store/{s3.rs,mem.rs}`,
`crates/bumbledb-log/tests/{s3_smoke.rs,lane_b_mem_store.rs,lane_b_fs_store.rs,f11_pins.rs}`,
`ts-log/src/{store.ts,store-s3.ts,index.ts}`,
`ts-log/test/{store.test.ts,s3-smoke.test.ts,interop-child.ts,recovery.test.ts,replica-writer.test.ts,tenants.test.ts}`,
`ts-log/package.json`, `ts-log/pnpm-lock.yaml`, `ts-log/README.md`,
`ts/PUBLISHING.md` (Node 24 sentence), `.github/workflows/{bumbledb-log.yml,ci.yml}`
(F2 node-version lines only), `proposals/40-object-store.md`,
`proposals/grail/90-rollout.md` (this receipt),
`scripts/spec-census.sh` (Refresh pin), `lean/Bumbledb/Bridge.lean`
(one census token path: `Writer::clear_pending` now lives in
`writer/discipline.rs` after B's split). Did not touch `ts/src`,
`duty.rs`, `examples/lambda/`, `c-abi.yml`.

## Lane C leftover

Lane S raised bumbledb-log.yml and both ci.yml linux cells to
nodejs24 and left `c-abi.yml` on nodejs22. That cell now matches:
`nodejs24`, `nodejs24-npm`, `alternatives --set node
/usr/bin/node-24`. After this commit no Ubuntu-or-AL2023 workflow
installs Node 22.

Landing hash: this commit (c-abi.yml + this note). Named deletions:
the three c-abi.yml spellings `nodejs22`, `nodejs22-npm`,
`/usr/bin/node-22`.

Leftovers this hop could not touch:

- grail/40 still writes `nodejs22` — context-only; Lane X amends
  numbered docs
- this file's original Lane C receipt still records the old dnf
  map — historical, not a live install
- grail/50 still names Node 22 for Lambda — E-prep
- ci.yml's `R22` is a ruling number, not a Node version

## Lane D receipt

The duty binary landed. `--once` and the resident loop share one
body. The prefix opens as a checkpointer: replica plus compact,
checkpoint-order publish, and the retention sweep — no commits, no
leases. Cadence is `CHECKPOINT_EVERY_SUM` / `CHECKPOINT_EVERY_BYTES`.
Retention is `CHECKPOINT_RETAIN_MS` (ninety days), one owner, census
lane (j). S3Store is constructed outside async from static env keys.

Landing hashes:

- `53fb9e8bad8ddc1f1dedfe7b4a6bc518aa8ef259` — `duty` binary, the
  `Checkpointer` body, the theory-file parse, FsStore tests, the S3
  duty smoke behind Lane S's gate, `CHECKPOINT_RETAIN_MS`

Deletion tally (3):

1. A Writer-shaped duty (commits and leases on the sidecar)
2. A second pair of cadence literals
3. An unnamed ninety-day retention window

Keep-ledger:

- The theory file. 20 named `--once` and the bucket args and did
  not name how a standalone process obtains a theory. Replica open
  requires one. The file is the crate's existing corpus schema
  object (`{relations, statements}`), parsed once at the boundary
  into a descriptor. Reopen: a host that cannot write that object.
- `serde_json` as a crate dependency (it was a dev-dep). The
  theory-file parse is the consumer.
- Refresh unused. Lambda/`--once` is static keys from
  `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` / optional
  `AWS_SESSION_TOKEN`. Reopen: a duty that must rotate.
- Resident sleep defaults to five minutes (`--sleep-ms`). That is
  50's schedule starting point, not a protocol constant.
- `--writer` defaults to 0. The checkpoint document still carries
  a publisher id.

`duty.rs` is 221 lines (argv parse, store construct, the loop).
The body itself is `Checkpointer::run` over existing replica,
`publish_checkpoint`, and `gc`.

Smoke status: loud-skip on this machine (all three required vars
absent). Test `s3_smoke_duty_once` is named `s3_smoke*` so CI
`cargo test … s3_smoke` matches. It never fails without credentials.

Close gates on this tree: `scripts/spec-census.sh` green (104
ledger rows; `CHECKPOINT_RETAIN_MS` single-sited; zero-dyn
exemption unchanged); `scripts/check.sh` green; `scripts/lean.sh`
green (277 conformance cases, three-way comparator). A green
suite with a red census is a red tree — this close is green on
all three.

Deviations:

- 20/`--once` and 50's "bucket args" did not name `--theory`,
  `--dir`, `--store`, `--root`, `--writer`, or `--sleep-ms`. Those
  flags are the parse; `--theory` is the gap fill above.
- `duty.rs` is 221 lines, not ~100: the parse is the extra, not a
  second compact path.
- Live S3 duty smoke did not run (no credentials).

Blockers for E-prep (the handler `execFile`s the bundled binary
with `--once`):

- Exact argv: `duty --once --store s3 --bucket $BUCKET --dir DIR
  --theory PATH [--prefix P] [--region R] [--endpoint E]
  [--s3-prefix KEY] [--writer N]`
- `--theory PATH` is the corpus schema object matching the app
  theory's fingerprint. There is no other way to open.
- Credentials are static from the process env, not the default
  provider chain and not a role. Refresh is unused.
- Construct is in the child process (sync `main`), so the
  construct-outside-async law holds without the handler's help.
- Node on the function must be >=24 (engines). grail/50 still
  says Node 22 — E picks the newest AL2023 runtime Lambda offers.
- Do not check CEREMONY or E-deploy from this lane.

Paths this lane changed: `crates/bumbledb-log/src/bin/duty.rs`,
`crates/bumbledb-log/src/{checkpointer.rs,schema_file.rs,lib.rs,
replica.rs,gc.rs,writer/mod.rs}`, `crates/bumbledb-log/Cargo.toml`,
`crates/bumbledb-log/tests/{duty.rs,s3_smoke.rs}`,
`scripts/spec-census.sh` (`CHECKPOINT_RETAIN_MS`),
`proposals/grail/90-rollout.md` (this receipt). Did not touch
`ts/`, `ts-log/`, `.github/`, `examples/lambda/`, `lean/`,
`store*` beyond what the binary already consumes.

## Lane E-prep receipt

Code only. No deploy, no `alchemy deploy` / `plan` / `destroy` /
`aws bootstrap`, no publish, no tag, no push. CEREMONY and E-deploy
stay unchecked.

Landing hashes:

- `9292a78cf505772341c40d09c45c21280f4ef669` — `alchemy.run.ts` +
  package pins
- `b47e4bb031909be89da7b0ec31a724bca6296726` — handler + theory /
  layer layout
- `48a38e0b6b48a8fb0b968c423d18a51b823dbf12` — README honesty
- this commit — this receipt

Deletion tally (1):

1. Node 22 as a Lambda runtime this example could have picked.
   Owner ruling: Node >=24; Alchemy's union is only `nodejs22.x |
   nodejs24.x` and its default is 22 — the program sets 24. 50's
   "Node 22" line is left for Lane X (this lane may edit only this
   file among grail docs). Node 26 is in preview; Alchemy cannot
   type it.

Keep-ledger:

- One function, two event arms. 00-scope IN#7's "two Lambdas"
  loses to 50 and the README: Scheduler `{ duty: true }` hits the
  same Function URL handler. No second function, no custom runtime.
- The IAM `Fn` role as the intended document: prefix
  GetObject / PutObject / DeleteObject, no ListBucket. Unattached.
- Standard-class S3; versioning omitted (not Suspended); no
  lifecycle rules.
- LayerVersion as the duty-binary representation. Rolldown has no
  `extraFiles`; `layer/duty/bin/bumbledb-log-duty` extracts to
  `/opt/bin/bumbledb-log-duty`.
- Registry pins, not workspace/link: `@bjornpagen/bumbledb@0.17.2`,
  `@bjornpagen/bumbledb-linux-arm64@0.17.2`,
  `@bjornpagen/bumbledb-log@0.18.0`.
- `alchemy@2.0.0-beta.74` + `effect@rc` (Effect 4 —
  `effect@latest` is 3.22.1). Official flavor: Effect Stack, not
  v1 async/await.
- Refresh unused. Credentials are static from process env.
- construct-outside-async: TS `s3Store` at module scope; duty
  constructs `S3Store` in the child.
- `architecture: "arm64"` (Alchemy default is x86_64).
- Timeout `Duration.seconds(60)` (Alchemy default 3 s is too
  short for `duty --once`).
- The six named resources and no extras: bucket, role-as-intent,
  Lambda, function URL, Scheduler, duty Layer. No VPC, no alarms.

IAM REPORT (Alchemy 2.0.0-beta.74, live types):

- `AWS.Lambda.Function` always mints its own role. No `roleArn`.
- `S3.GetObject` (and Put / Delete) as a binding always adds
  `s3:ListBucket` on the whole bucket and cannot prefix-scope.
- This program does not yield those bindings. The handler uses
  `s3Store`. The `Fn` role's inline `Prefix` policy is the
  intended document and is not the execution role.
- `Scheduler.every().toLambda` synthesizes a second invoke role
  so Scheduler can call Lambda. AWS-required plumbing, not the
  function execution role.

Owner chooses later (not this lane, not deploy):

1. Accept the derived Function role and the ListBucket leak.
2. Wait for Alchemy `roleArn` (or an attach) so the intended
   prefix-only role can be the execution role.
3. Inject a separate IAM user key via env.

Until one of those three, a deploy with the derived role and no
extra policy is `AccessDenied` on every store verb.

Duty argv the README tells the owner:

```
/opt/bin/bumbledb-log-duty \
  --once --store s3 --bucket $BUCKET --dir /tmp/duty \
  --theory /opt/bin/theory.json --region $AWS_REGION \
  --s3-prefix $PREFIX
```

`--theory` is required. The file is the crate corpus object
`{relations, statements}` matching D's `note` fixture (u64 id,
string body, functionality on field 0). Optional unused:
`--prefix` (empty; the store prefix already scopes), `--endpoint`,
`--writer` (defaults to 0). Owner places the linux-arm64 duty
binary at `layer/duty/bin/bumbledb-log-duty` (`+x`) before deploy.

Typecheck: `alchemy.run.ts` is clean against
`alchemy@2.0.0-beta.74` + `effect@4.0.0-rc.111`. Handler
typecheck is a ceremony skip — `0.17.2` / `0.18.0` 404 on the
registry (`@bjornpagen/bumbledb-linux-arm64` first). No live AWS
account was required or used.

Deviations from 50:

- Node 24, not 22 (owner ruling; 50 still writes 22).
- LayerVersion (50 said "binary as a packaged executable file";
  Rolldown cannot extraFile; the Layer is the representation that
  makes that sentence true).
- `--theory` (D's gap fill; 50 said "bucket args").
- Alchemy v2 Effect, not v1 `await alchemy(...)`.
- Official `@aws-sdk/client-s3` via `@bjornpagen/bumbledb-log`
  (owner killed aws4fetch).
- Role is inline-policy intent, not attached. 50's "one IAM
  role for the function, minimal, no ListBucket" is the document
  we wrote and cannot bind.
- `PREFIX=log` is the s3Store / `--s3-prefix` key prefix; the
  replica protocol prefix is empty so keys are not `log/log/…`.
- Stack output also names `intendedRoleArn` so the unattached
  document is visible.
- No deploy smoke, no Vercel call, no schedule fire, latency
  blanks stay `(owner smoke)`.
- Numbered 70 was not amended (X-prep).

Blockers for X-prep / E-deploy:

- CEREMONY must publish 0.17.2 (three packages) and ts-log
  0.18.0 before this example can install from the registry.
- Owner places `bumbledb-log-duty` in the layer path.
- Owner chooses IAM (1) / (2) / (3) above.
- Owner runs `alchemy aws bootstrap` (this lane did not).
- Owner smoke fills the two latency blanks.
- 50 still says Node 22 — X amends numbered docs, not this lane.
- Do not check CEREMONY or E-deploy from this lane.

Paths this lane changed: `examples/lambda/**`,
`proposals/grail/90-rollout.md` (this receipt). Did not touch
`crates/`, `ts/`, `ts-log/`, `.github/`, `lean/`,
`night-2026-08-22/`, `docs/research/`, `docs/free-join-paper/`,
numbered proposals except this grail receipt file.
