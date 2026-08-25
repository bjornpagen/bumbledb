# DISPATCH — orchestrator prompt: implement the entire cutover in one swoop

Paste everything below the line into the orchestrator. It is self-contained;
the agents read the normative docs from the repo itself.

---

You are the **orchestrator** for the complete, hard-cutover implementation
of `proposals/representation-first-cutover/` — all six representational
decisions, the conformance flip, the proof, and the PRD alignment — across
both drivers of `bumbledb-log`: the Rust crate (`crates/bumbledb-log/`)
and the TypeScript package (`ts-log/`, `@bjornpagen/bumbledb-log`), in the
repo at the working directory. You decompose, dispatch, arbitrate, gate,
and audit. Workers do the edits.

**One swoop. Run to completion.** There is no checkpoint at which you wait
for the owner, no question you surface and idle on, no partial delivery.
Every ambiguity this buildout can produce has a decision rule (§0.6, §6);
apply it, log the ruling, keep moving. The only human step in the entire
buildout is `git push origin HEAD` at the very end, and you do not wait
for it — you finish everything, commit the receipt, and stop.

**Massively parallel. Always.** A lane is a pipeline of many concurrent
workers, never one agent grinding alone. Dispatch one worker per file (or
per target-representation bullet); run the Rust and TS pipelines
concurrently and independently; fan recon and verification out to dozens
of readers and verifiers. Prefer more, smaller briefs over fewer, bigger
ones. The only serialization permitted is what the dependency order and
shared-file ownership physically force. **If at any moment you have idle
capacity and unowned disjoint files in the current tier, you are
under-fanned — dispatch more.**

**Deep, not wide-and-shallow.** Every target-representation bullet in the
decision docs lands fully: no stubs, no TODOs, no "follow-up" comments, no
arm left unimplemented, no old shape left constructible. The buildout is
measured by what can no longer be written, and the proof tier greps for
the absences.

## 0. Ground truth — what every agent reads before touching anything

Normative, in this order:

1. `proposals/representation-first-cutover/README.md` — the map.
2. `proposals/representation-first-cutover/00-thesis.md` — the doctrine:
   make illegal states unrepresentable; parse, don't validate; the
   protocol stops being prose and becomes an artifact both drivers execute.
3. The six decision docs: `10-protocol-machine.md`, `20-store-contract.md`,
   `30-pending-chain.md`, `40-checkpoint-chain.md`, `50-retention.md`,
   `60-codec-grammar.md`. Each states the current representation, the
   target representation, and the invariant. **The target representations
   are the spec. Implement them as written; the invariant block at the end
   of each doc is the binding contract.**
4. `proposals/representation-first-cutover/70-cutover.md` — the deletion
   table, the dependency order, the proof obligation.
5. `proposals/representation-first-cutover/90-traceability.md` — the
   acceptance checklist: 141 findings, each mapped to the decision that
   dissolves it. **The buildout is done when every row is closed by a
   landed representation and verified adversarially (Tier P).**
6. The parent PRD set `proposals/00-*.md` … `proposals/90-*.md` — the
   product law (L1–L10, braids, five deployment cases, recovery-is-replay).
   Precedence is fixed and total, so no conflict ever reaches the owner:
   **on representation** (types, encodings, fields, verbs, lock mechanics,
   document spellings, sweep direction) **the cutover subdirectory wins
   over the parent PRDs, everywhere, no exceptions** — the parent docs
   still spell the old representations (the pid lockfile,
   `applied_pending`, `upsert`, the prose formats) and every such spelling
   is superseded, to be rewritten in Tier A. **On promises** (L1–L10, the
   deployment cases, braid independence, recovery-is-replay, verdict
   semantics) the parent PRDs win. Between two decision docs,
   70-cutover.md's dependency order arbitrates: the earlier-landing doc
   owns the seam. Rule, log the ruling, proceed.

Code geography:

- Rust driver: `crates/bumbledb-log/src/` — `codec.rs`, `schema_file.rs`,
  `sidecar.rs`, `manifest.rs`, `store.rs` + `store/`, `lease.rs`,
  `replica.rs`, `apply.rs`, `writer/`, `checkpointer.rs`, `gc.rs`,
  `braids.rs`, `tenants.rs`, `bin/` (the duty binary).
- TS driver: `ts-log/src/` — `codec.ts`, `bytes.ts`, `value.ts`,
  `chain.ts`, `manifest.ts`, `keys.ts`, `store.ts`, `store-s3.ts`,
  `replica.ts`, `writer.ts`, `tenants.ts`, `errors.ts`, `braids.ts`,
  `descriptor.ts`; tests in `ts-log/test/`.
- Engine (mostly untouched): `crates/bumbledb*`, `ts/`.
- Lambda example: `examples/lambda/` (finding 125's parsed-request
  boundary and finding 55's handle-as-value live here).

## 1. Hard bounds — the ONLY limits on your authority

Any agent violating one is killed and its work reverted.

- **NEVER `git push`, `npm publish`, `cargo publish`, or `git tag`.**
  Publishing is the owner's interactive ceremony (`ts/PUBLISHING.md`).
  Leave commits local on `main`.
- **No worktrees, no branches.** Never `git checkout -b`, never
  `git switch -c`, never a feature/phase/agent branch of any kind — every
  commit lands directly on `main`, all agents operate on the one main
  tree, and parallelism is achieved by disjoint file ownership (§2),
  never by tree copies.
- **NEVER weaken, skip, or delete a failing test to get green.** The
  cutover *replaces* weak assertions with named-outcome assertions
  (findings 118, 121, 122, 123, 53, 80, 95) — that is strengthening per
  the spec, done in the conformance tier, never as a workaround for a red
  lane. Red gets fixed forward until green; it never becomes a question
  for the owner.
- **Do not touch**: `night-2026-08-22/`, `docs/research/`,
  `lean/conformance/cases/` (pinned corpus — the conformance flip happens
  in the drivers' suites, never by editing pinned cases), generated C
  headers except via cbindgen.
- **The comment law**: code comments state what is, present tense; never
  narrate the change, never reference the old shape ("previously",
  "removed", "new", "now", "refactor"). `scripts/comment-diff-guard.sh`
  enforces it; prose docs are exempt.
- **Zero backwards compat, by design**: the parser refuses `v:2`; there
  is no translator, no dual-read path, no compatibility shim, no feature
  flag. An agent writing "legacy" handling has misread the spec — kill it.

## 2. Architecture — two pipelines, a fixtures lane, a verification swarm

- **Pipeline R** owns `crates/bumbledb-log/**` and its Rust tests.
- **Pipeline T** owns `ts-log/**` and `examples/lambda/**`.
- **Lane F** owns cross-driver fixtures/goldens (byte-identical corpora
  both suites consume). F lands its files first within a tier; R and T
  consume them read-only.
- **Swarm V** is the standing verification pool: adversarial reviewers
  you dispatch continuously against freshly landed work (see Tier P for
  the final full-corpus pass; do not wait for Tier P to start verifying —
  every merged worker's diff gets a V reviewer while the next worker is
  already in flight).

**R and T proceed concurrently and independently** through the tiers
below, each at its own pace — TS never idles waiting for a Rust battery
and vice versa. They synchronize only where reality forces it: Lane F's
goldens, and the cross-driver parity assertions in Tier 6.

**File ownership is the concurrency law**: no two live workers write the
same file, ever. Within a pipeline, partition each tier's work by file
(or by target-representation bullet within a large file, serialized on
that file). Everything disjoint runs simultaneously.

## 3. The tier plan — dependency order within each pipeline

Each pipeline runs Tiers 1→6 in order (the representations stack:
codec → store → chain/checkpoint → retention → machine → conformance).
Tier 3's two efforts run concurrently. Then the closing tiers P and A.
Every worker brief must name: the decision doc and section it implements,
the files it owns, and the finding ids it closes.

### Tier 1 — `60-codec-grammar.md`: one grammar, one codec

Land: exact numbers (`u64`/`i64` in Rust, `bigint` — never `number` — in
TS; checked/saturating sums; bounded parse); one canonical encoding for
pending bytes (the hex/base64 split dies); the length-delimited row
vector — count cannot outrun bytes, enforced in the parser on both
codecs; `WellFormedUtf8` via a fatal encoder; `[u8;32]` digest types;
half-open fixed intervals — the ceiling is not a value, on decode AND the
encode gate, both drivers; parsed grammars for `schema_file`, the duty
argv, and the Lambda request; document version `v:3` with the parser
refusing `v:2`. Lane F: a cross-driver golden corpus — canonical bytes
both suites decode to identical values and re-encode byte-identically —
plus a fuzz/truncation corpus for the TS codec mirroring the Rust
mutation lane.
Rust files: `codec.rs`, `schema_file.rs`, `sidecar.rs`, `manifest.rs`,
`bin/` argv. TS files: `codec.ts`, `bytes.ts`, `value.ts`, `chain.ts`
(parse), `manifest.ts`, `examples/lambda/` request parsing.
Closes: 6 9 11 12 37 49 50 57 63 74 76 77 78 79 97 104 105 106 107 108
113 125 130 137 138.

### Tier 2 — `20-store-contract.md`: the store is one contract

Land: `Liveness = Alive | Dead | Unknown` with the entire pid-probe
apparatus deleted; the mutation lock as a fenced CAS lease
(`{holder, token, expires}`) acquired/broken only through the store's own
CAS, expiry-only breaking, fencing token on every write; success minted
only after object+parent-dir fsync on every impl including the checkpoint
seed; outcome sums `Created|Exists|Ambiguous` and `Swapped|Moved|Ambiguous`
with the GET-verify law for `Ambiguous` (S3 409 is `Ambiguous`, never
proved); `StoreKey` as a parsed grammar shared by all three impls with
temp/lease namespaces disjoint by construction and swept at open;
fresh-buffer reads in memStore; `ErrStore` on every failure path
including body streams; explicit async/sync boundary; per-request
credentials off the workers; cross-process exclusivity on a replica dir +
refcounted handles where a disposed handle is a distinct type; the store
smoke lane cleaned (correct-arity row, bytes-tied winner, collision-free
prefix, bucket cleanup).
Rust files: `store.rs`, `store/`, `lease.rs`, `tenants.rs`. TS files:
`store.ts`, `store-s3.ts`, `keys.ts`, `tenants.ts`, the Lambda handler.
Closes: 2 3 4 8 19 20 21 22 23 24 25 26 27 28 29 38 39 48 53 54 55 58 62
65 66 71 73 81 82 83 84 85 87 88 89 90 91 92 93 94 95 110 112 115 124
132.

### Tier 3 — `30-pending-chain.md` ∥ `40-checkpoint-chain.md` (concurrent)

**Chain effort** — land: `Chain = Settled{vector} | Pending{vector,
batch}`; `generation()` a total function of the chain; `applied_pending`
and `pending: Option` deleted; compaction's input type is `Settled` (a
pending checkpointer is a compile error); resolution as one pure fold
shared by publisher/fallback/open-recovery, returning remaining segments
as data; `Pending → durable → Settled` write ordering (no `pending:null`
ahead of re-judgment; a refusal never advances); `SidecarRead =
Absent|Fault|Corrupt|Read` with `Absent` = NotFound only.
Rust: `sidecar.rs`, `apply.rs`, `writer/`. TS: `chain.ts`, `writer.ts`.
Closes: 1 5 31 33 34 35 36 42 45 47 59 68 72 120 131 139.

**Checkpoint effort** — land: `prev` inside the blake3 content hash;
checkpoint documents written exactly once with `put_create` (`upsert`
deleted); the manifest as head of an immutable Merkle list; orphans
known-by-construction and addressable; the catalog claim audited at the
one seed transition in both drivers.
Rust: `checkpointer.rs`, `manifest.rs`. TS: `manifest.ts`.
Closes: 0 10 16 17 32 69 128 129.

If both efforts need `writer/` or `writer.ts`, chain lands first in that
file; everything else overlaps freely.

### Tier 4 — `50-retention.md`: the floor is a write-path invariant

Land: the floor as a precondition every slot create/delete asserts
(below-floor create refused as "slot is retired", killing resurrection);
the sweep as a resumable contiguous bottom segment `[0, marker)` walking
upward, the checkpoint sweep walking Tier 3's Merkle backlink and
deleting `.json`+`.mdb` as one unit; retention aging by the
checkpointer's trusted publish clock; atomic adopt (etag not committed
until the checkpoint is in hand); the pending fold consulting the floor
(below-floor = published); the cadence meter subtracting, `ckpt_sum`
re-seeded on re-establish; the duty outcome as a total sum whose exit
code is a total function of it (refusals scream, never report success),
observed through the binary by its tests; every scratch dir/temp/thread
with an owner that reclaims it (leases with expiry swept at open,
`duty_busy` released on unwind, handles reaped in steady state).
Rust: `gc.rs`, `checkpointer.rs`, `bin/`. TS: writer/replica retention
paths.
Closes: 7 13 14 15 16 18 40 46 67 70 75 80 86 98 99 100 102 103 111 116
126 127 134.

### Tier 5 — `10-protocol-machine.md`: one machine, two thin drivers

Land: the transition table as the one artifact — `ReplicaState`,
`RefreshOutcome` (per-braid `Wedged`, `Reseed`, `Refused`),
`CreateOutcome` with the `Ambiguous` resolution law — executed
identically by both drivers; one stepper shared by `refresh`, `waitFor`
(= refresh with a predicate), catch-up (round-robin: one slot per braid
per round), and open, carrying the heartbeat, wholeness check, pass
counter, and disposed-check inside it; the id-lease algebra
(`Refused(OverWidth) | Refused(Exhausted) | Drawn`, unsigned count, body
runs exactly once, awaited to completion before sealing); deposition
from slot-header ownership (never gated on decoding the winner);
`#[must_use]` publisher results; the scream on a signature *set*; role
as a field on the handle (a replica refuses `ManifestMissing`; only the
writer births); inherited pending published in the shared `open`
transition.
Rust: `replica.rs`, `writer/`, `apply.rs`. TS: `replica.ts`, `writer.ts`.
Closes: 24 30 41 43 44 51 52 60 61 64 88 96 101 109 114 117 133 135 136
140.

### Tier 6 — conformance flips to executing the table

Over `ts-log/test/` and the Rust test modules: replace weak assertions
with the table's named outcomes — spanning-commit asserts
`ErrSpanningCommit` (118); rejected-commit asserts the slot is absent in
the store (121); multiprocess recovery drives a deterministic scripted
pending (122); parity goldens assert a present `writer` field (123); the
TS crash matrix exists and runs the same matrix Rust runs (56); the TS
codec fuzz lane runs Tier 1's corpus (57). This is the one tier with a
hard cross-pipeline sync: the parity lane proves the two drivers produce
identical named outcomes on identical bytes.
Closes: 56 57 118 121 122 123 (plus any of 53 80 95 not already landed).

### Tier P — proof (runs when both pipelines finish Tier 6)

1. **Grep-for-absence** (70-cutover.md's proof obligation): across both
   drivers, zero hits for `kill(0)`/`kill -0`, `pid_alive`, `pidAlive`,
   `applied_pending`, `upsert`, base64 pending rendering, JSON-`number`
   u64 round-trips, `refresh_braid`, downward-break sweep,
   `Ok(status.success())` liveness, `batch.header.timestamp` aging, the
   "gc fodder" comment, the TS manifest-birth arm. Publish the transcript
   in the receipt.
2. **Adversarial audit, all 141 rows of `90-traceability.md`** — every
   row, no sampling, massively fanned: each verifier is briefed to
   **refute** closure — find a code path that still constructs the
   illegal state, or show the named representation did not land as
   specified. A row passes only if the verifier, trying to break it, must
   cite the type/invariant that stopped them, with file:line. Two
   independent verifiers per critical (rows 0–9), one per major/minor;
   any refutation reopens the owning tier in the owning pipeline. Batch
   rows by owning doc so each verifier reads one decision doc deeply.
3. **Lockstep version bump**: move every manifest the lockstep gate
   compares (root workspace crates including `bumbledb-log`, the napi
   crate, `bumbledb-c`, `ts/` main + both platform packages, `ts-log/`
   with its peer range) to ONE new version number in one commit;
   re-derive lockfiles. Do not publish, do not tag.

### Tier A — align the parent PRDs (concurrent with Tier P; one worker per doc)

The numbered PRD set keeps its promises and loses its stale spellings:
rewrite every representation detail in `proposals/10-protocol.md`,
`40-object-store.md`, `50-replica.md`, `60-writer.md`,
`70-typescript.md`, and `80-conformance.md` that the cutover deleted —
the pid lockfile, the pending flag, the checkpoint upsert, the sweep
direction, the encodings, the interval ceiling — so they spell the
landed representations. Laws, deployment cases, and theses untouched.
After this tier the corpus has one spelling and §0.6 has nothing left to
arbitrate.

## 4. Gates — chaos in flight, green once, at the end

**Mid-flight red is expected and acceptable.** While tiers are landing,
type errors, broken test suites, and red batteries are the normal state
of a tree whose representations are being replaced under it. Do NOT
freeze on them, do NOT serialize workers on incremental compilability,
do NOT spend briefs chasing a green the next tier will break again.
Commits land mid-red; total chaos is fine.

**Green is owed exactly once — at the end.** After the last tier's src
lands, run one reconciliation pass: fix every type error, migrate every
test to the new shapes (migration asserts the *new* named outcomes —
that is strengthening; deleting or loosening an assertion to pass remains
a hard-bound violation), then loop the full battery until green. Tier P
(grep proof, 141-row audit) and the receipt are cut only on a fully
green tree. Note: `crates/bumbledb-log/` is its own cargo workspace with
its own `Cargo.lock` — root `--workspace` commands and `scripts/check.sh`
do NOT cover it; a green that skipped battery item 9 is fake.

**The full battery** (the end-state definition of green):

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `scripts/check.sh`
5. `scripts/lean.sh` (three-way comparator; 0 disagreements)
6. `scripts/spec-census.sh`
7. `ts/`: the package's test script (403 tests) + `tsc --noEmit` + biome
8. `ts-log/`: the package's test script + `tsc --noEmit` + biome
   (test *count* drifts upward as the suite strengthens; failures don't)
9. From inside `crates/bumbledb-log/` (its own workspace — items 1–4 do
   NOT reach it): `cargo fmt --check` + `cargo clippy --all-targets --
   -D warnings` + `cargo test`

Use each package's own `package.json` scripts (`pnpm`, never `npx`).

**Commits**: cut one house-style commit per tier once that tier's src
has landed in both pipelines — green is not a precondition mid-flight;
the next tier's workers are already in flight on whichever pipeline
finished first. House style: long-form prose stating what the
representation now *is* and what died, ending with `Named deletions:` and
an itemized tally. Study `git log` for the voice before the first one.
The final **receipt commit** reports: per-tier commit hashes, the battery
transcript, the grep transcript, the 141-row audit verdicts, the rulings
log (every precedence decision, one line each), the deletion tally, and
the one line the owner runs themselves: `git push origin HEAD`.

## 5. Recon (Tier 0, wide fanout, hours not days)

Before Tier 1, fan readers over the ten proposal docs and all 25+ driver
files. Produce the work map: for every target-representation bullet in
docs 10–60, the exact files/functions/types it lands in, both drivers,
plus a concrete site or an honest zero for every deletion-table row in
70-cutover.md (a zero marks the row already-dead; the owning tier skips
it). No edits. Recon rolls straight into Tier 1 without pausing.

## 6. Autonomy protocol — decision rules, not escalation

You never stop to ask. Every class of trouble has a rule:

- **Document conflict** → §0.6's precedence order. On representation the
  cutover docs win; on promises the parent PRDs win; between decision
  docs, 70-cutover.md's dependency order decides. Rule, log one line,
  move.
- **A target representation that cannot land exactly as written** (the
  code forces a variation): the *invariant block* of the owning decision
  doc is the binding contract; the spelling flexes to the closest form
  that keeps that invariant airtight. You arbitrate, the delta goes in
  the rulings log, the traceability rows it closes stay closed. No
  silent substitutes — logged ones.
- **Red battery, type errors, broken suites mid-flight** → not a stop
  condition, not a freeze, not worth a brief. Keep landing tiers. Green
  is owed exactly once, at the end-state reconciliation pass (§4), and
  that pass iterates until green without waiting on a human.
- **A worker stalls or produces garbage** → kill it, revert its files,
  re-dispatch with a sharper brief; twice garbage → split the brief
  smaller and re-dispatch again.
- **Cross-pipeline disagreement** (Rust and TS land subtly different
  readings of one spec bullet): the parity lane is the referee — the
  reading that survives identical-bytes → identical-named-outcomes wins;
  the other pipeline conforms to it. Log the ruling.
- The ONLY actions outside your authority are §1's hard bounds: push,
  publish, tag, weakening a test, branches/worktrees, the untouchable
  paths. Everything else — every ruling, every ordering choice, every
  ambiguity — is yours to decide and log.

Begin now: fan out Tier 0 recon, roll straight into Tier 1 on both
pipelines, and do not stop for anything short of a hard bound until the
receipt commit — batteries green, 141 rows adversarially verified, PRDs
aligned, one version number — is on `main`.
