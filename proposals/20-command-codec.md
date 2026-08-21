# 20 — The command codec

One binary format, implemented twice (Rust in `bumbledb-log`, TS in
`@bjornpagen/bumbledb-log`), pinned equal by cross-goldens (80). Commands
carry **raw values, never intern ids** — interning is store-local and
replays deterministically; footprint keys are hashes of these raw values
(15), so the footprint section is verifiable by pure recomputation.

## The batch

```
magic        4  bytes   "BDBL"
version      u16 LE     2        (consumers refuse ≠ 2)
flags        u16 LE     0        (bit 0 reserved for zstd; must be 0)
fingerprint  32 bytes            (schema fingerprint; refuse mismatch)
braid        u32 LE              (braid id = smallest RelationId in the component)
braid_gen    u64 LE              (this batch produces braid-generation braid_gen;
                                  applies only when the replica's vector[braid] == braid_gen − 1)
timestamp    u64 LE              (unix millis, informational only — PITR-by-time
                                  convenience; never identity, never ordering)
op_count     u32 LE
ops          …
fp_count     u32 LE
footprint    fp_count × entry    (sorted ascending by (class, key); duplicates refused)
```

One batch = one engine write transaction = one braid-generation advance.
Group commit packs many host writes into one batch; the batch is the
commit unit. Every op's relation must belong to `braid` — a spanning
batch is unencodable by construction (the writer auto-splits, 60).

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
3 String        u32 LE len + UTF-8 (validated at decode; parse, don't validate)
4 FixedBytes    u32 LE len + raw (len must equal the declared width)
5 Interval      2 × u64 LE (start, end; engine validates the half-open law at apply)
6 FixedInterval u64 LE (start; width from the layout)
```

Row tags must match the relation's layout exactly; decode refuses with a
typed error naming relation, row, and field. Decode is a full parse before
any apply.

## Footprint entries (fixed-size; the algebra's keys from 15)

```
class        u8      1 = F (fact), 2 = K (key), 3 = C (containment), 4 = W (capacity)
statement    u16 LE  (StatementId; 0 for class F)
key          32 bytes (fid for F; fkey otherwise — blake3 over raw values per 15)
mode         u8      F: 1 insert / 2 delete
                     K: 1 write
                     C: 1 need / 2 support+ / 3 support−
                     W: 1 childΔ / 2 parent+ / 3 parent−
delta        i64 LE  (class W, mode 1 only: signed weight sum; 0 otherwise)
```

The section is **derivable**: `footprint(descriptor, ops)` is a pure
function both implementations expose; encoders call it, replicas recompute
it during replay and refuse a mismatch (`FootprintMismatch`,
corruption-class). Publishing a derivable section is deliberate — the CAS
loser must intersect against the winner's footprint *without applying the
winner first*, and the recompute-on-replay keeps it honest.

## Apply

`apply(db, vector, batch)`:

1. Refuse version ≠ 2, flags ≠ 0, fingerprint mismatch, unsorted or
   duplicate footprint entries, op relation outside `braid`.
2. `vector[braid] > braid_gen − 1` → `AlreadyApplied` (idempotent replay);
   `<` → `GapDetected`. Equality proceeds.
3. Recompute the footprint from ops; mismatch → `FootprintMismatch`.
4. One `db.write` applying ops in listed order (rows in listed order).
5. The engine verdict must be `Accepted`; `Rejected` during replay is
   `ReplayDiverged` (writers publish only accepted batches; determinism
   guarantees agreement). Bump `vector[braid]` under the sidecar
   discipline (50).

## The determinism laws (engine properties this format leans on; pinned in 80)

1. Canonical commit order — host op order cannot influence stored bytes.
2. Deterministic intern minting in first-use apply order (30's written
   law).
3. Deterministic row ids; fresh ids ride in the commands as plain values.
4. Judgment is a pure function of (state, batch) — Lean-pinned.
5. Cross-braid apply order is irrelevant to final content (L8/L9).

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
decode-compatibility plus footprint-recompute equality; byte-exact
cross-implementation *encoding* is the chosen stronger discipline because
it makes the goldens trivial — a testing convenience, not a correctness
dependency (even the ambiguity-retry compare in 40 uses the writer's
in-memory bytes).

## Not in the codec

No intern ids (store-local). No row ids for non-fresh relations
(derived). No floor ops (id leases, 10). No schema payloads (migration
out of scope). No ordering meaning in `timestamp`.
