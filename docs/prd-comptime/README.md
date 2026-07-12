# PRD set — the comptime pass: ground axioms and the staging law

This directory was the complete, ordered work plan for the phase after the
algebra pass: closed relations as ground axioms, the emission, virtual
storage, compiled subsets, the enum funeral, the folds, and the surface pass.
It **followed** `docs/prd-algebra/` (fully executed before this set began).
**The set is closed** — every content PRD landed and retired; this file
remains as the record (the organizing principle, the vocabulary discipline,
the policy, the phase ledger, the refusals). When this record and an
architecture chapter disagree, **the chapter wins**.

## The organizing principle: the staging law

**Every computation runs at the earliest stage where its inputs are fixed.**
The engine is a seven-stage evaluator — expansion, open, prepare, bind,
generation, execute, commit (the ladder is written down where the executor's
doctrine lives: `docs/architecture/40-execution.md` § the staging law) — and
this set did three things to it:

1. **Gave the theory constants.** A `closed relation` declares its extension
   in the schema: rows are **ground axioms**, identified by declaration-order
   handles, virtual in storage, frozen by the fingerprint. The `enum` type —
   a vocabulary pretending to be an encoding — died, and the roster dropped to
   six pure value types.
2. **Taught the evaluator to eat them.** Statements into closed relations
   compile to in-register word-sets at open; queries against closed relations
   fold at prepare (the chase generalized from eliminator to evaluator); a
   vocabulary join has zero runtime existence.
3. **Fixed the known late-stagers.** The staging audit (2026-07-10) found six
   computations running one stage after their inputs fixed; this set staged
   them correctly (σ-literal encodings, judgment flags, the literal latch,
   predicate folding) or recorded why they stay (fresh→FD order is a
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

## The phases — the ledger

Phase A — the ground — landed whole and retired (01 — closed relations:
the theory acquired ground axioms — the `closed relation` production,
declaration-order handles, the sealed extension encoded ONCE at validate,
the closed auto-key `R(id) -> R`, the roster's typed rejections (closed
`fresh`, rays as intrinsic values, nested closed refs, the 256-row cap),
and the intrinsic-vs-policy law; 02 — the emission: the host enum welded
to the row ids (`id`/`from_id` with the per-relation emitted weld test),
the handle newtype, bare handles in statement selections, and the
manifest's extension tables — the vocabulary as data for foreign hosts;
03 — virtual images: the theory as storage — the store holds zero
vocabulary bytes, the closed image synthesized once per process at the
sentinel generation, writes refused as a typed error); the rulings live
in `10-data-model.md` § closed relations and § fingerprint inputs,
`70-api.md` § the `schema!` grammar and § id constants and the manifest,
`50-storage.md` § virtual relations, and `40-execution.md` (D1's closed
carve-out, the view-memo sentinel generation).

Phase B — the cutover — landed whole and retired (04 — compiled subsets:
a containment into a closed target carries no probe strategy — its
enforcement plan IS the answer set, ψ applied to the sealed extension at
validate into a 4×u64 member word-set, judged in one AND and one test;
05 — the enum funeral: the `enum` type deleted whole, the value roster
down to six, the mechanical rewrite `k: enum K { A, B }` → closed
relation + reference + containment run across every theory, doc, and
cookbook recipe, the obituary written); the rulings live in
`30-dependencies.md` § enforcement (the compiled-subset worked example,
`Escalation(severity) <= Severity(id | pages == true)` — rot-proofed as
cookbook recipe 8), `10-data-model.md` (the enum's obituary), and
`00-product.md`'s deleted-vocabulary rows.

Phase C — the folds — landed whole and retired (06 — oracles over
axioms: the differential harness learned the closed shapes — the tax the
axioms charge, paid in the naive model and the translator before any
timing is believed; 07 — the chase-evaluator: folding stage-zero atoms —
a closed atom with prepare-resolvable filters evaluates at prepare into
the surviving id-set riding the siblings as a plan-constant membership,
the complement fold for negated atoms under the domain guarantee, the
rule-death channel, and EXPLAIN's fold lines; 08 — the late-stager
sweep: the checker consumes constants — σ-literal encodings sealed at
validate (`CompiledCheck`), stage-1-fixed judgment flags carried, never
re-derived; 09 — the literal latch: monotone resolution — a resolved
`str` literal rewrites its plan slot once, permanently, and a
zero-pending zero-param query skips predicate resolution entirely; 10 —
statically empty: the comptime-unreachable of queries — constant-refuted
rules die at prepare with their killing predicate as the record, and an
all-dead program plans to the `Empty` plan); the rulings live in
`40-execution.md` § the chase, § access paths (statically empty), and
§ measured mechanisms (the latch), `20-query-ir.md` § normalization, and
`60-validation.md`.

Phase D — the surface — landed whole and retired (11 — handles
everywhere: the renderer prints closed-reference words as handles with
the visibly-wrong `Kind(7?)` out-of-range fallback, the statement
renderer and EXPLAIN's fold lines print through the same convention (the
fold's surviving set as a handle set — the set IS the payload), the
round-trip goldens pin the handle spellings (`render(lower(text)) ==
normalize(text)` byte-exact, the bare fixed point where the closed
relation is named `UpperCamel` of its field), the cookbook gained the
vocabulary tier — recipes 6–8: the vocabulary, the classification, the
sub-vocabulary — the repo README gained the tier-2 example and the
staging-law summary, `40-execution.md` gained the staging-law ladder as
a named section, and the architecture chapters were swept for enum
residue); the rulings live in `20-query-ir.md` § the renderer and § the
query notation, `40-execution.md` § the staging law, and
`docs/cookbook.md`.

Dependency spine, discharged: 01→02→03 landed strictly ordered; 04 after
01/03, 05 after 02, both landed; 06–10 landed freely interleaved after
Phase B; 11 landed last and closed the set. **No content PRDs remain in
this directory** — what stays is the record: the organizing principle,
the vocabulary discipline, the policy, the phase ledger above, and the
refusals below.

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
