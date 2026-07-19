# PRD-H2 — The marshal bijection: writes, fact decode, violations

Wave H · Repo: bumbledb `ts/` · depends on: H1 · runs concurrent with H3, H5
(disjoint files: this PRD owns `ts/src/marshal.ts` + the `db.ts` decode
sites; do not touch `query/*`, `closed.ts`, `relation.ts`)

## Objective

The runtime honors what H1's types claim: handle NAMES cross the marshal
boundary as values, u64 row ids stay the engine's truth. Names→ids on every
write path; ids→names on every read path a user sees (facts, violation
offending facts). The bijection is total and static: the sealed roster,
declaration order, ≤256 rows (`MAX_EXTENSION_ROWS` is engine law).

## Work

1. **Write side** (`ts/src/marshal.ts::cellOf`): add a closed arm — when the
   field descriptor has `closed`, the value is a string handle name;
   translate `name → BigInt(roster.handles.indexOf(name))`. An unknown name
   THROWS with a pointed message naming the vocabulary and the roster
   ("\"DirectPas\" is not a handle of Kind — the roster is Checking,
   Savings"). This is an UPGRADE over 0.3.0 (any bigint used to sail
   through to a commit-time violation); say so in the module doc.
   `rowOf`/`keyRowOf`/`contains`/`delete` ride `cellOf` — verify each path
   reaches the arm (read them; do not assume).
2. **Read side** (`ts/src/marshal.ts::factOf` or wherever fact decode maps
   cells through declared fields): closed fields decode
   `id → roster.handles[Number(id)]`. An id outside the roster THROWS with
   the pointed message: this is only reachable in a store whose
   closed-typed column was never pinned by its containment law — name that
   fact in the error ("the column types Kind but no law pins it — a
   containment statement is the missing piece"). NEVER a silent fallback,
   NEVER `undefined`.
3. **Violations** (`ts/src/db.ts::offendingFactOf`, ~line 707): the
   offending-fact record translates closed cells to names using the
   open-time field descriptors it already reaches (verify it can reach
   them; the relation name is in hand at ~line 707 — thread the descriptor
   table if it is not). The `canonical` string already renders handle names
   (engine render) — after this the record and the string agree.
4. **Probes** (intrinsic):
   - runtime round-trip: insert with `"Savings"`, `scan`/`get` returns
     `"Savings"` (strict equality), the store's raw cell is `1n` (assert
     through exhume's raw row — exhume stays RAW by design, it is the
     recovery surface and carries the roster separately; pin that
     distinction with a comment + assertion);
   - the write throw (unknown name) and the read throw (out-of-roster id,
     constructed via a lawless test store) — both pinned with message
     fragments;
   - a violation whose offending fact carries a closed cell: the record
     holds the NAME and matches the `canonical` string's rendering;
   - the marshal stays literally cast-free (the module's own law — grep).

## Technical direction

- `Number(id)` is safe (roster ≤256); do not add BigInt-indexing cleverness.
- Build the name→index map ONCE per closed value at mint time if profiling
  the hot write path demands it — but start with `indexOf` (the roster is
  tiny); no premature machinery.
- Exhume (`ts/src/exhume.ts`) is explicitly out of scope — raw by design.

## Passing criteria

- All probes green; both pointed throws pinned.
- `marshal.ts` diff contains zero casts and no new `any`/`unknown` (grep).
- Query paths are untouched in this PRD (`git diff --stat` shows no
  `query/*` files — H3/H4 own them).
- `tsc --noEmit` green for marshal + its probes in isolation. Push per the
  wave's commit discipline.
