# 13 — Proofs, Rust, and a release that earns its claims

Status: **successor obligations, not completed implementation/proofs/tests**. This chapter selects what to retain and what to stop treating as permanent doctrine. The root release-gate document owns the complete product matrix; the stable gate labels here and in 10–12 are inputs to it, not a competing checklist.

## 1. The proof program should help the program get smaller

Keep Lean for the semantic and algorithmic claims where a counterexample would change the product. Keep Rust for the real engine. Remove the idea that historical formalization choices are untouchable: an old theorem about the wrong transition is not more valuable than a small replacement theorem about the one that actually runs.

This does not expand the language-support matrix. The core has public Rust/TypeScript surfaces; **the public C API, header and artifact matrix are dropped**. The log has a public TypeScript surface and one internal Rust machine. Qualification tests the internal machine and actual public surfaces, not a legacy SDK retained without a consumer. Native Node/LMDB boundaries still require lifetime/ABI tests; removing C does not remove unsafe foreign implementation dependencies.

The attachment's representation principle is useful here: a constructor should deliver the invariant its consumers need; a type should distinguish exclusive states; a bounded operation should own its resource allowance. It does not imply that Rust's type checker proves persistence, that a generic function is parametric in the presence of all Rust effects, or that half-open intervals eliminate every endpoint bug.

In particular:

- `CheckedDelta`, the native representation of the public `ChangeSet`, proves canonical typed input, not schema admission or remote durability; these names do not denote two change representations.
- `PreparedWrite` proves judgment of one private candidate at one core snapshot, not successful HEAD publication.
- `OwnedSnapshot` owns one read transaction, not an arbitrary collection of rows labeled with a convenient generation.
- A sealed `CompleteResult` proves completed query evaluation/storage for that attempt, not future transport infallibility.
- A numerical execution guard controls the supported thread's floating environment; it does not retroactively validate application arithmetic.

These boundaries should delete repeated checks and independent state machines. If implementing one requires a large family of adapters with identical switches, revisit the representation before calling that scaffolding rigor.

## 2. Replace the old constitutional constraints selectively

| Existing rule/assumption | Successor decision | Why |
| --- | --- | --- |
| Lean is the sole place semantic explanations may be written; prose is token-policed | One versioned executable specification plus concise human-readable normative contract and linked examples | Operators/users must understand the contract; symbol presence is not semantic consistency |
| “Verified Rust refused permanently” | No whole-Rust-verification claim for 1.0; targeted future refinement remains allowed | A permanent ban is not useful; an unearned verification claim is worse |
| No mathlib under any circumstance | Allow a tightly pinned, justified dependency if it removes home-grown arithmetic/set/proof machinery | Exact IEEE/rational rounding should not force a bespoke proof-library project; record actual build/trust costs |
| Existing mint theorem models every observable allocation | Delete core mint and log FreshRef; application owns 128-bit entity values before sealing | Removes abort-burn, sequence-derived identity and fresh-result casework entirely |
| Braids are distributed publication lanes | Mutable relation support is an admission/planning hint; tenant history is ordered by the log | Do not cite an independence theorem as a causal/read-visibility guarantee |
| Hash equality is fact equality | Exact canonical bytes decide logical equality; hash selects candidates | Formal set equality and implementation equality now have a direct bridge |
| Every general query requires full images | Keep direct probes and Free Join/COLT as preferred paths; bounded cursor fallback is complete | Preserve fast application reads without making relation image size a database limit |
| Projection-only interiors and terminal-only aggregate/arithmetic outputs | Typed nonrecursive relation stages may expose aggregate/computed results to consumers; recursive cycle stays projection-only | Useful composition without value creation or aggregation in recursive feedback |
| Equivalent window/query spellings require independent client ban tables | One finite canonical typed representation with proved normalizations | Preserve meaning and diagnostics without making source spelling a semantic axiom |
| Floating arithmetic is assumed mathematically associative | Typed operation roster distinguishes scalar rounded ops from exact aggregates | Optimizer legality follows the actual numerical domain |

Retain the useful integer interval point-domain/ceiling model, compile-time/runtime schema checks, final-state admission, and finite recursive fragment. Add dense numeric `Interval<F64>` through the shared exact endpoint-order algebra, with its separate length semantics. Breaking permission is not a reason to replace Free Join or a working compact representation merely to reduce the number of fast-path names.

## 3. The formal objects to own

The proposed normative model has five small layers. These are required *model capabilities*, not a demand for five new Rust crates or a generic verification platform.

### A. Canonical values and typed relations

Define finite canonical values; exact equality; F64 quotient normalization; F64 total order; integer domains; UTF-8 byte equality; bounded fixed bytes; and nonempty integer/float interval domains. Prove encode/decode roundtrip, canonical encoding injectivity, float normalization idempotence and order embedding, and interval-constructor equivalence to the correct domain predicate. Float intervals have dense numeric denotation with exact rational endpoints and unbounded sentinels, not discrete representable-F64 point sets; `[-Infinity,-MAX_FINITE)` is the distinguishing fixture. Keep infinity-bound and nonfinite-membership policies explicit.

Explicitly distinguish canonical wire bytes from bounded physical index keys. The index model includes candidate collisions and full-byte disambiguation. No collision-resistance premise is needed for tuple equality; cryptographic external object addressing remains a separate trust assumption.

### B. Final-state admission

Define normalized net delta, its final candidate, and each admitted law. Prove the reference judge's soundness/completeness for the supported finite forms. Model diagnostic statement IDs separately from bounded example citations. A rejected physical landing must not remove a proposed fact from the judgment universe.

Retain the grouped capacity denotation: for each selected keyed parent, sum the exact nonnegative count/source-u64/bounded-integer-duration measure over distinct complete selected source facts with the matching **scalar** key. Prove its empty-child total is zero, its zero-weight-membership distinction, dimension/defined-duration premises and widened fold bounds. This is not pointwise temporal occupancy or a weighted relation algebra; interval projection, implicit weight joins and float duration weights are outside the selected law grammar. Harmless window aliases normalize with denotation and authored-diagnostic correspondence, not a ban on every alternate client spelling. Query expressions remain query values, not a new arbitrary schema-assertion language.

For incremental admission, retain explicit support/delta-restriction lemmas and abstract consultation costs where useful. Do not label a law mathematically impossible merely because the fastest current incremental enforcement plan does not handle it. The admission grammar, baseline algorithm, optimization eligibility, and host resource policy are different boundaries.

### C. Queries, errors, and resource interruption

Define the finite typed relation-expression denotation: set bindings/rows; union/projection/negation; group formation; exact integer and float aggregates; temporal packing; and positive finite-active-domain linear least fixed point. Nonrecursive derived nodes may expose aggregate/computed rows to downstream queries. State the input grain explicitly: an aggregate folds distinct complete input rows/bindings; prior projection can change that grain, naming cannot. No input binding creates no query group, unlike a schema capacity's existing-parent empty-child total. A pure reference evaluator should return either a complete set or the specified semantic error.

Replace the current Lean `Interior` projection-only premise where it describes nonrecursive composition; do not cite it as evidence for aggregate-derived stages. Model an acyclic graph with an optional single positive linear recursive node, not a general rule-program interpreter. Frozen finite predecessors, including computed/aggregate outputs, extend the recursive input active domain once; prove induction that projection-only recursive heads stay inside it. No aggregate, partial arithmetic or value creation occurs in the cyclic component, and no computed node depending on recursion feeds it back. The finite-domain premise concerns actual frozen input values, not where a name was spelled in source.

Specify stage error and rounding boundaries independently of physical materialization. A stage's total input predicates precede its partial output calculations; a downstream filter cannot suppress a required upstream error. Unreferenced definitions need not execute. Prove any permitted predicate pushdown, inlining, streaming fusion or reuse preserves both completed values and errors, including duplicate grain and aggregate finalization. A named expression does not require a complete intermediate RAM table or an exported result owner. Reusing the core AST/evaluator for generated migration plans must preserve these same boundaries rather than create a second expression semantics.

Resource exhaustion and cancellation are not an alternative truth value. State a partial-correctness theorem: any successful bounded execution equals the unlimited denotation; interruption returns no completed result. Prove a separate counted-work/allocation protocol property for the abstract executor. Actual RSS and filesystem latency remain measured environmental properties.

The preferred Free Join/COLT plan, direct-probe eligibility, cursor fallback, derived-stage composition, semi-naive frontier transition, scratch set operations, and rewrite/distinctness witnesses need equivalence statements. Preserve and requalify the existing checked distinct-binding witness: it removes real dedup work rather than proving a seen-table mandatory everywhere. Do not formalize LMDB page layouts or hundreds of SIMD instructions to prove the cursor enumerates the right bindings; isolate the abstract cursor contract, then test its concrete adapter.

### D. Float arithmetic and aggregation

Model canonical binary64 as bits with exact integer/rational interpretation for finite values. **Both sum and mean remain required.** Prove the canonical numerical sum-case merge table, 34-limb finite bound under the count limit, exact merge associativity/commutativity, final ties-to-even rounding, and mean's exact-rational denominator behavior. Prove integer widened sum bounds and final-range failure semantics too. Float interval Allen/pack/coverage proofs reuse exact endpoint order; bounded length rounds endpoint subtraction once, while unbounded measure and finite-result overflow are distinct. A numerical length is not a discrete point count and is not an approximate capacity law.

Basic rounded scalar operations need a clear independent specification. The implementation can use guarded hardware operations on qualified architectures; the bridge to those operations is differential/architectural evidence, not a theorem that the Rust compiler honors every hardware instruction assumption. Never prove reassociation of mathematical reals and cite it for rounded binary64 expressions.

Aggregate-derived outputs are finalized canonical scalars. A consumer never secretly receives the producer's exact accumulator. Prove/negatively test subgroup-rounding and mean-of-means boundaries: inlining does not license substituting one global reduction. Share an exact total/count only when the same stage and distinct input/argument justify it. Frozen computed float values may enter a recursive input domain as ordinary values; numerical work is not repeated through the recursive cycle.

### E. History and lifecycle belong to the log/bindings model

The core models transaction commit/abort and owned snapshots. The log separately models decision publication, unknown outcomes, state witnesses, receipt identity and retention roots. SDK lifecycle models capabilities and borrow closure. Their shared boundaries are canonical delta, same-transaction attachment, and commit/abort/snapshot capabilities—not an interleaved giant “database state” theory.

Core old `Txn/Fresh.lean` is historical after its API is removed; there is no replacement database-generated entity-identity theorem. Entity IDs are application-owned 128-bit values sealed once and replayed unchanged. A UUIDv4 helper has 122 random bits after its fixed version/variant bits, not 128 independent random bits; probability is not injectivity. Prove only what the engine controls: canonical bytes survive retry/export/restore, conflicting key proposals are judged, and entity identity is not mistaken for request idempotency or lineage authority.

## 4. Fix ASS-001 without retaining a misleading theorem name

The current `ComponentClosed` premise includes closed relation targets, whereas Rust ignores their edges. Replace the support definition with **mutable consulted relations**. Prove that changing relations outside that support leaves judgment unchanged while all closed denotations remain fixed. Shared closed vocabulary need not merge two mutable components.

Under the new log design this theorem justifies scoped admission/planning work, not independent distributed commit histories or causal read cuts. Test the actual support derivation for shared closed targets, closed sources, selections, capacity weights, isolated relations, and every accepted statement form. Tie each optimized check to this concrete support calculation. No theorem about one stronger premise is recorded as proof of a different runtime premise.

## 5. Rust representation and unsafe boundary

Use private concrete types for invariants. Avoid sealed-trait gymnastics where an enum or private constructor suffices. Public extensibility accepts ordinary typed values or validated bytes; it never grants an accidental raw-storage proof capability.

Keep unsafe code localized to necessity: LMDB open/resize/lifetime substrate, well-audited vectorized loads, platform floating environment, and native ABI adapters. Each island states its incoming invariant, ownership/lifetime requirement, supported architectures, and how tests challenge it. `unsafe` does not mean “skip checking corrupt disk bytes,” and safe Rust alone does not prove logical canonicality.

For the proposed LMDB resize wrapper, the actual installed `heed 0.22.1` source states that **no transactions may be active**. Encode exclusive transaction-gate ownership in the wrapper's argument. For an uncommitted hosted candidate, the worker owns the transaction on one thread; no lifetime transmute makes it freely async/mobile. For a Rust `OwnedSnapshot`, owner close must wait/refuse while a borrow is live; managed Node handles may be revoked only after active operations drain and no borrowed mapped pointer can escape.

Do not add a custom allocator to every type just to make budget fields look rigorous. A small fallible, charged buffer abstraction and the single scratch relation can cover controllable bulk growth. Document allocation classes outside that accounting and the OS/process boundary for strict hosted isolation.

## 6. Nightly Rust: concrete uses, not a feature shopping list

Read-only inspection on 2026-09-04 found repository pin `nightly-2026-08-15`, compiler `rustc 1.99.0-nightly (d453bdd8f 2026-08-14)`, edition 2024, and existing `try_blocks` / `portable_simd`. The bundled rust-src on that pin contains the APIs below. **Availability was inspected; no successor feature prototype or benchmark was compiled in this proposal.**

| Feature | Decision | Exact useful scope / gate |
| --- | --- | --- |
| `portable_simd` | Retain behind a small kernel module | Bounded batch comparisons/bit normalization/membership operations; scalar fallback and forced-dispatch bit parity required |
| `try_blocks` | Retain where it simplifies one fallible operation | Private builder/admission cleanup structure; not a substitute for an explicit state enum or RAII ownership |
| `try_with_capacity` | Adopt for new bulk buffers when clearer | `Vec::try_with_capacity` is present and unstable on the inspected pin; maps allocation failure into the existing resource error without an infallible constructor |
| `allocator_api` | Do not adopt globally | `Box::try_new` is present; use only if a measured/identified large boxed allocation needs fallibility and a Vec-backed buffer is worse; no bespoke allocator framework |
| Generic-const/ specialization / nightly coroutine machinery | Do not require for 1.0 | No demonstrated need; ordinary const generics, enums, ownership and worker messages express this design |
| Fast floating intrinsics / unsafe math optimizations | Forbidden for normative F64 execution | They weaken the specified arithmetic; no benchmark exception in normal builds |

`Vec::try_reserve` is stable on this toolchain; do not market it as a nightly capability. Likewise use stable language/library constructs wherever they express the invariant cleanly. Every unstable feature must have a named owner, a tiny usage boundary, and upgrade/fallback tests. Pinning nightly enables reproducibility; it does not qualify untested compiler updates.

Do not set `target-cpu=native` on distributed artifacts. Runtime CPU dispatch may choose faster kernels only after feature detection and only if the scalar/optimized semantics agree. Float comparisons must use canonical total-order keys rather than inherit SIMD IEEE NaN comparison differences. Exact float sum is an integer accumulator, not a native SIMD reduction.

Apple Silicon is the primary performance target, not a license to export M2 Max constants as architectural truth. Qualify actual Apple generations, Graviton ARM instances and x86-64 hosted Node environments with identical logical results. NEON kernel structure can be portable across ARM64 while the best batch, prefetch lead, load factor and cache budget differ. Vercel-style x86 hosting still needs a supported Node/native runtime and sufficient real local storage; portable arithmetic does not create a persistent disk. [40](40-performance-contract.md) distinguishes structural constants, semantic limits and hardware-specific tuning evidence.

## 7. A bridge ledger with evidence, not only names

For every advertised invariant, record:

```text
guarantee -> exact model statement -> explicit premises
          -> concrete construction/transition site
          -> independent fixture/model test + platform/runtime gate
          -> known unsupported conditions and evidence revision
```

Keep term-level Lean references so deleted theorems fail the build. Add tests that instantiate the premises with the actual runtime representation. A file path and a function name existing does not prove that function checks the premise. ENG-001's overlooked constructor and ASS-001's stronger component premise are the counterexamples to that assurance style.

Preserve old findings, exact reproductions, fix/disposition commits, new tests, and reviewer challenges. Do not delete a counterexample once a census becomes green. The source/proof/spec package versions and platform artifact digests accompany every recorded gate run.

## 8. Independent assurance lanes

All lanes below are **required future evidence; not performed as part of this Markdown proposal**.

| Gate | What it must establish | What it does not establish |
| --- | --- | --- |
| `P-KERNEL` | Lean build and theorem axiom audit; no unfinished proofs, new unreviewed axioms, or proof escapes | Correctness of Rust/LLVM/LMDB/hardware |
| `P-SEMANTIC` | Independent staged evaluator versus baseline/optimized engine; grouped-capacity zero/weight/domain laws, projection grain, aggregate-derived consumers, finite frozen recursive predecessors and producer errors agree | Every infinite input shape or all schedules |
| `P-FLOAT` | Complete F-* roster from 11, exact reference arithmetic and changed host FPU state | Real-number algebra for rounded expressions |
| `P-REPRESENTATION` | Downstream safe API construction tests; parser fuzz/goldens; forced hash collisions; schema/write/read roundtrips | Absence of all unsafe ABI misuse |
| `P-DISK` | Q-* and E-* disk/native resource paths, map growth, larger-than-RAM data, crash/reopen | S3 authority semantics or raw `.mdb` portability |
| `P-MEMORY` | Miri on eligible pure-Rust ownership/codec components; sanitizers/native lifetime stress on actual ABI/LMDB boundaries | Miri execution of arbitrary foreign LMDB/assembly; untested platforms |
| `P-SCHEDULE` | Deterministic barriers and subprocess pause/death for transaction-gate/close/resize/candidate boundaries | Physical power-loss durability by itself |
| `P-ARTIFACT` | Fresh Rust core, Node native binding, TS source and packed packages on the published Apple/Graviton/x86 roster; removed public C package/header absent | Old installed native binaries matching new source |
| `P-PERF` | Controlled warm/cold/>RAM/maintenance/fleet workloads with correctness checks and retained raw runs | A universal “fastest database” claim |

The Lean axiom audit has an explicit approved foundational allowlist (for example the Lean kernel's standard logical foundations); it must not conflate those with an unproved database theorem introduced as a new axiom. Compiler-assisted proof shortcuts add trust and must be declared. If a needed numerical theorem is unfinished, the relevant gate is not green. Do not conceal it behind a broader theorem about ideal real arithmetic.

Fuzzers need structured valid generation plus invalid mutation. Pure random bytes alone poorly exercise a typed multi-relation query. Cross-product seeds include keys × temporal × text × floats × aggregates × negation × recursion × snapshots × error/reuse. Independently compute expectations; generating both expected and actual results with the production parser/judge is not differential testing.

Resource tests assert peak counted resources and cleanup, not merely an error code. History tests record intermediate reads and acknowledged outcomes, not merely eventual final-store equality. A credential-gated cloud test that did not perform an operation is an explicit unrun gate, never a passing real-S3 result.

## 9. Release discipline

The old audit recorded 2,049 workspace tests passing with 30 skipped, a 277-case Lean conformance pass, and selected 209 TypeScript tests against an existing native artifact. Those facts are preserved in [audit/90-evidence.md](../audit/90-evidence.md). They do not validate this new scalar domain, scratch executor, map lifecycle, or log architecture.

Before 1.0, every promised feature has its full applicable gate completed on fresh artifacts; unsupported surfaces are removed from the claimed product, not hidden behind a passing aggregate test count. Root's release graph includes explicit large-data, native ABI, real backend, restore, and packaging qualification rather than treating a root-workspace pass as all of them.

At handoff from each implementation change, include the invariant it replaces, deletion of obsolete casework/API/proof assumptions, tests that would catch regression, measured costs if performance-sensitive, and remaining unproved trust boundaries. Keep a small semantic kernel, one real LMDB substrate, one log authority machine, and thin clients. The proof/test program exists to make those few pieces trustworthy—not to become a second, larger product beside them.
