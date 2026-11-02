# Proposal 0003: a hard port to Idris 2 on idris-mlir

Status: **direction, not yet buildable.** First written 2026-10-09, with the code
read at 9e0b6728e. It depends on work in `../idris-mlir` that is in progress or
only proposed, and every dependency is named with its status in §8. Nothing here
changes bumbledb today. It is a hard port: Rust goes, there is no
interoperation period, and the on-disk and wire formats break freely, as
CLAUDE.md's hard-cutover rule already says.

## 1. Why

bumbledb and idris-mlir follow the same governing principle, nearly word for
word (`docs/design/representation-first.md`; idris-mlir's AGENTS.md "one thing,
one representation"). Much of bumbledb's machinery is Rust standing in for what
Idris 2 compiled by idris-mlir gives as language:

- **Sealed witnesses.** `ValidatedPlan` and the `provably_distinct` witnesses
  (`plan/fj.rs`, `plan/fj/provably_distinct.rs`) are parse-don't-validate built
  by hand. In Idris they are erased proofs indexed by what they validate.
- **Bit-identical twins.** Each SIMD kernel has a scalar twin kept
  bit-identical at every level (`exec/kernel.rs`, `mod reference`), plus
  runtime dispatch on the cached CPU level and a hand-tuned NEON Allen kernel
  (`exec/kernel/neon`). Under idris-mlir one array program is vectorized for
  each target by the compiler, and the twin is the same source compiled at one
  lane.
- **Index-addressed pools.** COLT keeps nodes, chunks, map slots and key words
  in pools, not pointers (`exec/colt.rs`). That is the arena style
  idris-mlir's acyclic heap asks for, so it carries over unchanged.
- **Scope as a convention.** `db.write(|tx| …)` relies on closure scope to end
  a transaction. In Idris a transaction is a linear handle that must end in
  commit or abort, and forgetting is a type error.

The new capability, the reason to port rather than polish, is §4: a prepared
query compiled into its own fused native kernel by specializing the executor,
with no query compiler written.

The second reason is §4.5. bumbledb's engine, without storage and laws, is a
relational core: typed relations, tries, worst-case optimal joins, folds,
fixpoints. Extracted as its own library and shipped with idris-mlir, the same
engine also serves idris-mlir's type checker and analyses. A faster join or a
better planner then lands once and speeds up the database, the compiler and
every program that queries relations.

## 2. What idris-mlir is

idris-mlir is a compiler for Idris 2. It goes from checked TT to an MLIR dialect
(`idr`) to LLVM and produces one statically linked native executable: a static
PIE on Linux, and PIE on libSystem on macOS. Its two first-class targets are
x86_64 Linux and arm64 macOS. What it provides that matters here:

- **Facts Idris proved stay in types through every pass.** Multiplicities
  (`!idr.erased`, `!idr.lin`) and ownership grades are checked after each
  pass. Erased proofs cost nothing at run time.
- **Reference counting with exact frees on an acyclic heap.** A type whose
  mutable cell could reach itself is refused, so counting never leaks. In-place
  reuse happens where uniqueness is proved, and `--demand-in-place` turns a
  missed in-place update into a compile error.
- **Static normalization, no comptime feature.** The frontend normalizes the
  static part of the program on the type checker's interpreter, with array
  operations as whole-array primitives (idris-mlir proposal 0003): closed
  calls of total code become values; partial code never runs at compile time.
  `idr-specialize`, defunctionalization and inlining remove indirection.
- **Arrays as memrefs, vectorized by default.** Array combinators become
  `linalg`, and `idr-in-bounds` erases bounds checks it can prove.
- **No escape hatches in user code.** No `%foreign`, no C ABI, no raw
  pointers, no threads with shared memory. Rust is the one foreign world
  (idris-mlir proposal 0013).

Measured on 2026-10-07 (idris-mlir `bench/runs/2026-10-07-f5a4dff9-darwin-arm64`),
with the speedup shown as clang -O2's time over idris-mlir's:
- spectral-norm-linear runs at 1.86× clang -O2's speed;
- binary-trees 2.67×, rbtree 1.76×;
- numeric code is at parity;
- byte and string code trails C by 2 to 8×, which matters for text columns.

## 3. The port, piece by piece

| bumbledb today (9e0b6728e) | In Idris on idris-mlir |
| --- | --- |
| `schema!` macro and the TS schema (`bumbledb-macros`, `ts/`) | A schema is an Idris value whose type indexes every relation, field and law. Relation and field identifiers are types, not strings. |
| closed relations (`Status = { Open, Frozen }`) | Plain sum types. A closed relation's ids are its constructors. |
| keys, containments, interval keys, coverage, capacity windows (`schema/judge`) | Split by what can be decided when. Laws over values (a closed id, a non-empty interval, a checked integer range) become types. Laws over the stored state stay commit-time checks, expressed as queries over the final state (anti-joins, group counts) that the same engine compiles (§4.3). |
| rejections naming every violated law with its facts (`schema/evidence`) | The same evidence, produced by the compiled law queries: each violation is a result row, not a hand-built report. |
| values (`bumbledb-theory`): checked integers, canonical `f64`, half-open intervals, Allen's 13-bit mask | Checked integers are types whose operations fail instead of wrapping. `CanonDouble` can only be built by canonicalizing (one NaN, one zero). `Interval` carries an erased proof that its start is not after its end. The Allen mask stays a 13-bit word. |
| `ValidatedPlan`, distinctness witnesses | Erased proofs indexed by the query: `ValidatedPlan q`. |
| column images, SoA slabs, immutable, shared by `Arc` (`image.rs`) | Shaped columnar arrays (idris-mlir proposal 0007). One image per shard (§5). |
| COLT, `Cursor = Node \| Row`, `KeyCount = Exact \| Estimate` | Unchanged in representation: sum types over arenas. |
| SIMD kernels: scans, compaction, Allen masks, folds, gathers (`exec/kernel/*`) | One array program each, lowered to `linalg` and vectorized per target. The scalar reference is the same source at one lane, and the gate is a differential test of one lane against N. The NEON Allen specialization and the runtime level dispatch go (measurement rule in §4.2). |
| `db.write(\|tx\| …)`, `WorkContext` | A linear transaction handle. `WorkContext` becomes an explicit argument of the shell. |
| LMDB through `heed` (`storage/`) | Through Rust: heed and LMDB reached by idris-mlir proposal 0013's generated bindings, linked statically as bitcode. LMDB's C is reached only inside the Rust crate. |
| the command log in S3 (`bumbledb-log`) | Its logic (frames, fold, receipts, replica) is ported to Idris. The S3 client is a Rust crate through 0013. |
| Node binding through napi (`bumbledb-node`) | No C ABI exists by design, so napi is impossible. Decided: the TypeScript package loads a WebAssembly build (§6). |
| `blake3` | Through Rust (0013), not re-implemented. |

## 4. The query engine: specialize the executor, fuse across the boundary

### 4.1 The first Futamura projection

bumbledb's executor is a query interpreter: plan nodes, cursors, kernels and
sinks (`exec/run.rs`, `exec/sink.rs`, `exec/dispatch`). Port it as an
interpreter, written once and kept readable. Then:

- **Prepared queries.** A query known at compile time is a closed call of the
  interpreter on a constant query. Planning (`plan/fj/*`: `binary2fj`,
  `factor`, `gj_split`, cover enumeration, residual and anti-probe placement)
  is total, so static normalization computes the plan (0003).
  `idr-specialize` clones the executor for that plan, defunctionalization and
  inlining remove the dispatch, and `linalg` fusion merges each pipeline
  (scan → filter → probe → fold → sink) into one loop nest, tiled and
  vectorized. The output is a compiled query, with no query compiler written.
  The planner's totality is now a requirement: a planner Idris cannot check
  total would leave every plan for run time.
- **Ad-hoc queries.** The same interpreter runs them, batched and vectorized.
  There is no JIT in a program (it would link LLVM into every program).
- **Prior art:**
  - HyPer and Umbra (Neumann, "Efficiently Compiling Efficient Query Plans for
    Modern Hardware", VLDB 2011; adaptive execution in Umbra);
  - LegoBase/DBLAB, and Rompf and Amin's "A SQL to C compiler in 500 lines of
    code" (staging an interpreter);
  - LingoDB (Jungmair et al., VLDB 2022: query compilation on MLIR);
  - Tensor Query Processor (Gandhi et al., VLDB 2022: queries as tensor programs).

  What is new here is the next three subsections.

### 4.2 Fusion across the database/application boundary

The database is a library compiled with the program. A prepared query's result
sink can fuse into the application code that consumes it: query → aggregation →
serialization, or query → a numeric kernel, become one optimized loop nest.
Every existing engine stops at its own API boundary.

The measurement rule for the kernels:
- a vectorized array program must reach at least 90% of the Rust kernel's
  throughput on bumbledb's kernel benches (`exec/kernel/bench.rs`) on both
  targets;
- where it does not, the missing vector pattern becomes an op of the `idr`
  dialect or a runtime primitive, never hand code in bumbledb;
- the hand-tuned NEON Allen kernel is the first test of this rule.

### 4.3 Laws compile too

The schema is known statically, so every law over stored state is a query over
the final state, generated with the schema:
- a key is a group count greater than one;
- a containment is an anti-join;
- coverage and capacity windows are interval folds.

Each is planned and compiled like a prepared query. The judge runs these
compiled queries, and their result rows are the evidence (§3).

### 4.4 Plans that depend on data

A plan fixed at compile time can be wrong for the data it meets. Decided:
- each prepared query compiles a small set of plan variants (the planner's
  covers under different cardinality assumptions, at most 4);
- a guard on the image statistics (`plan/selectivity`) picks among them at run
  time, Umbra-style;
- Free Join's robustness to bad estimates bounds the cost of a wrong choice;
- a query whose statistics leave every variant's range is planned at run time
  and runs on the interpreter.

### 4.5 The relational core is its own library, shared with the compiler

What bumbledb's engine is, without persistence, laws and the log, is a general
relational core: typed relations, lazy tries, Free Join, the planner, the
executor as an interpreter, semi-naive fixpoints and e-graphs. It is
idris-mlir's `libs/mlir-relational`, specified there in proposal 0010 (its
layers, its consumers including idris-mlir's elaborator and frontend, its
gates against Soufflé and egglog, and why it lives in the compiler's tree).
bumbledb depends on it as a package, and its own code shrinks to what is
genuinely about being a database: storage, laws and the judge, the log,
transactions and shard partitioning.

## 5. Concurrency: shards own partitions

idris-mlir's runtime is thread-per-core shards with no shared heap (its
proposal 0009): values cross shards by move or copy. bumbledb today shares
immutable images across threads through `Arc`.

Decided:
- each shard owns a partition of every relation's image, by hash of the
  relation's key;
- a query forks across shards and joins their partial results;
- writes go through one writer shard that owns the LMDB write transaction, as
  LMDB requires;
- sharing a whole image read-only across shards waits on 0009's "lend" step
  and its measurement rule.

On a single core, one shard holds everything, so the Pi Zero 2 pays nothing for
the design.

## 6. TypeScript and the Pi

- **TypeScript.**
  - napi needs a C ABI, which idris-mlir refuses by design.
  - Decided: a WebAssembly target entry in idris-mlir (wasm32 with WASI as the
    platform layer), with the TS package loading the module in process. This
    keeps bumbledb's in-process speed promise. A local server would break it,
    so it is rejected.
  - The hosted mode stays a TS-side concern over the same module.
- **Raspberry Pi Zero 2.**
  - Needs an aarch64 Linux target entry in idris-mlir (static PIE with musl, as
    x86_64 Linux has).
  - Cross-compiled: the compiler itself does not fit in 512 MB, but programs
    do.
  - Reference counting with no collector, plus a static binary, suits the
    device.

## 7. What is harder in Idris than in Rust, honestly

- **Ecosystem.** LMDB, S3, blake3 and serde-like encoding all come through
  Rust interop, which idris-mlir has not built.
- **Text columns.** idris-mlir trails C 2 to 8× on bytes and strings today.
  Contiguous runs (idris-mlir work item W11) must land before text-heavy
  queries reach parity.
- **Predictability.** In-place updates and erased checks depend on analyses.
  Every hot path gets the in-place promise (`--demand-in-place`) and an
  `idr-expect` property, so a lost optimization fails the build instead of
  slowing the database.
- **Compile time.** Specializing every prepared query costs compile time and
  code size. The budget: a program's prepared queries add at most 2× to its
  compile time, else the variant count (§4.4) drops before anything else.
- **No unsafe.** The NEON intrinsics and the `unsafe` pool accesses go. The
  compiler must prove bounds or keep a guard, and §4.2's rule decides which
  gaps become compiler work.

## 8. Dependencies, each with its status on 2026-10-10

| Needed from idris-mlir | Status |
| --- | --- |
| a self-hosting frontend, static native compiler | stage 0 done; `make self` shows stage 2 identical to stage 3 (1ce907d3); the C++ toolchain is built on arm64 (8395d4f7); x86_64 Linux is not |
| typed arrays of any rank, fusion, bufferization (proposal 0007) | proposed |
| shards, the reactor, fork/join (proposal 0009) | proposed |
| Rust interop: heed/LMDB, S3, blake3 (proposal 0013) | proposed, not started. Storage waits on it; the rest of this port does not |
| IORef, growable arrays, Integer shifts, record boxing | written, and compiled on the arm64 toolchain (f2d87554) |
| `mlir-relational` in `libs/` (§4.5): relations, joins, fixpoints, e-graphs | proposed in idris-mlir 0010; nothing written |
| the fork's elaborator on flat, hash-consed terms, with constraints and rewrite on `mlir-relational` | proposed in idris-mlir 0006, 0005 and 0010. The term arena is 0005's |
| static normalization, no comptime feature | decided (idris-mlir 0003); `idr-eval` is out of the simplify round; the pass is not deleted |
| aarch64 Linux and wasm32 target entries | not proposed yet; each is a new target entry, not a rewrite. wasm32 is a target, not the evaluator |

## 9. Staged plan

Each stage has a gate. The correctness gate for every stage is bumbledb's own
oracle (2,879 cases in the 2.0 suite) and its tests. The performance gate is
bumbledb's benchmark suite (`docs/perf/results.md`: 12 lanes, 32 read families,
34 scenario queries), run against the Rust engine at the same commit of the
suite.

| Stage | Content | Gate |
| --- | --- | --- |
| P0 | idris-mlir prerequisites: self-hosting, 0013 for heed and S3, 0007's columns, 0009's shards | their own gates |
| P1 | `bumbledb-theory`: Allen relations, intervals, canonical floats, as types | its tests ported, all passing |
| P2 | schema, laws, judge (§3, §4.3) | the oracle's admission cases |
| P3 | images and kernels as array programs (§4.2) | kernel benches at ≥ 90% of Rust on both targets |
| P4 | `mlir-relational` layer 1: relations, tries, Free Join, the planner, the executor as an interpreter (§4.5) | read families and scenarios at ≥ 80% of the Rust engine |
| P4b | `mlir-relational` layers 2 and 3: semi-naive fixpoints, e-graphs (§4.5) | within 1.5× of Soufflé (compiled) and of egglog on their benchmarks |
| P4c | idris-mlir's elaborator on the library: unification and shape-equality rewriting (§4.5) | a shape-equality corpus from idris-mlir proposal 0007 never blocks; elaboration of base no slower than before |
| P5 | prepared-query specialization and plan variants (§4.1, §4.4) | the 34 scenario queries at ≥ 1.5× the Rust engine, prepared |
| P6 | storage and log (LMDB, S3) through Rust | the storage, CRUD and lifecycle lanes |
| P7 | shards (§5) | the multi-core lanes at ≥ the Rust engine |
| P8 | WebAssembly build and the TS package (§6) | the TS test suite |
| P9 | aarch64 Linux and the Pi Zero 2 | the Pi lane within the 512 MB budget |

## 10. Rejected alternatives

- **An incremental port, Rust and Idris side by side.** There is no C ABI to
  join them, and the hard-cutover rule forbids a transition period.
- **Keeping the Rust kernels behind Rust interop.** Allowed only as §4.2's
  measured fallback for a kernel the compiler cannot match, never by default:
  it would keep two copies of every kernel's meaning.
- **SQL strings as the query language.** Queries stay typed values; their
  types are what make compile-time planning possible.
- **A server for TypeScript.** It breaks the in-process promise (§6).
- **Sharing images across shards from the start.** It needs a shared heap that
  idris-mlir's runtime deliberately does not have; it waits for "lend" (§5).
- **Keeping the engine inside bumbledb.** The compiler would then need a second
  join engine, unifier and rewrite system of its own: two copies of one
  mechanism, which drift (§4.5).
- **The relational core in bumbledb's repository.** The compiler builds only
  from its own tree, and it consumes the library. So the library lives in
  idris-mlir's `libs/`, and bumbledb depends on it.
