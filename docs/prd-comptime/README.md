# PRD set — the comptime pass: ground axioms and the staging law

This directory is the complete, ordered work plan for the next phase after the
algebra pass. **Baseline assumption: `docs/prd-algebra/` is fully executed** —
rules-shaped IR, DNF lowering, `Allen(mask)` + the configuration kernel,
`Duration`, `Pack`, the chase (`plan/chase.rs`), `bytes<N>` (variable bytes
deleted), the generation witness, the data surface (`ir::render`, emitted id
constants, the theory manifest), the query notation, and the cookbook. Where a
file path below has moved during that execution, the *mechanism name* is
authoritative and the executor re-locates it.

## The organizing principle: the staging law

**Every computation runs at the earliest stage where its inputs are fixed.**
The engine is a seven-stage evaluator — expansion, open, prepare, bind,
generation, execute, commit — and this set does three things to it:

1. **Gives the theory constants.** A `closed relation` declares its extension
   in the schema: rows are **ground axioms**, identified by declaration-order
   handles, virtual in storage, frozen by the fingerprint. The `enum` type —
   a vocabulary pretending to be an encoding — dies, and the roster drops to
   six pure value types.
2. **Teaches the evaluator to eat them.** Statements into closed relations
   compile to in-register word-sets at open; queries against closed relations
   fold at prepare (the chase generalizes from eliminator to evaluator); a
   vocabulary join has zero runtime existence.
3. **Fixes the known late-stagers.** The staging audit (2026-07-10) found six
   computations running one stage after their inputs fixed; this set stages
   them correctly (σ-literal encodings, judgment flags, the literal latch,
   predicate folding) or records why they stay (fresh→FD order is a
   fingerprint input).

**The boundary clause, constitutional: folding produces data, never code.**
Plans, id-sets, masks, word-sets, latched words — consumed by fixed,
asm-gated, measured kernels. No JIT, ever: runtime-generated code cannot be
audited by the disassembly gates or pinned by the fact ledger. This is the
"comptime, not JIT" position and it is a refusal with a derivation, not a
preference.

## Vocabulary discipline

The register extends the algebra set's: *ground axiom*, *handle*, *closed
relation*, *extension*, *virtual image*, *word-set*, *fold*, *latch*, *stage*.
Banned: *enum* (dies in PRD 05; the Rust host construct emitted by the macro
is "the host enum" and is an emission, not a type), *seed/seeding* (closed
relations are never seeded — they are axioms), *lookup table* (it is a closed
relation), *JIT/codegen* (refused).

## Policy (read before executing any PRD)

1. **A PRD is a work-organizational unit, not an atomic passing-code state.**
   No transitional shims, no compatibility aliases, no feature flags. Rip the
   old thing out and cut directly to the end state; the tree may fail to
   typecheck between PRDs — downstream breakage is the next PRD's job.
2. **Passing criteria are typed.** `[shape]` — checkable by reading or grep
   the moment the PRD lands. `[test]` — unit tests written in this PRD,
   co-located with the code they pin. `[gate]` — holds when the campaign
   closes: `cargo fmt --all --check`, `clippy --workspace --all-targets --
   -D warnings`, `cargo test --workspace`, `scripts/check.sh`.
3. **No migrations, ever.** No PRD writes store-conversion code. Stores are
   regenerated; ETL is the human's story.
4. **No smoke-test or end-to-end-test PRDs.** Unit tests pinning this set's
   code are in scope where a PRD says so; running verify/bench harnesses is
   human/orchestrator work.
5. **Conflict protocol:** if executing a PRD reveals the architecture docs
   are wrong or silent, stop and record the conflict in the PRD file.
6. **Doc amendments land in the same change** (architecture README rule 5).
   There are no doc-only PRDs in this set; every chapter change rides the
   code PRD that makes it true.

## The PRDs

Phase C — the folds:
- [07 — The chase-evaluator: folding stage-zero atoms](07-chase-evaluator.md)
- [10 — Statically empty: predicate folding at normalize](10-statically-empty.md)

Phase D — the surface:
- [11 — Handles everywhere: render, notation, cookbook](11-handles-everywhere.md)

Dependency spine: 01→02→03 strictly (each consumes the previous's types);
04 requires 01 (and 03 for its virtual-image reads); 05 requires 02 (hosts
need the replacement before the type dies) and touches everything; 06
requires 05's final shapes; 07 requires 03+04+05 (folds virtual images,
consumes word-sets, operates on the post-enum IR); 08–10 are independent of
Phase A/B and of each other (they may run any time after baseline, in any
order); 11 lands last. Phases: A strictly ordered; B strictly ordered; C
freely interleaved; D closes.

## Refusals (recorded with derivations — do not re-litigate)

- **Narrow encodings for closed references.** A closed-relation reference is
  a u64 word like every reference. Buying back 7 bytes per field with a new
  encoding arm re-imports the enum hack in a coat; the churn dividend of PRD
  05 is spent exactly once.
- **`str` columns on closed relations (v0).** The handle *is* the label; the
  renderer prints handles from the theory. Interned columns on virtual
  relations would force dictionary writes at open, breaking "the store
  contains zero vocabulary bytes." *Trigger:* a real schema needing display
  text the handle cannot carry.
- **`bool` as a closed relation.** Bool is the image of predicates
  (comparisons, memberships), not a vocabulary. The slope stops there.
- **Open extension of closed relations** (runtime row addition). Closed rows
  are axioms; changing them is a new theory (fingerprint). Policy over a
  vocabulary that must drift without a rebuild is an *ordinary* relation —
  the intrinsic-vs-policy law (PRD 01 records it).
- **Folding to code.** The boundary clause above. HyPer/Umbra compile
  queries; this engine folds to data consumed by pre-audited kernels,
  because the measurement epistemology cannot gate JIT output.
- **Extension size beyond 256 rows** (v0 roster cap). A vocabulary larger
  than 256 is policy data wearing a vocabulary costume; the cap also keeps
  every compiled word-set a fixed 4×u64 bitset. *Trigger:* a census sighting.
- **Closed-relation `fresh`, rays as intrinsic values, nested closed refs**
  — each rejected at the validation roster (PRD 01) with a typed error;
  handles are the only identity, and intrinsic columns are value types only.
