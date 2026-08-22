# Replication prior art — the reading corpus for bumbledb-log

The literature behind `proposals/15-conflict-algebra.md`, curated with one
question per paper: *what do they know that our design should absorb, and
what do we have that they could not?* **Status: all fourteen PDFs are in
the subdirectories and all have been read cover to cover** (six
adversarial deep-reads against the PRD set, 2026-08-21); the findings are
folded into the proposals, [THESIS.md](THESIS.md) (the synthesis, fully
page-cited), and [IMPOSSIBLE.md](IMPOSSIBLE.md) (the ledger: every bad
state the prior art must represent that this design cannot — and the
states our own drafts represented until the deep read deleted them).
[fetch.sh](fetch.sh) remains for re-fetching (fault-tolerant, mirrors
included). [turso-notes.md](turso-notes.md) holds the industrial findings
and the applied steal list.

## A — Invariant-driven coordination avoidance (the algebra's direct ancestors)

1. **Bailis, Fekete, Franklin, Ghodsi, Hellerstein, Stoica — "Coordination
   Avoidance in Database Systems" (VLDB'15; extended
   [arXiv:1402.2237](https://arxiv.org/abs/1402.2237)).**
   Defines **invariant confluence (I-confluence)**: an invariant is
   coordination-free iff merging any two invariant-satisfying divergent
   states preserves it. Their Table 3 classifies SQL-ish invariants:
   uniqueness is NOT I-confluent (⇒ coordination); foreign-key *insertions*
   are; FK insert-vs-delete is not; per-row bounds are with escrow. **That
   table is our four matrices, derived twenty rows at a time instead of
   per-cell** — read it side by side with 15 and note where their
   classification is per-invariant-*type* while ours is per-commit-*pair*
   (the footprint makes the decision procedure dynamic and exact).
   Steal: their merge framing sharpens L8's statement; their proof obligations
   preview L6's shape.
2. **Whittaker & Hellerstein — "Interactive Checks for Coordination
   Avoidance" (VLDB'19; no arXiv —
   [PVLDB PDF](http://www.vldb.org/pvldb/vol12/p14-whittaker.pdf), extended
   [VLDBJ 2021](https://doi.org/10.1007/s00778-020-00628-3)).**
   Makes I-confluence *decidable in practice*: necessary-and-sufficient
   conditions, an interactive decision procedure (Lucy), and — the part
   that reads like our design doc written a decade early — **segmented
   invariant confluence**: partition state so invariants hold segment-wise
   and coordinate only across segments. **Braids are segmented
   I-confluence made static**: our segments are the statement graph's
   connected components, derived from the schema at validation, no
   interaction required. Steal: their criteria for when a segmentation is
   sound feed L9's proof; their counterexample-driven interaction is a
   template for the Lane-3 fixture generator.
3. **Hellerstein & Alvaro — "Keeping CALM: When Distributed Consistency Is
   Easy" (CACM 2020; [arXiv:1901.01930](https://arxiv.org/abs/1901.01930)).**
   The CALM theorem: a program has a coordination-free, consistent
   distributed implementation **iff it is monotone**. This is the
   set-semantics keystone: under set semantics, inserts are monotone;
   deletion and negation are the non-monotone fragments. Read THESIS.md
   §CALM for the cell-by-cell mapping — our CONFLICT cells are *exactly*
   the non-monotone cells of the closed constraint language, which is why
   the matrices are so small.

## B — Commutativity, escrow, reservations (the W-class lineage)

4. **O'Neil — "The Escrow Transactional Method" (TODS 1986; ACM paywall —
   library item, no free PDF; the mechanism is summarized in THESIS.md).**
   The original quantitative-slack idea: partition a numeric constraint's
   headroom among concurrent transactions. Our W-matrix *is* escrow with
   the slack arithmetic as the revalidation rule; the v2 grant objects are
   O'Neil's escrows serialized into CAS objects.
5. **Balegas et al. — "Putting Consistency Back into Eventual Consistency"
   (Indigo, EuroSys'15;
   [author PDF](https://asc.di.fct.unl.pt/~nmp/pubs/eurosys-2015.pdf)).**
   Reservations (escrow, multi-level locks) bolted onto causal stores to
   preserve *declared* application invariants. Closest system-shaped
   cousin; their reservation taxonomy stress-tests our claim that
   K/C/W + leases cover everything a closed theory can express.
6. **Li et al. — "Making Geo-Replicated Systems Fast as Possible,
   Consistent when Necessary" (RedBlue, OSDI'12;
   [MPI-SWS PDF](https://www.cs.cmu.edu/~pavlo/courses/fall2013/static/papers/osdi2012-final-162.pdf)).**
   Blue (commutative, coordination-free) vs red (serialized) operations —
   colored *by the programmer*. Our advance is precisely that the color is
   **computed per commit from the theory**: footprint-disjoint = blue,
   conflict-cell = red, nobody annotates anything. Their shadow-operation
   technique (split an op into a decision part and a commutative effect
   part) is worth stealing conceptually: our writer's local-judge-then-
   publish *is* a shadow operation, and saying so connects the designs.
7. **Roy et al. — "The Homeostasis Protocol" (SIGMOD'15;
   [Cornell PDF](https://www.cs.cornell.edu/~jnfoster/papers/homeostasis.pdf)).**
   Automatically derives *local treaties* (per-node arithmetic predicates)
   from program analysis so nodes proceed without coordination while
   treaties hold. Our slack arithmetic is a treaty; their treaty-repair
   protocol is prior art for escrow-grant renegotiation (v2).

## C — Deterministic replay (the replica's lineage)

8. **Thomson, Diamond, Weng, Ren, Shao, Abadi — "Calvin: Fast Distributed
   Transactions for Partitioned Database Systems" (SIGMOD'12;
   [Yale PDF](http://cs.yale.edu/homes/thomson/publications/calvin-sigmod12.pdf)).**
   Agree on the log, then execute deterministically — replicas need no
   output coordination. Our replica story is Calvin with the sequencer
   replaced by object-store CAS and the deterministic executor replaced by
   the engine's canonical apply (which Lean pins harder than Calvin ever
   could).
9. **Lu, Yu, Cao, Madden — "Aria: A Fast and Practical Deterministic OLTP
   Database" (VLDB'20; [PVLDB PDF](http://www.vldb.org/pvldb/vol13/p2047-lu.pdf)).**
   Deterministic batches *without* pre-declared read/write sets: execute a
   batch speculatively, detect conflicts by reservations, deterministically
   abort a subset, rerun. Directly inspires the recorded v2 upgrade to our
   group-commit rejection fallback: partition a failed batch by footprint
   instead of one-by-one replay (60 §group commit).

## D — The log on object storage (the substrate's lineage)

10. **Verbitski et al. — "Amazon Aurora: Design Considerations for High
    Throughput Cloud-Native Relational Databases" (SIGMOD'17;
    [Amazon PDF](https://web.stanford.edu/class/cs245/readings/aurora.pdf)).**
    "The log is the database." Our law 1, at cloud scale, a decade of
    production proof. Their treatment of storage-tier crash recovery
    informed the forced-resolution style of our sidecar rules.
11. **Armbrust et al. — "Delta Lake: High-Performance ACID Table Storage
    over Cloud Object Stores" (VLDB'20;
    [PVLDB PDF](https://www.vldb.org/pvldb/vol13/p3411-armbrust.pdf)).**
    A transaction log of actions in an object store with optimistic
    concurrency on the next log slot — structurally our protocol, for
    analytics tables, pre-conditional-writes (they needed a coordination
    service where we use `If-None-Match`). Their checkpoint/manifest
    mechanics and the mistakes they document are directly reusable.
12. **Turso — "Turso Cloud Goes Diskless" (2026, industrial;
    [blog](https://turso.tech/blog/turso-cloud-goes-diskless)).**
    See [turso-notes.md](turso-notes.md): Express-then-fold two-tier,
    cross-database batching, measured latencies, cost math. The substrate
    half of our envelope, running in production.

## E — The dependency-theoretic frame (why our constraint language is special)

13. **Fagin, Kolaitis, Miller, Popa — "Data Exchange: Semantics and Query
    Answering" (ICDT'03; TCS full version —
    [UPenn PDF](https://www.cis.upenn.edu/~val/CIS650/DataX-tcs.pdf)).**
    The TGD/EGD vocabulary and the chase. Bumbledb's keys are EGDs
    exactly; containments are *embedded* INDs/CINDs by syntax with the
    full-TGD operational profile (the existential is discharged by
    writer-side minting, never fired); capacity is outside first-order
    dependency language on purpose (the disjunctive-EGD encoding of
    "at most k" has a branching chase — refused). The deep read's
    sharpest lines: on ground instances refusal *is* the chase
    (admission = the successful zero-step chase), and `Var(store) = ∅`
    deletes their entire certain-answers/cores/coNP apparatus — every
    one of those theorems is a theorem about labeled nulls. THESIS §7
    carries the audited version; L7 is the locality of the satisfaction
    check, and guardedness — not fullness — is the boundary.
14. **Bailis et al. — "Feral Concurrency Control" (SIGMOD'15; extended
    [arXiv:1502.02005](https://arxiv.org/abs/1502.02005) — if the id
    404s, use the [author PDF](http://www.bailis.org/papers/feral-sigmod2015.pdf)).**
    Empirical proof that application invariants live *outside* general
    databases (ORM validations, racy and unenforced) — the negative-space
    argument for a closed, compiled, engine-enforced constraint set. Cite
    it whenever someone asks why Postgres can't just do our algebra.

## F — The CRDT boundary (what we are deliberately not)

15. **Preguiça — "Conflict-free Replicated Data Types: An Overview"
    ([arXiv:1806.10254](https://arxiv.org/abs/1806.10254)).**
    The convergence-by-weakening school, surveyed by one of its authors.
    Read to sharpen the boundary: CRDTs guarantee convergence for
    operations that always commute and cannot state cross-object
    invariants; we keep the invariants and compute the commuting subset.
16. **Laddad et al. — "Keep CALM and CRDT On" (VLDB'23;
    [arXiv:2210.12605](https://arxiv.org/abs/2210.12605)).**
    The reconciliation attempt from the CRDT side: queries over lattices
    are trustworthy exactly when monotone — CALM again, arriving from the
    other direction. Confirms the same non-monotone frontier our CONFLICT
    cells trace.

## Reading order

3 → 1 → 2 (the theory spine), then THESIS.md, then 6 → 4 → 7 → 5 (the
commutativity/escrow school), 8 → 9 (determinism), 11 → 10 → 12 (the
substrate), 13 → 14 (the dependency frame), 15 → 16 (the boundary).
