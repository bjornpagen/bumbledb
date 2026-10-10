# Proposal 0001: incremental images and dependency-aware invalidation

Status: **to investigate, not to implement.** First written 2026-10-07 at
d76d31abc; option B expanded into a research program 2026-10-08. Nothing here
changes the engine. The first deliverable is a measurement; any implementation
decision waits for it.

## 1. What happens today

Read at d76d31abc:

- Every relation has a durable change version, `[0x07, relation u32 BE] →
  u64` in the meta database (`storage/store/format.rs`,
  `K_RELATION_VERSION_TAG`). A commit advances exactly the relations whose
  rows it actually changed, in the same LMDB transaction as those rows
  (`storage/store/candidate.rs`, `changed_relations`). Set semantics mean a
  no-op add or remove advances nothing.
- The image cache holds one slot per relation, keyed by `(relation,
  version)` (`image/cache.rs`, `image/cache/get_or_build.rs`). A reader asks
  for the version its own snapshot sees. On a miss it rebuilds the **whole**
  relation image from one canonical-row scan (`image/build.rs`). Publishing
  a newer version drops older map entries; readers holding an older image
  keep it through their `Arc`. A reader behind the newest version builds a
  private image and does not publish it.
- Writes never touch the cache. Invalidation is lazy and exactly
  relation-granular: a write to relation A never invalidates relation B.
  `Db::clear_cache` is the only whole-cache wipe.
- Nothing above the image is cached. A `PreparedQuery` holds a plan, not a
  result; every execution re-runs the join over the current images.

Consequence: a workload that alternates writes and reads on a large
relation pays a full scan and decode of that relation on every read after a
write, and a workload that repeats the same query pays the whole join every
time even when the write could not have changed the answer. No benchmark
lane measures either today. The warmth panel (`docs/perf/results.md`) shows
the stakes: `busy_scan` is 1,424 µs cold and 4.25 µs warm at scale S.

## 2. Step zero: measure

Before any design work, add two benchmark lanes:

1. **Image rebuild.** Write k rows to relation R, then run a read that needs
   R's image, for a range of |R| and k. Report the rebuild cost against the
   warm cost, against SQLite on the same loop, and as a share of the
   commit's own durable cost (~4–5 ms on the macOS reference host).
2. **Repeated query.** Run one prepared join repeatedly while committing
   small writes, in three mixes: writes to relations the query never reads,
   writes the query reads but that cannot change its answer (section 4.3's
   examples), and writes that do change it. Report the per-execution join
   cost. This prices option B before anyone builds it.

If both costs are small next to the commit, stop here and record that as a
finding.

## 3. Option A: delta-maintained images (engineering)

This has been built once. PR #10 (merged 2026-07-19) shipped copy-on-append
maintenance for delete-free relations: per-column copy plus a suffix-scan
decode of the new rows, checked by a differential referee (append path
byte-identical to rebuild), with delete-bearing commits keeping
evict-and-rebuild. It did not survive the successor rewrite (4a057369). Read
that PR and its three-report investigation before redesigning anything.

Conjecture for the next version, not designed:

- Keep a bounded in-memory log of per-relation net deltas keyed by version.
  The sealed `ChangeSet` already carries the normalized net delta at commit.
- On a miss at version v+k, if the cache holds v and the log covers
  v → v+k, patch the image instead of rescanning: append added rows' words,
  and remove deleted rows by swap-remove or a survivor mask. Image
  positions are not row ids, so reordering is free. Any gap, including
  commits from another process, which the in-process log cannot see, falls
  back to a full rebuild.
- Chunk the column slabs and share unchanged chunks between versions
  (copy-on-write), so old readers keep exact images without whole-image
  copies.
- New text in a delta interns in the current generation, as a build does.
- Proof obligation: patched image == rebuilt image, checked differentially,
  as PR #10 did and as the kernels are checked against
  `exec/kernel/reference.rs`.

Open questions: memory accounting for shared chunks; delta-log bounds;
interaction with heap epochs, which never enter the cache; whether
selection-local bucket images (`image/selection.rs`) need the same
treatment; why PR #10 didn't carry over and whether the reason still holds.

A further variant: LMDB stores large values on contiguous overflow pages, so
a column slab could be persisted under `(relation, version)` and mapped
zero-copy after a restart. It is blocked on durable string tokens: interner
tokens are generation-scoped and in-memory today.

## 4. Option B: dependency-aware invalidation of derived results (research)

### 4.1 The target is derived state, not images

Dependencies do not help invalidate images. An image is base data, so any
change to R changes R's image, and relation-granular invalidation is already
exact for base data. Declared laws matter one level up, for **derived**
state: cached query results, aggregates, and any future materialized view.
For those, relation-granular invalidation is badly conservative: a write to
R invalidates every cached answer that reads R, even when the laws prove the
answer cannot move.

This is incremental view maintenance (IVM), with one unusual advantage. IVM
systems normally treat the schema as types and must assume any delta is
possible. bumbledb's schema is a theory: FDs, conditional containments, and
capacity windows, enforced on every commit by the judge. Every law the judge
guarantees in the final state is a premise the invalidator may use for free.
That is the research bet.

### 4.2 What the cache would do

Cache the results of prepared queries, keyed by the versions of the
relations they read. On each commit, take the sealed `ChangeSet` Δ and sort
every cached result touched by Δ into one of three bins:

| bin | meaning | cost |
| --- | --- | --- |
| keep | Δ provably cannot change the result | zero, or a probe per changed row |
| patch | compute ΔQ from Δ and apply it | proportional to the delta, not the data |
| recompute | no proof available | today's cost |

Soundness is non-negotiable and asymmetric: a false "keep" is a wrong answer,
and a false "recompute" is only slow. Like the judge, the invalidator refuses
when unsure; it never guesses.

### 4.3 The ladder, from engineering to research

Each rung is independently useful and sound on its own. Rungs are listed in
order of increasing theory.

**L1. Read-set versions (engineering).** A cached result records the
versions of every relation it read. Any bump means recompute. This is the
baseline every other rung must beat, and it already wins the "writes to
relations the query never reads" mix.

**L2. Static independence (dependency theory).** At prepare time, for each
relation R the query reads, decide whether updates of a given shape (add or
remove, to R, under a selection) can change the result under the schema's
theory Σ. The output is a write mask stored with the plan; at commit time
the check is a lookup.

The motivating example uses two laws bumbledb already expresses. Let S have
a key on `id`, and let `R(s_id) <= S(id)`. A query joins `R.s_id = S.id`.
Then a pure Add to S, one that does not replace an existing key, is
statically irrelevant to that join:

```math
R_{\text{old}}.s\_id \subseteq S_{\text{old}}.id, \quad
\Delta S^{+}.id \cap S_{\text{old}}.id = \emptyset
\;\Longrightarrow\;
R_{\text{old}} \bowtie \Delta S^{+} = \emptyset
```

New S rows can only join R rows added in the same commit, and those show up
in R's own delta, which is handled separately. No IVM system that ignores
the schema can make this argument.

The classical form of the question is query independence of updates (Elkan
1990; Levy and Sagiv 1993), and it is undecidable in general.

**L3. Dynamic irrelevance (per-commit, value-aware).** When static analysis
says "maybe", look at the actual delta rows. Does a changed row satisfy the
query's selections? Does its join key exist on the other side? This is
Blakeley, Coburn and Larson's irrelevant-update test (1989). Keys make it
cheap: one probe per changed row into an index the judge already maintains
(`det_index`).

**L4. Constraint-aware delta maintenance.** When the result does change,
compute ΔQ instead of Q. Laws help in three ways:

- **Keys remove the deletion problem.** A projection that drops columns can
  give one output fact several derivations, which forces counting or DRed.
  If `provably_distinct` proves the projection keeps a key, each output has
  one derivation and deletes propagate directly. The witness already exists
  for aggregate sinks.
- **Capacity windows bound the delta.** A capacity statement `{lo..hi}` is a
  degree constraint: at most hi matching facts per target group. Degree
  constraints bound join output size (the polymatroid bound), and the same
  machinery applied to the delta query bounds |ΔQ| from |Δ|. That bound
  decides patch vs recompute with a proof, not a heuristic.
- **FDs change the complexity class.** Conjunctive queries that admit
  constant-time updates with constant-delay enumeration are exactly the
  q-hierarchical ones (Berkholz, Keppeler, Schweikardt 2017). FDs can make a
  non-hierarchical query hierarchical once the chase closes over them. The
  question is which bumbledb queries cross that line.

**L5. Tuple-level provenance.** Annotate each cached output row with the
keys of the source facts that derived it (a provenance semiring, Green,
Karvounarakis, Tannen 2007). A delete then invalidates exactly the outputs
that cite it. This rung shares its algebra with the event-algebra thread
(semiring-annotated rows), which is a reason to study both together.

### 4.4 The open mathematics

Each item is a real research question rather than an engineering task.
Answers become either rules the invalidator may use or documented reasons it
refuses.

1. **A decidable fragment for independence.** Implication of FDs plus
   inclusion dependencies is undecidable in general (Mitchell 1983; Chandra
   and Vardi 1985), and bumbledb's containments carry selections, which
   makes them conditional INDs. Find the largest fragment of bumbledb's law
   language where L2 is decidable and cheap: key-based containments, acyclic
   containment graphs, bounded chase depth. Outside it, the invalidator
   answers "recompute". The goal is a sound, incomplete prover with a
   documented boundary.
2. **Delta bounds from capacity windows.** Formalize capacity statements
   (unit, field-weighted, and duration-weighted) as degree constraints. Then
   derive |ΔQ| bounds via polymatroid or entropy bounds (Abo Khamis, Ngo,
   Suciu 2017; Gottlob, Lee, Valiant, Valiant 2012 for the FD case).
   Weighted and duration windows are not standard degree constraints; how
   they enter the bound is open.
3. **Hierarchy under laws.** Characterize which queries become q-hierarchical
   after chasing bumbledb's FDs and containments. Berkholz, Keppeler and
   Schweikardt's ICDT 2018 follow-up treats integrity constraints; check its
   exact scope before extending it.
4. **Final-state semantics.** The judge checks laws on the final state of a
   commit, not per row. Every rule above must be stated against old state,
   net delta and new state, with the laws holding at both ends but not
   necessarily in between. Paired remove-and-add on one key (a replace) is
   the edge case for the L2 example.
5. **Recursion.** Derived occurrences (`Finished`/`RecDelta`) already run
   semi-naive. DRed and counting under laws, for recursive queries, is the
   hardest rung and the last to attempt.
6. **Intervals.** Interval keys and Allen-relation laws suggest temporal
   irrelevance: a write in a disjoint window cannot affect a query pinned to
   another window. State it as a containment over interval projections, and
   check whether it reduces to item 1.

Every rule that ships carries a differential referee: kept or patched result
== recomputed result, over randomized commits, as the kernels are checked
today. A rule without a referee does not ship. A mechanized proof (Lean) is
optional for any rule whose argument is subtle enough to warrant it; item
1's fragment boundary is the likeliest candidate.

### 4.5 Prior work to read before inventing anything

- Blakeley, Coburn, Larson (1989), updating derived relations: detecting
  irrelevant and autonomously computable updates (TODS)
- Elkan (1990), independence of logic database queries and updates (PODS);
  Levy and Sagiv (1993), queries independent of updates (VLDB)
- Gupta, Mumick, Subrahmanian (1993), maintaining views incrementally:
  counting and DRed (SIGMOD)
- DBToaster (Koch et al.): higher-order delta queries
- DBSP (Budiu et al., VLDB 2023): incremental computation over weighted sets
- differential dataflow (McSherry et al.)
- Dynamic Yannakakis (Idris, Ugarte, Vansummeren, SIGMOD 2017)
- F-IVM (Nikolic and Olteanu, SIGMOD 2018): factorized IVM over rings
- Berkholz, Keppeler, Schweikardt (PODS 2017; ICDT 2018): conjunctive
  queries under updates, and with integrity constraints
- Abo Khamis, Ngo, Suciu (PODS 2017): Shannon-type inequalities, submodular
  width, degree constraints
- Gottlob, Lee, Valiant, Valiant (JACM 2012): size and treewidth bounds with
  FDs
- Mitchell (1983); Chandra and Vardi (1985): undecidability of FD + IND
  implication
- Green, Karvounarakis, Tannen (PODS 2007): provenance semirings
- Abiteboul, Hull, Vianu, *Foundations of Databases*: the chase, dependency
  implication

## 5. Decision gates

1. Step zero's two lanes show whether image rebuild cost or repeated-query
   cost matters for a real workload. If neither does, stop.
2. If image rebuild matters, revive option A from PR #10 behind its
   differential referee.
3. If repeated-query cost matters, build L1 (read-set versions) first. It
   needs no theory and sets the baseline.
4. Climb past L1 one rung at a time, and only where the step-zero mix shows
   the next rung would turn recomputes into keeps or patches. L2 and L3 come
   before L4; L5 waits on the event-algebra work.
5. Each section 4.4 question opens with the prior-work read and a written
   statement of the claim, then a referee, and only then code.
