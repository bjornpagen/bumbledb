# 11 — C ref liveness lives inside the allocation it guards; the UAF gate and the code contradict

- **Status:** **fixed this pass** — destroy leaks retired slots so a stashed
  pointer stays allocated; `alive=false` answers `MISUSE`. Test:
  `stashed_instance_ref_after_db_destroy_is_misuse`. The `alive` word stays
  on the slot because the slot never dies (leak is the C parse of
  "outlives the handle").
- **Severity:** should-fix + one owner ruling.
- **Supersedes:** BND-03, VER-03.

## Principle

Insight 8 inverted: the alive bit is the boundary object, and it was placed
*inside* the allocation whose death it reports. Reading it after the owner
died is itself the use-after-free.

## Evidence

- `crates/bumbledb-c/src/db.rs` — `bdb_instance_ref` / `bdb_witness` /
  `bdb_tx_ref` carry `alive: AtomicBool` in their own allocation.
- The `Retired` mechanism (`db.rs:45-48` — "boxes are held so stashed C
  pointers stay allocated") keeps refs alive **across callback return**:
  that window is genuinely closed, and it is the real footgun.
- `bdb_db_destroy` (`db.rs:853-862`) refuses while busy, then drops the box
  — and the `Retired` vec with it. A ref used after destroy dereferences
  freed memory to read its own flag.
- Proposal representation gate: "A C ref stashed past `bdb_db_destroy`
  answers `BDB_STATUS_MISUSE` — pinned under the sanitizer lane, never
  use-after-free."

## The ruling (owner decision — the gate and code cannot both stand)

1. **Refcounted slot (the gate as written):** the alive word lives in an
   `Arc`'d slot handed to every ref/witness; `bdb_db_destroy` drops the
   handle but the slot outlives it, flipped dead. Post-destroy use answers
   `MISUSE`; the ASAN lane pins it. Cost: one atomic refcount per ref, and
   destroy no longer frees ref memory immediately (freed when the last
   stashed pointer's… is never freed if the caller leaks the pointer — the
   leak becomes the caller's, which is the honest C shape).
2. **Reword the gate (the code as written):** post-destroy use of *any*
   freed handle is the ordinary C contract (`bdb_db` itself has this
   property); `Retired` covers the callback-scope window; the gate text and
   BND-03 are rewritten to claim exactly that and no more.

Either exit is representationally honest; holding both is not. If (1),
delete the per-ref `alive` field in the same change — the slot *is* the
liveness object.

## Acceptance

- One of: (a) ASAN-lane test — stash a ref, destroy the db, use the ref,
  observe `MISUSE` with no sanitizer report; or (b) the gate text in the
  proposal + this file's supersessions updated to the callback-scope claim,
  with a doc comment on `Retired` naming the boundary.
- No `alive: AtomicBool` inside a ref allocation under exit (1).
