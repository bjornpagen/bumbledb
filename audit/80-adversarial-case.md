# 80 — The adversarial case against the FFI cutover

**Proposal under attack:** move `ts-log`'s pure TS machinery (codec,
braids, Vector, document parsers, chain) behind the engine's shared
Rust/napi core, so TS becomes typed-payload construction + IO glue on
the model of `ts/src/query/` ⇄ `ts/crate/src/marshal.rs`, and unify the
type algebra so ts-log reuses the engine SDK's types verbatim.

**Method:** read-only sweep of the present tree at `c5947e3c` (version
fact 0.19.2), the retired proposal corpus recovered from git
(`git show 5c7a693a:proposals/70-typescript.md`,
`git show 49d45b5c^:proposals/settlement/*`,
`git show 49d45b5c^:proposals/lockstep/*`), the conformance and census
apparatus, and both drivers' sources. Every argument below is graded by
the evidence behind it; weak arguments are called weak; the pro-refactor
evidence I found is stated rather than buried.

---

## Argument A — The mirrored pair is a curated post-decision equilibrium, not inertia. **Grade: STRONG**

The repo did not drift into two implementations; it drew the line three
separate times, with the FFI alternative on the table each time, and the
line survives as a *policy*: **truth derivation crosses the FFI once;
the wire protocol has two independent speakers.**

1. **The strongest surviving statement** is the retired
   `proposals/70-typescript.md` (recoverable via
   `git show 5c7a693a:proposals/70-typescript.md`, retired whole at
   `48d47a48`), §"Mirrored pure functions (the parity-critical set)":

   > "The mirrored pair must be byte-equal with Rust, pinned by the
   > goldens (80): `encodeBatch`/`decodeBatch` (20) and
   > `braidsOf(descriptor)` (10) … Both are pure, take the descriptor
   > the SDK already lowers, and touch no I/O — **they are the
   > protocol; the rest of the package is plumbing around them.**"

   The surviving canon (git `49d45b5c^:proposals/settlement/00-canon.md`
   §1) states the same law as a conformance obligation, not an
   implementation accident: *"A behavioral divergence between the
   drivers is a conformance failure, not a design choice — neither
   driver defines arms."* Two speakers, one law, divergence typed as
   failure — that is a designed redundancy, and the design survives in
   the shipped docs (`ts-log/src/index.ts:1-9`, `ts-log/README.md:7-11`).

2. **The boundary was actively renegotiated, not fossilized.** Commit
   `a99c1470` shrank "the mirrored pure trio" to "the mirrored pair"
   when `footprintOf` died — membership in the mirrored set is curated
   per function, with deletions named. A stale architecture does not get
   pruned this precisely.

3. **The owner already ran the proposed policy where it pays — and
   stopped where it doesn't.** `ts/PUBLISHING.md` (0.17.1 and 0.18.0
   entries) records two deliberate moves of *derivation* behind the FFI:
   `internalBlake3` ("one export, one consumer") and
   `internalDescriptor(spec): SealedDescriptor`, the latter shipped
   precisely "so the driver's **741-line re-derivation** and
   fingerprint-mirror string-axiom refusal **delete rather than get a
   second fix**." `ts-log/src/descriptor.ts:20,265` consumes it today.
   So the counterfactual the proposal imagines — "what if we shared the
   Rust core?" — was *actually executed* for schema truth and hashing,
   evaluated, and deliberately not extended to the wire codec. The
   present shape is the output of that experiment, and the experiment's
   principle is legible: derivations have one writer; **protocol bytes
   have two independent speakers and one corpus that pins them.**

4. **The line is a standing gate, not a comment.**
   `ts-log/test/temporal-gate.test.ts:15-23` fails the build if any pure
   protocol module (`codec.ts`, `braids.ts`, `value.ts`, `bytes.ts`,
   `descriptor.ts`, `keys.ts`, `manifest.ts`) declares `async`, awaits,
   or imports the store. The refactor doesn't merely change code; it
   deletes an enforced invariant the test suite names "70's temporal
   law."

## Argument B — Both drivers are in production *simultaneously, on the same bytes*; the refactor deletes the only independent oracle while keeping most of the apparatus. **Grade: STRONG**

This is not a "Rust core + one TS client" system. It is a two-driver
protocol whose deployments **mix the drivers on one store**:

- The shipped Lambda example (`examples/lambda/README.md:33-47,59-60,
  85-93`) has the **TS writer** (`ts-log` in the `nodejs24.x` handler)
  committing batches while the **Rust duty binary**
  (`/opt/bin/bumbledb-log-duty`, a LayerVersion) checkpoints and gc's
  the same bucket on an EventBridge cadence. Rust reads what TS wrote,
  every five minutes, in production.
- Deployment case 5 (canon §"five deployment cases") races N processes
  over one `FsStore` prefix, and the interop lane
  (`crates/bumbledb-log/tests/lane_b_interop.rs` driving
  `ts-log/test/interop-child.ts`) races the Rust and TS fs stores
  against each other cross-process — "one protocol, two conforming
  implementations, raced against each other" (retired 70, §Dependency
  ruling).

Consequences the proposal has to answer:

1. **The Rust driver keeps its full standalone machine no matter what**
   (duty binary, resident mode, cases 2/3/5). The refactor cannot delete
   the Rust codec; it can only delete the TS one. What it deletes with
   it is **N-version detection** — and the bug bash is direct evidence
   of what that detection is worth. The divergences it surfaced were
   frequently *spec* bugs, answerable only because two independent
   implementations disagreed: the BOM decode split (row 6, critical —
   settled as WHATWG `ignoreBOM: true`, commits `cc547fc6`, `49469bde`),
   the fixed-interval domain-ceiling family (rows 37/49/63 — became the
   half-open-interval law), hex-vs-base64 pending (rows 9/107/130 —
   became the one-encoding decision that grew into
   `settlement/20-one-encoding`). One shared core makes this class of
   finding **undiscoverable**: parity passes vacuously when both sides
   are the same code.
2. **The expensive apparatus survives anyway.** The layer's duty binary
   and the bundle's `.node` deploy separately and can skew across a
   rolling deploy, so wire-stability fixtures, version/fingerprint
   refusals, and the interop lanes must be maintained regardless. The
   refactor trivializes only the cheap half (the in-repo TS parity unit
   lanes) while keeping the cross-process half. The claimed
   apparatus-retirement savings largely do not exist.
3. **FFI does not end mirroring; it relocates it.** The napi crate's own
   `ts/crate/src/tags.rs:36-40` documents "the old admitted drift gap: a
   new variant satisfied `tag()` and still refused at runtime as an
   'unknown … kind'" — a drift bug *inside* the single-core boundary,
   between its tag table and its parsers. And `ts/crate/src/fingerprint_lock.rs`
   exists precisely to pin what crossing the FFI un-pinned. The boundary
   moves from "two codecs pinned by checked-in bytes" (easy to pin) to
   "JS spelling ⇄ Rust enum pinned by hand-mirrored tables" (the thing
   that already drifted once).

## Argument C — The conformance/parity apparatus already drove residual drift risk to ~zero, at far lower cost than an architecture inversion. **Grade: STRONG**

The numbers, from the tree and the recovered settlement corpus:

- **One inventory, two walkers.**
  `crates/bumbledb-log/conformance/v3/inventory.json`: 7 `batch_ok` +
  24 `batch_refusal` + 5 `chain` + 23 `documents` + 33 materialised
  fuzz cases + a fuzz storm. Rust walks it in
  `tests/conformance_v3.rs` and `tests/f7_parity.rs` ("any drift between
  the two codecs or braid derivations lands here or there as a typed
  disagreement" — f7_parity.rs:1-7); TS walks the identical files in
  `ts-log/test/conformance-v3.test.ts` (695 lines) and
  `ts-log/test/parity.test.ts` (451 lines). Refusals must carry the same
  cross-implementation identity, not just the same bytes.
- **One battery.** `scripts/battery.sh` runs fmt, clippy, `cargo
  nextest run --workspace`, `check.sh`, the Lean lane, the census, and
  both TS packages — one exit code. The census
  (`scripts/spec-census.sh:400-431`) pins **shared constants to one
  defining site per driver** (`LOSS_BOUND`, `LEASE_WIDTH`,
  `WAIT_FOR_POLL_MS`, `LOCK_RETRY_MS` — e.g. `ts-log/src/store.ts:108`
  and `crates/bumbledb-log/src/store/fence.rs:25` both spell
  `LOCK_RETRY_MS = 10`), and lane (k) executes the banned-token roster
  (`scripts/banned-tokens.txt`) over `crates/bumbledb-log/src`,
  `ts-log/src`, and `examples/lambda/src` on every run.
- **The bug bash's own arithmetic.** Of the 141 rows in
  `settlement/90-traceability.md` (git `49d45b5c^`), 13 spell
  "diverges / Rust vs TS / drivers disagree" literally (rows 6, 9, 22,
  63, 66, 73, 107, 114, 115, 117, 130, 136, 139) and a wider read adds
  another 17 one-driver-gap rows (4, 24, 25, 28, 37, 49, 54, 60,
  61, 62, 64, 65, 68, 69, 90, 91, 140). **Every one is FIXED** — closed
  by a representation move, not a site patch (90-traceability header),
  and re-verified by refute-briefed auditors with `file:line` citations;
  the surviving receipt (`lockstep/receipt-audit-0-9-a.md`) shows all
  ten criticals PASS with two independent verifiers each.
- **The divergences were not where the refactor thinks they were.** By
  the roll-up (90-traceability §Roll-up), the codec bucket (Doc 60) is
  25 of 141; the **store contract (Doc 20) is 45 — the single largest
  bucket** — and the machine (Doc 10) is 24 more. Of the ten criticals,
  only two (rows 6, 9) sit in the pure machinery the refactor would
  absorb; four (2, 3, 4, 8) are lock/liveness/store — code that is IO
  glue **by definition and stays TS under any version of the proposal**
  (the AWS-SDK S3 signer, node:fs). The refactor hardens the bucket the
  apparatus now pins hardest and cannot touch the bucket where
  divergence actually concentrated.

The marginal residual risk after all of the above is: a *new* mirrored
arm added to both codecs with a fixture forgotten on both sides,
slipping both drivers' refusal-identity lanes and the census
simultaneously. That is a real but narrow corridor — and the honest
comparison is against the refactor's *own* new risk surface (Arguments
B.3, D, F), which is wider.

## Argument D — Async poisoning: the seam gets uglier, not cleaner. **Grade: STRONG**

The two sides of the proposed boundary run under **opposite temporal
laws, both of which are enforced**:

- Engine: the data plane is synchronous by law — `SyncResult<R>` types
  and a runtime thenable probe that throws `ErrAsyncCallback`
  (`ts/src/db.ts:233,358-369,712-713,1117`).
- Log: async ⟺ network/disk by law — the temporal gate test (A.4) and
  the surviving README (`ts-log/README.md:33-35`).
- Rust log machine: the store trait is **five sync verbs**, with a
  standing ruling that an impl must *refuse* calls from an async context
  rather than `block_on` (`crates/bumbledb-log/src/store.rs:1-5,
  364-366`; `settlement/RULINGS.md` final entry, which exists because
  row 29 was a `block_on` panic).

Now walk the two possible shapes of the refactor:

1. **Share the whole machine (replica/writer in Rust, TS as IO glue).**
   The Rust machine calls sync store verbs; every TS store is
   Promise-async (`@aws-sdk/client-s3`, `node:fs/promises`). Driving the
   shared machine from Node means async-callback FFI (threadsafe
   functions re-entering the event loop under a native frame — exactly
   the block_on-hazard class the RULINGS entry refuses to represent) or
   blocking Node's event loop on network IO. The repo already litigated
   this seam *within one language* and settled on a runtime refusal; the
   proposal reopens it *across* languages.
2. **Share only the pure fragments (codec/braids/Vector/parsers).**
   Then **zero awaits are removed**, because every await in the log
   machine is IO placed at a law-bearing point:
   - `writer.ts:582-584` — "Pending → durable, before any apply": the
     encoded bytes must be fsynced into the sidecar *between* the sync
     encode and the sync local apply.
   - `replica.ts:400-409` — the sidecar advances only *after* the
     NoOpSlot identity check, then `await persistSidecar(core)`.
   - `chain.ts:1-14,207-220` — the sidecar write is temp + rename +
     fsync of file and parent via `node:fs/promises` (off-loop). Moving
     "chain" native either splits this file across the FFI or turns
     per-slot fsyncs into event-loop-blocking sync calls.
   Meanwhile the decoded ops become a *cross-boundary liability*: the
   machine holds them in memory across awaits as the Pending arm's
   payload (`HeldBatch`, `replica.ts:227-239`), re-applies them through
   the engine (`applyOps` → `tx.insert`, `replica.ts:353-374`), and
   re-encodes them at a new tip (`readdressPending`,
   `replica.ts:970-1003`; the loss loop, `writer.ts:483-557`). As native
   values they are either opaque handles with lifetimes spanning the
   async loss loop, or they get marshaled JS⇄native repeatedly.
3. **The double-marshal.** Today: bytes decode in JS → named facts
   (closed u64 → handle name, `replica.ts:326-351`) → **one** native
   crossing at `db.write`. With a native decode: bytes → native decode →
   rows cross native→JS (the SDK's insert surface speaks handle *names*
   and named objects — `ts/src/marshal.ts:64-141`) → JS→native again at
   insert. Two crossings per applied slot, and the name⇄id bijection
   stays in TS regardless.

The current structure has the pure/sync fragments slotted *between*
awaits at exactly the points the durability order requires. There is no
async simplification available to the refactor — only new places for
the seam to leak.

## Argument E — The type algebra is already unified where unification is possible; the remainder is runtime wire law that type reuse cannot delete. **Grade: MEDIUM-STRONG**

- ts-log already consumes the engine's types and truth wholesale:
  `ValueTypeSpec` (`ts-log/src/value.ts:10`), `SealedDescriptor` /
  `internalDescriptor` / `lower` (`descriptor.ts:8-20,265`), `Fact`,
  `MemberRelation`, `Schema`, `WriteOutcome`, `Violation`
  (`replica.ts:13`, `writer.ts:19-26`), `internalBlake3`. "Unify the
  type algebra" is ~90% the status quo.
- The engine's `Fact<Rel>` **does** erase what the codec needs — it is a
  compile-time named-object type — but the erasure problem is already
  solved: runtime shape flows from `internalDescriptor` (relation ids,
  sealed field order, `ValueTypeSpec` per field, closed rosters), which
  is exactly what `readTagged`/`checkAgainst` consume. No new seam is
  required to get shape.
- What "verbatim reuse" would actually unify: `ts-log`'s
  `Value`/`Interval` (`value.ts:15-20`) and the engine's
  `FactValue`/`IntervalValue` (`ts/src/native.ts:27-34`) are
  **structurally identical five-arm unions**. Unifying the spelling is a
  type alias worth of change — a fine, cheap idea that needs no FFI
  move.
- What no reuse deletes: the wire tag table (`value.ts:57-66`,
  normative to the byte), the fatal-encoder `WellFormedUtf8` brand
  (rows 105/57), the half-open `fixedInterval` law (rows 37/49/63), the
  big-endian order-preserving canonical-literal alphabet with
  `i64beFlipped` (`value.ts:278-323`), and the closed handle-name ⇄ u64
  coercion at both directions of the wire (`writer.ts:286-319`,
  `replica.ts:326-351`). This is wire-value coercion — behavior, not
  types. It exists in Rust too; giving TS the Rust copy is Argument B/D
  territory, not a types win.
- **Honest concession:** there is real *intra-TS* duplication —
  `writer.ts:lowerFact` ≈ `ts/src/marshal.ts:rowOf` and
  `replica.ts:factOf` ≈ `marshal.ts:factOf` (same roster bijection,
  same shape checks). The cheap fix is exporting the engine's marshal
  helpers for TS→TS reuse, the same way `internalDescriptor` was
  exported. No native boundary needed.

## Argument F — Packaging and release-mechanics regression. **Grade: MEDIUM**

- **Today** `ts-log` ships raw source: `files: ["src", "README.md"]`,
  `exports` → `./src/index.ts`, deps = S3 client + errors + arkregex
  only (`ts-log/package.json`). A protocol-law fix is a source publish.
  **After**, log protocol behavior rides the `.node` binary: shipped set
  exactly `{darwin-arm64, linux-arm64}` with a hard load-time refusal
  elsewhere (`ts/src/native.ts:421-439`), built via owner ceremony on a
  darwin host plus an amazonlinux:2023 CI artifact placed by hand
  (`ts/PUBLISHING.md:1-18`).
- **Honest concession:** this creates no *new* platform constraint —
  importing ts-log already loads the engine binding transitively
  (descriptor/blake3/Db). The regression is in **granularity and
  cadence**: today the binary carries engine internals; after, every
  wire-law fix reminted two platform binaries and re-ran the ceremony.
  The napi crate is already the lockstep's most awkward member — it
  needed its own remint commit *today* (`e6c21b98` "A lockstep bump
  remints the napi crate and the ts-log peer"), it sits on the
  version-roster of manifests that cannot inherit the workspace version
  (`scripts/version-roster.txt`), and version skew is this repo's proven
  failure mode (`lockstep/00-thesis.md`: the log crate sat at 0.17.0
  "through an entire release, and no gate noticed").
- **Workspace exclusion.** `ts/crate` is *excluded* from the one
  workspace (root `Cargo.toml:13-16`), so protocol law moved there falls
  outside `cargo nextest run --workspace` — recreating, letter for
  letter, the defect lockstep 10-one-workspace was written to kill
  ("the one command the repo calls 'test everything' does not compile
  the component").
- **Binary weight.** `ts/crate` depends on `bumbledb` + `napi` only
  (`ts/crate/Cargo.toml:26-30`); `bumbledb-log` pulls
  `object_store` (aws) + `tokio` rt-multi-thread
  (`crates/bumbledb-log/Cargo.toml:7-11`). Exposing the log crate
  through the napi crate means either a prior feature-split of the log
  crate into pure/IO halves (an unbudgeted precondition) or shipping a
  tokio runtime and an unused Rust S3 stack inside the 2.8–3.0 MB
  `.node` (`ts/npm/*/bumbledb.node`) that the Lambda bundle carries per
  deploy. Cold start is "checkpoint pull plus tail replay"
  (`examples/lambda/README.md:65-67`) — network-bound; a native codec
  buys nothing there, and a heavier zip costs a little.

## Argument G — Timing: the blast radius lands on a settlement that closed *this week*. **Grade: MEDIUM (and mostly dissolves by waiting)**

The premise "mid-flight toward 0.19.0" is stale in the letter — the
tree is past 0.19.0 — but right in spirit: the **entire arc** (PRD set
`15e2d671` Aug 21 → cutover → settlement → lockstep → proposals declared
complete and deleted `49d45b5c` Aug 26 → version facts 0.19.0/0.19.1/
0.19.2, three of them minted **today**, `5b714912`/`e6c21b98`/`c5947e3c`,
with release packaging churn `72c23861` in between) ran Aug 21–26, 2026.
The publish ceremony is owner-pending or just done; the Lambda example
pins registry installs at 0.19.0 (`examples/lambda/README.md:7-10`).
Landing an architecture inversion now would:

- invalidate the 141-row verdict citations, which cite `ts-log/src`
  `file:line` (receipt-audit-0-9-a.md cites `ts-log/src/store.ts:61,
  151-170`, `ts-log/src/replica.ts:481-486`, `ts-log/src/writer.ts:
  495-498`, …) — the acceptance artifact of the whole campaign;
- gut the census lanes whose scopes are `ts-log/src` (lanes j/k,
  `spec-census.sh:400-431`, `banned-tokens.txt`);
- reopen "settlement and lockstep live in the code and the gates"
  (`49d45b5c`) the same week it was declared;
- stack onto 0.19.0, which is already a no-migration breaking cutover
  ("**0.19.0 reads nothing 0.18.0 wrote**", `ts/PUBLISHING.md`),
  maximizing correlated risk in one release window.

## Argument H — Greenspun-in-reverse: the query-builder pattern is genuinely healthy here — and that is precisely why the analogy fails. **Grade: MEDIUM**

Honest finding first: the attack vector as posed ("does ts/src/query/ +
marshal.rs show pain?") mostly **fails**. Grep for
workaround/TODO/FIXME/hack across `ts/crate/src/*.rs` and
`ts/src/query/*.ts` returns essentially nothing; `tags.rs` generates
both directions of every wire-tag map from one table with
compile-breaking exhaustiveness; the IR needs no version field because
the binary ships inside the same npm package, pinned by
`engineVersion()`. The pattern is good.

Why it still doesn't license the log refactor:

1. **Control plane vs data plane.** The query IR crosses the FFI at
   prepare time — rare, stateless, no durability ordering. The log
   codec is per-commit/per-slot machinery entangled with fsync ordering,
   the Pending arm's in-memory payload, and the loss loop (Argument D).
   The pattern's success in the easy position is not evidence for the
   hard one.
2. **The pattern is not free.** The engine's boundary costs ~2,650
   lines of hand marshaling and type twins (`marshal.rs` 1,657 +
   `tags.rs` 418 + `native.ts` 574), needed `fingerprint_lock.rs` to
   re-pin what the FFI un-pinned, and carries the one documented drift
   scar in the repo (`tags.rs:36-40`). The machinery it would replace is
   ~1,200 lines of dependency-free, runtime-agnostic TS (codec 335 +
   value 336 + vector 173 + braids 50 + bytes 304; zero `node:` imports
   in any pure module) pinned by ~92 shared fixtures plus fuzz. Line for
   line this is a wash at best, relocated to a boundary that is harder
   to golden-pin than byte arrays.
3. **Dual validation survives.** Even the healthy query boundary
   validates on both sides (`ts/src/query/parse-ir.ts` +
   `marshal.rs`'s `req`/`req_at` parsing). A dynamic boundary cannot
   skip either side; moving the codec doesn't end mirrored checking, it
   re-homes it.

## Weak or failed attack vectors (stated for honesty)

- **Performance:** FFI call overhead is noise against tens-of-ms S3
  conditional PUTs (`examples/lambda/README.md:105-107`). No case
  either way.
- **Platform reach of the pure pair:** although the pure modules are
  runtime-agnostic in isolation, importing anything from `ts-log` loads
  the engine binding transitively today (`native.ts:439` loads at module
  scope), so "the pure pair runs where native can't" is not a real
  present-tree property. Not usable against the refactor.
- **A standing weakness of the mirrored approach, conceded:** the TS
  fingerprint mirror records its own gap — "closed ground axioms with
  string columns are the mirror's recorded gap"
  (`ts-log/src/value.ts:272-293`). The production path sidesteps it (the
  real fingerprint arrives via `internalDescriptor`), but it is a live
  asterisk the shared-core design would erase.
- **A real duplication the refactor correctly smells:** the
  `Value`/`FactValue` twin unions and the duplicated roster bijection
  (Argument E's concession). Both are fixable TS→TS at ~1% of the
  refactor's cost.

## Arguments that DISSOLVE if the refactor waits until after the 0.19.x release settles

- **G in its entirety** (blast radius on the just-closed settlement, the
  correlated-risk stacking on the 0.19.0 cutover, the pending publish
  ceremony, the freshly-cited receipt).
- The *ceremony-churn* half of **F** (the napi remint and roster
  pressure are hottest mid-release; after the publish lands, a binary
  remint is routine rather than destabilizing).
- Nothing else. **A, B, C, D, E, H are structural and survive any
  waiting period.** In particular: waiting does not restore the deleted
  second speaker (B), does not un-blur the temporal law (D), and does
  not change where the bug bash found its divergences (C).

## Net verdict (as devil's advocate)

**The case against is strong, and it is strongest on structure, not on
timing.** The proposal reads the repo as "one implementation too many";
the repo's own history, gates, and deployments read the second
implementation as a load-bearing component: it is the independent oracle
that made the 141-row bug bash possible (and 13+ of its rows findable at
all), it is one of two conforming speakers that *interoperate in
production* on shared bytes (TS handler + Rust duty binary on one
bucket), and its purity is an enforced invariant with a named temporal
law. The drift risk the refactor targets has already been driven to a
narrow corridor by a cheaper mechanism (one inventory, two walkers, one
battery, a census that pins constants and absences forever), while the
refactor's own risk surface — an async/sync seam both sides have
standing rulings against blurring, a hand-mirrored napi vocabulary that
has already drifted once, protocol law moving out of the workspace
battery and into a two-platform binary ceremony — is wider than the
corridor it closes.

If the owner remains drawn to the proposal's kernel, the honest
extraction is: (1) unify the `Value`/`FactValue` spelling and export the
engine's marshal helpers for TS→TS reuse; (2) grow the conformance
inventory wherever drift anxiety points; (3) do both after the 0.19.x
publish settles. The full FFI cutover should be attempted only if
someone first refutes Arguments B and D specifically — and nothing in
this tree currently does.
