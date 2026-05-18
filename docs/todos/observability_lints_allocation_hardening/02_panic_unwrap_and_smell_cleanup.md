# 02: Panic, Unwrap, And Smell Cleanup

**Goal**
- Remove panic-oriented control flow and obvious allocation/code smells exposed by the lint policy.
- Keep behavior equivalent while making failure modes typed and observable.

**Current Production Sites To Address**
- `crates/bumbledb-lmdb/src/sorted_trie.rs`: `SortedTrieIter::key` currently calls `expect("sorted trie key requested before open")`.
- `crates/bumbledb-lmdb/src/hash_trie.rs`: `insert_row` currently uses `unreachable!()` after replacing a node with `HashNode::Inner`.
- `crates/bumbledb-lmdb/src/query.rs`: aggregate finish currently uses `key_iter.next().unwrap()` and `state_iter.next().unwrap()`.
- `crates/bumbledb-lmdb/src/query.rs`: `build_hash_atom_indexes` has `let _ = query`, which should not survive `unused = "deny"` cleanup.
- `crates/bumbledb-bench/src/main.rs`: CLI parsing uses `expect` and `panic!` for missing values, numeric parse failures, unknown `--format`, and unknown args.

**Required Runtime API Changes**
- Change trie key access so misuse is represented as `Option` or `Result`, not panic.
- Update `LinearIter::key`, `key_owned`, `LeapfrogState::key`, and LFTJ call sites consistently.
- Ensure LFTJ still increments `trie_key_reads` only when a key is actually read.
- Rewrite hash trie insertion to avoid impossible-state `unreachable!()` by using explicit match/control flow.
- Replace aggregate finish unwraps with typed internal errors that include enough structural context to diagnose malformed aggregate state.
- Remove unused parameters or rename intentionally unused parameters with a leading underscore only when the parameter must stay for trait/API shape.
- Do not hide unused values with `let _ = value` except for intentional ignored `fmt::Result` from writing into `String`, and prefer a helper if Clippy requires it.

**Required Test Cleanup**
- Convert crate tests that use `unwrap`/`expect` into `fn test_name() -> Result<()>` or `fn test_name() -> std::result::Result<(), Box<dyn std::error::Error>>`.
- Use `?` for fallible setup such as `tempfile::tempdir`, `Environment::open`, `StorageSchema::new`, `parse_and_typecheck`, writes, reads, and query execution.
- For `Option` assertions, use `assert_eq!(option, Some(value))`, `assert!(option.is_some())`, or `ok_or_else(...)` with a typed test error.
- Keep assert macros for invariant checks; `assert!` and `assert_eq!` are not the target smell.
- Avoid changing test datasets unless a test was relying on panic behavior.

**Required Benchmark CLI Cleanup**
- Change `Config::from_env()` to return `Result<Config, CliError>` or equivalent.
- Add typed errors for missing option values, invalid numeric values, invalid output format, unknown args, and no matching dataset.
- Preserve `--help` behavior without `panic!`; returning a help sentinel or printing help and returning `Ok(None)` is acceptable.
- Keep the benchmark binary simple. Do not add a full CLI framework unless the implementation stays small and improves error quality.

**Allocation Smells Allowed In This PRD**
- Remove only obvious accidental allocations directly touched by panic cleanup.
- Do not attempt the full stack/GAT hot-path refactor here; that belongs to `06_stack_gat_and_hot_path_allocation_cleanup.md`.
- Do not replace projection/aggregation set semantics in this PRD.

**Passing Requirements**
- The deny lint policy from PRD 01 passes without broad exceptions.
- Production code has no `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `unreachable!`, or `dbg!` sites.
- Tests have no `unwrap`/`expect` except any explicitly documented temporary exception approved by PRD 01 rules.
- Benchmark CLI invalid input returns a useful error message instead of panicking.
- Existing query result tests still pass.
- Focused benchmark smoke shows no unexplained regression above 5% from baseline.

**Stop Conditions**
- Stop if changing trie key APIs expands into a broad executor rewrite.
- Stop if aggregate unwrap cleanup reveals a real invariant bug; add a focused regression test before continuing.
- Stop if benchmark CLI cleanup makes scripts incompatible with existing documented commands.
