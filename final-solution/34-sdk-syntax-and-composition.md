# 34 — One relational vocabulary, three ways to run it

Execution routing: P07 Rust/core-TS syntax; P08 log reuse; P10 migration intent; P13 installed consumer fixtures. C01/C05/C10 and chapter 35 govern signatures; examples are targets, not current exports. See [work packets](62-work-packets.md) for source ownership and complete deliverables.

Status: **proposed 1.0 syntax, not current compile-tested exports**. These are the concrete examples to implement and qualify under API-12/PKG-03, not source changes made by this proposal. All snippets describe the same `Learning` schema. Application paths, authenticated bindings and measured execution policies are inputs, not SDK-provided authentication or invented deployment services.

The aesthetic is deliberate: recognizable `schema!`/`query!` in Rust; ordinary `relation`/`schema` and `query(...).rule(...)` values in TypeScript; one small core change builder. The log adds a durable envelope around that change. It does not make the developer learn a second database API.

TypeScript uses Effect 4 exclusively for work and Scope for resource ownership. No Promise/disposal twin or optional adapter survives. Plain schema/query/scalar descriptions remain ordinary values; Rust remains blocking/RAII. [35](35-effect-typescript-contract.md) fixes the complete signature, dependency, interruption and finalizer contract, grounded in version-matched Effect 4 docs and the actual explanation-store consumer.

TypeScript is **async-only for database/data work**: ingestion/finalization, command sealing/hashing, storage, reads/queries/sessions, row codecs, result/cursor operations, inspect, maintenance and resource close/disposal. There are no synchronous twins. Small schema/query/scalar/intent constructors, inert layer/effect construction and reads of already-owned identity/witness/stamp metadata remain ordinary synchronous expressions with no hidden native data work. Rust retains its native blocking/RAII interface. This does not introduce sync or async application transaction callbacks.

## One home for each concept

| Core: `bumbledb` / `@bjornpagen/bumbledb` | Log: `@bjornpagen/bumbledb-log` |
| --- | --- |
| Scalars, `Id128`, relations, schema and `SchemaId` | `DatabaseIdentity`, history stamps, receipt epochs |
| `ChangeSet` and its checked canonical encoding | `Command` = core change + scope/ID/precondition/result |
| Typed query expressions/templates and parameters | No log query, parameter or change-builder counterpart |
| `QueryReader`, sessions, `CompleteResult`, row/value codecs | Published snapshot wrapper with identity/stamps/freshness |
| `ExecutionPolicy`, `DbError`, `Violations`, core apply outcomes | Publication certainty, receipts, remote policy and protocol errors |
| Local `Db`, coherent snapshot, core-local witness | `LocalHistory`, `HostedHistory`, explicit generated migrations/backup/restore |

`ChangeSet` is the public name of the engine's checked immutable delta, not an extra copy beside `CheckedDelta`. Asynchronous `Command.seal` retains that native value, checks its schema against the command scope and adds bounded log fields. The change grammar/normalization/value codec is core-owned; command envelope framing/digest is log-owned and computed on the bounded native worker. No row-by-row JS re-marshaling occurs when a core change enters the log. Request idempotency and durable result metadata never leak into a core change.

Rust examples import the query macro from `bumbledb` too. This is a direct re-export of the query proc-macro implementation, not a dependency on the existing `bumbledb-query` facade that already depends on the core. Macro packaging must preserve an acyclic crate graph. Rust core stays independent of log/AWS dependencies. The two TypeScript packages use chapter 32's one exact-version native artifact/loader; the core's public exports still contain no log internals.

## The same schema in both languages

An attempt belongs to one student, carries a floating score and consumes exact integer units. Its `active` interval is recorded in integer seconds. A student's total units cannot exceed that student's budget. `score` is intentionally not a capacity weight: floating scores are not exact integral capacity measures.

```rust
use bumbledb::{schema, query, params, ChangeSet, Db, ExpectedGeneration,
              ExecutionPolicy, ApplyOutcome, F64, Id128, Interval};

schema! {
    pub Learning;

    relation Student { id: id128 as StudentId, name: str, budget: u64 }
    relation Attempt {
        id: id128 as AttemptId,
        student: id128 as StudentId,
        score: f64,
        units: u64,
        active: interval<i64>,
    }

    Student(id) -> Student;
    Attempt(id) -> Attempt;
    Attempt(student) <= Student(id);
    Student(id) <=[units]{0..budget} Attempt(student);
}
```

`id128 as StudentId` retains Rust's useful nominal newtype; `AttemptId` is not assignable to it. These host aliases lower to the same structural `Id128` scalar and do not add their Rust names to the schema hash. There is no `fresh` modifier. `f64` in the schema emits the canonical `F64` value type, not an unchecked host-float field. Record names, field names, order and laws below are identical, so both languages produce the same canonical schema identity.

```ts
// schema.ts — ordinary declarations; app queries import these exact values
import {
  capacity, contained, f64, i64, id128, interval, key, on,
  ref, relation, schema, str, u64, weigh, within
} from "@bjornpagen/bumbledb"

export const Student = relation("Student", { id: id128, name: str, budget: u64 })
export const Attempt = relation("Attempt", {
  id: id128, student: id128, score: f64, units: u64, active: interval(i64)
})

export const Learning = schema("Learning", { Student, Attempt }, [
  key(Student, ["id"]),
  key(Attempt, ["id"]),
  contained(on(Attempt, "student"), on(Student, "id")),
  capacity(on(Student, "id"), {
    from: on(Attempt, "student"),
    weight: weigh("units"),
    within: within(0n, ref("budget"))
  })
])
```

The TypeScript schema keeps the current law-derived field relationships and inferred row types. IDs are canonical 32-lowercase-hex `Id128` values, integers are `bigint`, scores are `number`, and text is `string`. Unlike Rust's explicit `StudentId`/`AttemptId` newtypes, these TS entity values share the core `Id128` host type; the schema's typed field/query-variable relationships do not invent a distinct host brand per primary key. A student parameter infers `Id128`, not a nonexistent generated `StudentId` class. Application wrappers may add brands, while native keys/references remain the integrity rule. The native checked boundary remains authoritative for untyped callers. No required generated runtime-type file or handwritten migration module sits between this schema and application queries.

## Queries are reusable typed values, including their intermediate results

```ts
// queries.ts
import { query, v } from "@bjornpagen/bumbledb"
import { Attempt, Learning, Student } from "./schema"

export const attemptsFor = query(Learning).rule((r) => {
  const { id, student, score, units, active } = v(Attempt)
  return r.match(Attempt, { id, student, score, units, active })
    .where(r.eq(student, r.param("student")))
    .find({ id, student, score, units, active })
})

export const attemptStats = query(Learning).rule((r) => {
  const { id, student, score } = v(Attempt)
  return r.match(Attempt, { id, student, score })
    .find({ student, total: r.sum(score), mean: r.mean(score) })
}).named("attemptStats")

export const studentSummary = query(Learning).rule((r) => {
  const { student, total, mean } = v(attemptStats)
  const { name } = v(Student)
  return r.match(attemptStats, { student, total, mean })
    .match(Student, { id: student, name })
    .find({ student, name, total, mean })
})
```

`attemptsFor` infers `{ student: Id128 }` parameters and its named result row. `attemptStats` preserves the identity-bearing `id` in its input binding set, so two different attempts with equal scores both contribute. Its result is a typed relation expression: `v` and `match` accept it just as they accept a stored relation. `.named(...)` supplies a useful diagnostic name, not a new schema relation, persistent view or separate CTE type. The same query value can be executed on its own or used downstream; it contains no tenant rows or live snapshot.

The equivalent Rust composition retains the repository's relational notation:

```rust
let attempts_for = query!(Learning {
    (id, student, score, units, active) |
        Attempt(id, student, score, units, active), student == ?student;
});

let attempt_stats = query!(Learning {
    (student, total: Sum(score), mean: Mean(score)) |
        Attempt(id, student, score);
});

let student_summary = query!(Learning {
    use stats = &attempt_stats;
    (student, name, total, mean) |
        stats(student, total, mean), Student(id: student, name);
});
```

The proposed `use stats = &attempt_stats;` clause binds an existing schema-bound typed query value into the macro's lexical relation roster. The returned template retains owned immutable IR, not a borrow of a database or execution session. It checks the imported output shape/schema, then builds the same relation-expression nodes as TypeScript. Existing `interior stats(...) | ...;` can remain inline declaration sugar for those nodes; it must not impose a separate no-aggregate result type. This is token/typed-value construction at the Rust build boundary, not a runtime query-string parser.

`Attempt(id, student, score)` uses the current macro's **field-name punning**, not a three-column positional tuple for a six-field relation. Omitted relation fields are unbound/existential; the bound key `id` preserves attempt identity. `Student(id: student, name)` makes the rename explicit. The imported `stats(student, total, mean)` binds the derived expression's three declared output positions, checked against its output shape.

All query templates are named ordinary variables and reusable across matching-schema core/log snapshots. Parameters are supplied at execution; no store-specific `prepare` object is required just to retain a schema-level template. Reusable execution sessions remain available for measured warm workloads, separately owned and snapshot-bound as in chapter 30.

## One Effect read helper for core and log

The following is proposed syntax, to be compiled against the successor packages. Imports named from local app modules are app-owned values/policies, not extra Bumbledb helpers. All database work is yielded; no generic adapter is needed.

```ts
// reads.ts
import { Effect } from "effect"
import type { ExecutionPolicy, Id128, QueryReader } from "@bjornpagen/bumbledb"
import { attemptsFor } from "./queries"
import { Learning } from "./schema"

export const readAttempts = Effect.fn("readAttempts")(
  function*(reader: QueryReader<typeof Learning>, student: Id128, work: ExecutionPolicy) {
    const result = yield* reader.execute(attemptsFor, { student }, work)
    return yield* result.collect({ maxBytes: work.outputBytes })
  },
  Effect.scoped
)
```

The function infers Effect of owned rows, core DbError, and no additional services. Its inner scope closes the completed result; it does not close the supplied reader. Core and log snapshots satisfy the exact same QueryReader. A log snapshot adds copied history stamps, never a writable Db escape. Missing exact-key reads return Effect Option, not nullable rows.

Large completed answers use the same core result through Effect Stream:

```ts
import { Stream } from "effect"

// Inside a generator with a live snapshot; consume within its enclosing scope.
const result = yield* snapshot.execute(attemptsFor, { student: studentId }, work)
yield* result.pages({ pageBytes: pageBudget }).pipe(
  Stream.runForEach(consumeOwnedPage)
)
```

pageBudget and consumeOwnedPage are app inputs. Each emitted item is a bounded owned page array. The first stream run consumes the sealed result into a private scoped cursor; a second run refuses. Early termination/failure/interruption drains its backing. The entire query finishes before any page is delivered. This replaces the public TS cursor API, not the complete-result semantics or Rust consuming cursor. No Effect per tuple is needed.

## One TypeScript change, usable by either product

```ts
// changes.ts
import { Effect } from "effect"
import { ChangeSet, type Id128, type ExecutionPolicy, span } from "@bjornpagen/bumbledb"
import { Attempt, Learning, Student } from "./schema"

export const newAttempt = Effect.fn("newAttempt")(
  function*(studentId: Id128, attemptId: Id128, work: ExecutionPolicy) {
    const draft = yield* ChangeSet.builder(Learning, work)
    const active = yield* Effect.fromResult(span(0n, 60n))
    yield* draft.insert(Student, [{ id: studentId, name: "Ada", budget: 10n }])
    yield* draft.insert(Attempt, [{
      id: attemptId, student: studentId, score: 0.9, units: 1n, active
    }])
    return yield* draft.finish()
  }
)
```

This helper intentionally retains Scope in R: it returns a scoped ChangeSet, not a closed handle. Do not add Effect.scoped to a helper returning resources. The draft and successful result register independently in the caller's scope; finish moves native ownership and spends the draft. Ingestion effects return void and accept immutable-owned input on successful completion. Before completion the caller keeps input stable. Failure spends/drains the draft. Sequential insert/delete effect reruns are ordinary executions while the draft is building; they read input and charge work again, without automatic retries or iterator replay. Concurrent/reentrant use and use after finish refuse. Chapter 35 fixes the late-completion/interruption and incomplete-drain cases.

### Core: direct local admission, no receipt ceremony

```ts
import { Effect } from "effect"
import { Db, Id128, NativeRuntime } from "@bjornpagen/bumbledb"
import { Learning } from "./schema"
import { newAttempt } from "./changes"
import { readAttempts } from "./reads"
import { localPath, runtimePolicy, work } from "./runtime-policy"

const program = Effect.scoped(Effect.gen(function*() {
  const db = yield* Db.open(localPath, Learning, work)
  const studentId = yield* Id128.random()
  const attemptId = yield* Id128.random()
  const changes = yield* newAttempt(studentId, attemptId, work)
  const outcome = yield* db.apply(changes, { ...work, expected: { kind: "any" } })
  if (outcome.kind !== "accepted" && outcome.kind !== "no-change") return outcome
  const snapshot = yield* db.snapshot(work)
  return { outcome, rows: yield* readAttempts(snapshot, studentId, work) }
}))

// One boundary for this script; an Effect app supplies this layer in its app graph.
await Effect.runPromise(program.pipe(Effect.provide(NativeRuntime.layer(runtimePolicy))))
```

Db.open never creates on missing/error; Db.create is the explicit constructor. Id128.random is effectful entropy, not a pure constant. The example is a single original local intent, not a retry loop. Core apply returns semantic refusal in A and operational DbError in E. Production intent IDs live in the app's request/job record when cross-process retries matter; raw core apply has no log receipt guarantee.

### Log: only the durable envelope changes

```ts
import { Effect } from "effect"
import { Command, HostedHistory } from "@bjornpagen/bumbledb-log"
import { Learning } from "./schema"
import { newAttempt } from "./changes"
import { hostedBinding, hostedOptions, submitOptions, work } from "./runtime-policy"
import { intent, rememberCommandRef, rememberSubmitOutcome } from "./request-state"

const submitAttempt = Effect.scoped(Effect.gen(function*() {
  const history = yield* HostedHistory.open(hostedBinding, Learning, hostedOptions)
  // Local alternative: LocalHistory.open(localBinding, Learning, localOptions).
  const changes = yield* newAttempt(intent.studentId, intent.attemptId, work)
  const command = yield* Command.seal({
    scope: history.identity,
    id: intent.commandId,
    changes,
    precondition: { kind: "blind" },
    result: { attempt: intent.attemptId }
  }, work)
  yield* rememberCommandRef(command.ref)
  const outcome = yield* history.submit(command, submitOptions)
  yield* rememberSubmitOutcome(outcome)
  return outcome
}))
```

The app-owned intent already contains stable entity IDs and commandId (receiptEpoch plus RequestId). The two remember functions represent existing request/job persistence, not SDK hooks or required new services. A stored ref permits resolution; retain canonical Command.encode bytes too if the application must resubmit after process loss without rebuilding meaning. Do not replace the epoch or generate IDs on a retry. In the same scope, submit the same sealed command again when appropriate; never retry this whole construction program automatically.

Submit is Effect<SubmitOutcome, never>, but interruption/defects can prevent delivery of A. Its decided/not-submitted/outcome-unknown cases retain the chapter 30 distinction. Decided includes committed/no-change **or durable rejection**. Later local/close failure cannot change that receipt. The retained ref remains available if scope finalization fails; no whole-request uninterruptible block is needed. Log imports the same core changes, QueryReader, policies, errors and codecs.

### Witnessed correction: keep the read scope short

```ts
import { Effect, Option, Schema } from "effect"
import { ChangeSet } from "@bjornpagen/bumbledb"
import { Command } from "@bjornpagen/bumbledb-log"
import { Attempt, Learning } from "./schema"

class AttemptMissing extends Schema.TaggedError<AttemptMissing>()("AttemptMissing", {}) {}

// Inside a generator holding history, intent, and the app's work policy.
const observed = yield* Effect.scoped(Effect.gen(function*() {
  const snapshot = yield* history.snapshot({ ...work, consistency: { kind: "latest" } })
  const previous = yield* snapshot.get(Attempt, { id: intent.attemptId }, work)
  if (Option.isNone(previous)) return yield* new AttemptMissing({})
  return { previous: previous.value, at: snapshot.stateStamp }
}))

const draft = yield* ChangeSet.builder(Learning, work)
yield* draft.delete(Attempt, [observed.previous])
yield* draft.insert(Attempt, [{ ...observed.previous, score: 0.95 }])
const changes = yield* draft.finish()
const command = yield* Command.seal({
  scope: history.identity, id: intent.correctionCommandId, changes,
  precondition: { kind: "exact-state", at: observed.at },
  result: { attempt: intent.attemptId }
}, work)
yield* rememberCommandRef(command.ref)
const corrected = yield* history.submit(command, submitOptions)
```

The inner scope returns owned data and a copied stamp, not a native resource, and closes before publication. A new correction intent has its own retained command ID. A stale witness becomes a durable precondition-failed receipt; rereading/revising requires a new intent/ID. Core uses the same pattern with snapshot.witness and db.apply's exact expected generation instead of log StateStamp. No ambient transaction, callback replay, reserve/refill or silent mutable replica access remains.

## Rust core: local operations and ordinary RAII

The following body takes `local_path` and a measured `policy: ExecutionPolicy`; it returns a `Result` supporting the displayed typed errors. The schema/query variables above are in scope. `params!` is proposed typed named-parameter construction against the template, not a text parser or a generated runtime client.

```rust
let student_id = StudentId::from(Id128::random()?);
let attempt_id = AttemptId::from(Id128::random()?);
let db = Db::create(local_path, Learning, &policy)?;

let mut draft = ChangeSet::builder(Learning, &policy);
draft.insert([&Student { id: student_id, name: "Ada", budget: 10 }])?;
draft.insert([&Attempt {
    id: attempt_id, student: student_id, score: F64::from(0.9),
    units: 1, active: Interval::new(0i64, 60i64)?,
}])?;
let changes = draft.finish()?;
match db.apply(&changes, ExpectedGeneration::Any, &policy)? {
    ApplyOutcome::Accepted { .. } | ApplyOutcome::NoChange { .. } => {}
    ApplyOutcome::InvariantRejected { violations } => return Err(violations.into()),
    ApplyOutcome::Moved { .. } => unreachable!("Any has no witness to move"),
}

let snapshot = db.snapshot(&policy)?;
let parameters = attempts_for.bind(params! { student: student_id })?;
let result = snapshot.execute(&attempts_for, &parameters, &policy)?;
let rows = result.collect(policy.output_bytes)?;
let previous = snapshot.get(&AttemptById { id: attempt_id }, &policy)?
    .ok_or("Attempt is missing")?;

let mut draft = ChangeSet::builder(Learning, &policy);
draft.delete([&previous])?;
draft.insert([&Attempt { score: F64::from(0.95), ..previous }])?;
let correction = draft.finish()?;
let expected = ExpectedGeneration::Exact(snapshot.witness());
drop(snapshot); // The copied witness/change do not retain a read transaction.
let corrected = db.apply(&correction, expected, &policy)?;
```

`?` handles operational failure; the `ApplyOutcome` match handles semantic admission. The final `corrected` outcome must likewise be inspected for accepted/no-change/rejection/movement. `AttemptById` retains the existing schema macro's typed key-object aesthetic. The shown `get` returns an owned row, and the correction is sealed before dropping the snapshot; `ChangeSet` already owns the canonical bytes. Any separately offered Rust borrowed-view method retains its explicit guard lifetime and is not the operation shown here. `result` owns independent completed-result resources, never a hidden source-snapshot pin. Dropping result/change/snapshot guards releases their resources; explicit `db.close(&policy)` provides chapter 31's drain report when needed. No lifetime-erased closure or GC is necessary, and no public Rust log surface is introduced.

## Grouped capacity without a spelling police

The main schema's named-options TypeScript call replaces the old four positional arguments, while preserving `on`, `weigh`, `within`, `ref` and the compact Rust law. These additional **alternative** laws use the same schema fields:

```ts
// At most 20 distinct attempt facts per existing student: unit weight is default.
capacity(on(Student, "id"), {
  from: on(Attempt, "student"), within: within(0n, 20n)
})

// Total recorded duration, not simultaneous occupancy, is at most one hour.
capacity(on(Student, "id"), {
  from: on(Attempt, "student"),
  weight: weigh(duration("active")), within: within(0n, 3600n)
})
```

Here `duration` is another core import. Count is the same per-parent sum with unit weight; an existing selected parent with no children has measure zero. Weight fields are exact nonnegative integer values or bounded integer-interval durations on source rows. The group key stays scalar. A temporal group/pointwise weighted-occupancy feature is not being added, and a reference path is not an implicit weight join.

Harmless equivalent spellings such as `within(3n)` and `within(3n, 3n)` canonicalize to the same law. Vacuous windows and equivalent law spellings can normalize without rejecting the author's style. Negative/inverted bounds, wrong dimensions, invalid fields and genuinely unsupported meanings still refuse. Weighted total zero does **not** mean absence: nonempty zero-weight rows remain possible. Duration counts each distinct fact's length; overlapping intervals are not automatically coalesced by a capacity statement.

## Composition preserves meaning, not incidental implementation walls

An explicit projection before aggregation changes the distinct grain. Projecting attempts to `{ student }`, then counting that relation, counts students; aggregating the original bindings including attempt ID counts attempts. Naming a query does not project anything. Each nonrecursive aggregate stage emits canonical rounded scalars before a downstream stage consumes them; an optimizer cannot erase that rounding/error boundary by flattening the expression.

The supported closure remains one positive, projection-only linear recursive component. Frozen finite predecessor relations—including nonrecursive computed/aggregate outputs—can feed its base/step. No aggregate, arithmetic-created value or negation may flow through the recursive feedback cycle; no mutual/general Datalog expansion is implied. Nonrecursive consumers can use the completed recursive result normally. This is a dependency/denotation boundary, not a ban on where a query variable happens to be declared.

The same core expression IR is available to the schema-generation planner's explicitly supported deterministic transforms. This does not turn every query into a valid migration or allow opaque JavaScript. Generated plans still carry chapter 33's coverage, intent, intermediate-law and restart requirements, and the runtime runner still consumes inert plan data only.

## Finite checks before calling this pleasant API real

API-02/07/08/12 and PKG-03 must compile/typecheck and execute these examples from exact staged packages: same schema identity across Rust/TS, inferred keys/parameters/results, direct `QueryReader` reuse, same core change admitted through core/local/hosted variants, and no second native representation. Include compile-fail/untyped attacks on foreign-schema queries, Rust nominal entity-ID misuse, TS foreign field-variable domains, entity/request-role confusion, mutation through a read capability, a spent change/command and handles from another native runtime instance. Do not falsely assert separate TS entity brands that the schema did not declare. Equal package version alone does not establish shared native ownership.

API-01/10 and FFI-05/07 additionally require successful Effect ingestion acceptance, mutation after completion, hostile mutation/detachment during yielded copying, oversized cells, getter/iterator exceptions, overlapping/reentrant calls, failed-draft native drain with GC disabled, and event-loop delay under large admitted ingestion/finalization/hash/row conversion. The declaration/export gate requires Effect-only work, scoped owners and completed-result page Streams, forbids Promise/sync/disposal twins, and preserves ordinary metadata constructors. Chapter 35 adds exact A/E/R, interruption, finalizer-Cause and reexecution obligations. API-04/06 cover stable entity/request IDs and lost replies; core schema/query/value tests cover F64 and capacity/composition equivalence; TS-MIG-04/07/10 cover reused core IR and preserved intermediate semantics. Creation/open/migration examples use exactly these constructors and imports. Source scans alone are not passes, and no new SDK framework or gate family is needed.
