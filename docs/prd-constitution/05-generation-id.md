# PRD 05 — GenerationId: the transaction clock gets a type

**Depends on:** nothing (parallel-safe with 02-04; touches storage/env
and api/db, not schema.rs).
**Modules:** `crates/bumbledb/src/storage/env.rs`
(`generation: OnceCell<u64>` :110), `storage/env/readtxn.rs:15`,
`storage/env/writetxn.rs:62` (`fn generation() -> Result<u64>`),
`storage/commit.rs:147` (`new_generation: u64`),
`api/db/write.rs:137` (the optimistic-CC compare),
`api/db/maintain.rs:70`, `api/db.rs:199` (`commit_seq: AtomicU64`) and
`api/db/read.rs` (the parked-reader compare), plus every signature that
passes a generation.
**Authority:** the audit: a bare `u64` invites accidental comparison
with row ids, statement ids, or the OTHER clock — and this codebase has
two clocks (the persisted `generation`, the in-process `commit_seq`
read-cache sequence), which is exactly the confusion a newtype pair
forecloses.
**Representation move:** two `#[repr(transparent)]` newtypes; the two
clocks become mutually incomparable types.

## Context (decided shape)

```rust
/// The persisted storage transaction id — the generation a snapshot
/// witnessed and a commit advances. Ord: generations are a clock.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct GenerationId(u64);

/// The in-process commit sequence guarding parked-reader reuse
/// (api/db). NOT the persisted generation; a different clock with a
/// different lifetime (resets per process).
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct CommitSeq(u64);
```

- `GenerationId` is public where generations already surface in the
  API (the witness/generation accessors); `CommitSeq` stays crate-
  private. `AtomicU64` remains the storage cell for `commit_seq`; the
  newtype wraps at load/store boundaries.
- No arithmetic surface beyond what call sites prove needed (the
  compare at write.rs:137 is equality; `maintain` may need Ord —
  derive it on `GenerationId` only).
- On-disk bytes unchanged: the newtype encodes/decodes through the same
  u64 word.
- **`FinalStateView` (brief A11, approved):** the commit judgment's
  input becomes a named seam. Today judgment reads base + delta through
  ad-hoc plumbing that IS final-state by construction; the decided
  shape names it — a `FinalStateView<'_>` borrowing (base snapshot,
  delta) that is the ONLY type `judge`/the check plans accept. This
  makes "dependencies judge one transaction final state, never
  operation order" a signature instead of a doc sentence, and
  forecloses any future per-operation judgment path. Zero behavior
  change: the existing judgment tests (including the citation-set
  suite) pass byte-identical.

## Technical direction

Compiler-driven: wrap the OnceCell and the two `generation()` returns,
chase every error. The one semantic assertion to preserve verbatim: the
optimistic-CC compare (`witnessed != current ⟹ GenerationMoved`) — its
test values unchanged. Bench naive model mirrors any public signature
change mechanically.

## Passing criteria

- `[shape]` `grep -rn "generation" crates/bumbledb/src --include="*.rs" | grep ": u64\|-> Result<u64>"`
  → zero (all typed); `commit_seq` reads/writes wrap `CommitSeq`.
- `[shape]` No `From<u64>`/`Into<u64>` pair that would let the two
  clocks launder into each other; conversions are named constructors at
  the storage boundary only.
- `[test]` GenerationMoved conflict tests green with unchanged values;
  full workspace suite green.
- `[gate]` Fingerprint pin untouched; clippy; fmt.

## Doc amendments (rule 6)

`70-api.md`: the generation/witness section names the type; one
sentence distinguishing the two clocks lands where `commit_seq` is
described (or in 50-storage.md if that is its home — mechanism-name
rule applies).
