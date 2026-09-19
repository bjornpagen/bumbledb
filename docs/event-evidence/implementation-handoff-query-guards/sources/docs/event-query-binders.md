# Scoped Event fixed points in queries

A query can now solve a finite monotone Event equation using row-bound inputs.
For example, `X = Goal ∪ May(Edges, X)` starts with no states, adds the goal and
its predecessors, and stops at the exact region from which the goal is reachable.
The answer is an ordinary Event: later queries can intersect it, complement it,
Pack it, compare it through dependencies, or observe its probability.

Least and greatest fixed points may nest and refer to outer predicates. Relation
regions can be bound too: `X = Id ∪ Compose(Edges, X)` constructs closure. These
are operator trees inside a computed query head. Ordinary relational recursion
and the Rust macro grammar retain their existing contracts.

## Authoring and transport

Rust exposes the inspectable IR:

```rust
use bumbledb::{EventExpr as E, EventScope, FixedPointKind, PredicateDepth};
use bumbledb::event::{BoolOp4, ModalOp};

let scope = EventScope::capture(&states, &work)?;
let reachable = E::FixedPoint {
    kind: FixedPointKind::Least,
    scope,
    body: Box::new(E::Apply {
        op: BoolOp4::OR,
        left: Box::new(E::Var(goal_variable)),
        right: Box::new(E::Modal {
            operation: ModalOp::May,
            relation: Box::new(edges_expression),
            input: Box::new(E::Bound(PredicateDepth(0))),
        }),
    }),
};
// Use FindTerm::Event(reachable) in an ordinary ir::Query.
```

`PredicateDepth(0)` is the nearest enclosing fixed point; depth 1 is its parent.
It is distinct from a `VarId`, which names a database binding. A query still needs
its ordinary positive body. A captured scope lets the head omit row Event inputs.
The proposal's `Least(X, …)` notation is not new Rust macro syntax.

The TypeScript SDK closes lexical handles during pure authoring:

```ts
const reachable = EventExpr.least(fullStateEvent, x =>
    EventExpr.or(row.goal, EventExpr.may(edges, x)))
const safe = EventExpr.greatest(fullStateEvent, x =>
    EventExpr.and(row.safe, EventExpr.all(edges, x)))
const recurring = EventExpr.greatest(fullStateEvent, x =>
    EventExpr.least(fullStateEvent, y =>
        EventExpr.or(
            EventExpr.and(row.goal, EventExpr.may(edges, x)),
            EventExpr.may(edges, y))))
```

Callbacks run once to build owned query data, never during native iteration.
Distinct temporary handles close to lexical depths, including references through
nested maps and relations. Escaped handles refuse at query lowering; a detached
inner expression cannot capture a different outer binder later. Queries and
query descriptions own their scope bytes. Exporting or mutating an inspection
copy does not change a prepared expression.

The strict wire forms are `{ kind: "bound", depth }` and
`{ kind: "fixed", op: "least" | "greatest", scope, expr }`.
`scope` is full-space BEVT bytes. Shape parsing owns the payload; the worker
reconstructs the context and verifies lexical scope, type and monotonicity before
execution, including when the query has no matching rows. Descriptions contain
no callbacks or temporary authoring handles.

## A sealed context and a monotone equation

`EventScope` owns the full original context and its finite logical-cell count.
Its canonical bytes retain support, source identity, designated law and parameter
guards. A partial Event cannot supply a scope. There is no operation inside the
binder that creates a fresh source, adds guards or changes the original support.
Shared unknown parameters therefore remain shared. A cell may denote infinitely
many actual parameter assignments; the bound counts representable logical cells.
Zero-mass worlds remain in that count and in the output.

The compositional variance checker requires each body to be independent of or
increasing in its own predicate. It reuses the core truth-table analysis for
Boolean operations and ITE. Maps, composition, May, Post and Star preserve
inclusion. All reverses the direction of its relation operand; residuals reverse
the constrained operand. Must includes both enabledness and universal outcomes,
so a changing relation is generally mixed. At-least counts are increasing;
at-most counts reverse direction; proper count windows are generally mixed.
Constant/impossible cardinality bounds are recognized after scope checks.

Every nested body must pass independently. Its solution preserves the body's
variance in each outer predicate. The analysis is conservative: `Ite(X,G,G)` may
be refused even though rewriting it to `G` establishes independence. This is a
refusal to certify monotonicity, not an assertion that the expression is wrong.
A negative inner body cannot hide under a constant outer Boolean operation.

## Free Join, diagnostics and execution

Free Join finds complete row bindings as usual. Admission visits every written
external Event leaf once, including repeated variables and operands beneath
constant truth functions. Context faults retain the ordinary stage, rule, head,
occurrence and variable coordinates. Bound predicates are not diagnostic row
operands. No iteration or simplification can erase an external participant.

The evaluator holds a lexical stack of owned Events. Each body application
restarts the same external occurrence roster while updating its bound predicate.
After stabilization, the caller resumes after that roster; a subsequent sibling
or Probability evidence expression consumes its own operands normally. Nested
bindings push and unwind their own stack entries, including on failure.

Least iteration starts empty; greatest starts full. Canonical equality detects
stabilization after alignment to the sealed carrier. Each changing step must
respect the admitted inclusion direction. At most `atoms + 1` applications are
needed to detect a fixed point; failure to honor that contract is an invariant
error. Nested binders and Star share the default operational budget for the whole
computed head on one row binding: 1,000,000 applications and 20,000,000 interpreter/
program steps. Separate heads/bindings have separate budgets. Kernel limits and
query cancellation also apply. Exhaustion publishes no approximant.

This integrates solving with the current complete-binding Free Join path. It
adds no certified planner factoring or new performance claim. The interpreter
may recompute invariant subexpressions. The general engine retained-byte policy
and host program/strategy/memory descriptor transport remain separate work.

## Evidence and limits

Five native integration tests include all 16 two-state relations × 4 goal regions
on both resident and cursor Free Join, using an independent matrix iteration
oracle. They cover alternating binders, relation closure, detached owners,
complete occurrence faults, static refusal on absent rows, variance directions,
fixed/univariate source scopes and final Probability composition. Unit tests
exercise cumulative nested/Star limits and cancellation at multiple depths.
SDK tests cover hygienic authoring, owned descriptions, stages, Pack and tests;
raw Node and parser fixtures reject malformed scopes, depths and oversized imports.
The 4,096-node, 128-depth and 16 MiB combined descriptor/scope limits apply.

`QueryBinders.lean` adds 21 reference reports: extremal solutions, covariance of
nested solutions, lexical extension and the relation-operand variance laws.
`FixedPoint.lean` separately proves finite stabilization and successful bounded
detection; `Programs.lean` proves the conservative Boolean variance calculus.
These are semantic references, not verification of the Rust interpreter, JS
closing algorithm, codecs or resource accounting.

The [qualification](event-evidence/native-query-binders-qualification/check.json),
[semantics](event-evidence/native-query-binders-semantics/check.json) and
[handoff](event-evidence/implementation-handoff-query-binders/check.json) retain
the checked source snapshots. General observation staging/arithmetic, remaining
source/solver/import work, complete Coup consumers and integrated performance
remain M0–M8 obligations. No release, tag or version bump is made here.
