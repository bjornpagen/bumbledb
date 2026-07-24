## BatchToken catches force-staleness by representation but reset-staleness only by luck

category: missing-free-feature | severity: low | verdict: CONFIRMED | finder: r2:concurrency-unsafe-ffi
outcome: fixed b5e130d6

### Summary

`BatchToken`'s bit-63 tag exists to make one class of illegal state loudly unrepresentable: a resume token minted under one node state and presented after that state changed. It fully covers the **force** axis (positions token → node forced → release assert fires, with a test pinning the behavior). It covers the **reset** axis not at all: `Colt::reset` truncates every pool and re-mints node/chunk/map indices, and a token that crosses that boundary — in either kind — passes the asserts and is silently reinterpreted against the new generation's state. This is precisely the "silent-omission wrong-results class" the tag's own doc comment says the design refuses (crates/bumbledb/src/exec/colt.rs:105-109), guarded on one axis and open on the other.

### Evidence (all verified against source)

- **The tag and its stated purpose** — crates/bumbledb/src/exec/colt.rs:105-123: "Bit 63 tags every nonzero token with the node state that minted it ... caught by a release assert instead of being silently reinterpreted as a dense index — the silent-omission wrong-results class." One axis (node state) is encoded; generation is not.
- **Force axis guarded and tested** — crates/bumbledb/src/exec/colt/iter.rs:196-199 asserts `token.0 == 0 || token.0 & DENSE_TOKEN_TAG != 0`; crates/bumbledb/src/exec/colt/tests/dense.rs:131-165 (`a_token_that_outlives_a_force_is_refused`) verifies the panic message and recovery. There is no reset counterpart test.
- **Reset re-mints indices** — crates/bumbledb/src/exec/colt/new.rs:65-78: `reset` clears `nodes`/`chunks`/`maps`/`ctrl`/`buckets`/`dense` retaining capacity and pushes a fresh `NodeState::Unforced(Positions::Root)` at index 0. Pool slots are reused across generations. No epoch/generation counter exists anywhere in the colt module (grepped).
- **Untagged pre-reset token sails through** — crates/bumbledb/src/exec/colt/iter.rs:106-109: the comment "a stale token from before a reset lands here too" is attached to `assert!(token.0 & DENSE_TOKEN_TAG == 0)`, which by construction cannot fire for an untagged positions token. Then:
  - Root arm (iter.rs:112-127): `take = max.min(self.view.len().saturating_sub(index))` against the **new** view — silent truncation or silent empty yield.
  - Chunks arm (iter.rs:129-172): the packed `(chunk+2, offset)` indexes `self.chunks[chunk]` in the new generation's pool — silently yields another node's positions when the slot is occupied; panics on bounds only by accident of pool size.
- **The gap is symmetric** (stronger than the original finding): a dense-tagged pre-reset token presented to `iter_map` after reset also passes iter.rs:196-199 and walks the new map's dense list (iter.rs:200-223) — silent cross-map yield.
- **Not a live bug today** — both executor call sites mint tokens locally inside a drain loop (crates/bumbledb/src/exec/run/run_node.rs:140, crates/bumbledb/src/exec/run/pump.rs:128), and `Colt::reset` is called only from api/prepared (fixpoint.rs:569, run_join.rs:117,157). But docs/architecture/40-execution.md:443 confirms a per-round `Colt::reset` in the fixpoint loop, so the boundary recurs many times per execution — the class the tag was built for.
- **Spec check**: the Free Join paper (docs/free-join-paper, §4.2 COLT) specifies the lazy trie but says nothing about resume tokens; the token machinery is this codebase's own addition, so its governing spec is the colt.rs doc comment — which declares loud-over-silent as intent for exactly this staleness class.

### Correction to the original finding

The claim that the token has "30+ unused high bits below the tag" is wrong for the chunked packed form: `(chunk + 2) << 32 | offset` (iter.rs:165) occupies bits 32-62 with the chunk field. However, the mint-site comment (iter.rs:166-169) already establishes the physical bound: ~5×10⁸ positions at 32 GiB → ≈2²³ chunks, so bits 56-62 are free **in practice** and the existing `debug_assert_eq!(packed & DENSE_TOKEN_TAG, 0)` pattern extends naturally to a chunk-bound assert. The suggested fix stands with that adjustment.

### Failure scenario

Any future driver/executor change that carries a `BatchToken` across a fixpoint round boundary or a prepared-query re-execution (e.g. a resumable/incremental drain, a suspended pipeline) produces silently truncated batches (Root arm) or cross-node position yields (Chunks arm, dense arm) — wrong join results with no assert — while the *identical* mistake across a force is caught loudly today, and the doc comment claims the class is closed. (`Cursor::Node(NodeRef)` has the same unguarded reset-staleness, though current code also re-derives cursors per generation.)

### Suggested fix

Add a small reset-epoch counter to `Colt`, incremented in `reset` (new.rs:65). Mint it into a few of bits 56-62 of every nonzero token (all three mint sites: iter.rs:127, iter.rs:165-171, iter.rs:223) and assert equality on presentation next to the existing tag asserts (iter.rs:109, iter.rs:196-199). Add a mint-site assert that the chunk index stays below the epoch field (well above the physical bound). Cost: one and+compare per batch, same class as the existing tag assert ("once per batch: noise" — iter.rs:195). A companion test mirroring `a_token_that_outlives_a_force_is_refused` for the reset axis closes the loop.
