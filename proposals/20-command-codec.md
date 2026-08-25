# 20 — The command codec

One grammar, one codec, two thin drivers (Rust in `bumbledb-log`, TS in
`@bjornpagen/bumbledb-log`), pinned equal by cross-goldens (80). Commands
carry **raw values, never intern ids** — interning is store-local and
replays deterministically. A batch is a header and its ops, nothing
else: every consumer judges the ops through the engine, and no carried
claim exists for a consumer to trust or a hostile writer to lie in.

## The batch

```
magic        4  bytes   "BDBL"
version      u16 LE     3        (consumers refuse ≠ 3; a well-formed
                                  v:2 batch is Version — no translator)
flags        u16 LE     0        (bit 0 reserved for zstd; must be 0)
fingerprint  32 bytes            (schema fingerprint; refuse mismatch)
braid        u32 LE              (braid id = smallest RelationId in the component)
braid_gen    u64 LE              (this batch produces braid-generation braid_gen;
                                  must equal the slot number in the object key)
prev         32 bytes            (blake3 of the predecessor log object's bytes;
                                  32 zero bytes at braid genesis. The chain as
                                  representation: a wrong-base or out-of-sequence
                                  slot is refusable before any apply — Aurora's
                                  backlink, at one hash per batch)
writer       u64 LE              (writer instance id, minted at writer open;
                                  provenance + the field that makes "corruption-
                                  class, naming both writers" constructible)
timestamp    u64 LE              (unix millis; the writer clamps at encode:
                                  ts = max(now, predecessor's ts), so the braid's
                                  timestamps are monotone by construction and
                                  apply refuses violations. PITR-by-time
                                  convenience; never identity, never ordering)
op_count     u32 LE
ops          …
```

The version is **3**. A well-formed v:2 batch is a typed `Version`
refusal — there is no translator, no dual-read path. The one version
number stands on both the wire and the documents (10, 50).

One batch = one engine write transaction = one braid-generation advance.
Group commit packs many host writes into one batch; the batch is the
commit unit. Every op's relation must belong to `braid` — a spanning
batch is unencodable by construction (the writer refuses spanning
commits; `commit_split` is the explicit verb, 60).

## Ops

```
op_kind      u8      1 = Insert, 2 = Delete        (3 was FloorBump; DELETED — id
                                                    leases in 10 replace it; decoders
                                                    refuse kind 3 like any unknown)
relation     u32 LE
row_count    u32 LE
rows         row_count × row     (row-major, fields in declaration order)
```

Field values are `tag u8` + payload, tags mirroring `ValueType`:

```
0 Bool          u8 (0|1; other bytes refuse)
1 U64           u64 LE
2 I64           i64 LE
3 String        u32 LE len + UTF-8 (refused at decode if invalid; parse, don't validate)
4 FixedBytes    raw, exactly the layout's declared width (no length field —
                the layout already answers it; a length would be a second
                answer that could disagree)
5 Interval      2 × u64 LE (start, end; `start ≥ end` refuses at decode
                as `EmptyInterval` — the engine's half-open law,
                `start < end`, enforced at the wire boundary the way
                the engine enforces it at its own)
6 FixedInterval u64 LE (start; width from the layout). A start whose
                end would be the domain ceiling is `IntervalOverflow`
                — the ceiling is not a value; decode and the encode
                gate refuse `end == MAX` identically.
```

Row tags must match the relation's layout exactly; decode refuses with a
typed error naming relation, row, and field. Decode is a full parse before
any apply. The row count and the row bytes are **one length-delimited
type**: a declared `row_count` or `op_count` the remaining bytes cannot
back is `Truncated` immediately — a zero-field relation cannot amplify
a hostile count into an unbounded loop. A string cell is
`WellFormedUtf8` from a fatal encoder; lone surrogates refuse rather
than emit U+FFFD. Every digest field is `[u8; 32]`; a short `prev`
cannot encode.

## Apply

`apply(db, chain, batch)` — `chain` is the replica's per-braid position
`(g, prev_hash, prev_ts)` from the sidecar (50):

1. Refuse version ≠ 3, flags ≠ 0, fingerprint mismatch, op relation
   outside `braid` — and the
   chain discipline, one identity with three proved causes:
   `ChainMismatch{Slot}` when `header.braid_gen ≠` the slot number in
   the key the object was fetched from; `ChainMismatch{Prev}` when
   `header.prev ≠ chain.prev_hash`; `ChainMismatch{Timestamp}` when
   `header.timestamp < chain.prev_ts`. All corruption-class: the chain
   itself proves which writer misbehaved (the header carries its id).
2. One `db.write` applying ops in listed order (rows in listed order).
3. The engine verdict must be `Accepted`; `Rejected` during steady-state
   replay is `ReplayDiverged` (writers publish only accepted batches;
   determinism guarantees agreement). Advance the chain to
   `(header.braid_gen, blake3(batch bytes), header.timestamp)`, under
   the sidecar law (50).

**A first-applied slot must change state.** The publish law guarantees
every log slot is a state-changing commit, and apply enforces it: a
   net-no-op apply that leaves `generation` *below* the post-advance
   `generation(chain)` identity is a publish-law violation in the log —
corruption-class, naming the slot's writer. The legitimate no-op — a
crash-window re-absorption, where the store was already one generation
ahead — lands the identity exact and passes. One instrument, both
verdicts.

**There is no `AlreadyApplied`.** Apply is idempotent by set semantics:
re-applying a batch whose effects are already present net-disposes every
op (insert-of-present and delete-of-absent are engine no-ops), the delta
is empty, and the engine takes its no-op arm — judgment never runs, no
LMDB commit happens, and the generation does not advance
(`crates/bumbledb/src/storage/commit/write.rs`: "The empty delta is the
*only* no-op commit shape … the tx id does not advance"). The crash
window between an engine commit and its sidecar bump therefore needs no
detection state at all: recovery re-applies the slot, the engine absorbs
it, the vector catches up. The one state machine deleted here is the one
every page-image WAL (SQLite frames, Aurora LSNs) is forced to carry —
salts, checksums, and applied-watermarks exist downstream of a
representation whose replay is not idempotent. Ours is.

`GapDetected` is not a codec outcome either — the replica layer decides
tip-vs-hole from the manifest's current checkpoint before any fetch (50).

## The determinism laws (engine properties this format leans on; pinned in 80)

1. Canonical commit order — host op order cannot influence stored bytes.
2. Deterministic intern minting in first-use apply order (30's written
   law).
3. Deterministic row ids; fresh ids ride in the commands as plain values —
   replay never calls engine reserve.
4. Judgment is a pure function of (state, batch) — Lean-pinned.
5. Cross-braid apply order is irrelevant to final content (L9).
6. **Replay idempotence (L10).** Applying a batch whose effects the store
   already contains is the engine's no-op commit: empty net delta, no
   judgment, no LMDB commit, no generation advance. Corollary: after full
   catch-up, `db.generation() == generation(chain)` on every honest
   store — `generation(Settled{v}) = v.sum()`,
   `generation(Pending{v, _}) = v.sum() + 1` (50) — the equality is the
   phantom detector, not a bookkeeping aspiration.

## Refused alternatives (recorded ruling — reopen triggers only)

Protobuf, FlatBuffers, Cap'n Proto, CBOR/MessagePack, serde-native formats:

- **The schema already exists** — the fingerprint pins the theory; row
  layouts derive from it; an IDL would be a second schema system and a
  third source of truth.
- **Wire-optional fields are validation, not parsing** — proto3 decodes
  absent headers as zeros; illegal states become representable and every
  consumer re-guards. This format makes them unparseable.
- **Non-canonical encodings** break the golden discipline, and
  unknown-field tolerance is an anti-feature for a replication log —
  skipping an unknown op is corruption, not compatibility. Versioned
  refusal is the evolution story.
- **Zero-copy buys nothing** — batches are read once, sequentially,
  materialized into `Value`s for the dyn apply path; and the offset-free
  sequential parser is the auditable, fuzzable shape for bytes a
  compromised writer could poison.

Reopen triggers: third parties producing/consuming batches without our
libraries, or mixed-version fleets. Neither exists; both out of scope
by 00.

**Precision on determinism:** the protocol's hard requirement is
decode-compatibility; byte-exact
cross-implementation *encoding* is the chosen stronger discipline because
it makes the goldens trivial — a testing convenience, not a correctness
dependency (even the ambiguity-retry compare in 40 uses the writer's
in-memory bytes).

## Not in the codec

No intern ids (store-local). No row ids for non-fresh relations
(derived). No floor ops (id leases, 10). No schema payloads (migration
out of scope). No ordering meaning in `timestamp`. No carried conflict
claims of any kind: the batch is header + ops, and every consumer
judges the ops through the engine.
