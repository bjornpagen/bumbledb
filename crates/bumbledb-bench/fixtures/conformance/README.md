# Native conformance fixtures

Cases on which the production engine and the independent evaluators agree. The
generators and replay live in `crates/bumbledb-bench/src/oracle/conformance.rs`.

| Files | Checked behavior |
|---|---|
| `judgment-*.json` | Final-state judgment after a delta against a lawful parent. |
| `complete-*.json` | Complete judgment without a lawful-parent assumption. |
| `reach-hand-*.json` | Derived stages and restricted recursive queries. |
| `hand-*.json` | Canonical query answer sets. |
| `seeded.digests`, `reach-seeded.digests` | `name case_seed blake3` for every generated case. |
| `structural-algebra.json` | Exact quotients and interval intersection/difference against independent Python rational and endpoint-cell oracles. |

Curated cases are checked in and must replay byte-identical. Seeded cases are
regenerated from their seeds on every run; only their BLAKE3 digests are checked
in. A seeded case is excluded when the naive evaluator would visit more facts
than its step budget, so the roster depends on the seeds alone, never on the
machine.

```sh
cargo nextest run -p bumbledb-bench -E 'test(/conformance::tests/)'
python3 scripts/structural-corpus.py
```

`BUMBLEDB_BLESS=1` rewrites the curated cases and both digest lists from the
current generators. Inspect every changed expectation first; blessing must not
silence a disagreement. The structural corpus is independently owned; its
generator checks by default and writes only with `--write`.

The interchange format uses explicit value tags, hexadecimal float payloads,
full-range integer strings, and per-case string dictionaries. It does not cover
computed heads, UUIDs, or dense float intervals; dedicated structural-stage and
numeric tests exercise those features. Judgment compares every violation in
canonical statement order.
