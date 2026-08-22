# The thesis: a closed dependency theory makes coordination computable

The claim underneath `proposals/15-conflict-algebra.md`, stated against
the literature after a full read of the corpus — fourteen papers on
disk plus the Turso post; O'Neil's escrow paper stays paywalled and is
cited through its descendants (every quote below is from the PDFs in
this directory, page-cited). Nothing here is
mystical: four known results — CALM, invariant confluence, escrow,
deterministic replay — become *mechanically applicable* the moment the
constraint language is closed, compiled, and set-semantic, which is the
one design decision general databases can never retrofit.

## 1. CALM, applied pointwise — and stated honestly

CALM (Hellerstein–Alvaro, Thm 1, p. 3): *"A program has a consistent,
coordination-free distributed implementation if and only if it is
monotonic."* Their monotone fragment admits insertions and rejects
deletion outright (p. 4: "insertions are allowable, but in general
updates and deletions are not"); their canonical non-monotone act is
declaring a negative — garbage collection's "it must ensure that it has
heard everything there is to hear before it declares an object
unreachable" (p. 2).

Cross bumbledb's two write polarities with its three constraint families
and every CONFLICT cell in 15's matrices lands on a non-monotone edge,
*in their formal sense*: admitting `support−` asserts ¬∃ over exactly
the facts a concurrent `need` writes (the GC example, byte for byte);
uniqueness is falsifiable by growth in both directions; a ceiling is a
negation over a growing measure. Two precision points, stated
exactly:

- The W family's commute-within-slack is **not** CALM-monotone — no
  graded notion exists in their Definition 1. CALM contributes only W's
  CONFLICT cells; the graded commute region is escrow arithmetic (§3),
  a different theorem. The correct sentence: every commute cell is
  monotone, complement-monotone (two-growing-sets, their own tombstone
  pattern), or escrow-graded.
- bumbledb commits are **not** coordination-free in their Definition-5
  sense and are not claimed to be: a rejecting judgment is non-monotone
  (verdicts flip in both directions as the base grows), so by their own
  theorem a judging store cannot be coordination-free — which is *why*
  the CAS slot exists. The design's actual position is the regime their
  paper cites as "Win-move is coordination-free **(sometimes)**" (Zinn
  et al., their [48]): instance-dependent monotonicity certificates,
  decided per pair of commits, with the residual coordination scoped to
  one conditional PUT per braid slot and moved off the hot path exactly
  as their §3.4 ("Coordination In Its Place") prescribes. Reads and
  rejections touch nothing; commits pay one PUT.

**Reads got their own law because of this school.** Laddad et al.:
CRDT queries "are unconstrained and may perform arbitrary computations
on the underlying state" (p. 2) — the unguarded door their whole paper
retrofits, since CRDT replicas are under-approximations and non-monotone
queries over them "appear to go backwards over time" (p. 4). A bumbledb
replica is never an under-approximation of anything: every vector state
is a *real serial prefix* that passed the full judgment (00 law 9 —
"Schrödinger consistency" is structurally impossible; there is no merge
to be unobserved). What their framework still correctly denies us:
cross-instance sequential consistency of non-monotone reads — hence
session vectors and `wait_for` (50), with watermark facts the only
observables stable without them. Their taxonomy, our vocabulary:
default reads are the Zanzibar stale-fast lane; `wait_for` is the safe
lane; the choice is the host's, in the type.

## 2. Invariant confluence, upgraded from types to pairs

Bailis's Theorem 1 (p. 5) is necessary *and* sufficient: a set of
transactions executes coordination-free iff merging any two I-valid
reachable states is I-valid. Read both directions:

- **Sufficiency** is what L6–L8 mechanize per pair. His Table 2 agrees
  with our matrices row for row wherever semantics are held equal:
  uniqueness-choose-value No (our K conflict), FK-insert Yes (our C
  commutes), FK-delete No (our `need`×`support−`), ceiling+increment No
  / floor+increment Yes (our W, as the slack = ∞ degenerate rows),
  SIZE= No (our slack-0). The two departures are both deliberate: his
  CONTAINS rows are Yes only under CRDT-style merge that picks a winner
  (the weakening we refuse), and his Claim 8 (FK cascading delete:
  Yes) buys freedom with merge-time op synthesis — the cascade deletes
  a committed concurrent insert with no verdict — which is incompatible
  with commands-ride-in-batches determinism; 15 records the refusal.
- **Necessity** is the theorem that our CONFLICT cells are *minimal*:
  uniqueness (Claim 3), FK-under-deletion (Claim 7), and over-slack
  bounds (Claims 12/13) provably cannot be free without weakening.
  The matrices are not cautious; they are the frontier.

The per-pair upgrade is real and his model cannot express it: his
classification coordinates *all* writes touching a non-I-confluent
invariant type; the footprint proves two bookings under the same
uniqueness statement but different determinants independent. His own
escrow paragraph (§8, p. 12: "Adapting Escrow… is a promising area for
future work") is the W matrix, done, with declared weights instead of
guessed shares. His TPC-C result — 10 of 12 invariants I-confluent,
12.7M New-Order/s on 200 servers, coordination confined to sequential
ID assignment — is the shape of every real schema; the two holdouts
are *sequentiality*, which we refuse at the design level (10's id
leases are unique-never-dense, and stay that way).

**Whittaker's Theorem 2 is the deepest single connection in the
corpus** (p. 17): invariant *closure* (I closed under merge) equals
invariant *confluence* exactly when every invariant-satisfying state is
reachable. His entire interactive apparatus — Z3, human-labeled
unreachable states, coreachability — exists to bridge the gap between
satisfying and reachable. **Admission closes that gap by construction**:
in bumbledb every representable state is an admitted state, I =
reachable(S), and the closure/confluence distinction collapses — the
representation-over-control-flow law, stated in his vocabulary. His
segments are our braids with every cost removed: user-supplied ("it is
the responsibility of the programmer… This can be an onerous process,"
p. 21) becomes read-off-the-schema; his stop-the-world segment
transitions (all servers join, speculate, vote) become nothing, because
schema-derived segments never transition. And his measured collapse —
factor 2 throughput at 1 % coordinating transactions, 10 % of eventual
at 10 %, scale-out ceilings of 24/12/4/1 servers at 1/5/20/50 % — is
the quantitative argument for our degradation shape: one loser
re-judging locally, never a barrier.

Merge deserves its own sentence: Bailis's ⊔ must *join states* validly
with no arbitration, his footnote 4 has to outlaw merges of
never-executed combinations by fiat, and I-confluence only promises the
merged state is legal, never which one. Our merge is CAS-ordered
deterministic replay: the outcome is byte-identical to **a** serial
execution — the one the log realized — and losers receive that serial
history's verdict with violations as data. Convergence with proofs, not
convergence instead of them.

## 3. Escrow, absorbed rather than revived

O'Neil's escrow method (1986) never went mainstream because general
schemas don't declare numeric constraints — nothing to escrow against.
The lineage runs Demarcation → escrow → Bounded Counter → Indigo
reservations → Homeostasis treaties, and the modern survey draws the
boundary plainly (Preguiça §2.3.2, pp. 16–17: CRDTs "are unable to
enforce such global invariants"; the only conflict-free escape is
escrow's pre-split rights, where exhausted rights "fail or require
synchronizing"). Homeostasis states the soundness condition every such
scheme owes: local treaties whose *conjunction* implies the global
invariant (their H1, p. 6).

The deep reads caught our own v2 escrow violating exactly that: grant
objects the judgment cannot see have no H1, and the "skip the W check
while holding a grant" fast path could publish a ceiling violation into
the immutable log (60 records the failure chain and the deletion). The
sound form dissolves the subsystem entirely: **a reservation is a row**
in a declared weighted child relation of the capacity statement. Mint
is judged against real slack (over-granting is rejected at mint, which
is H1 discharged by the ordinary judgment); spend commutes by the W
matrix's own interval arithmetic while headroom exceeds its units — the
fast path is a theorem; reclaim races spend into a CONFLICT cell near
the bound, degrading an expired grant to an honest rejection. (The
interval form of the W test — deltas as op-derived bounds widened by
possible evaporation, never effect claims — was forced by an
adversarial trace during this design's final review: the point-Δ test
every escrow paper writes is unsound under set semantics, because a
delete that evaporates against the moved base raises the effective
delta above the published one. No prior escrow system had idempotent
set-semantic replay to collide with, so nobody had to notice.) Indigo's future-work list —
leaked rights, revocation fan-out, fenced recovery of dead holders'
rights — is machinery for states that are now unrepresentable, because
the rights live in the arbitration domain (the log) instead of beside
it. This is the first setting where the treaty is *derived from a
proved statement* and enforced by the same judgment it treats.

## 4. RedBlue, with the colors computed — and the labels checked

RedBlue asks programmers to color shadow operations: blue iff globally
commutative *and* invariant-safe, where invariant safety (Def. 7,
p. 270) must hold "for all valid states S′" — because shadow ops are
never re-judged at apply time. That universal quantifier is why almost
nothing is blue in practice: their own TPC-W port makes `doBuyConfirm`
"produce red shadow operations 98 % … of the time" (p. 276), every red
waits on a single global token rotating at "up to 1 second" per site,
and "every fifth request must wait k − 1 seconds" (p. 274) — although
two purchases of different items never interact. Nothing checks a label
at runtime; a mislabeled blue diverges silently.

Ours is the same generator/shadow split — the commit body decides
locally, the logged batch of concrete ops is the effect, replay never
re-decides — with three upgrades, each a representation: colors are
computed per *pair at a key* (the refinement ladder: op-type → their
state-specific op-instance → commit-pair-at-key-with-slack; nobody else
reaches the third rung); L7 is the *conditional* form of their Def. 7
(stability only under proven disjointness, re-judgment everywhere
else — weaker to prove, and checked at every replica by footprint
recomputation, so a wrong claim is a typed corruption error instead of
silent divergence); and the arbitration unit is the braid slot, not the
world, so there is no global token to queue on. Their LWW bolt-on
("to make operations that overwrite part of the state commute,"
p. 274) is lost updates rebranded as commutativity — the exact move
this design exists to refuse.

## 5. The deterministic school: we log after-effects, and hold nothing

Calvin's own related-work section draws the axis (p. 11): Hyder's log
is "the after-effects of transactions," Calvin's "contains unexecuted
transaction requests." bumbledb-log is on the Hyder side with a theory
attached: the log carries judged effects, and footprint intersection is
the meld. Logging requests costs Calvin advance knowledge — "All
transactions are therefore required to declare their full read/write
sets in advance" (p. 5) — and dependent transactions get the OLLP
bolt-on: a reconnaissance query, a recheck, and a deterministic restart
loop whose termination is statistical ("One therefore expects the OLLP
scheme seldom to result in repeated transaction restarts under most
common real-world workloads," p. 6). Our footprints are computed *after*
the closure runs, over recorded ops: there is nothing to predict, no
reconnaissance state to go stale, no restart loop to hope about.

Calvin also names the quantity we drive to zero (p. 2): the "contention
footprint" — the duration locks are held. Every Calvin section on disk
prefetch and every Aria barrier is management of that duration. Ours is
zero by construction: writers couple only at an immutable CAS slot;
conflicts are computed over published bytes; nothing is ever held.
Aria's numbers make the case for per-commit over per-epoch certificates:
one 20 ms straggler in a 1 000-transaction batch costs 81 % throughput
(Fig. 5), batch sizes are hand-tuned per benchmark, and their fallback
engages on a moving-average abort threshold — a control loop where we
have a matrix lookup. What we take from them rather than beat: the
progress guarantee stated precisely (slot CAS is lock-free, not
wait-free; every loss is somebody's commit; the 16-loss bound converts
starvation into a typed signal — 15/60), their no-short-circuit
determinism discipline for the v2 drain partition, and their measured
basis for when fallback machinery pays (60's group-commit section).

## 6. The log-on-storage school: their repair fleets, our headers

Aurora's "the log is the database" (p. 1044) is the kinship claim — and
the divergence is what each design lets exist. Aurora's 4/6 quorum
makes *holes normal state*: segments gossip to fill gaps, an entire
LSN taxonomy (VCL/CPL/VDL/SCL) exists to say which suffix of the
durable log is *not real*, and recovery actively annuls it with
epoch-versioned truncation ranges "so that there is no confusion over
the durability of truncations in case recovery is interrupted and
restarted" (p. 1047). Every epoch is a confession that a representable
ambiguity shipped. Our chain has no holes to fill (slot g+1 is created
only by a writer that applied g, and the `prev` hash proves it per
object), no complete-but-not-durable band (ack *is* the created
object), and no truncation (nothing above the tip exists). We kept one
Aurora idea as a 32-byte field — the backlink — and one as a knob (the
LAL backpressure leash, reborn as `max_pending` on local acks).

Delta Lake is the strongest industrial validation because they publish
their pain: "Amazon S3 does not have atomic 'put if absent'… In
Databricks service deployments, we use a separate lightweight
coordination service" (p. 3416) — a coordination *service*, grown into
a per-vendor LogStore plugin API, for the operation that is now one
header (`If-None-Match: *`, S3-native since 2024). Their admitted
ceiling — object-store latency "limiting the write transaction rate to
several transactions per second" per table (p. 3417) — is the baseline
number 80's pins quote and braids × group commit × Express exist to
embarrass. Their `_last_checkpoint` + LIST-forward discovery became our
manifest + checkpoint backlink chain (no LIST anywhere); their
tombstone-retention treaty with stale readers became our gc window with
the tip-vs-hole rule decided from the manifest checkpoint vector.

And the recovery contrast that owns the section: SQLite-lineage WALs
(Turso's diskless included) replay page images, which are order-fragile
and non-idempotent — hence frame salts, checksums, and applied-
watermarks. Set semantics makes our replay *idempotent by theorem*
(L10): re-application net-disposes to the engine's no-op arm, so the
intent fields, forced-case recovery tables, and applied-watermark
machinery all stopped existing (50). Recovery is replay. That is the
"extremely aggressive and elegant simplification" set semantics was
suspected of hiding, found, and it is unavailable to every page-image
system by representation, not by effort.

## 7. The chase connection, audited

In data-exchange vocabulary (Fagin–Kolaitis–Miller–Popa, TCS version):
bumbledb keys are EGDs under the standard encoding — exactly. The
containments are **embedded** TGDs by syntax (single-atom INDs/CINDs
whose head existentials range over unprojected target columns — FKMP's
own canonical *non-full* examples, p. 17) with the **full-TGD
operational profile**, because the engine never fires a chase step:
admission checks that no step is applicable to the final state, so the
existential is only ever eliminated by witness lookup, never introduced.
Say it that way and never "full TGDs"; the fullness properties we enjoy
(termination trivially, confluence trivially, no labeled nulls) come
from *verification-only semantics*, not from the syntax class.

Three results become exact rather than evocative:

- **Refusal is the ground chase.** On instances with no nulls, FKMP's
  EGD chase step has only its failure case reachable (Def 3.1: both
  sides constants ⇒ ⊥), and TGD satisfaction is trigger absence. So
  admission is not "like" the chase — it *is* the chase restricted to
  ground instances, where success means zero steps. `Violations` is ⊥
  delivered at the boundary, typed, with citations, pre-commit.
- **Writer minting is skolemization of the write, not the constraint.**
  The constraint keeps its ∃-reading; what moves to the boundary is the
  existential *introduction* — writers extend Const under an exclusive
  lease, and the codec makes the undischarged form unrepresentable (no
  null encoding exists on the wire — parse-don't-validate applied to
  existentials). In FKMP's semantics our states are solutions but
  deliberately not universal: a system of record is *entitled* to the
  extra information; there is no downstream solution space to keep
  generic. The honest cost, recorded: "a manager exists but is unknown"
  is refused, not deferred, and identity merges are explicit data
  operations, never retroactive null unification.
- **Var(store) = ∅ deletes their §4–5 wholesale.** Certain answers,
  the ↓ operator, hom-equivalence-only uniqueness, cores, the
  coNP-completeness walls (two inequalities, Thm 5.11), and the
  FO-inexpressibility result (Thm 5.14) are all theorems *about labeled
  nulls*. Every bumbledb state is the unique model of its own history;
  Sol is a singleton; naive evaluation equals certain answers for
  **all** queries — Prop 4.2's universal-solution characterization
  (p. 21), satisfied trivially, which is the strongest possible form of
  "replica reads need nothing." A replica at a stale vector serves an
  *older complete model*: staleness across states, never incompleteness
  within one.

L7's chase-theoretic content is the locality of the **satisfaction
check** (one round of the immediate-consequence operator, no fixpoint),
not of chase steps — nothing steps. It is provable because every
statement is *star-guarded*: all rows of one obligation instance share
one full projection each row computes from its own raw values. The
boundary ladder, so the enabling condition stays cited when someone
proposes enriching the language: guarded cross-relation EGDs would
still be safe (the one known-safe extension); full-but-unguarded TGDs
(transitivity) degrade footprints to value-level keys and then kill
per-row emission entirely — fullness is not the boundary, guardedness
is; generative semantics (chase actually firing) makes footprint values
instance-relative and, with cycles, non-terminating (FKMP Ex. 3.6);
and "at most k" has a native disjunctive-EGD encoding whose chase
*branches into a tree* — refused by name, because a branching chase is
poison for deterministic replay; k-bounded counting lives in W, where
it grades instead of branching.

## 8. What is genuinely new (the honest novelty claim)

Not the ingredients — CALM, I-confluence, escrow, RedBlue's split,
deterministic replay, and log-on-object-storage are all known, and the
Feral study measured the demand (declared invariants outnumber
transactions 37:1 in production code, enforced ferally with 70–6,300
leaked duplicates — the constraint families people actually write are
exactly these three). The composition is new in four places: (a) **the
conflict relation as a per-commit compiled artifact** — byte-keyed
footprints from raw values, the third rung of a refinement ladder
(op-type, op-instance, pair-at-key-with-slack) the literature climbed
one rung at a time; (b) **serial-verdict-preserving merges** — losers
get the rejection a serial history would have produced, with violations
as data, because merge is deterministic replay rather than state join;
(c) **the closure/confluence gap closed by admission** — every
representable state is reachable-and-legal, collapsing the distinction
Whittaker built an interactive prover to bridge, with the whole chain
machine-checked (L6–L10) against the same Lean corpus that defines the
judgment; (d) **set-semantic idempotent replay as the recovery story**
— re-application is a proven no-op, so the recovery state machines
every page-image log carries (salts, watermarks, intent fields,
truncation epochs) are not simplified but *deleted*. If a paper comes
out of this, it is those four sentences.
