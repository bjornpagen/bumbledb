# 06: Optimize LFTJ Inner Loop

**Goal**
- Reduce sorted-trie key reads, seeks, participant allocations, and per-candidate byte allocations in broad LFTJ joins.

**Trace Evidence**
| Query | Avg | Candidates | `trie_seek` | `trie_key_reads` |
|---|---:|---:|---:|---:|
| `joinstress/triangle_count` | `67.3ms` | `90,000` | `119,995` | `589,992` |
| `sailors/high_rating_red_boats` | `70.3ms` | `10,002` | `100,586` | `237,830` |
| `sailors/red_boat_sailors` | `57.6ms` | `34,153` | `17,491` | `105,789` |

**Current Hot Operations**
- `participants(variable)` scans atom-variable lists and allocates a new `Vec` per recursion call.
- `LeapfrogState::new(participants.clone())` clones that vector.
- `LeapfrogState::init/search/key` repeatedly calls `key_owned`.
- `key_owned` copies `EncodedRef` into `EncodedOwned` and increments `trie_key_reads`.
- Candidate binding allocates `Vec<u8>` via `value.as_bytes().to_vec()`.
- Comparison operands clone bound `Vec<u8>` values.

**Required Design**
- Precompute `participants_by_variable` in `LftjRuntime`.
- Add arity-specialized execution paths:
  - arity 1: direct iterate, no `LeapfrogState`
  - arity 2: directed binary intersection
  - arity 3+: cached-key leapfrog
- Cache current keys inside `LeapfrogState` and refresh only after `next`/`seek`.
- Change `EncodedValue` from `Vec<u8>` to inline `EncodedOwned`.

**Implementation Steps**
1. Add `participants_by_variable` to `LftjRuntime`.
2. Remove recursive participant vector allocation.
3. Add unary depth fast path.
4. Add binary intersection fast path choosing smaller current frame as driver.
5. Add key cache to `LeapfrogState`.
6. Refactor `EncodedValue` to own `EncodedOwned` and expose `as_bytes()`.
7. Remove comparison byte clones for variable operands.
8. Add counters for intersection arity and cached key refreshes if useful.

**Tests**
- Unary path emits same candidates.
- Binary intersection handles overlap, no overlap, one side exhausted, duplicate row ranges.
- Cached-key arity 3+ path matches current output.
- `EncodedBinding` works with inline `EncodedOwned`.
- Existing differential tests pass.

**Acceptance Criteria**
- No per-candidate `Vec<u8>` allocation in candidate binding.
- No per-depth participant `Vec` allocation in recursive execution.
- Scale-10000 `triangle_count` reduces `trie_key_reads` at least 35% from `589,992`.
- Scale-10000 `high_rating_red_boats` reduces `trie_seek` at least 25% from `100,586` and `trie_key_reads` at least 35% from `237,830`.
- Result counts, materialization, dictionary lookup counters remain correct.

**Risks**
- Borrowed-key caching can create lifetime complexity; use cached `EncodedOwned` first.
- Binary intersection heuristics can alter seek/next ratios; correctness comes first.
- Counter reductions must reflect real fewer key reads, not redefined counters.
