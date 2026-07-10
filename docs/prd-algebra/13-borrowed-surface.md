# PRD 13 — The borrowed surface: structs and params — LANDED via `docs/prd/22`

**Status: implemented before this set began execution** (`241ccfc`, PRD 22 of
the correctness-and-elegance campaign, owner-approved batch of 2026-07-09).
This file is the reconciliation record; the design context lives in this set's
git history.

What landed, verified against this PRD's intended criteria:

- `schema!` emits `str → &'a str`, `bytes → &'a [u8]`; one lifetime per
  variable-width struct; all-fixed-width structs lifetime-free. `Fact` became
  the lifetime-parameterized trait (`impl<'a> Fact<'a> for Account<'a>`), with
  the GAT alternative recorded at the definition.
- Typed `get`/`scan_facts` return txn-lifetime views resolving from both
  borrow sources (committed dict mmap, this-txn pending interns), UTF-8
  validated at resolve without a copy. No owned twins exist.
- Borrowed params: `BindValue<'a>` is the scalar bind vocabulary;
  `ir::Value` stays owned by decision; sets stay owned-element slices.
- Bonus beyond this PRD's scope: the delta guard map was restructured
  (`StatementId → guard bytes → disposition`) so write-txn point reads borrow
  instead of boxing a key copy — review finding F (guard_overlay allocation)
  died as a side effect.
- Gate scenarios extended: insert+get and borrowed-param selection measure
  zero host allocations.

**Residual work: none.** The results redesign remains refused (recorded in
this set's README); the borrowed surface is complete.
