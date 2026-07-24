## The root README's Rust quickstart is the one unpinned code surface in a doc estate built on fence pins

category: missing-free-feature | severity: low | verdict: CONFIRMED | finder: r2:docs-vs-code-drift

### Summary

Every other code-bearing document in the repo is machine-pinned against the real API surface, and the estate's own comments record why (the TS README's examples "once rotted all the way back to a deleted API"). The repo-root README — the highest-traffic document — carries two ```rust fences (the front-page quickstart and the closed-relation payload example) that no test reads, extracts, compiles, or token-pins. The same file already exhibits live unwatched drift: it advertises "twenty-nine worked schemas" in the cookbook while the cookbook contains thirty numbered recipes. All machinery for the free pin already exists in-tree (the cookbook.rs fence-extraction + duplicate-and-token-pin test).

### Evidence

- **The unpinned fences.** `README.md:19-72` — the quickstart: `schema!` block with `closed relation`/`fresh`/containment statements, `Db::create`, the `db.write` closure (`tx.alloc`, `tx.insert`), and the `query!`/`prepare`/`execute` flow. `README.md:420-434` — the two-tier `closed relation` payload example (`Kind as KindId { mastered, rank }` plus ψ-selection containments).
- **No pin exists.** A repo-wide grep for `README` across `.rs`/`.ts`/`.sh`/`.toml` yields only doc comments, `scripts/lean.sh` (which concerns `lean/README.md`), and the ts tests. `ts/test/readme.test.ts:21-22` resolves `readmePath = path.join(packageRoot, "README.md")` with `packageRoot = ts/` — it pins `ts/README.md` only. No crate carries `#![doc = include_str!(...README...)]`, so rustdoc doctests never touch these fences either (verified by grep over `crates/` and `ts/crate/`).
- **Every sibling doc is pinned.** `ts/test/readme.test.ts:2-11` (compile pin over `ts/README.md`, with the rot-history comment quoted above); `ts/test/cookbook-doc.test.ts:31,73` (compile pin over `ts/COOKBOOK.md`, section by section); `crates/bumbledb-query/tests/cookbook.rs:29` (`include_str!("../../../docs/cookbook.md")`) with `doc_blocks_match_the_compiled_copies` at `cookbook.rs:918` slicing the doc's rust fences and token-pinning them against compiled duplicates.
- **The drift is already live.** `README.md:526` says the cookbook holds "twenty-nine worked schemas"; `docs/cookbook.md` has thirty numbered recipes (`## 30. The keyed read` at `docs/cookbook.md:1468`). The root README demonstrably rots unwatched today.

### Failure scenario

A surface change — a `query!` notation change, a `prepare`/`execute` signature move, a `schema!` grammar edit (all of which have happened across 0.3.0 → 0.6.0) — silently breaks the project's front-page quickstart while every lower-traffic document fails its gate. The estate's own recorded history (`readme.test.ts:2-4`) says this exact failure mode is what the pins were built to prevent; the front page is the one place the prevention was not applied.

### Suggested fix

Add a `#[test]` in the `cookbook.rs` style (e.g. in `crates/bumbledb-query/tests/`) that `include_str!`s the root `README.md`, slices its ```rust fences, and token-pins each against a compiled duplicate living in the test. Note the fences are deliberate fragments — the 420-434 block is `schema!`-interior syntax and the quickstart uses unbound `path`/`params`/`results` — so direct fence compilation will not work; the duplicate-and-token-pin pattern of `doc_blocks_match_the_compiled_copies` (`cookbook.rs:918`) is the right mechanism and its extraction/normalization helpers can be reused. While there, the pin naturally hosts the "twenty-nine" count assertion against the cookbook's actual recipe count.
