# Rulings

Precedence decisions that flexed a decision-doc spelling to keep its
invariant. One entry per ruling; the owning doc's invariant block stays
binding.

## 40 §3 — orphans are addressable without a LIST complement (17, 10, 128)

40 §3 spelled collection as "walk the reachable set once and delete the
complement." `gc.rs` walks with GETs alone (no LIST), so that complement
is not a store verb.

1. **Loser self-deletes on Kept and every refused publish.** The
   candidate knows its own digest. `Ran::Kept { incumbent }` and every
   `Refused` arm delete that digest's `ckpt/{digest}.json` and
   `ckpt/{digest}.mdb`. Finding 17 is dissolved: a kept or refused
   loser is not gc fodder and is not uncollectable.

2. **Crash orphans: candidate digests live in a scratch lease; the
   successor sweeps at open.** Before the upload-before-decision
   window, the publisher writes its digest under the reserved
   `~lease` namespace at the known document `ckpt-scratch`. The
   successor GETs that document at open and deletes the named objects
   when they are not the live head. GET-only GC does not LIST-delete
   the complement; that spelling is flexed to this invariant.

The binding contract is unchanged: losers and crashes leave
known-orphan, collectable objects, never live objects with clobbered
links.

## Parent 10 — complement prose caught up to 40 §3

`proposals/10-protocol.md` still spelled orphan collection as the
reachable complement — the 40 §3 form this file already flexed. Parent
10 now states the flexed form (loser self-deletes; crash orphans named
in `ckpt-scratch`). No new flex.

The parent PRD set and the cutover folder retired into
proposals/settlement/ (00-canon.md); doc references above are historical.
