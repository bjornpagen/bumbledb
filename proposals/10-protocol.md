# 10 — The braided object protocol

## Braids

At driver initialization the schema descriptor's statement graph —
ordinary relations as nodes; an edge wherever a containment or capacity
statement relates two of them (FDs are self-loops; closed relations and
closed-target statements contribute nothing, per 15) — decomposes into
connected components: **braids**. The braid id is the smallest
`RelationId` in the component, rendered `c{id:08x}`, and is scoped to the
schema fingerprint: braid identity across fingerprints is migration's
problem, out of scope with the rest of migration. Braid assignment is a
pure function of the descriptor, implemented in both languages, pinned by
the codec goldens (80). A theory whose relations are all connected has one
braid and this protocol degenerates to the serial log — the serial design
survives as the special case, not a mode.

Statements never span braids, so braids never conflict (L9): each has an
independent chain, and cross-braid ordering is semantically invisible.

Recorded v2 refinement, with prior art: the data-exchange dependency
graph (Fagin et al., Def 3.7) is built over *positions* — (relation,
field) pairs — not whole relations, so two statements sharing a relation
through disjoint field sets are provably independent at a finer grain
than the statement graph sees. Position-level braids would split further
with zero semantic change; the trigger is a measured hot braid whose
statements touch disjoint projections.

## Key layout

Generation numbers zero-padded lowercase hex, 16 chars. A prefix is a
store; a tenant is a prefix (`<root>/t/<tenant>/…`, control-plane at
`t/_shared/…`).

```
<prefix>/manifest.json                     — the pointer (CAS-guarded)
<prefix>/log/{braid}/{g:016x}              — batch producing braid-generation g (create-only)
<prefix>/ckpt/{digest}.mdb                 — compacted store, named by its own blake3 (immutable)
<prefix>/ckpt/{digest}.json                — the checkpoint's vector, chain heads, and backlink
<prefix>/ids/{relation:08x}/{field:04x}    — fresh-id lease counter (CAS)
```

There is no escrow prefix: capacity reservations are ordinary rows in the
log, not protocol objects (15).

Checkpoints are content-addressed. The earlier `ckpt/{vector-sum}` naming
had a reachable collision — two writers on different braids can hit
checkpoint duty at distinct vectors with equal sums, and the loser's
digest-refusal would wedge bootstrap until a new checkpoint landed. Naming
by digest deletes the collision *and* makes every local checkpoint cache
a pure revalidation (Delta Lake's immutability-caching argument).

## Generations and the vector

The store generation is the **vector sum** (00 law 2): each braid's
chain is indexed by its applied count, and the engine's one counter is
their total. Concretely: batch objects
carry `(braid, braid_generation)` in the header; a replica's chain state
is `{braid → (applied count, head hash, head ts)}`; the engine generation
equals the sum of the applied counts, because every published batch is
state-changing (the publish law below) and every state-changing commit
advances the generation exactly once. Nothing else counts anything.

## The manifest (the protocol's one mutable object)

Canonical single-line UTF-8 JSON, strict parse, field order fixed:

```json
{"v":2,"fingerprint":"<64 hex>","checkpoint":"<64 hex digest>"}
```

Three fields; `checkpoint` is `null` from store birth until the first
checkpoint lands (a JSON null, a real sum arm — never `""`). The manifest
is a **pure pointer**: every checkpoint fact (sum, vector, chain heads,
backlink) lives in the immutable, content-addressed `ckpt/{digest}.json`
it points to, so the mutable CAS surface of the entire protocol is one
64-hex field. The deleted candidates are deletions on purpose: a `floors`
head-hint answered "where is the head?" a second way (the probe is the
way — one way per question) with no named consumer; a `writer` string was
an advisory arrangement whose null was `""` — the house crime by name.
Who may write is answered by slot arbitration and nothing else; writer
identity for diagnostics rides in every batch header (20). Version
evolution is a recorded reopen trigger (split `min_reader`/`min_writer`
gates, Delta's `protocol` action) that activates the first day two
deployment versions coexist — not before.

Heads are discovered by forward probing (`GET log/{braid}/{k+1}` until
404), and the current checkpoint's vector doubles as the **gc floor**:
the retention law never deletes a log object ≥ that vector, so a 404 at
`k+1` is the tip if `k+1 > checkpoint vector[braid]` (or if `checkpoint`
is null — nothing has ever been gc-eligible), and a gc'd hole
(`GapDetected`) otherwise — the 404 is never ambiguous (50 owns the
rule). The freshness this rule needs is cheap by construction: poll the
manifest with `get_if_changed` (304 in the steady state), and fetch the
checkpoint json once per checkpoint change — it is immutable and
digest-keyed, so it caches forever. Manifest creation:
`If-None-Match: *`; update (checkpoint publication): `If-Match: <etag>`;
412 ⇒ re-read and apply the **checkpoint order**: the candidate replaces
the incumbent iff its vector sum is strictly greater; otherwise the
incumbent stays and the candidate's objects become `gc` fodder. Vectors
are pointwise-incomparable in general (two braids, two checkpointers),
so "newer" needed a defined total order — sum is it; every checkpoint is
a real serial state either way, and the gc exemption law is conservative
under either choice. The manifest is never updated per commit.

## Log objects

`log/{braid}/{g}` is created with `If-None-Match: *` and is immutable
forever. Exactly one writer wins each slot. `Exists` resolves by
**fetch-and-compare first**: byte-equal means *we* created it earlier (an
ambiguous PUT retried — 40's law, absorbed here rather than special-cased);
unequal means we lost, and the loser algebra runs (15): recompute the
winner's footprint from its ops, intersect, then subsume (drop),
republish (fully key-disjoint), or re-judge (anything else). Both batches share base g−1 in that
braid — a fact the `prev` chain hash now *proves* per object rather than
assumes (20): a batch built on the wrong base is `ChainMismatch` at every
replica, refusable before any apply. The batch carries its footprint
section; replicas recompute and refuse mismatches.

**The publish law.** A batch is published only if its local application
advanced the generation. Rejections never reach the network; net-no-op
commits (all effects already present — e.g. a loser fully subsumed by the
winner) never reach it either. Consequence: every log slot is a
state-changing commit, so `engine generation ≡ Σ vector` on every honest
store at rest after catch-up (50 states the general form, with the
applied-pending term) — the identity the whole recovery story leans on
(50), kept true by construction at the only place it could break.

## Checkpoints

`compact()` output uploaded as `ckpt/{digest}.mdb` (digest = blake3 of
the `.mdb` bytes), beside `ckpt/{digest}.json` — one map, one fact per
braid, nothing derivable stored:

```json
{"braids":{"c00000001":{"g":80,"hash":"<64 hex>","ts":1755801600000},"c00000005":{"g":43,"hash":"<64 hex>","ts":1755801599120}},"catalog":"<64 hex>","writer":12345,"prev":null}
```

Per braid: the applied count `g`, the blake3 of the head log object, and
its timestamp — the hash seeds the `prev`-chain verification and the
timestamp seeds the monotone-ts check (20) across a checkpoint jump, so
both chain disciplines survive bootstrap, not just tail replay.
`catalog` is the `catalog_digest` (30) of the checkpointed state and
`writer` the publisher's id — the checkpoint's **content** claim, not
just its name: a fresh open verifies it after opening (one sequential
pass, off any hot path), and any store that reaches the same vector by
its own replay compares and refuses a mismatch as corruption-class,
naming the publisher. Without this, every other carried claim was
recomputed while the checkpoint verified only against itself — a
poisoned `.mdb` at the right generation with honestly copied heads would
have become the unauditable root of truth once gc passed. The recorded
trust boundary that remains: history nobody ever replays again is vouched
for by the publisher alone, and the gc window R is therefore also the
audit window — retention is not only backup policy but the time the
fleet has to catch a lying checkpoint. The
vector is the `g` column; its sum is derived, never stored (storing the
design's own central identity beside its derivation would be a second
answer waiting to disagree). `prev` backlinks to the previous
checkpoint's digest (`null` at the first checkpoint): the checkpoint
history is walkable from the manifest with GETs alone (the PITR restore
search needs old checkpoints; we banned LIST; the backlink is the
discovery mechanism — Delta's `_last_checkpoint`-plus-search, without
the LIST half). Cadence — owned here; other files cite: every K = 256
applied batches (vector-sum delta) or 16 MiB of log, whichever first
(chosen constants; F11's cold-open and log-volume pins re-size them).
`compact()` and the upload run **off the braid commit loops** —
checkpoint duty never stalls a hot braid. Publication: `put_create` both
objects in either order — both are immutable and unreferenced until the
manifest CAS lands, which is the linearization point; a crash before the
CAS leaves unreferenced `gc` fodder, never a dangling pointer. Restore
verification: blake3(`.mdb`) = digest, opened generation = Σ `g`,
fingerprint match — refusals, never warnings. Publication races are
benign (the manifest CAS applies the checkpoint order above; losing
checkpoint objects are `gc` fodder).

## Fresh-id leases

`ids/{relation}/{field}` holds a canonical u64 (decimal ASCII): the next
unleased id. Birth is `put_create` with body `4096` — the creator thereby
claims `[0, 4096)`; `Exists` at birth means someone else was first, and
the ordinary path applies. A writer leases `[n, n+4096)` by
CAS-incrementing (width owned here, cited by 40; chosen — it amortizes
counter traffic 4096× below slot traffic, which is also the recorded
reason the counter race needs no pressure valve of its own); commands
carry concrete ids.
Cross-writer collision is structurally impossible; the counter object is
the failover floor (adoption reads it — no in-log floor ops exist; the
old FloorBump op is deleted from the codec). The counter is coordination,
not truth: replay determinism never depends on it, because ids ride in the
commands. Leased ids are unique, never dense, deliberately: sequentiality
is the one uniqueness flavor that is provably non-coordination-free
(Bailis Table 2, AUTO_INCREMENT; the only two non-I-confluent TPC-C
invariants are the sequential-ID ones), and nobody gets to "improve"
leases into sequences later.

## Retention, truncation, PITR

- Restore point = a **vector** — and *any* vector is a legal restore
  point: braids are independent (L9), so every pointwise combination of
  braid prefixes is a real serial state satisfying the full theory. This
  is strictly stronger than single-chain PITR (a WAL restores to points
  on one line; we restore to a lattice). Restore = walk the checkpoint
  backlink chain from the manifest to the first checkpoint whose vector
  is `≤ v` pointwise, open it, then replay each braid to its target —
  braid order irrelevant (L8).
- By-time restore maps a wall-clock instant through the batch timestamps:
  per braid, the largest g with `ts ≤ T`. Timestamps are clamped monotone
  per braid at publish (20) and refused otherwise, so the mapped set is a
  prefix by construction, not by hope. Cross-braid, wall clocks are
  writer-local, so an instant names no single cross-braid cut; the
  restored vector — not the instant — is the truth the restore reports.
- The `gc` verb (v1; lifecycle rules are v2 automation) deletes log
  objects and checkpoints older than window R, always exempting the
  current checkpoint and every log object ≥ its vector, per braid; a
  store whose manifest still says `checkpoint: null` has nothing
  gc-eligible, by the same rule. The
  default checkpoint window is generous (90 days): checkpoints live on
  standard-class storage where old versions are nearly free, and keeping
  them is what makes continuous backup a non-feature (Turso's diskless
  production made the same call — `docs/research/replication-prior-art/turso-notes.md`).
- **Storage classes are part of the protocol's shape**: `log/*` targets
  the low-latency class (S3 Express One Zone directory buckets, or R2);
  `ckpt/*` and `manifest.json` target the standard class (cheap,
  multi-AZ). Measured basis: Express 4 kB PUT avg ≈ 6.4 ms p99 ≤ 10 vs
  standard avg ≈ 31 ms p99 102 — the split is not optional at
  latency-sensitive commit rates.
- The protocol needs exactly immutable logs + one CAS key from its
  store; anything a bucket offers beyond that (versioning, replication
  tiers) is the deployment's business, not this document's.

## Store properties required (verified per vendor in 40)

Strong read-after-write GET; atomic create-only PUT; atomic If-Match CAS.
Nothing else — no LIST consistency, no multi-key atomicity, no append.

## Failure semantics

| Event | Outcome |
| --- | --- |
| Crash anywhere in a commit or catch-up | recovery **is** the apply loop: resolve `pending` (60), then replay forward; re-application of anything already applied is the engine's no-op arm (L10) — there is no separate recovery procedure to get wrong |
| CAS `Exists` on a log slot | fetch and compare: equal bytes ⇒ ours (ambiguous PUT absorbed); else the loser algebra (15): subsume, republish, or re-judge |
| Manifest CAS 412 | re-read, apply the checkpoint order (greater sum replaces; otherwise incumbent stays), retry |
| `generation ≠ Σ vector` after full catch-up + pending resolution | phantom or torn store — discard the directory, re-pull (cache, never truth) |
| Footprint recompute ≠ published section | `FootprintMismatch` — corruption-class, never retried |
| Chain discipline violated (`prev` ≠ predecessor hash, header gen ≠ key slot, or ts < predecessor's) | `ChainMismatch{Prev \| Slot \| Timestamp}` — one identity, three proved causes; corruption-class, naming braid, slot, and writer (the header carries the writer id) |
| Batch rejected during steady-state replay | `ReplayDiverged` — corruption-class (the publish law + determinism make it impossible for honest writers) |
| 404 at `vector+1` with `vector+1 ≤` the current checkpoint's `vector[braid]` | `GapDetected` (gc'd tail) — discard, re-open from the current checkpoint; the same 404 above that vector (or with no checkpoint yet) is the tip, by the gc exemption law |
| Poisoned slot (corruption-class refusal fleet-wide) | the braid wedges at g−1 *by design* — refusal is the point — while the store's other braids keep serving and accepting writes (50; L9 makes partial service sound). Supersession requires a checkpoint whose vector passes the slot, and only a store that applied the slot can produce one: the faulty writer itself, or a store running fixed code that accepts it. A slot every honest store permanently refuses halts its braid; the recovery is then an operator's PITR restore to the pre-poison vector under a fresh prefix — the same ending every corrupted-log system reaches, stated instead of discovered. Nobody edits a bucket by hand |
| Fingerprint mismatch anywhere | typed refusal; migration out of scope |
