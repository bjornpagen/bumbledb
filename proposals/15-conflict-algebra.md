# 15 — The conflict algebra

The centerpiece. Bumbledb's constraint set is closed and compiled — three
families (functionality, containment, capacity) plus fact identity — so the
question "do two concurrent commits interfere?" is *computable per commit*,
as data. General databases cannot have this document; their constraint set
is open. This one is four tables and five theorems (L6–L10).

CRDT distinction, recorded: CRDTs obtain convergence by weakening to
always-commuting operations, which is why no CRDT expresses an FD, an IND,
or a ceiling. This algebra keeps the full theory and **derives** which
operations commute *from* it. Invariants are never weakened; concurrency is
extracted, and the extraction is machine-checked (Lean obligations below).

## Footprint keys are raw-value hashes (a load-bearing ruling)

Every footprint entry is keyed by

```
fkey = blake3( statement_id_le ∥ tagged raw values of the projection, in field order )
```

computed from the **raw command values** (strings as UTF-8, never intern
ids) and the schema descriptor's own projections. Consequences, each
deliberate:

1. **No engine seam.** The footprint is a pure function
   `footprint(descriptor, ops)` implemented in the driver, twice (Rust/TS),
   pinned by cross-goldens. The engine never learns replication exists.
2. **No aliasing.** Interned images are store-state-relative: two
   concurrent writers can mint the same intern id for *different* strings,
   making image bytes collide across commits. Raw-value hashes are
   state-independent — equal keys mean equal values, full stop.
3. **Verification is recomputation.** A replica recomputes the footprint
   from the ops during replay; a mismatch with the published section is
   `FootprintMismatch` (corruption-class). Claims are carried *and*
   checked — the three-oracle habit at the protocol layer.

Fact identity uses the same discipline:
`fid = blake3(relation_id_le ∥ tagged raw values of the full row)`.

## The footprint of a batch

Derived from the ops and the descriptor (all projections, key rosters,
containment mappings, and weight specs are descriptor data):

| Class | Entry | Emitted when |
| --- | --- | --- |
| **F** fact | `(relation, fid, +)` / `(relation, fid, −)` | net insert / net delete of the row |
| **K** key | `(K, fkey(det))` for every key statement K of the relation | any insert or delete of a row (its determinant is written either way) |
| **C** containment, target-side | `(C, fkey(target det), need)` | inserting a **source** row whose projection references that target group |
| | `(C, fkey(target det), support+)` | inserting a **target** row that establishes the group |
| | `(C, fkey(target det), support−)` | deleting a target row that supported the group |
| **W** capacity | `(W, fkey(parent det), Δ)` with `Δ ∈ ℤ` (i64, signed sum of child weights added minus removed; unit weight = 1) | inserting/deleting children of the parent group |
| | `(W, fkey(parent det), parent±)` | inserting/deleting the parent row itself |

Closed relations never emit entries: sealed rows never change, and
closed-target checks are delta-local — **closed statements are
conflict-free by construction**.

## The commutativity matrices

Two commits built on the same base conflict iff any table below says
CONFLICT for some shared key. "Commute" is proven (Lean L7 + L8): either
apply order yields the identical final state (L8) and identical verdicts
(L7).

The tables are two-sided. The commute cells are *sufficient* freedom,
mechanized per pair by L7/L8. The CONFLICT cells are *necessary*
coordination: Bailis's Theorem 1 (invariant confluence is necessary and
sufficient for coordination-free execution) proves per invariant class
that uniqueness (his Claim 3), referential integrity under deletion
(Claim 7), and over-slack numeric bounds (Claims 12/13) cannot be
coordination-free without weakening semantics. The matrices are not
cautious; they are minimal — shrinking any CONFLICT cell requires either
cascade-style merge synthesis or CRDT-style weakening, both refused
by name in this document.

**F — same `fid`:**

| | insert | delete |
| --- | --- | --- |
| **insert** | commute (second no-ops) | **CONFLICT** (final presence is order-dependent) |
| **delete** | **CONFLICT** | commute |

**K — same `fkey(det)` under the same key statement:**

Any two writers of the same determinant **CONFLICT** — two inserts with
equal determinants and different dependents are each valid alone and
jointly violate the FD; insert-vs-delete of the determinant reorders
visibility. (Exception already covered by F: byte-identical rows are the
F-table's commute case; the K row fires only when `fid`s differ.) Distinct
determinants never interact — this is the workhorse: different bookings,
different invoices, different customers are *provably* concurrent.

**C — same `fkey(target det)` under the same containment:**

| | need | support+ | support− |
| --- | --- | --- | --- |
| **need** | commute | commute | **CONFLICT** (dangling-reference race) |
| **support+** | commute | commute | commute (the add only strengthens the remover's premise) |
| **support−** | **CONFLICT** | commute | **CONFLICT** (each remover counted the other's row as the survivor) |

Recorded refusal: Bailis proves `need`×`support−` *becomes*
coordination-free under cascade semantics (his Claim 8 — delete the
target and every referencing source, including ones the deleter never
saw). We refuse the cell's freedom because the price is merge-time op
synthesis: the cascade destroys a committed concurrent insert with no
verdict, and ops that materialize at merge cannot ride in commands —
replay would re-decide, which is the one thing replay never does. The
CONFLICT cell is the cost of serial verdicts, paid knowingly.

**W — same `fkey(parent det)` under the same capacity:**

Quantitative, not boolean — and interval-valued, because set semantics
can *evaporate* an op against the final base (a delete of a row another
commit already deleted, an insert of a row already present, each a
proven no-op). Evaporation is harmless to the existential classes — an
over-claimed K write or C need only makes intersection conservative —
but a **measured** class cannot treat the published Δ as an effect: it
is an op-derived bound. Each batch's effective delta at any reachable
base lies in the interval

```
[ Δ − Σ w(its F+ entries on weighted children),
  Δ + Σ w(its F− entries on weighted children) ]
```

(evaporated inserts pull the effect down; evaporated deletes pull it
up; both sums are recomputable by any intersector from the ops it
already holds — no wire change). Let `slack⁺ = ceiling − measure(base)`
and `slack⁻ = measure(base) − floor` (∞ where unbounded). Concurrent
batches **commute iff the worst-case endpoints respect both bounds**:
`Σ max-endpoints ≤ slack⁺` and `Σ min-endpoints ≥ −slack⁻`. The
endpoints encode exactly the verdict-flip boundary — the test passes
precisely when every republish-without-re-judgment is safe, and goes
CONFLICT precisely when an interleaving could flip a verdict. It is
deliberately conservative in one place: two batches whose intervals
widen from the *same* fid (e.g. two deletes of one reservation row — at
most one can materialize) are tested uncorrelated, so near a bound they
re-judge when they might have commuted; re-judgment is cheap and the
bound is where precision matters. `parent−` **CONFLICTS** with any
child interval ≠ [0,0] and with `need`-style existence of children;
`parent+` commutes with child adds (a parent must exist for children to
be admitted against it — a child add whose parent arrives concurrently
was individually *rejected*, so the pair never reaches the matrix).
On a W conflict, the arithmetic shortcut — recompute the intervals
against the winner-updated measure — is allowed immediately and settles
most losses without the full re-judgment; when it cannot certify, the
loser algebra's re-judgment (below) is the answer, as everywhere else.

**Fresh ids:** not a class. Writers lease disjoint ranges
(`ids/{relation}/{field}` CAS counter, 10); commands carry concrete ids;
cross-writer collision is structurally impossible, and an in-range replayed
collision is an ordinary K conflict caught above.

**The statement-class boundary (why these four tables are total).** Every
bumbledb statement is *star-guarded*: each obligation instance's rows all
share one full projection (the determinant / target key / parent key)
that each row computes from its own raw values — which is exactly why a
state-independent 32-byte key can name the obligation and why L7 is
provable. The boundary, recorded so nobody crosses it casually:
cross-relation *guarded* EGDs (`R(k,x) ∧ S(k,y) → x = y`) are the one
known statement class the engine could someday admit without damage —
both atoms carry the guard, fkey(k) stays exact. Multi-atom bodies
*without* a shared guard (transitivity is the canonical full-TGD example)
degrade footprints to value-level keys and then, past pairwise
intersection, make per-row emission unsound entirely — fullness is not
the boundary, guardedness is. And "at most k" uniqueness has a native
dependency-theoretic encoding (a disjunctive EGD) that is refused by
name: the disjunctive chase *branches* — a tree of possible instances —
which is poison for deterministic replay and serial verdicts; k-bounded
counting lives in the W family's arithmetic instead, where it grades.

## The loser algebra (what a CAS loser does, exactly)

Loser L (published footprint F_L) lost braid slot **g** to winner W;
both batches were built on base g−1 (the `prev` chain proves it, 20).
Raw-value keys are state-independent, so the comparison is sound
regardless of what else moved; L's verdict survives all cross-braid
churn automatically (L9 + L7) and same-braid winners exactly when the
matrices say disjoint.

**F_W is recomputed, never trusted.** The loser has already fetched W's
batch (it must apply it); `footprint(descriptor, W.ops)` is one pure local
call. Intersecting against the *published* section instead would let one
buggy or hostile writer understate its footprint, steer every loser onto
the republish path, and brick the braid with `ReplayDiverged` on all
replicas — the carried-and-checked law (batch section 3) applies to the
loser exactly as it applies to replay.

1. **Subsumed** (every F-entry of L appears in F_W with the same mode):
   W already performed L's net effect; replay of L's batch anywhere would
   net-dispose to the empty delta — a no-op commit slot, forbidden by the
   publish law (60: publish only what advanced the generation). L never
   republishes; it applies W and reports `Accepted` at W's generation —
   the F-matrix's "second no-ops" cell, lifted to the protocol. Whether
   L's store survives in place is decided by the engine at that apply:
   no-op (W's effects were exactly L's) → survive; state-changing (W
   strictly contains L, so one slot now covers two local commits) → the
   store forked and the disposable law runs (60 carries the mechanics
   and why the wholeness identity is the deciding instrument).
2. **Disjoint** — and disjoint means **no shared key of any class,
   commute cells included**, plus a passing W interval test on any
   shared parent. This is stricter than "no CONFLICT cell", and the
   strictness is load-bearing: a shared same-mode F key means one
   batch's op can *evaporate* against the other's effect, so the
   republished batch's own publish-law standing at the new base — and
   the W entries' additivity — would rest on facts the fast path never
   re-checks. (The adversarial trace that forced this: a batch sharing
   one insert with the winner, whose other ops were base-redundant,
   republishes into a slot that net-noops everywhere — a fleet-wide
   `generation ≠ Σ vector` with no bug in either writer.) Under full
   key disjointness, L's verdict is **still valid** by footprint
   stability (L7), the winner's apply on L's store is provably
   state-changing-accepted, and the identity holds with two applies and
   two slots. Apply W's batch locally, re-address the header (slot g+1,
   `prev` = W's hash, timestamp re-clamped), republish — ops, footprint,
   and verdict untouched. Cost: one intersection + one PUT.
3. **Conflict** (anything else — a CONFLICT cell, a failed W interval
   test, or any shared key outside the subsumed case): rebuild the
   pristine base (50's discard-and-reopen — the local store contains
   L's commit, which cannot be unwound under the winner), then
   **re-judge L's recorded ops** — one `db.write` of the same ops
   against the winner-current state. Accepted and state-changing →
   republish (the footprint is unchanged: it is a pure function of the
   ops). Accepted as a net no-op (the moved base already contains L's
   effects) → publish nothing, report `Accepted` at the current
   generation — the publish law, holding at the *new* base because the
   re-judgment just evaluated it there. Rejected → return the
   violations to the host: exactly the verdict serial execution would
   have produced for *the transaction the host submitted*. The driver
   never re-invokes the host closure — the engine's own law ("the
   engine ships the outcome, never a loop — retry is host policy")
   holds one layer up: deciding to write something different against
   the moved world is the host's move, not ours. The per-obligation
   partial revalidation is a recorded v2 optimization, with the W-class
   arithmetic shortcut allowed immediately.
4. Repeat on further losses at the live tip, bounded (default 16, then
   `Err::Contention` — 60). Losses to *history* never count: a writer
   whose slot attempt lands far behind the tip (a recovered pending, a
   long-deposed resident) first catches up through ordinary replay,
   running these same pairwise tests against each intermediate winner —
   L7 composes across a prefix exactly as it composes across one loss —
   and then attempts once at tip+1. The bound counts consecutive
   live races only. The guarantee, stated precisely: slot CAS makes the
   protocol **lock-free, not wait-free** — every contended slot is won
   by someone, so every loss is somebody's commit and the system always
   advances; an individual writer can starve, and the bound converts
   starvation into a typed operational signal rather than a silent loop.
   Braid locality (10), reservation relations on hot capacities (below),
   and resident mode are the pressure valves.

## Capacity reservations (the algebra absorbs escrow)

The grant-object design (CAS-claimed `escrow/{W}/{fkey}` bodies with
wall-clock TTLs and a check-skipping fast path) is deleted as unsound —
the engine cannot see grants, so a grant-ignorant winner can consume
promised slack and the holder's unchecked republish poisons the log
(60 records the full failure chain). The sound v2 is a **schema idiom**:
declare a reservation relation as one more weighted child of the hot
capacity statement. Mint = an ordinary judged insert (pays the slot race
once, priced against real slack); spend = delete-reservation + insert
children in one commit, net Δ = 0 with evaporation interval [−w, +w],
which **commutes by the W interval test** whenever the group holds
headroom beyond the spend's own units — the fast path is the matrix's
own arithmetic; at the bound, and in reclaim-vs-spend races (two deletes
of one reservation row — correlated intervals the test reads
uncorrelated), the same test goes CONFLICT and forces the honest
re-judgment that prices the children against the real slack. O'Neil's escrow rights, Indigo's reservations, and
Homeostasis's treaties all became side-cars with their own conservation,
revocation, and fencing machinery; here the rights are rows, conservation
is the capacity judgment, and expiry is an event in the arbitration
domain. No new objects, verbs, or obligations exist.

One degeneracy is reified as data rather than warned about: a capacity
or key statement whose determinant projection is *empty* names a single
global group — every commit under it shares one fkey, and the braid
degenerates to a serial log at that statement (Homeostasis's Delivery
transaction, a treaty forced to renegotiate on every run, is the
canonical prior misery). The braid derivation returns these
serial-at-statements as a typed field beside the braid map; the schema
author reads data, not a log line.

## Lean obligations (the soundness spine; the optimism path ships behind them)

- **L6 — Footprint soundness.** The driver's raw-value footprint
  *over-approximates* the judgment's dependency set: formally, under
  **full key disjointness** (σ and δ share no footprint key of any
  class; shared W parents additionally pass the interval test) σ's
  application changes no obligation instance that δ's judgment reads,
  writes no fact δ writes, and evaporates none of δ's ops. The
  hypothesis is deliberately stronger than "no CONFLICT cell": shared
  commute-cell F keys break op-effect independence, and the theorem is
  false without excluding them.
- **L7 — Footprint stability.** `judge(base ⊕ σ, δ) = judge(base, δ)`
  and δ's net effect at `base ⊕ σ` equals its net effect at `base`,
  whenever L6's hypothesis holds — the strengthening of the
  delta-restriction theorem this design rests on, and the theorem that
  licenses republish-without-re-judgment *and* keeps the publish law
  true at the moved base.
- **L8 — Commutativity.** Under the same hypothesis, `apply(apply(base,
  σ), δ) = apply(apply(base, δ), σ)` (set-level state equality; the
  engine's canonical order makes the representations equal too — pinned
  by `catalog_digest` in 80).
- **L9 — Component independence** (trivial corollary): statements never
  span braid components (10), so cross-component footprints are disjoint
  by construction.
- **L10 — Replay idempotence.** Re-applying a batch whose effects the
  state already contains yields the identical state, an accepted verdict,
  and no generation advance: every op net-disposes, the delta is empty,
  and the engine's no-op arm never reaches judgment. This is the theorem
  the whole recovery story stands on (20, 50) — crash windows heal by
  replaying forward, because replaying backward-overlap is proven
  harmless.

L7 deserves one framing sentence against the strongest prior art:
RedBlue's invariant safety must hold for **all** valid states (their
Def. 7 quantifies over every S′, because shadow ops are never re-judged
at apply), which is why almost nothing qualifies as blue in practice. L7
is the *conditional* form — stability only under proven disjointness —
and the protocol re-judges everywhere else. Weaker to prove, checked at
every replica, and the reason our "blue" fraction is a per-pair fact
instead of a per-op hope.

## Worked example (the booking race)

Two Vercel instances, base g=41. A books slot S for account 7; B books
slot S for account 9. Both inserts carry the key statement
`key(Booking, ["slot"])`. Both footprints contain `(K, fkey(slot=S))` —
CONFLICT cell (K, two writers). A wins log 42. B intersects, hits the K
row, discards its forked store, re-opens through A's 42, and re-judges
its recorded ops: the insert now violates the slot key →
`rejected(FunctionalityViolation)` to the host — the double-booking
refused with a *proof*, no lock ever taken, at the price of one round
trip and one (cache-warm) re-open. Had B booked slot T instead: disjoint
footprints, B applies A's batch in place and republishes at 43 without
re-judging — two commits, one race, zero serialization beyond the slot
claim itself.
