# PRD 01 — COLT selection levels

Authority: `docs/architecture/30-execution.md` (COLT laziness, the suffix rule),
the Free Join paper §4.2, suite README finding 1.

## Purpose

Teach the COLT to carry **selection levels**: trie levels prepended before the
join-variable levels, one per `Selection`, probed once per execution with the
resolved constant. Force the level once per generation (O(view) — the same
class as an image build), probe it per param (O(1)); everything below a
successful probe is exactly the filtered subtrie the old view scan produced,
built lazily and only for keys actually asked about.

This PRD is pure `exec/colt.rs` (+ its tests). The executor does not use the
capability yet — PRD 02 wires it.

## Technical direction

- `Colt::reset` (the view-adoption entry point) gains the selection spec:
  alongside the existing `trie_schema`-derived `schema_columns`, the caller
  passes the selection fields, and the COLT's level layout becomes
  `[sel_0], [sel_1], …, [join level 0], …` — each selection level is a
  single-column level (`arity == 1`). Store `selection_levels: usize` so every
  existing `level` consumer can be audited: `iter_batch`, `key_count`,
  `get_prehashed`, `force`, `arity`, `position_matches` all already operate on
  `schema_columns[level]` — prepending columns is mechanical, but the **suffix
  rule check** (`level + 1 == self.schema_columns.len()`) and any caller-side
  level arithmetic must be re-audited against the new offset. Grep every use of
  `schema_columns` and `level` in `colt.rs` and list them in the PR
  description; the review artifact is that list.
- New probe API:

  ```rust
  /// Probes the selection levels with this execution's resolved words,
  /// in level order. `Some(cursor)` sits at the first join level;
  /// `None` = no fact matches (the occurrence — and therefore the whole
  /// conjunctive query — is empty on this snapshot).
  pub fn select(&mut self, keys: &[u64]) -> Option<Cursor>
  ```

  Implemented as sequential `get_prehashed` from the root cursor through
  `selection_levels` levels (hash via the existing `hash_words` on a
  single-word slice). Forcing happens lazily inside `get_prehashed` exactly as
  at join levels — no new forcing machinery. Zero selections ⇒
  `Some(root cursor)`.
- The root force at selection level 0 ingests every view position once per
  `reset` — this is the amortization contract: **O(view) once per generation,
  O(1) per subsequent param**. Document it on `select`.
- Byte columns (bool/enum) widen through the existing `word_at` — a selection
  on a bool is a 2-key level; nothing special.
- Two selections on one occurrence = two chained levels; a contradictory pair
  yields `None` at the second probe with no special casing.
- Memory/allocation story: selection levels reuse the same slab discipline as
  join levels (`slots`/`keys`/`chunks` recycled through `reset`). A
  never-before-probed key forces a fresh subtrie — allocation on a warm path is
  bounded by slab high-water exactly as join-level laziness already is; the
  gate protocol implications land in PRD 02.

## Non-goals

Executor integration, view construction changes, plan changes (PRD 02). Range
selections (do not exist). Cover-choice interaction (PRD 06 — but note:
`key_count` at a post-selection cursor already returns the subtrie's count,
which is precisely the "selected cardinality" PRD 06 wants).

## Passing criteria

- Pure-COLT unit tests in `colt.rs`'s test module, over a hand-built image
  (mirror the existing fixtures):
  - One selection level: `select(&[k])` for a present key yields a cursor whose
    `iter_batch` at the first join level yields exactly the positions carrying
    `k`, and nothing else; an absent key yields `None`.
  - Two selection levels chain, including the contradictory-pair `None`.
  - Zero selections: `select(&[])` returns the root and behavior is identical
    to a pre-PRD COLT over the same view (assert equal iteration output).
  - `key_count` on a post-selection cursor labels honestly (Estimate before
    force, Exact after — extend `key_count_labels_are_honest_in_both_states`).
  - Reset/recycle: two `reset` + `select` rounds on the same COLT reuse slabs
    (assert via the existing capacity-retention test pattern).
- Every existing `colt.rs` test passes unmodified (zero-selection COLTs are the
  old COLTs).
- `scripts/check.sh` green.
