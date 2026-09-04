# 34 — One relational vocabulary, three ways to run it

Status: **proposed 1.0 syntax, not current compile-tested exports**. These are the concrete examples to implement and qualify under API-12/PKG-03, not source changes made by this proposal. All snippets describe the same `Learning` schema. Application paths, authenticated bindings and measured execution policies are inputs, not SDK-provided authentication or invented deployment services.

The aesthetic is deliberate: recognizable `schema!`/`query!` in Rust; ordinary `relation`/`schema` and `query(...).rule(...)` values in TypeScript; one small core change builder. The log adds a durable envelope around that change. It does not make the developer learn a second database API.

TypeScript keeps ordinary values, promises and explicit disposal; neither package requires Effect or implements its own effect system. An Effect application can use its normal scoped acquisition and cancellation adapters around the same owners and operations. That is application composition, not a second database SDK or a promise that interruption undoes publication.

## One home for each concept

| Core: `bumbledb` / `@bjornpagen/bumbledb` | Log: `@bjornpagen/bumbledb-log` |
| --- | --- |
| Scalars, `Id128`, relations, schema and `SchemaId` | `DatabaseIdentity`, history stamps, receipt epochs |
| `ChangeSet` and its checked canonical encoding | `Command` = core change + scope/ID/precondition/result |
| Typed query expressions/templates and parameters | No log query, parameter or change-builder counterpart |
| `QueryReader`, sessions, `CompleteResult`, row/value codecs | Published snapshot wrapper with identity/stamps/freshness |
| `ExecutionPolicy`, `DbError`, `Violations`, core apply outcomes | Publication certainty, receipts, remote policy and protocol errors |
| Local `Db`, coherent snapshot, core-local witness | `LocalHistory`, `HostedHistory`, explicit generated migrations/backup/restore |

`ChangeSet` is the public name of the engine's checked immutable delta, not an extra copy beside `CheckedDelta`. `Command.seal` retains that native value, checks its schema against the command scope and adds bounded log fields. The change grammar/normalization/value codec is core-owned; command envelope framing/digest is log-owned. No row-by-row JS re-marshaling occurs when a core change enters the log. Request idempotency and durable result metadata never leak into a core change.

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

## One TypeScript read helper for core and log

```ts
// reads.ts — imports only the core and the application's typed declarations
import type { ExecutionPolicy, Id128, QueryReader } from "@bjornpagen/bumbledb"
import { attemptsFor } from "./queries"
import { Learning } from "./schema"

export async function readAttempts(
  snapshot: QueryReader<typeof Learning>,
  student: Id128,
  work: ExecutionPolicy
) {
  await using result = await snapshot.execute(attemptsFor, { student }, work)
  return await result.collect({ maxBytes: work.outputBytes })
}
```

`QueryReader<S>` is just the core's read-only `get`/`execute` interface with their inferred keys, parameters and results. It is not a database-supertype/capability framework. Core and log snapshots directly satisfy it, so this helper needs no overload, adapter or log import. `return await` finishes conversion before `await using` disposes the result.

Complete results are the same core owners in every mode. Execution finishes before delivery; `collect` returns a bounded owned array, while `intoCursor` transfers the sealed backing to one paged cursor. Neither is an early ordered-feed/top-k API. The helper's returned rows outlive the snapshot safely. A log snapshot adds `identity`, `decisionStamp`, `stateStamp` and freshness outside the `QueryReader` interface; it does not offer a writable core `Db` or a raw transaction.

## One TypeScript change, usable by either product

```ts
import { ChangeSet, Id128, span } from "@bjornpagen/bumbledb"
import { Attempt, Learning, Student } from "./schema"

// Run once for the original app intent; retain these values across retries.
const studentId = Id128.random()
const attemptId = Id128.random()
using changes = ChangeSet.builder(Learning, work)
  .insert(Student, [{ id: studentId, name: "Ada", budget: 10n }])
  .insert(Attempt, [{
    id: attemptId, student: studentId, score: 0.9,
    units: 1n, active: span(0n, 60n)
  }])
  .finish()
```

`work` is the app's core `ExecutionPolicy`, including finite input/work/memory/output limits and cancellation/deadline. The builder is synchronous and database-free; it copies/checks values and normalizes add/remove once. Chain methods return that same owned builder; `finish` spends it. Any construction/ingestion/finalization failure also spends it and releases accumulated native buffers when the call unwinds, including getter exceptions; there is no partially reusable builder or GC-dependent failed-chain cleanup. Reentrant use refuses before mutation. Application IDs are generated before sealing, not substituted on conflict. The displayed relative seconds and random IDs are example data, not a database clock or uniqueness guarantee.

### TypeScript core: ordinary insert, read, witnessed replacement

```ts
import { Db } from "@bjornpagen/bumbledb"
import { readAttempts } from "./reads"

await using db = await Db.create(localPath, Learning, work)
const inserted = await db.apply(changes, { ...work, expected: { kind: "any" } })
if (inserted.kind !== "accepted" && inserted.kind !== "no-change") {
  throw new Error(`Insert refused: ${inserted.kind}`)
}

await using snapshot = await db.snapshot(work)
const rows = await readAttempts(snapshot, studentId, work)
const previous = await snapshot.get(Attempt, { id: attemptId }, work)
if (!previous) throw new Error("Attempt is missing")

using correction = ChangeSet.builder(Learning, work)
  .delete(Attempt, [previous])
  .insert(Attempt, [{ ...previous, score: 0.95 }])
  .finish()
const expected = snapshot.witness
await snapshot.close()
const corrected = await db.apply(correction, {
  ...work, expected: { kind: "exact", at: expected }
})
```

`Db.create` is explicit and refuses an existing store; `Db.open(localPath, Learning, work)` opens an existing one without creating on failure. Both are asynchronous in TypeScript. `apply` returns `accepted`, `no-change`, `invariant-rejected` with core `Violations`, or `moved`; operational failures reject with typed core `DbError`. The example rejects domain failure explicitly instead of pretending `await` implies admission. It finishes the owned correction and copies the witness, then closes the snapshot before writing to avoid needless reader/map-growth pressure; later `await using` disposal is idempotent. `corrected.kind === "moved"` means the core-local observation is stale, so read again before deciding a new correction. Neither result is a named receipt or a promise of retry deduplication after process failure.

### TypeScript log: only the durable envelope changes

```ts
import { Command, LocalHistory, HostedHistory, RequestId } from "@bjornpagen/bumbledb-log"

// Choose one constructor; bindings were explicitly initialized by the plan runner.
await using history = await HostedHistory.open(hostedBinding, Learning, hostedOptions)
// Local alternative: await using history = await LocalHistory.open(localBinding, Learning, localOptions)

const requestId = RequestId.from(Id128.random()) // Generate/retain once for this intent.
using command = Command.seal({
  scope: history.identity,
  id: { receiptEpoch: history.receiptEpoch, requestId },
  changes,
  precondition: { kind: "blind" },
  result: { attempt: attemptId }
}, work)
const submitted = await history.submit(command, submitOptions)

switch (submitted.kind) {
  case "decided":
    // Inspect receipt.outcome.kind: committed/no-change or a durable rejection.
    console.log(submitted.receipt.outcome.kind, submitted.localHealth.kind)
    break
  case "not-submitted":
  case "outcome-unknown":
    // Persist/report this original reference; resolve or retry the SAME command.
    console.log(submitted.kind, submitted.error.code)
    break
}
```

The constructor is the deployment choice; the rest is unchanged, not a dual-write recipe. `localOptions`/`hostedOptions` share the core execution policy and exact generated schema/history expectation; only hosted binding/transport/cache settings differ. The snapshot/read/submit calls have the same spelling on either history owner. `RequestId` is the log's nominal request role, explicitly constructed from the core's 128-bit value and retained for this original intent. It is not implicitly interchangeable with an entity ID and adds no allocator or second scalar codec. `submitOptions` extends the core policy only with log retry/admission/publication controls. `LogError` is simply core `DbError` or a log-only `ProtocolError`; core failures retain their original type/codes. A `decided` submission is not necessarily committed: inspect its terminal outcome. Its separate local health cannot change that outcome. The example logs tags/codes, not fact/violation/result payloads; retaining an unresolved command reference is a separate explicit application action.

After a committed/no-change receipt, the read and read-dependent correction still use the same core helper and change builder:

```ts
await using snapshot = await history.snapshot({
  ...work, consistency: { kind: "latest" }
})
const rows = await readAttempts(snapshot, studentId, work)
const previous = await snapshot.get(Attempt, { id: attemptId }, work)
if (!previous) throw new Error("Attempt is missing")

using correction = ChangeSet.builder(Learning, work)
  .delete(Attempt, [previous])
  .insert(Attempt, [{ ...previous, score: 0.95 }])
  .finish()
const expected = snapshot.stateStamp
await snapshot.close()
const correctionRequestId = RequestId.from(Id128.random()) // Retain for this new intent.
using correctionCommand = Command.seal({
  scope: history.identity,
  id: { receiptEpoch: history.receiptEpoch, requestId: correctionRequestId },
  changes: correction,
  precondition: { kind: "exact-state", at: expected },
  result: { attempt: attemptId }
}, work)
const corrected = await history.submit(correctionCommand, submitOptions)
```

`correctionRequestId` is retained for this new app intent before dispatch. The owned correction and copied log stamp survive explicit snapshot close; no reader pin is held during remote publication. Inspect `corrected` using the same certainty/terminal-outcome distinction above. Do not substitute a core `snapshot.witness`, core generation or `DecisionStamp` for the log witness. State movement records a durable `precondition-failed` receipt. Reading again and changing the correction requires a new command ID; resolving uncertainty retries the original scope/ID/digest/change instead. There is no command-row builder or replayed application callback.

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

API-04/06 cover stable entity/request IDs and lost replies; core schema/query/value tests cover F64 and capacity/composition equivalence; TS-MIG-04/07/10 cover reused core IR and preserved intermediate semantics. Creation/open/migration examples use exactly these constructors and imports. Source scans alone are not passes, and no new SDK framework or gate family is needed.
