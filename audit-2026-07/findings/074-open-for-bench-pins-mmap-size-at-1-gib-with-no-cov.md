## open_for_bench pins mmap_size at a constant 1 GiB — the memory-residency parity claim silently breaks at L, the gating scale, and no fairness rule checks it

category: bench-honesty | severity: medium | verdict: CONFIRMED | finder: bench:honesty
outcome: fixed e5f35cb2

### Summary

The SQLite fairness session documents `mmap_size=1 GiB` as "zero-copy page reads — the analogue of the engine's LMDB mapping" and says the corpus "should be memory-resident, as it is for bumbledb". But the value is a hard constant while the oracle file grows with scale. Measured S-scale oracle files are 18.4 MB at 100k postings; the L-scale corpus is 100x the dominant tables (10M postings, 10M posting_tags, 5M entries, fully indexed per the statement-derived registry plus family composites), extrapolating to roughly 1.8 GiB — about twice the mmap pin. At L — the only scale where the p99 budget gates engage — nearly half of SQLite's file would fall off the mmap and be served through the 256 MiB page cache via pread+memcpy, a one-sided handicap on exactly the whole-scan/join families. The engine pays no analogue: LMDB maps 32 GiB unconditionally. `FairnessCheck` verifies five other parity rules by readback but never mmap_size, cache_size, or mmap coverage vs file size, so the documented parity claim is a comment, not a checked invariant — and the direction of the error flatters the engine.

The defect is currently **latent**: `docs/architecture/60-validation.md` (line 273) records that "the 10 ms warm-p99 budget binds only at scale L; because no L corpus exists, S reports it as informational", and the README claims validation at S scale only — no published pin is tainted today. But `Scale::L` is fully wired (CLI accepts it, budget gates key on it), so the first gated L run walks straight into it with nothing to detect the shortfall.

### Evidence (all verified against the working tree)

- `crates/bumbledb-bench/src/sqlite_run/open_for_bench.rs:33` — `conn.pragma_update(None, "mmap_size", 1_073_741_824_i64)?;` under the doc comment (lines 13-16) claiming cache/mmap memory-residency parity with the engine.
- `crates/bumbledb-bench/src/sqlite_run/fairness_check.rs:41-87` — `run_with` reads back `journal_mode`, `synchronous`, `fullfsync`, `checkpoint_fullfsync`, `index_list` per expected index, and `sqlite_stat1` count. No mmap_size, cache_size, or coverage assertion exists anywhere in the crate (grep over `crates/bumbledb-bench/src` finds mmap only at the two set-sites, open_for_bench.rs:33 and duralane.rs:112, plus comments).
- `crates/bumbledb-bench/src/corpus_gen/sizes.rs:19-36` — L = 10,000,000 postings; `posting_tags = postings`, `entries = postings/2`, `accounts = postings/200` (50k), `mandates = accounts * MANDATE_SEGMENTS` (4, `corpus_gen/mandate.rs:9`) = 200k. (The original finding said 400k mandates; the correct figure is 200k — immaterial to the size arithmetic, which postings/posting_tags/entries dominate.)
- Measured file sizes: `bench-data/06e9620d6ec88418/oracle.sqlite` = 18,464,768 bytes and `bench-data/2ef4b3c64a82712c/oracle.sqlite` = 18,464,768 bytes, both verified S-scale (`SELECT COUNT(*) FROM posting` = 100,000). B-tree storage is ~linear in rows, so L extrapolates to ~1.8 GiB — roughly 2x the 1 GiB pin. Every oracle currently on disk (including all six scenario worlds, max 38 MB) sits far below 1 GiB, confirming the break engages only at L.
- `crates/bumbledb-bench/src/driver/bench.rs:299` — `budget_gates: cfg.scale == corpus_gen::Scale::L`, consumed at line 310 (`gates_ok`). `cli/parse.rs:33` accepts `"L"`.
- `crates/bumbledb/src/storage/env.rs:182` — `const MAP_SIZE: usize = 32 << 30;`, applied unconditionally at `storage/env/open_env.rs:34`. The engine's whole store is always mapped; the asymmetry is one-sided against SQLite.
- `open_for_bench.rs:35` runs `wal_checkpoint(TRUNCATE)` at open, so the WAL is empty and the main-file mmap is the actual read path up to the 1 GiB boundary — beyond it, SQLite falls back to `pread` into the 256 MiB page cache (`corpus.rs:90`, `cache_size=-262144`) with a copy per page.
- Spec check: `docs/architecture/60-validation.md` pins the parity config as "WAL, `synchronous=FULL`, 256 MiB cache, 1 GiB mmap, `wal_autocheckpoint=0`..." (§ scenario parity config, ~line 510) — the constant is doc-blessed, so the doc and code agree with each other while jointly contradicting the doc's own gating protocol (§ protocol, line 273: the budget binds only at L). The fix must update both.

### Bench impact

A gated L-scale run with a ~1.8 GiB oracle: SQLite's scans and joins over the un-mmapped ~0.8 GiB tail pay buffer-cache copies the engine never pays (LMDB maps everything), inflating SQLite's p50/p99 on the large-scan families and flattering the ALL-WIN ratios at exactly the scale the 10 ms p99 budget gates on. Nothing records or asserts mmap coverage, so the distortion would be invisible in `report.json` and would pass `FairnessCheck` clean. Direction of error: always in the engine's favor — a bench-honesty violation of the project's own "honest opponent" protocol (60-validation.md § protocol), even though no currently published number is affected.

### Suggested fix

1. Derive `mmap_size` in `open_for_bench` from the oracle file's byte size (stat the path, round up to a page boundary with headroom for WAL growth), instead of the 1 GiB literal.
2. Assert against SQLite's compile-time mmap ceiling (`SQLITE_MAX_MMAP_SIZE`, default ~2 GiB / 0x7fff0000 in the bundled build): `PRAGMA mmap_size` returns the value actually in effect, so read it back and fail loudly if it is less than the file size — never silently truncate. At ~1.8 GiB the L oracle still fits under the default ceiling, but only barely; a larger future scale would need `-DSQLITE_MAX_MMAP_SIZE` raised in the libsqlite3-sys build.
3. Add the readback to `FairnessCheck::run_with` as a sixth rule: `PRAGMA mmap_size >= file_bytes` (stat via `conn.path()`), making the memory-residency parity claim a checked invariant like WAL, synchronous, fullfsync, indexes, and ANALYZE.
4. Update the parity-config paragraph in `docs/architecture/60-validation.md` (and the mirrored pragma list in `duralane.rs:112` / `lanes/curves.rs:40`) from "1 GiB mmap" to "full-file mmap, coverage asserted".
