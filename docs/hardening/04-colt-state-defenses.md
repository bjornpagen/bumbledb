# PRD 04 — COLT state defenses: tagged tokens, hard start

Findings fixed (docs/audit/colt.md): **MEDIUM** "Forcing a node with an
outstanding iteration token silently reinterprets the token"; **LOW**
"`Cursor::Row` iteration ignores `max`"; **LOW** "`start()` without `select()`
is debug-guarded only; the release failure mode is wrong results"; **NOTE**
"`position_matches` truncates via `zip`"; **NOTE** "`WordMap::grow`
re-allocates the dense list on every doubling".

## Purpose

The COLT audit found the trie itself correct but under-defended: three
latent wrong-results modes are held off only by executor discipline and debug
asserts. Wrong-results classes get *hard* defenses (release-visible, or
representation); everything else gets the honest assert where the invariant
lives.

## Technical direction

- **Tokens carry their kind.** `exec/colt.rs`: spend `BatchToken`'s top bit as
  a state tag — `POSITIONS` (root/chunk iteration) vs `DENSE` (forced-map
  iteration). Mint sites set it (`iter_positions` returns tag POSITIONS;
  `iter_map` returns tag DENSE; `BatchToken::default()` stays 0 = untagged
  start, valid for either). `iter_batch_at` checks: a nonzero token whose tag
  does not match the node's current state is the audit's silent-omission
  scenario — make it a release `debug_assert!` is NOT enough for a
  wrong-results class per this suite's rules, but the check runs per *batch*
  (not per tuple), so a release `assert!` costs one branch per ~batch-size
  tuples: make it a release assert with a named message ("iteration token
  outlived a force — drain before probing this cursor"). Verify the tag bit
  cannot collide with the chunk packing (`(chunk+2) << 32 | offset` uses bits
  0..~34; `EXHAUSTED = 1<<32`; bit 63 is free — assert statically via a
  `const _:` check that the packing never reaches bit 63 under the u32 chunk
  space).
- **`start()` becomes a release assert.** `colt.rs:237-241`: the
  unselected-start failure mode is silently-dropped selections — wrong
  results. Promote `debug_assert!(self.selected)` to `assert!` (once per
  occurrence per execution — noise against the join, as the audit already
  priced).
- **`Cursor::Row` honors `max`.** `colt.rs:427-435`: `if token.0 > 0 || max == 0
  { return (0, token); }` — the audit's exact fix; kills both the
  empty-buffer panic and the `yielded > max` contract violation.
- **`position_matches` guards its own arity.**
  `debug_assert_eq!(key.len(), self.schema_columns[level].len())` inside the
  function (the truncating `zip` stays — it is correct once the lengths
  agree, and every caller is exact today; the assert localizes the invariant
  where the truncation lives).
- **`WordMap::grow` rebuilds its dense list in place.**
  `wordmap.rs:134-136`: keep the old dense allocation (swap to a scratch,
  clear, refill) instead of `mem::take` + fresh `reserve` — the entry count
  is unchanged by a rehash, so no allocation is ever needed beyond the first.
  (Growth is pre-fixpoint and sanctioned; this is still the right shape —
  the suite's rule: prefer deleting a hazard, even a small one.)
- The two colt NOTEs the audit closed as no-action (chunk-token wrap beyond
  the scale axiom; pre-probe growth counting appends) get one-line comments
  at the cited sites so the closures are readable in place — nothing else.

## Non-goals

Changing iteration order, token layout beyond the tag bit, forcing rules, or
the suffix-rule strengthening (audited sound); making tokens generation-
checked (the executor's structural discipline is real; the assert is the
tripwire, not a new protocol).

## Passing criteria

- Token-tag test (pure COLT): drain two batches from a Chunks-state node at
  its suffix level, then `get` the same cursor (forcing it), then resume with
  the stale token → the named release assert fires (via `catch_unwind` in the
  test). A fresh `default()` token after the force drains the full, correct
  key set (the recovery path works).
- `iter_batch(Cursor::Row(..), .., max = 0)` returns `(0, token)` — no panic,
  no over-yield; `max = 1` yields exactly once then `(0, _)`.
- `start()`-before-`select()` on a selection-bearing colt panics in release
  builds (test compiled without debug_assertions where the harness allows;
  otherwise assert the panic in the default profile — `assert!` fires in
  both).
- WordMap growth allocation shape: after a grow, a test-only capacity probe
  shows the dense buffer was reused (capacity monotone, no fresh-alloc
  signature — mirror the existing retention-test pattern).
- Every existing colt/wordmap test passes verbatim (the tag bit is invisible
  to well-behaved drains). `scripts/check.sh` green, alloc gate included.
