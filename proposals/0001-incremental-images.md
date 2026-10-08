# Proposal 0001: incremental relation images and dependency-aware maintenance

Status: **to investigate, not to implement.** Written 2026-10-07 at
d76d31abc. Nothing here changes the engine. The first deliverable is a
measurement; any implementation decision waits for it.

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

Consequence: a workload that alternates writes and reads on a large
relation pays a full scan and decode of that relation on every read after a
write. No benchmark lane measures this today. The warmth panel
(`docs/perf/results.md`) shows the stakes: `busy_scan` is 1,424 µs cold and
4.25 µs warm at scale S.

## 2. Step zero: measure

Before any design work, add a benchmark lane: write k rows to relation R,
then run a read that needs R's image, for a range of |R| and k. Report the
rebuild cost against the warm cost, against SQLite on the same loop, and as
a share of the commit's own durable cost (~4–5 ms on the macOS reference
host). If the rebuild is small next to the commit, stop here and record that
as a finding.

## 3. Option A: delta-maintained images (engineering)

Conjecture, not designed:

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
  as the kernels are checked against `exec/kernel/reference.rs`.

Open questions: memory accounting for shared chunks; delta-log bounds;
interaction with heap epochs, which never enter the cache; whether
selection-local bucket images (`image/selection.rs`) need the same
treatment.

## 4. Option B: dependency-aware maintenance of derived results (research)

Dependencies do not help invalidate images. An image is base data, so any
change to R changes R's image, and relation-granular invalidation is already
exact for base data. Declared dependencies matter one level up, for
**derived** state: prepared-query memos, aggregates, and any future
materialized view. That is incremental view maintenance, and the schema
already declares what IVM systems usually have to guess.

Candidate uses, all conjecture:

- **Key plus containment gives exact join deltas.** If S has a key on `id`
  and `R(s_id) <= S(id)`, an inserted R row joins exactly one S row, so
  the change to R ⋈ S is ΔR ⋈ S, one row per changed row, by law rather
  than by estimate.
- **Keys remove the set-semantics deletion problem.** A projection that
  drops columns can give one output fact several derivations, which forces
  counting or delete-and-rederive. If a dependency proves the projection
  keeps a key, each output has one derivation and deletes propagate
  directly. `plan/fj/provably_distinct.rs` already proves distinctness from
  schema facts; start there.
- **Grouped aggregates keyed by a determinant** update one group per
  changed row, in place.
- **Interval and window laws** bound which rows a change can affect.

Prior work to read before inventing anything:

- DBSP: incremental computation over weighted sets (Budiu et al., VLDB 2023)
- differential dataflow (McSherry et al.)
- DBToaster: higher-order delta queries (Koch et al.)
- the Dynamic Yannakakis algorithm (Idris, Ugarte, Vansummeren, SIGMOD 2017)
- counting and DRed for recursive views, against the Datalog fixpoint
  already used downstream

## 5. Decision gates

1. Step zero shows rebuild cost matters for a real workload, or it doesn't,
   and we stop.
2. If it matters, prototype option A behind the differential check before
   touching option B.
3. Option B starts only if prepared-query memos or views become a product
   need, and starts with the prior-work read, not code.
