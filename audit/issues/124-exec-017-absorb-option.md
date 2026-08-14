# exec-017: `PipeTables.absorb: Option<usize>` is root-skip vs node-absorb as a hole

- **Severity:** medium
- **Tree:** exec
- **Status:** FIXED(44db7ad6)
- **Source:** audit/plan-exec.md (validation add; F7's `PipeTables` product)
- **Depends on:** exec-004 (same `PipeTables`; land with or after); exec-010 (Root-absorb writes the drive stop)

## The bug

`crates/bumbledb/src/exec/run.rs:693-699` and `run/pipe_tables.rs:45-52`:

```rust
/// `Some(N-1)` (the leaf itself) means skips never cross a node;
/// `None` means a skip ends the whole execution.
absorb: Option<usize>,
```

Construction already parsed the plan's `SuffixSkip` roster:

```rust
let absorb = (0..n_nodes)
    .rev()
    .find(|&m| plan.nodes()[m].suffix_skip == SuffixSkip::Forbidden);
```

`None` = every node `Licensed` (a leaf skip ends the execution). `Some(a)` = deepest `Forbidden` node. `probe_pass.rs:667-670` re-interprets the hole:

```rust
match tables.absorb {
    Some(a) if node_idx >= a => self.cancel_origin(origin),
    Some(_) => {}
    None => self.all_cancelled = true,
}
```

Option-as-tag. Representable: `Some` past `n_nodes`, or `None` on a plan that still has a `Forbidden` node (construction would not mint that today; the type would).

## Why it's wrong

Insight 6: construction walked `SuffixSkip` and stored a hole; the skip path re-decodes "virtual root vs node" from `Option`. Insight 4: `None` meaning "root" is Hoare's null as a third node index. The two essential answers — cancel an origin at/below the absorb node, or stop the whole execution — are a sum. exec-010's `DriveState::SkipDone` is what the `None` arm writes; this issue is the *which absorb* coordinate, not the stop product.

## The fix

Per `audit/CONTRACT.md` §C1 (trusted layer is a sum):

```rust
enum SkipAbsorb {
    /// Every node is Licensed — a leaf skip ends the execution.
    Root,
    /// Deepest Forbidden node; skip at/below it cancels that origin.
    Node(usize),
}
```

`PipeTables::of` matches `find` → `Node(m)` / miss → `Root`. `probe_pass` matches `Root` vs `Node(a)`. No `Option`. D2 policy unchanged (when skip is legal, which origin dies, skip-is-an-answer).

## Acceptance criteria

- [ ] Gone: `rg -n 'absorb: Option' crates/bumbledb/src/exec` → no matches; `rg -n 'tables\.absorb' crates/bumbledb/src/exec` still matches, but the type is `SkipAbsorb`.
- [ ] Unchanged tests: D2 skip / pipeline suites green; a Licensed-only plan still ends the execution on a leaf skip; a Forbidden absorb node still cancels only that origin. Zero assertion edits.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Observably identical cancellation. Do not merge this into exec-010's `DriveState` (that is Running / SkipDone / Poisoned — *that* a stop happened, and why). Do not treat `Root` as `Node(usize::MAX)` or another sentinel. Land with exec-004 if `Drive::Pipeline(PipeTables)` is being reshaped anyway.
