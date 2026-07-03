# PRD 14 — The read query families

Authority: `50-validation.md` (families + versioned query list), `00-product.md`
("every family must win"; aggregate families required), PRD 06 ids, the suite's
family table.

## Purpose

The eight gated read families, written as code: exact IR, exact param policy,
hand-written SQL golden, gate classification. This file of queries **is** the
benchmark's identity.

## Technical direction

- `families::Family { name: &'static str, kind: Kind /* Gate | Report */, query:
  fn() -> Query, params: fn(&GenConfig) -> Vec<Vec<Value>>, golden_sql: &'static
  str }` and `families::all() -> &'static [Family]` (a const-ish registry;
  `families::digest()` = blake3 over names + Debug(query) + golden SQL — the
  stamp ingredient).
- The eight, precisely (field ids via `schema::ids`; all params drawn seeded from
  `GenConfig` so verify and bench see identical sets; every set of 4 includes the
  documented miss where marked):
  1. **point** — `Q(amount, at) :- Posting(id = ?0, amount, at)`. Guard probe.
     Params: 3 existing posting ids + 1 miss (id = postings + 10⁶).
  2. **fk_walk** — `Q(name, amount) :- Posting(account = a, amount), Account(id =
     a, holder = h), Holder(id = h, name)`, filtered `Posting.account = ?0`.
     Params: 2 cold accounts, 1 hot account, 1 miss.
  3. **chain** — `Q(region, amount, at) :- Posting(account = a, amount, at),
     Account(id = a, holder = h, status = Open), Holder(id = h, region)` with
     `at >= ?0` (range param at the window edge). 4 window starts.
  4. **range** — `Q(id, amount) :- Posting(id, amount, at)`, `at >= ?0`,
     `at < ?1` — the pure O(n)-scan family (the range-accelerator trigger).
     4 windows of the pinned ≈2% selectivity.
  5. **balance** — `Q(a, Sum(amount)) :- Posting(account = a, amount)` gated to
     one holder: join Account(id = a, holder = ?0). 4 holders (1 hot-owning).
  6. **stats** — `Q(k, Min(at), Max(amount), Count) :- Posting(instrument = i,
     amount, at), Instrument(id = i, kind = k)`. No params (literal-free full
     fold; one empty param set).
  7. **string** — `Q(id, amount) :- Posting(id, amount, memo = ?0)`. Params: 3
     vocabulary memos + 1 never-interned miss.
  8. **skew** — `Q(label, amount) :- Posting(account = a, amount), AccountTag
     (account = a, tag = t), Tag(id = t, label = ?0)` where the chosen tags are
     attached (by the generator, guaranteed: PRD 07 amendment if needed — tag 0
     is always attached to every hot account; add that rule to the generator in
     this PRD, same change, with its determinism goldens updated) to hot
     accounts: the small-side/hot-side shape where dynamic cover choice decides.
     Params: 2 hot-attached tags, 2 uniform tags.
- All eight are `Kind::Gate`. The golden SQL for each is **hand-written** into
  `sqlmap::goldens` (three exist from PRD 09; write the remaining five here) and
  pinned equal to `translate` output.
- `families::render_queries_md() -> String` — the human-readable versioned query
  list (IR pretty-form + SQL + param policy per family), emitted by PRD 18 into
  the repo.

## Non-goals

Write families (PRD 15). Weights or scoring — a family wins or it does not.

## Passing criteria

- Unit tests: all eight validate and prepare against a schema-only db; each
  golden equals its translation; params determinism and documented miss presence
  (point, fk_walk, string include one); `digest()` changes when any query
  changes (perturbation test on a copy); skew params reference tags the S-scale
  generator actually attached to hot accounts; `render_queries_md` golden
  (structure, all eight sections present).
- `scripts/check.sh` green.
