# Native conformance fixtures

These JSON files record semantic inputs, expected results, and deterministic
provenance. The production engine and independent Rust evaluators must agree,
and replay must reproduce the checked-in bytes. The serializer and replay
entry point live in `crates/bumbledb-bench/src/conformance.rs`.

| Files | Checked behavior |
|---|---|
| `judgment-*.json` | Final-state judgment after a delta against a lawful parent. |
| `complete-*.json` | Complete judgment without a lawful-parent assumption. |
| `reach-*.json` | Derived stages and restricted recursive queries. |
| `seeded-*.json`, `hand-*.json` | Canonical query answer sets. |
| `structural-algebra.json` | Exact quotients and interval intersection/difference against independent Python rational and endpoint-cell oracles. |

Run from the repository root:

```sh
cargo test -p bumbledb-bench the_corpus_replays_byte_identical
python3 scripts/structural-corpus.py
cargo test -p bumbledb-log --test structural_conformance
```

The regular battery runs these checks. The structural corpus is checked by its
own test target; it is not counted among the provenance replay cases.

To deliberately regenerate query fixtures:

```sh
cargo test -p bumbledb-bench regenerate_the_conformance_corpus -- --ignored --nocapture
```

Inspect the coverage report and every changed expectation. Generation excludes
slow cases, so a noisy machine may select a different roster. Regeneration must
not be used to silence disagreement. The structural corpus is independently
owned; its generator checks by default and writes only with `--write`.

The interchange format uses explicit value tags, hexadecimal float payloads,
full-range integer strings, and per-case string dictionaries. It does not cover
computed heads, UUIDs, or dense float intervals; dedicated structural-stage and
numeric tests exercise those features. Judgment compares every violation in
canonical statement order. The historical `phase` field describes the verdict
and never licenses skipping a statement.

These fixtures do not establish crash recovery, native lifetimes, S3 behavior,
or performance. Those properties have separate test targets.
