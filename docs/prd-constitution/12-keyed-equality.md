# PRD 12 — Keyed equality: `==` states its real theorem

**Depends on:** 11 (adjacent docs; serialize).
**Modules:** `README.md` (:254-265), `docs/architecture/30-dependencies.md`
(:33-39, :78), `crates/bumbledb/src/schema/tests/reject.rs` (the new
lock), `crates/bumbledb-query/tests/` (macro-level lowering pin if
absent).
**Authority:** the deep audit §2, verified: accepted `A == B` lowers to
two containments, and because EACH direction's target must resolve to a
declared key, both projections are keys — so the accepted statement is
a key-backed 1:1 correspondence (existence + uniqueness both ways) on
the σ-selected projections. The Lean pair
(`KeyBackedEquality.unique_target/.unique_source`) proves it; the bare
form's insufficiency has a countermodel (`bare_containsEq_nonunique`).
Current docs say "set equality … both containments, each judged
independently" — TRUE but an understatement; and the reverse-direction
key requirement has NO dedicated rejection test (audit F8, confirmed:
only the one-way `rejects_no_matching_target_key` exists).
**Representation move:** none. The docs state the stronger theorem; the
acceptance gate's reverse half gets its missing lock.

## Context (decided shape)

1. **README `==` section** upgrades: keep "mutual inclusion … read
   `==` as exactly," add the theorem sentence — "because each
   direction's target must be a declared key, accepted `==` is a
   key-backed one-to-one correspondence on the selected projections:
   every selected A-fact has exactly one selected B-witness with the
   same projected value, and vice versa. It is not literal row
   equality (unprojected payloads may differ) and says nothing about
   unselected facts — which is the discriminated-union idiom's whole
   point."
2. **30-dependencies.md** gains the precise decomposition (mutual
   inclusion + injectivity from each key ⟹ bijection on σ-subsets),
   the two non-claims (no whole-row equality, no unselected-fact
   claim), the composite-projection note (the key applies to the
   product — `key_permutation` merely reorders), and the Lean row
   cross-reference.
3. **The reverse-key rejection lock** (schema tests): an `==` whose
   LEFT projection is not a declared key of the left relation must
   reject via the REVERSE containment's `NoMatchingTargetKey` — the
   test asserts the citation identifies the reverse half (statement
   identity, using the violations/citation machinery), pinned for a
   composite (2-field) projection as well as the singleton case.
   (Brief A6, approved:) plus an arity-sweep acceptance lock — 3-field
   composite equality with reordered target-key declaration order
   (exercising `key_permutation`) and mixed scalar types validates and
   enforces; unprojected-payload difference stays legal (positive
   witness).
4. **Lowering pin** (only if none exists after search): the macro
   lowers `L == R` to exactly `[L <= R, R <= L]` adjacent statements —
   a golden on the emitted descriptor order, protecting the mirror
   pairing the diagnostics rely on.

## Technical direction

Docs first (they state what the locks then pin). For the lock, build
the failing schema in-test via the descriptor API (not the macro) AND
via the macro (compile-fail or runtime-reject per the current macro
error path — follow the house pattern in schema/tests/reject.rs); assert the
error names the projection and relation of the reverse half.

## Passing criteria

- `[shape]` README + 30-dependencies carry the theorem with both
  non-claims; `grep -n "one-to-one\|bijection" docs/architecture/30-dependencies.md` ≥ 1.
- `[test]` Reverse-key rejection locks green (singleton + composite);
  the existing one-way rejection test untouched; lowering pin present
  and green.
- `[gate]` Docs + tests only; fingerprint pin untouched; full suite
  green; clippy; fmt.

## Doc amendments (rule 6)

This PRD is its amendments; the theorem↔evidence `==` rows update
their evidence cells to cite the new locks.
