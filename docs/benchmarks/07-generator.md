# PRD 07 — The deterministic corpus generator

Authority: `50-validation.md` (seeded 10⁵–10⁷ data), README rule 7 (determinism),
PRD 06 (the schema).

## Purpose

Seeded, re-iterable, skewed ledger data at three scales. Identical config ⇒
identical bytes, forever — corpora are never stored, always regenerated.

## Technical direction

- `gen::Rng`: the house LCG (`state = state * 6364136223846793005 + 1442695040888963407;
  state >> 33`), `u64()`, `range(n)`, `pick(&[T])`, `chance(p_num, p_den)`.
- `pub struct GenConfig { pub seed: u64, pub scale: Scale }`,
  `pub enum Scale { S, M, L }`. Derived sizes (write this table into the code as
  consts, documented):

  | quantity        | S      | M       | L        |
  |-----------------|--------|---------|----------|
  | postings        | 100_000| 1_000_000| 10_000_000|
  | transfers       | postings / 2                  |
  | accounts        | postings / 200                |
  | holders         | accounts / 4                  |
  | instruments     | 512                           |
  | currencies      | 16                            |
  | tags            | 256                           |
  | account_tags    | accounts × 2 (distinct pairs) |
  | tag_notes       | account_tags / 4              |

- **Skew (the Free Join showcase and the ledger reality):** a hot set of
  `max(1, accounts/1000)` accounts receives 50% of postings; the rest uniform. Hot
  membership and routing decided by `Rng` from the config seed only.
- Value rules: every u64 id `< 2^63` (the SQLite mapping axiom — assert in one
  choke-point `fn checked_id`); `amount` in `-5_000_000..=5_000_000` excluding 0;
  `at` timestamps monotonic base + bounded jitter spanning a documented range so a
  fixed window selects ≈2% of postings (the range family's selectivity, pinned as a
  const `RANGE_WINDOW`); `memo` drawn from a 4096-word seeded vocabulary with 1/64
  chance of a unique never-repeated memo; `extref` 16 random bytes; enums/bools
  uniform except `status`: 90% Open.
- Referential closure **by construction**: ids are assigned densely 0..n per
  relation and every FK value is drawn `range(parent_count)` — no rejection, no
  fixup passes.
- Streaming: `pub fn relation_rows(cfg, rel) -> impl Iterator<Item = Vec<Value>>`
  — dynamic-fact form, ready for `bulk_load`, O(1) memory, re-invocable
  (deterministic restart). Row order per relation is generation order.
- `pub fn corpus_digest(cfg) -> [u8; 32]`: blake3 (via the engine's dep) over every
  relation's streamed bytes — the identity of a corpus for stamps and reports.

## Non-goals

Loading (PRD 08). Zipf tails beyond the two-tier hot set (owner decision if the
skew family proves too easy).

## Passing criteria

- Unit tests: same config twice ⇒ identical `corpus_digest`; different seeds ⇒
  different digests; golden digest pinned for `(seed=1, Scale::S)`; hot-share test
  at S (hot accounts receive 50% ±2% of postings); id-range assertions (< 2^63,
  dense 0..n); range-window selectivity at S within 1.5–3%; memo vocabulary test
  (≤ 4096 + uniques distinct memos, uniques ≈ postings/64 ±20%); FK closure spot
  check on 1000 sampled rows.
- `scripts/check.sh` green.
