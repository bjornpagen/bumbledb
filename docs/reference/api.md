# Public API contract (permanent)

This page is the authoritative public usage contract. Packed consumers
and examples must use these concepts through public entry points. L18
owns the specimens; this page owns the meanings those specimens must
keep. Target syntax in examples is not evidence that a tree compiles
today.

## Product barbell

A small Rust machine owns data admission, compiled schemas, execution,
resource accounting, native ownership, local storage and the log
protocol. TypeScript owns typed authoring, Effect composition, a thin
checked transport boundary and framework integration.

The log is the core database with durable command identity and a
publication/lifecycle envelope. Adding S3 must not require a second
schema DSL, row codec, query reader, change builder, scalar evaluator,
cancellation mechanism or runtime.

There is one exact peer core version and one exact Effect version,
**4.0.0-rc.112**, for the first converged release. There is no public C
API and no `@superbuilders/errors` dependency. Rust crates remain
`publish = false` until crate publication is separately authorized.

## Public vocabulary

| Concept | Owns / means | Must not become |
| --- | --- | --- |
| Schema and query values | Pure, typed, reusable metadata | An import-time native operation or a live tenant handle |
| `Schema.compile` | Checked canonical schema identity and compiled native theory | A second schema interpreter in TypeScript |
| `ChangeSet.builder` | Scoped, cumulative, single-flight construction of an exact final delta | A transaction callback or arbitrary executable migration |
| `ChangeSet` | Immutable normalized changes, independently retained | Mutable row arrays the caller can edit after sealing |
| `Db` | Core local database owner | A log history with optional identity fields |
| `Snapshot` | One pinned coherent generation and its witness | A database-global prepared-query singleton |
| `QueryReader` | Core `get` and complete `execute` capability | A transport-specific query wrapper |
| `ExecutionSession` | Optional snapshot-bound reusable execution resources | A boolean alias that recompiles on every call |
| `CompleteResult` | A successfully completed answer set with owned RAM/scratch backing | A partially evaluated result masquerading as a complete set |
| `History` | Published state, named commands, recovery and receipts | An application tenant router or a second row store |
| `HistoryBorrow` | One scoped borrow from a shared history owner | Permission to close another request's owner |
| `Command` / `CommandRef` | Sealed intent / small durable recovery coordinate | A retry closure or a newly minted ID on each retry |
| `TenantCache` | One native registry under count/byte pressure | A JS LRU, TTL authority, or fleet scheduler |

Common interfaces and errors are imported literally from core. Do not
re-export them under log aliases.

## Pure declarations versus operational work

Pure means no native loading, filesystem access, hashing, canonical
compilation, row iteration, Promise execution or runtime acquisition.
Schema/query/scalar constructors may validate bounded local authoring
structure. Invalid authoring syntax can produce a clear authoring error;
caller-supplied operational input must enter the typed failure channel.

All operations are lazy Effects. There is no Promise twin, sync twin,
`AsyncDisposable` twin, raw callback API or SDK-level `runPromise`.
Framework handlers and executable CLI entry points are the legitimate
places to run Effects.

The root TypeScript export may remain convenient for pure and operational
concepts **if its imports do not load the addon**. Defer the addon load
to `NativeRuntime` acquisition. Authoring-only consumers must import a
schema on a machine without a native package installed.

Unresolved scalar field nodes must construct (`Scalar.field("units")`
plus a known operator/literal) without native loading. Native compilation
establishes kinds against the verified schema before effects, including
empty input. Known-invalid literal-only combinations refuse at their
promised authoring boundary.

## Ownership and bounded work

The native registry owns actual payloads and their transitions. A JS
handle is a checked capability into that registry. Each acquired
intermediate resource is registered for cleanup before another
interruptible step.

`close()` returns `closed`, `incomplete` with bounded obligations, or
`failed`. Repeated close joins the same transition. Session close leaves
the parent usable. A counter reaching zero in a subset of registries is
not proof.

An execution policy bounds input, work units, rows, working capacity,
scratch capacity, result capacity and monotonic time. Draft construction
has one cumulative lifetime budget including all chunks and finish.
Result delivery is new work with a new delivery deadline. `collect` and
`pages` start fresh bounded delivery operations. Native delivery is one
transaction: a ticket advances only after the complete admitted output is
registered. Predelivery refusal returns no data and does not advance.

For operations that may publish, certainty belongs in the success value.
Interruption and finalizer failures remain in the Cause. The caller
retains the command reference before dispatch. Never use a generic
catch-all to turn interruption into `not-submitted`.

Use one `ManagedRuntime` at the application boundary. A request supplies
its abort signal only to that boundary. No per-row fibers, Effects,
schema services, proxies or getters in returned rows.

Ordinary Rust operations take an explicit bounded `WorkContext`. Delete
the effectively unlimited convenience surface. One selected API accepts
an explicit host policy.

## Generated migrations

TypeScript schema SDK generates a canonical migration plan/history. The
log owns freeze, staged execution, admission and one final new
incarnation. Generation refuses unresolved ambiguity. Every required
intermediate source/target is bound and compiled before writing a new
authoritative manifest or freezing source. No handwritten callback
escape. Valid prefix retry appends only the intended suffix.

## Packed consumer expectations (L18)

L18 specimens no longer self-provide `NativeRuntime.layer`. Programs that
need native work run under one process-lifetime runtime:

`ManagedRuntime.make(NativeRuntime.layer(...))`

via `makeConsumerRuntime()` from the copied core-ts specimen.

Fresh built tarballs installed outside the workspace must run:

- core, log, and native-ledger consumers copied into the isolated
  project (`examples/consumers/{core-ts,log-ts,native-ledger}`)
- `scripts/packed-consumer.ts` providing that ManagedRuntime
- Notes `test/specimens.test.ts` and `test/routes.test.ts` (missing
  generated migrations fail; never skip green)
- Rust core consumer (`examples/consumers/rust`) in the same
  packed-import path

D07: `collectUnderTinyBudget` / native-ledger
`collectPublishedUnderTinyBudget` / Rust `collect(8, &tiny)` must fail.
D12: same-cursor tiny collect refuses, then a full `collect` still
returns the rows — not `typeof`/`type_name`/`size_of` of a delivery
type. D27: `scripts/packed-pure-authoring.ts` constructs
`Scalar.add(Scalar.field("units"), Scalar.u64(1n))` in a second isolated
project with platform packages absent. That cell must not import or
invoke `NativeRuntime.layer`.

No private imports, force casts, handwritten plan bytes, stub native
module, stale dist, Promise wrapper or missing-peer duplicate Effect
runtime. Pure import with addon unavailable is separate from native
operation success. Local packing is not registry publication proof
(`PKG-07B` remains separately authorized).

See [packaging.md](packaging.md) and [release-gates.md](release-gates.md)
D07/D22/D27.
