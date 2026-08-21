# 20 — The command codec

One binary format, implemented twice (Rust in `bumbledb-log`, TS in
`@bjornpagen/bumbledb-log`), pinned equal by cross-goldens (80). Commands
carry **raw values, never intern ids** — interning is store-local and
happens deterministically at apply.

## The batch

```
magic        4  bytes   "BDBL"
version      u16 LE     1        (consumers refuse ≠ 1)
flags        u16 LE     0        (bit 0 reserved for zstd; must be 0 in v1)
fingerprint  32 bytes            (schema fingerprint; apply refuses mismatch)
base         u64 LE              (base_generation: applies only when store.generation() == base)
op_count     u32 LE
ops          …                   (op_count ops, in order)
```

The batch producing generation `g` has `base = g − 1` and lives at
`log/{g:016x}`. One batch = one engine write transaction = one generation
advance (group commit packs many host writes into one batch; the batch is
the commit unit).

## Ops

```
op_kind      u8      1 = Insert, 2 = Delete, 3 = FloorBump
```

**Insert / Delete** (`1` / `2`):

```
relation     u32 LE              (RelationId — stable under the pinned fingerprint)
row_count    u32 LE
rows         row_count × row     (row-major, fields in declaration order)
```

Each field value is `tag u8` + payload; tags mirror the engine's
`ValueType` arms:

```
0 Bool          u8 (0|1; other bytes refuse)
1 U64           u64 LE
2 I64           i64 LE
3 String        u32 LE len + UTF-8 bytes (validated at decode; parse, don't validate)
4 FixedBytes    u32 LE len + raw bytes (len must equal the field's declared width)
5 Interval      2 × u64 LE (start, end — the engine validates half-open law at apply)
6 FixedInterval u64 LE (start; width from the layout)
```

A row's field tags must match the relation's layout exactly; decode refuses
otherwise (typed error naming relation, row index, field index). Decode is
a full parse before any apply — the batch-level parse-all-first law, same
as the engine's collection writes.

**FloorBump** (`3`) — the failover safety op (60-writer.md):

```
relation     u32 LE
field        u16 LE              (FieldId of the fresh field)
floor        u64 LE              (advance Q floor to at least this value)
```

Applied via the engine's existing `reserve_at`-shaped floor advance inside
the same write transaction; a floor already ≥ the value is a no-op.

## Apply

`apply(db, batch)`:

1. Refuse `version ≠ 1`, flags ≠ 0, fingerprint mismatch.
2. If `db.generation() > base`: the batch is already applied — return
   `AlreadyApplied` (idempotent replay). If `< base`: `GapDetected` (the
   caller must fetch the missing objects first). Equality proceeds.
3. One `db.write` applying ops **in listed order**: inserts via the dyn
   insert path, deletes via the dyn delete path, floor bumps via the floor
   advance. Rows within an op apply in listed order.
4. The result is the engine's `Admission`. **`Rejected` during replay is a
   protocol violation** (`ReplayDiverged` — corruption-class, never
   retried): the writer only publishes accepted batches, and determinism
   (below) guarantees replicas reach the same verdict.

## The determinism laws (what makes replay byte-exact)

These are properties the engine already has; the codec relies on them and
80 pins them:

1. **Canonical commit order** — the engine sorts the delta into
   `(relation, fact_hash)` order at plan time; host/list order inside a
   batch cannot influence stored bytes.
2. **Deterministic intern minting** — pending intern ids are assigned in
   first-use order during apply; identical batches applied to identical
   stores mint identical ids. (30 records this as a written engine
   guarantee with its own test.)
3. **Deterministic row ids** — non-fresh row ids assign in canonical
   order; fresh-keyed rows take the id from the fresh field value carried
   *in the command* (raw u64), so replicas never re-mint.
4. **Rejection determinism** — the two-phase judgment is a pure function
   of (store state, batch); Lean pins it.

Corollary (the equality gate): same checkpoint + same log prefix ⇒
identical `catalog_digest` (30) on every replica. LMDB *file* bytes may
differ (page allocation is not part of the contract); catalog **content**
bytes may not.

## Refused alternatives (recorded ruling — do not re-litigate without the trigger)

Protobuf, FlatBuffers, Cap'n Proto, CBOR/MessagePack, and serde-native
formats were considered and refused:

- **The schema already exists.** The theory pinned by the header
  fingerprint *is* the schema; row layouts derive from relation layouts;
  value tags mirror `ValueType` 1:1. An IDL would be a second schema
  system describing data that has a first — a third source of truth
  beside this doc and the engine layout.
- **Wire-optional fields are validation, not parsing.** Proto3 decodes a
  missing header field as a zero value; the illegal states become
  representable and every consumer re-guards. This format makes them
  unparseable instead.
- **Non-canonical encodings break the golden discipline**, and
  unknown-field tolerance is an anti-feature for a replication log — a
  replica skipping an unknown op is corrupt, not compatible. Versioned
  refusal (the format-8 philosophy) is the evolution story.
- **Zero-copy buys nothing here**: batches are read once, sequentially,
  and materialized into `Value`s for the dyn apply path regardless.
  Length-prefixed sequential reads with no offsets are also the most
  auditable and fuzzable parser shape for bytes a compromised writer
  could poison.

Reopen trigger: third parties producing/consuming batches without our
libraries, or a requirement for mixed-version fleets. Neither exists; both
are out of scope by 00.

**Precision on determinism:** the protocol's hard requirement is
decode-compatibility only — log objects are written once, and even the
ambiguity-retry compare (40) uses the writer's in-memory bytes, never a
re-encode. Byte-exact cross-implementation *encoding* is the chosen
stronger discipline because it makes the golden corpus trivial; it is a
testing convenience, not a correctness dependency.

## What is deliberately not in the codec

No intern ids (store-local). No row ids for non-fresh relations (derived).
No timestamps (a command is content, not an event record; the log key is
the order). No per-op fingerprints (the batch header's one fingerprint
covers all ops). No schema payloads (migration is out of scope).
