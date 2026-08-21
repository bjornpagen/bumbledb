# 15 — The conflict algebra

The centerpiece. Bumbledb's constraint set is closed and compiled — three
families (functionality, containment, capacity) plus fact identity — so the
question "do two concurrent commits interfere?" is *computable per commit*,
as data. General databases cannot have this document; their constraint set
is open. This one is four tables and a theorem.

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
CONFLICT for some shared key. "Commute" is proven (Lean L7): either apply
order yields the identical final state and identical verdicts.

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

**W — same `fkey(parent det)` under the same capacity:**

Quantitative, not boolean. Let `slack⁺ = ceiling − measure(base)` and
`slack⁻ = measure(base) − floor` (∞ where unbounded). Concurrent deltas
`Δ₁, Δ₂, …` **commute iff every prefix sum stays within
[−slack⁻, +slack⁺]** — and since each Δᵢ was individually admitted, it
suffices that `ΣΔᵢ` respects both bounds when all same-signed
(mixed signs: check both extremes `Σmax(Δᵢ,0) ≤ slack⁺` and
`Σmin(Δᵢ,0) ≥ −slack⁻` — the conservative test; order-free by
construction). `parent−` **CONFLICTS** with any child Δ ≠ 0 and with
`need`-style existence of children; `parent+` commutes with child adds
(a parent must exist for children to be admitted against it — a child add
whose parent arrives concurrently was individually *rejected*, so the pair
never reaches the matrix). Revalidation on conflict is arithmetic, never
re-judgment: recompute the sum against the winner-updated measure.

**Fresh ids:** not a class. Writers lease disjoint ranges
(`ids/{relation}/{field}` CAS counter, 60); commands carry concrete ids;
cross-writer collision is structurally impossible, and an in-range replayed
collision is an ordinary K conflict caught above.

## The loser algebra (what a CAS loser does, exactly)

Loser L (published footprint F_L) lost the braid slot to winner W
(footprint F_W). Raw-value keys are state-independent, so the comparison
is sound regardless of what else moved; L's verdict survives all
cross-braid churn automatically (L9 + L7) and same-braid winners exactly
when the matrices say disjoint:

1. `F_L ∩ F_W = ∅` (per the matrices — intersection means *a CONFLICT
   cell*, not mere key sharing): L's verdict is **still valid** by
   footprint stability. Apply W's batch locally, set `base = g+1`,
   recompute nothing, republish at g+2. Cost: one intersection + one PUT.
2. Intersection non-empty: apply W's batch, then **re-run L's write
   locally** (full re-judgment of L's ops against g+1 — v1 ruling; the
   per-obligation partial revalidation is a recorded v2 optimization with
   the W-class arithmetic shortcut allowed immediately). Accepted →
   recompute footprint (base changed ⇒ entries may change) → republish.
   Rejected → return `rejected` to the host: exactly the verdict serial
   execution would have produced.
3. Repeat on further losses. Livelock is bounded in practice by braid
   locality (10) and, if ever measured, by the v2 escrow/lease layer.

## Escrow (v2, spec'd; correctness never depends on it)

For a measured-hot capacity parent `p`: grant objects
`escrow/{W}/{fkey(p)}` claimed by CAS, each granting `w` units of slack
for a wall-clock TTL. A writer holding a grant treats its Δ ≤ w as
conflict-free at that key without re-checking. Stale or violated grants
cost only spurious conflicts (the loser algebra still guards); wall-clock
TTLs are therefore acceptable — escrow is avoidance, never truth.

## Lean obligations (the soundness spine; block Layer-2 shipping)

- **L6 — Footprint soundness.** The driver's raw-value footprint
  *over-approximates* the judgment's dependency set: formally, if
  `F(σ) ∩ F(δ) = ∅` (no CONFLICT cell shared) then σ's application
  changes no obligation instance that δ's judgment reads and writes no
  fact δ writes.
- **L7 — Footprint stability.** `judge(base ⊕ σ, δ) = judge(base, δ)`
  whenever L6's disjointness holds — the strengthening of the
  delta-restriction theorem this design rests on.
- **L8 — Commutativity.** Under the same hypothesis, `apply(apply(base,
  σ), δ) = apply(apply(base, δ), σ)` (set-level state equality; the
  engine's canonical order makes the representations equal too — pinned
  by `catalog_digest` in 80).
- **L9 — Component independence** (trivial corollary): statements never
  span braid components (10), so cross-component footprints are disjoint
  by construction.

## Worked example (the booking race)

Two Vercel instances, base g=41. A books slot S for account 7; B books
slot S for account 9. Both inserts carry the key statement
`key(Booking, ["slot"])`. Both footprints contain `(K, fkey(slot=S))` —
CONFLICT cell (K, two writers). A wins log 42. B intersects, hits the K
row, applies A's batch, re-judges: its insert now violates the slot key →
`rejected(FunctionalityViolation)` to the host — the double-booking
refused with a *proof*, no lock ever taken, one round trip lost. Had B
booked slot T instead: disjoint footprints, B republishes at 43 without
re-judging — two commits, one race, zero serialization beyond the slot
claim itself.
