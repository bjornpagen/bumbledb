# PRD 00: Suite Contract

## Purpose

Establish the hard vocabulary, invariants, and completion rules for the Free Join paper alignment refactor. This PRD prevents later work from silently changing product scope or using paper terminology incorrectly.

## Dependencies

None.

## Required Reading

- `docs/ROSETTA_STONE.md`
- `docs/free-join-paper/arXiv-2301.10841v2/tex/02-background.tex`
- `docs/free-join-paper/arXiv-2301.10841v2/tex/03-free-join.tex`
- `docs/free-join-paper/arXiv-2301.10841v2/tex/04-optimizations.tex`
- `docs/free-join-paper/audits/01-formal-free-join-plan-audit.md`
- `docs/free-join-paper/audits/04-rosetta-set-semantics-paper-adaptation-audit.md`

## Contract

Bumbledb must implement paper Free Join only after adapting it to Rosetta. The paper is algorithmic authority for Free Join plan structure and execution. Rosetta is product authority for semantics and scope.

## Definitions

- Set relation: a relation whose membership is exact full facts with no duplicates.
- Solution binding set: the set of satisfying variable assignments for a typed positive query.
- Projection set: the duplicate-free set of projected result facts.
- Atom occurrence: one occurrence of a relation atom inside a normalized query. Self-joins produce distinct atom occurrences even when they reference the same base relation.
- Subatom: an ordered subset of variables from one atom occurrence, as defined by the paper.
- Atom partition: the set of subatoms for one atom occurrence across all Free Join nodes, covering every atom variable exactly once.
- Free Join node: a list of subatoms.
- Available variables: variables bound by earlier Free Join nodes.
- New variables: variables in the current node that are not available before the node.
- Cover: a subatom in a node containing every new variable for that node.
- GHT: the paper Generalized Hash Trie interface with relation metadata, current tuple schema, `iter`, and `get(tuple)`.
- COLT: an execution-local Column-Oriented Lazy Trie implementing GHT over immutable relation base images using offset vectors and lazily forced hash maps.
- LFTJ: the current singleton-variable leapfrog triejoin-style executor. It is not the full paper Free Join model.

## Hard Invariants

- No bag semantics may surface in base storage, query execution, counters, output, benchmark correctness, or docs.
- Multiplicity from duplicate witnesses may exist only as internal redundant search work and must collapse before public output.
- No SQL API may be introduced.
- No DuckDB dependency may be introduced.
- SQLite may remain only as an external exact-value benchmark/test oracle using `SELECT DISTINCT`.
- Projection is native set projection, not `SELECT DISTINCT` as a Bumbledb concept.
- Aggregation remains out of scope unless Rosetta is explicitly updated in a future task.
- LMDB remains the only durable backend.
- Query images and COLT structures are private implementation details.
- Malformed typed IR must be rejected at execution boundaries with product errors, not internal planner failures.

## Required Changes

- Keep this PRD suite as the single ordered source of truth for paper alignment work.
- Any later PRD that contradicts this contract must be corrected before implementation.
- Every later PRD must reference this contract and state any intentional adaptation from the paper.

## Passing Criteria

- This suite contains a README and numbered PRDs from 00 through 22.
- Each PRD has dependencies, scope, technical direction, non-goals, acceptance criteria, and validation commands.
- No PRD asks for SQL, bag output, DuckDB planning, public aggregation, or a non-LMDB storage backend.
- The suite makes clear that current LFTJ is not paper Free Join unless lowered into and validated as a singleton-subatom special case.

## Validation Commands

```text
rg "bag semantics are allowed|SQL frontend|DuckDB dependency|public aggregation" docs/free-join-paper-alignment-prds
```

The command must return no matches except inside explicit rejection text if wording changes later.
