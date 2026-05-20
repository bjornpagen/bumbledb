# PRD 02: Generic Static Semijoin Empty Proof

## Goal

Recover the `job_q24_voice_keyword_actor` regression by replacing the removed JOB-specific static-empty proof with a generic structural semijoin emptiness proof.

This must not reintroduce relation-name-specific logic. The proof must work from query structure, literal/range predicates, variable sharing, access path metadata, and encoded relation images.

## Explicit Non-Goals

- No backwards compatibility with removed JOB-specific static-empty functions.
- No migration layer for old plan names or old explain output.
- No relation-name hardcoding to emulate v1/v2 behavior.
- No compatibility fallback that marks queries empty using old JOB-specific assumptions.
- No support for old Datalog-era query shapes beyond what typed query IR represents.

## Regression Being Fixed

Current JOB 10k:

```text
job_q24_voice_keyword_actor: 176002us BDB vs 9859us SQLite
runtime: pure_lftj
rows: 0
```

Old scale-10000 artifact:

```text
job_q24_voice_keyword_actor: ~43us BDB
rows: 0
```

The old engine had a JOB-specific proof that knew how to combine `Keyword`, `MovieKeyword`, `MovieCompanies`, `CompanyName`, `Title`, and related predicates. PRD 10 deleted this hardcoding, which was correct, but the replacement proof only checks whether one literal-filtered atom is empty by itself.

## Current Code Anchors

- `crates/bumbledb-lmdb/src/query.rs`
- `static_literal_atoms_prove_empty`
- `static_atom_row_matches`
- `query_image_scope_for_query`
- `RelationIndexImage::prefix_count`
- `RelationIndexImage::entries_with_prefix`
- `RelationIndexImage::component_bytes`
- `query_access.rs` prefix helpers from PRD 10

## Required Generic Proof

Add a structural proof that can prove a query empty by propagating candidate sets through semijoins before full planning/execution.

Target concept:

```text
literal/range filtered atoms produce candidate values for shared variables
candidate sets propagate through binary/n-ary relation atoms
if any required variable candidate set becomes empty, query is empty
if any atom has no row compatible with current candidate sets, query is empty
```

This can be implemented conservatively. It does not need to prove all empty queries. It only needs to prove enough real JOB-empty shapes generically.

## Data Structures

Recommended internal types:

```rust
struct StaticSemijoinProof {
    empty: bool,
    atoms_checked: u64,
    rows_scanned: u64,
    prefixes_probed: u64,
    candidate_values: u64,
    reason: StaticSemijoinReason,
}

enum StaticSemijoinReason {
    None,
    LiteralAtomEmpty,
    CandidateSetEmpty { variable: VarId },
    AtomSemijoinEmpty { atom: AtomId },
}

struct CandidateSet {
    value_type: ValueType,
    values: BTreeSet<EncodedOwned>,
    complete: bool,
}
```

The exact type names may vary. The proof must expose diagnostics through `PlanCounters` and explain output.

## Required Algorithm

### Step 1: Seed Candidate Sets

For each atom with static literals or range predicates, use the most selective available access path to collect candidate values for variables in that atom.

Examples:

```text
Keyword(keyword = "hero") -> keyword variable candidate ids
CompanyName(country_code = "[us]") -> company variable candidate ids
RoleType(role = "actor") -> role variable candidate ids
Name(gender = "m") -> person variable candidate ids
Title(production_year > 2010) -> movie variable candidate ids
```

This must be generic:

- choose access path by leading static fields when available
- otherwise scan relation image if row count is under a safe threshold
- collect variable values from matching rows

### Step 2: Propagate Through Relation Atoms

For each relation atom whose variables overlap known candidate sets, probe or scan compatible rows and narrow unbound/known candidate sets.

Example generic shape:

```text
MovieKeyword(movie: ?movie, keyword: ?keyword)
known keyword candidates -> candidate movie set
```

Use full-covering access path prefix probes when a candidate variable appears in leading fields.

Fall back to bounded scan only if relation size is small enough.

### Step 3: Iterate To Fixed Point

Repeat propagation until no candidate set changes or a safety budget is hit.

Required budgets:

```rust
const STATIC_SEMIJOIN_MAX_PROBES: u64 = ...;
const STATIC_SEMIJOIN_MAX_SCANNED_ROWS: u64 = ...;
const STATIC_SEMIJOIN_MAX_CANDIDATES: usize = ...;
```

If budget is exceeded, return `empty = false` and let normal planning run. Never return an unsound empty proof.

### Step 4: Prove Empty

Return `empty = true` when:

- any required candidate set is complete and empty
- any atom has no compatible row under complete candidate constraints

## Soundness Rules

- The proof may be incomplete but must never be unsound.
- Only mark a candidate set as complete when it was derived from exhaustive access path enumeration or an exact prefix/range over a full relation image.
- Do not infer emptiness from sampled planner stats.
- Do not infer emptiness from estimated distinct counts.
- Do not special-case relation names.

## Integration Points

Call this proof from the same places as `static_literal_atoms_prove_empty`, before full planning.

Recommended order:

1. simple literal atom empty proof
2. static semijoin proof
3. normal planning

If semijoin proof succeeds, the runtime should be `StaticEmpty` and output should respect global count semantics.

## Required Tests

Add generic tests, not JOB-name-specific tests only:

- Literal dimension row exists individually, but semijoin through fact relation is empty.
- Two literal-filtered dimensions produce disjoint central variable candidates.
- Enum literal semijoin proof works.
- Serial literal semijoin proof works.
- Compound relation semijoin proof works.
- Budget exhaustion falls back safely to normal execution.
- Non-empty query is not incorrectly proven empty.
- `job_q24_voice_keyword_actor` is proven empty on JOB 10k.

## Required Counters

Extend `PlanCounters` or existing static-empty counters with:

```rust
static_semijoin_prefixes_probed
static_semijoin_candidate_values
static_semijoin_rounds
```

If adding counters is too invasive, include these in explain/debug only, but tests must be able to assert the proof path was used.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- JOB 10k benchmark completes.
- `job_q24_voice_keyword_actor` chooses `StaticEmpty` or an equivalent semijoin-empty runtime.
- `job_q24_voice_keyword_actor` Bumbledb avg is under `1000us` on JOB 10k.
- No hard-coded JOB relation names appear in engine query code.

## Completion Criteria

- q24 regression is fixed generically.
- The old relation-name-specific static-empty functions remain deleted.
- Static empty proof is structural, typed, and conservative.
- This PRD is deleted and committed after passing.
