# Query runtime, set answers, and per-tenant execution limits

Audit date: 2026-09-04. Scope: query validation/normalization, prepared execution, aggregate finalization, derived relations, view reuse, and embedding constraints. No production implementation was changed.

## Executive judgment

The query system has unusually strong internal structure: a validated IR, explicit set-valued answers, scalar and temporal type distinctions, DNF normalization, a Free Join execution core, separate projection/aggregate sinks, and bounded-language linear recursion. The next step is not to make this “more like SQL.” It is to preserve those semantics while making failure, resource use, and hosting behavior equally explicit.

Two problems stood out: a failed aggregate execution leaves valid-looking partial answers, and the supposed derived-tuple budget is checked after the expensive materialization it is supposed to constrain. Combined with no public execution cancellation or usable budget setter, a semantically lawful query can still be operationally unacceptable in a multi-tenant application process.

## Finding inventory

| ID | Priority | Kind | Confidence | Finding |
|---|---|---|---|---|
| QRY-001 | P2 | Error-result correctness | Reproduced | Aggregate failure leaves partial results in caller-owned `Answers` |
| QRY-002 | P1 for shared hosted workers | Resource isolation / architecture | Confirmed static | Derived limits are post-materialization checks, with no general execution resource boundary |
| QRY-003 | P2 | API / documentation mismatch | Confirmed static | Documented host-settable query budgets have no public setter |

## QRY-001 — An aggregate error leaves partial answers visible

**Evidence:** `crates/bumbledb/src/api/prepared/finalize.rs:20-35`; `crates/bumbledb/src/exec/sink/aggregate/finalize.rs:19-35`, `42-71`; `crates/bumbledb/src/api/prepared/execute.rs:28-38`, `87-107`; current regression test `api/prepared/tests/answers.rs:6-81`.

Projection finalization remembers its starting output length and truncates cells on failure. Aggregate finalization has no equivalent rollback. It iterates groups, finalizes each group, and appends it immediately. If a later group overflows, the error escapes after earlier rows have already been written into the caller's `Answers`.

**Reproduction:** insert 100 one-row groups whose sum is `1`, plus a group containing `i64::MAX` and `1`. Run a grouped signed sum into a caller-owned `Answers` using normal `ReadInstance::execute`:

```text
aggregate result=Err(Overflow(Aggregate { find: FindIndex(1) })) leftover_rows=100
```

The values left in the buffer depend on group iteration order. Re-executing does clear the buffer first, so buffer *reusability* is intact; that is what the existing overflow test verifies. Reusability is not the same guarantee as an empty result on error. The finalization comment suggesting “never a partial result” does not cover this branch.

**Impact:** code that retains or displays the output buffer after a failed request can observe a plausible but incomplete set. A returned `Err` must still be honored; this is not a claim that `execute_collect` returns `Ok` with wrong rows. It is an inconsistent and hazardous out-parameter contract, especially for UI adapters, pooled buffers, and generic wrappers.

**Recommendation:** decide one output contract for all errors. Prefer clearing all logical output on failure, including text/blob heaps as appropriate, while retaining capacity. Alternatively leave the prior successful output untouched via staged publication. Apply it consistently to aggregates, projection decoding, point probes, bind failures, and foreign-query errors. Today identity checking occurs before `out.begin`, while bind failure occurs after it, so different error families already preserve/clear old output differently.

**Regression requirements:** assert exact output state after grouped overflow with valid groups before the failing one; failure during string decoding after earlier groups; point-probe decode failure; foreign prepared query; failed bind; success/error/success reuse. Retain tests of the reusable-capacity property separately.

## QRY-002 — Derived-tuple budgets do not bound the work or allocation of a query

**Evidence:** `crates/bumbledb/src/api/prepared/reach.rs:206-245`, `346-380`; `crates/bumbledb/src/exec/sink/projection/sink.rs:6-14`, `47-103`, `124-137`; `crates/bumbledb/src/api/db/mutation_core.rs:363-399`; public execution interfaces in `api/db/read_instance.rs:48-75` and `api/db/owned.rs:83-110`.

For an interior relation, every rule executes fully before `sink.len()` is compared with the tuple budget. The recursive base likewise materializes fully before the driver checks its tuple limit. Each recursive step can overshoot the limit by an entire round. Ordinary main-query projection and aggregate sinks have no equivalent output or byte budget.

The sinks' inserts and scan materialization do not consult a budget. There is also no host cancellation token, deadline, or general work budget on the public execute call. `exec/run/cancel.rs` handles internal provenance/subtree cancellation; its name must not be mistaken for user-request cancellation.

**Why this is concrete:** a normal join whose output exceeds the configured tuple count still allocates and computes that output before returning `DerivedBudgetExceeded`. The postcondition detects excess; it does not cap peak memory. A tuple count also does not cover wide rows, retained dedup tables, intermediate join tables, image construction, text output, or total work when the final result is tiny. We did not run an OOM-inducing experiment on the user's machine; control flow suffices to establish this limitation.

**Impact:** an application query can monopolize a tenant worker or exhaust a process serving several tenants. A caller-side future timeout does not stop synchronous engine work. Even a logically terminating query can be too large to execute. The current 10,000,000 tuple default is not a memory guarantee; ten million narrow two-word rows alone represent 160 MB of raw row payload before index/table/allocator overhead.

Write ingestion has the analogous design pressure: an arbitrary input iterator is first collected, then all encoded rows are retained before application. Parse-all-first preserves batch atomicity but requires explicit transaction-size admission. It should not be silently replaced by a partial-write streaming loop.

**Recommendation:** introduce an execution context with a deadline/cancellation mechanism and explicit budgets for work, live memory, derived/output tuples, and output bytes. Charge before growth or at bounded batch points, not only after a phase completes. Preserve exact set semantics: budget exhaustion returns a typed failure, never an undisclosed truncated set. A separate explicitly partial/paged API can be designed later if needed. Until engine-level enforcement exists, isolated bounded workers are the safest hosted deployment boundary.

**Regression requirements:** deliberately tiny configured budgets with small fixtures; assert bounded overshoot and no partial results; cancel at image build, hash-build, ordinary joins, recursive base, recursive rounds, aggregate finalization, and parameter-set bind; success after cancellation with reused prepared state. Measure resident bytes at failure, not merely the returned error. Add fair-concurrency tests demonstrating a slow tenant does not indefinitely block unrelated tenants.

## QRY-003 — Query budgets described as host-settable are not settable by the host

**Evidence:** `crates/bumbledb/src/api/prepared.rs:300-303`; `crates/bumbledb/src/api/prepared/build.rs:238-247`; `crates/bumbledb/src/api/prepared/reach.rs:23-29`; `crates/bumbledb/src/api/prepared/bind.rs:14-25`.

`tuples_budget` is a private field whose documentation says “Host-settable on every prepared query.” The build path initializes it to 10,000,000. The recursive-round budget is fixed at 65,536. Inspection of the public prepared-query methods found `set_batch_size`, but no tuple-budget or round-budget setter, execution-options parameter, or corresponding constructor option.

**Impact:** applications cannot lower limits for interactive requests or intentionally raise them for controlled offline work. A long narrow graph can exceed the default round limit despite acceptable memory, while a wide intermediate can remain unsafe despite the large tuple default. The comment promises control that embedding code cannot use.

**Recommendation:** expose deliberate execution policy, preferably as the same execution-context design used for QRY-002. At minimum align public API, comments, bindings, and tests about whether these limits are fixed product constraints or host choices. Do not expose a setter that falsely implies the current checks bound memory.

**Regression requirements:** downstream compile/run tests configure both limits; the same query succeeds/fails at expected boundaries; every supported binding expresses the same policy; introspection reports effective limits. Test a long chain separately from a high-cardinality shallow recursion.

## Architecture review: taking the philosophy to its logical conclusion

### A set-semantic application database should make multiplicity visible in the query, not implicit in execution

Projection deduplication, union deduplication, and aggregation over the binding vocabulary are load-bearing semantics. The execution engine legitimately removes work when it can prove that suffix multiplicity cannot affect projected results. Keep those optimization proofs tied to the exact query semantics. Avoid adding row-bag behavior to fix application misunderstandings; make identity variables and aggregate input distinctions clear in examples and diagnostics instead.

The review did not establish an aggregate multiplicity bug beyond QRY-001's error-output behavior. Existing separate tests for union, DNF, projection, folded execution, negation, and temporal packing are valuable. Future tests should cross these features rather than only extend isolated feature examples.

### Compile-time schema typing and runtime catalog identity solve different problems

The schema parameter rejects many cross-schema mistakes at compile time. Runtime identity rejects a prepared query used against another database, including another tenant with the same schema. This is good isolation, not an accidental obstacle to reuse. If many-tenant planning overhead becomes material, separate a schema-level immutable query template from a per-catalog execution object; do not remove identity checks to share mutable plans and view caches.

### Warm-query optimization needs a lifetime and memory policy

Prepared objects retain view/index state and pools for reuse, while plans deliberately pin prepare-time statistics rather than replan on writes (`api/prepared.rs:1-9`). The identity and generation mechanisms inspected were coherent, and no stale-answer defect was demonstrated. Operationally, however, a tenant registry containing many long-lived prepared objects is also a memory-retention registry. The host needs retained-byte accounting, pressure-based disposal, and a policy for re-preparing materially changed data distributions. The performance companion report covers costs and benchmarking evidence.

### Bounded-language recursion is valuable but is not resource isolation

Linear recursion without arbitrary value invention gives a finite, comprehensible semantic model. Its round/tuple guards are a useful start. Completion in that model is different from acceptable latency, memory, and fairness in a hosted service. Work cancellation and byte budgets should be treated as a separate operational layer, not as a weakening of relational truth.

### Plans, answers, and error outcomes deserve observable provenance

A strong application debugging surface should report the schema/catalog identity, effective execution limits, preparation generation/statistics age, selected fast path, and whether output was published. It should not require enabling benchmark-only instrumentation to answer basic operational questions. Keep the normal embedding API understandable; detailed diagnostics can remain opt-in without being absent.

## Inspected hypotheses not promoted to bugs

- A literal or string parameter interned in a later snapshot may be memoized and reused against an older snapshot. Intern IDs are catalog-local and never reused; an older snapshot cannot contain a fact referencing that later ID. The inspected equality/inequality and negation paths did not establish a wrong-answer case from this alone. Reclamation changes would require revisiting this reasoning.
- Negated occurrences deliberately keep constant equalities in filters rather than positive selection probes. The empty-selection path does not turn an empty negative relation into a failed positive join. No such polarity bug was found.
- Prepared views retain/reap generation-stamped state. We did not mistake cached old images held by active readers for a stale-data defect.
- Zero-rule and statically empty paths are intentional IR outcomes. Dead main queries can still evaluate live interiors; that is potentially wasted work, not evidence that an empty main returns nonempty answers.
- The existence of many invariant `expect` calls is not itself a bug. ENG-001/ENG-002 identify actual creation-boundary holes; the rest require a reachable violated invariant before escalation.

## Test record and next acceptance gate

QRY-001 was reproduced in the external path-dependent Rust harness described in `20-engine-semantics.md`. QRY-002 and QRY-003 are source-confirmed; no unbounded query was run to exhaustion.

A useful next gate is a small cross-product test suite covering: ordinary versus heap-backed catalogs; scalar versus interval/fixed-byte/string data; projection versus grouped aggregate versus recursion; parameters versus literals; error versus success; and repeated prepared execution before/after writes. Every error case should assert both the error and the state of reusable output/execution buffers. Every resource-limit case should assert the peak resource envelope. Those tests turn the philosophy into executable obligations without expanding the query language.
