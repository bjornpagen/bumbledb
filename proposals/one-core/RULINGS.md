# Rulings — the one-core pass

Precedence decisions that flexed a decision-doc spelling to keep its
invariant, one entry per ruling. The owning doc's invariant block stays
binding.

## S1 — vocabulary and pins

- **`Commit` dies rather than aliasing.** Rust call sites say
  `Admission<Slotted<R>>` with log-owned
  `Slotted { value, braid, slot, durability }`; TS composes the same
  shape under its tagged-sum house style. The no-alias law applied to
  the outcome shell itself.
- **`BraidOutcome` stays a named struct** `{ braid, admission }` in both
  languages so the rejected arm keeps its braid tag; the container of
  `commit_split` stays language-idiomatic (Rust `(R, Vec<BraidOutcome>)`
  tuple, TS tagged `split` arm) — the per-braid *element* is the one
  shared shape, the container is host idiom.
- **`WAIT_FOR_POLL_MS` fact ruled 10 ms** (the Rust value); the
  machine-constants table (`conformance/v3/machine-constants.json`) is
  the writer; TS's 20 ms was a pinned divergence and conforms.
- **`reserve_capacity` deleted, not twinned**: zero non-test callers
  repo-wide; `Error::ReservationShape` and the reservations derivation
  died with it. The verb-parity ruling for doc 10 §5 is "dies in Rust."
- **The empty-commit refusal stays and is typed** in both drivers (law
  6: the empty commit is not a commit); TS spells it as a named arm, not
  a bare throw.
- **A lease-cache refill is not an exhaustion**: `ErrRefillNeeded` is a
  driver-internal operational signal, not a `LeaseRefusal` arm, because
  Rust's `LeaseRefusal` has no such arm — the sum mirrors the shared
  algebra; the signal is host-local.
- **`Waited` is surfaced in TS as a lowercase-tagged sum**
  (`reached | wedged | refused`) per the file's house style; `wedged`
  carries the cause string TS has (Rust carries braid only) — a
  superset payload, same arms.
- **The fs lease protocol is the Rust spelling**: `LEASE/1` body,
  `~lease/{key}/{n}` tokens + `~head` pointer, 5 s TTL, break by expiry
  of the lease's own bytes only; release rewrites `expires=0` rather
  than deleting. TS `Liveness`/probe machinery deleted — the mutation
  lock never probes a foreign process.
- **TS temp sweep uses mtime age** (> stale window) instead of Rust's
  own-pid ledger — the pid ledger does not survive the TS process
  model; the honest analog never touches a live sibling's fresh temp.
- **The tilde table is the wider set** (15 code points; NFKC-closed at
  generation, so the runtime NFKC fallback died). Rust's 10-point set
  is the pinned drift and conforms in S2.
- **Key-grammar refusal posture pins the TS spelling**: Cc/Cf/Zl/Zp/Zs
  anywhere in a segment refuses; Rust's strip-Cf and Zs-blind paths are
  the pinned drift and conform in S2.
- **The counter body pins Rust's canonical decimal** (no leading zeros,
  refuse at parse); TS's `^\d+$` acceptance conforms in S2.
- **The Vector wire form is deleted, not pinned**: `encode`/`parse`
  had zero non-test callers in BOTH drivers; a wire form nothing
  produces is not a surface. TS side deleted in S1; Rust side dies in
  S2; the `vector-wire` surface left the manifest.
- **`surfaces.json` is its own roster**, separate from `inventory.json`
  (case roster) — two rosters, two jobs, no entanglement. `fuzz/` is
  not a surface; its subfamilies attribute to the surfaces they mutate.
- **The ts-log `Generation` brand keeps its name**: it brands the
  per-braid coordinate (the count), not an outcome slot; the
  one-name-per-coordinate rename applied to outcome fields that carried
  slots.
- **`LeaseRefusal` in ts-log stays despite zero external consumers**:
  it mirrors the shared algebra and is an identity-table pin candidate
  for S3, not dead code.
- **Refusal sidecars for lease/scratch carry no refusal name**: both
  drivers yield an undifferentiated `None`/`null` from those parsers;
  naming variants would over-pin.
- **Banned-token roster gains `conformance/` allowance** for the
  `.json` store-key line in the two driver scopes: corpus tables read
  by drivers are data references, not key spellings.

## S3 — the oracle and the purge

- **One speller per family**: `EncodeError::identity()` and
  `LeaseRefusal::identity()` minted in the core; the bridge's tag table
  became an assertee. Outcome families (admission/waited/refreshOutcome)
  emit the lowercase tags hosts narrow; PascalCase `identity()` stays a
  refusal-family convention.
- **`RefreshOutcome` pins as a partition**: table rows
  `advanced`/`refused` plus the frozen host-local arms `wedged`/`reseed`
  — the machines keep two executors, and their extra arms are pinned,
  not tabled. Host-side kinds are a frozen roster (`DigestWidth`,
  `CheckpointDigest`, `NoOpSlot`), pinned as non-table kinds.
- **Payload fields are seat-owned enrichments**: bridge mints carry the
  bare kind ({kind, message} is the boundary); optional fields exist
  only where the minting seat owns the datum. 28 invented per-site
  fields died. `CheckpointBraids` died into the core's `BraidSet`.
- **Unconstructible identity rows are asserted as unconstructible**,
  never skipped: encode `FingerprintMismatch` by the wire itself (the
  handle fills bytes 8..40), `TooManyOps`/`TooManyRows` by the
  ECMAScript array ceiling (2^32−1 = u32::MAX exactly).
- **The spec generator's honest boundary**: batch/document/counter/
  lease/scratch ok-goldens generate byte-identical from metadata alone;
  the chain family's bytes are not spellable from its sidecars and stay
  owned by the decode/re-encode fixpoint lane. Corpus fingerprints are
  synthetic pins, never recomputed — re-deriving would import the
  reader's derivation into the third mind. Truncation sets generate at
  test time (852 prefixes; python3 is already a census dependency).
- **The storm raise**: 4–6× iterations across all four recipes plus the
  `copy_chunk` mutation class (length-preserving, alphabet-plausible);
  the knob is the recipe data, not the code.
- **`codec.ts` stays on the temporal gate's pure roster**: the gate's
  definition is synchronous and store-blind; a napi call is neither
  async nor a store — FFI is not a temporal boundary.
- **A sidecar naming a foreign braid is corrupt cache**, reseeded under
  the disposable law — not a host-side refusal; the identity is pinned
  by the corpus row, and replica open's braid-validation loops died as
  unreachable behind the codec-backed reads.
- **`BraidId::from_raw` died by honest minting**: the vector tests now
  mint braids through `braids()` + `Braids::parse`, so "minted only by
  braids" is true without exception. `ErrStore` fell to the no-alias
  law; `Lease::expired` folded into `breakable`; nine in-file-only pubs
  demoted to private so `dead_code` polices them.
- **A test consumer is a consumer; type inference is a consumer**
  (`ClosedSelectable` stays exported on a TS4023 proof). An export seat
  without a consumer dies while its in-file-consumed declaration lives.
- **The bridge's parsed-then-discarded interior/rec heads died on the
  napi side only**; removing the head from the wire whole (and the C
  bridge's twin discards, with its ABI bump) is a coordinated cut
  deferred with the trigger written in the S3 openIssues.
- **`ts`'s test script keeps its inner build**: CI's sdk lane invokes
  `pnpm test` standalone, so the battery is not the only invoker.

## S2 — the bridge and the cutover

- **The handle is the fingerprint authority.** The batch-header wire
  carries no fingerprint in either direction: encode fills it from the
  sealed handle, decode already refused any mismatch.
  `EncodeError::FingerprintMismatch` is unconstructible from TS and
  stays a mint-table row. Consequence ruled for S3: the TS
  `EncodeHeader.fingerprint` field is dead data and dies.
- **Tagged in, plain out.** Encode ops cross as tagged values; decode
  rows cross as the engine's `ValueOut` walk (`FactValue`-shaped) — the
  engine's own query-literal/answers asymmetry, keeping the bridge
  layout-blind so the core is the one judge.
- **Grammar outcomes cross as one sum** —
  `{ok:true,value} | {ok:false,kind,message}` — never throws; a refusal
  of hostile bytes is a domain outcome. Bridge refusals mint bare
  `{kind}` causes: per-site fields have no honest source one
  implementation away; the detail rides the message.
- **The mint table is the marshal's gate** (`ts/crate/log-identities.json`,
  include_str!-locked to the Rust rosters): an unknown identity is a
  loud bridge error, never a silent new wire string. `EncodeError` has
  no core `identity()` — spelled by an exhaustive `wire_tags!` table;
  S3's generator unifies to one speller.
- **`verifyChain` stays host-side** — pure slot algebra over decoded
  headers, not grammar. `DigestWidth` and the lone-surrogate gate stay
  seat-side: they refuse before a value can cross corrupted.
- **The `known: ReadonlySet<Braid>` parameters died** — the sealed
  handle is the one braid authority; callers pass the codec, not a
  roster. The handle's cache home is `Descriptor.codec`, minted once
  per theory; no seat mints a lifecycle twin.
- **The tenant dir-lease lives at `{parent}/~lease/{tenant}`**,
  outside the swept replica dir (fence.rs `acquire_named` as law);
  tenants carries no lease codec of its own.
- **The tilde table is consumed at runtime in Rust too**
  (include_str! + LazyLock); Zl/Zp/Zs spelled as
  `char::is_whitespace` minus Cc — the subsumed special case died.
- **f3's reservation fixtures were rewritten, not deleted**: the race
  shapes test loss re-judgment against the surviving weighted Capacity
  statement; only the verb was reservation-shaped.
- **`headerWriter`'s fixed-offset usurper sniff stays** — Rust's writer
  machine reads the same offset by design (an undecodable occupant
  still names the slot's owner); `headerTimestamp` had no Rust twin and
  died — the timestamp rides the held pending arm.
- **The scratch grammar and lease namespace left the public surface**;
  the seat-backed `encodeBatch`/`decodeBatch`/`verifyChain` stay public
  (the grammar is log-owned; the seat is its one reader).
- **Inventory registration styles**: case families register
  chain-style (mixed `ok_`/`r_` stems); table goldens register as
  single paths; `surfaces.json` stays out of `inventory.json` — two
  rosters, two jobs.

## S1 — engine-side

- **`catalogDigest` required a new napi binding**, not a type export:
  the bridge never exposed the core's `Db::catalog_digest`; the binding,
  the `native.ts` declaration, and the index export landed together.
- **`ManifestField` widened with `fresh`/`newtype`** so the descriptor
  spec re-join could die; the bridge stays dumb — shape only.
