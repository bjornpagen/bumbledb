# Free Join Purge PRD Suite

This directory is the ordered deletion-and-unification contract for the remaining code the Free Join paper would reject.

The prior set-engine PRD suite documents the whole semantic rebase. This suite is narrower and harsher: it names the hybrid execution structures that must die, the exact replacement path, and the strict gates that prevent a half-deleted system.

## Thesis

Free Join is not "LFTJ plus special cases" and it is not "direct kernels beside a generic join engine".

The paper's core claim is that the system should unify the traditional iteration/probe model and WCOJ-style value intersection under one physical plan and one access abstraction. Bumbledb still has sidecar direct execution, sidecar hash tries, and pre-plan proof machinery that are not represented as Free Join nodes. These structures must either become Free Join implementations or be deleted.

## Non-Negotiable Rules

- No compatibility modes.
- No hidden old execution option.
- No fake optimizer candidates.
- No pre-planning direct execution bypass.
- No sidecar execution family that cannot be expressed in `FreeJoinPlan`.
- No benchmark timing for a path that was not correctness-checked.
- No result-set correctness hidden behind final duplicate cleanup if execution is known to overproduce.
- No new standalone hash/trie/proof abstraction unless it is wired through Free Join node execution.

## Global Validation Gate

Every PRD that changes Rust must pass:

```text
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
```

Every PRD that changes query execution must also pass:

```text
cargo test -p bumbledb-test-support --test golden_examples --all-features
cargo test -p bumbledb-test-support --test property_and_differential --all-features
cargo test -p bumbledb-test-support --test sqlite_comparison --all-features
```

## Mandatory Source Hygiene Gate

After each PRD, run source grep checks proving the targeted deleted names are absent from Rust source unless the PRD explicitly allows a temporary reference.

The suite-level final source gate must have zero Rust matches for:

```text
DirectKernel
DirectKernelKind
DirectKernelSummary
DirectPrefixRange
DirectChain
direct_kernel
direct_chain
direct_prefix
IndexNestedLoop
PlanFamily::Direct
PlanFamily::IndexNestedLoop
HashTrieIndex
HashTrieKey
LeafMode
PrefixProbe
PrefixFacts
query_access
static_semijoin
static_empty_fast
StaticProof
```

If a name remains because the replacement intentionally keeps the concept under a new Free Join abstraction, it must be renamed and documented in that PRD. The old name still dies.

## Ordered PRDs

1. `01-delete-direct-kernel-selection.md`
2. `02-delete-direct-chain-and-prefix-runtime.md`
3. `03-delete-hash-trie-sidecar.md`
4. `04-collapse-static-proof-into-free-join.md`
5. `05-delete-static-proof-caches-and-counters.md`
6. `06-replace-eager-temp-trie-builds.md`
7. `07-rebase-optimizer-and-plan-families.md`
8. `08-final-source-and-benchmark-purge.md`

## Final Done Definition

- `FreeJoinPlan` is the only join execution plan authority.
- All join execution implementations are node implementations or access implementations selected by Free Join planning.
- Static-empty behavior is either a Free Join semijoin/proof node or gone.
- Hash access is either a GHT/COLT implementation selected by Free Join or gone.
- No direct execution family remains.
- No benchmark, explain output, counter, or public enum references deleted sidecar names.
- Full validation and source hygiene gates are green.
