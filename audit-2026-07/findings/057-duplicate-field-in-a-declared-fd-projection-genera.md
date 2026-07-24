## Duplicate field in a declared FD projection dies as rustc E0124 in generated code instead of a spanned macro diagnostic

category: bug | severity: medium | verdict: CONFIRMED | finder: macros:core
outcome: fixed 8d05568e

### Summary

`schema!` emits one Rust key struct per declared functional-dependency statement, with one `pub` field per projected name. Neither the macro's key-struct emitter nor the shared spec lowering deduplicates the projection, so `Task(kind, kind) -> Task;` generates `pub struct TaskByKindKind { pub kind: KindId, pub kind: KindId }` and rustc rejects the expansion with `error[E0124]: field 'kind' is already declared`, spanned at the **entire** `schema!` invocation. The engine owns this judgment as a typed error — `SchemaError::DuplicateProjectionField` at `Db::create` — but for the macro surface that path is unreachable: compilation dies first. This breaks the macro's own documented diagnostic contract (every name-resolution issue lands as a `compile_error!` at the offending token; everything semantic surfaces as the typed `SchemaError`) — a duplicate projection field gets neither.

### Evidence (all verified against the working tree)

- `crates/bumbledb-macros/src/lib.rs:2604-2614` — `emit_key_struct` maps each projection entry to its relation field with `find(...).expect(...)`; a duplicated name resolves twice with no dedup.
- `crates/bumbledb-macros/src/lib.rs:2634-2642` — the struct-field loop writes `pub {name}: {ty},` once per entry, producing the duplicate field.
- `crates/bumbledb-theory/src/schema/spec.rs:869-874` — the shared lowering's `StatementSpec::Fd` arm resolves each projected field independently and pushes all of them; duplicates pass through silently. The `SpecIssue` enum (spec.rs:250-309) has no duplicate-projection variant (roster: UnknownRelation, UnknownField, NotAHandleField, UnknownHandle, RowArityExcess, DuplicateHandleNewtype, WindowInverted, WindowExactRespelled), so the macro's spanned-`compile_error!` machinery cannot fire for this case.
- `crates/bumbledb-macros/src/lib.rs:1443-1456` — `lower_statements` also passes the duplicated projection through unchanged (spans are recorded per field, so the offending token's span IS available).
- `crates/bumbledb-macros/src/lib.rs:1071` — the whole emission is string-parsed (`out.parse().expect("schema!: generated code parses")`), so generated tokens carry call-site spans; the E0124 points at the full invocation. (The original finding cited line 2071; the correct parse site is 1071.)
- `crates/bumbledb/src/schema/validate.rs:1035` and `crates/bumbledb/src/error.rs:262-268` — the engine's typed judgment (`FieldSet::new` → `SchemaError::DuplicateProjectionField`) exists and is tested at `crates/bumbledb/src/schema/tests/reject.rs:391`, but only via a hand-built descriptor — the macro path never reaches `Db::create`.
- `crates/bumbledb-macros/src/lib.rs:114-124` — the contract: grammar/literal issues are expansion errors at the call site, every `SpecIssue` is a spanned `compile_error!`, and "everything semantic beyond names surfaces as the typed `SchemaError` from `Db::create`/`Db::open`".
- Live reproduction: a test file containing `bumbledb::schema! { pub T; relation Task { kind: u64 as KindId, subject: u64 } Task(kind, kind) -> Task; }` fails `cargo test --no-run` with `error[E0124]: field 'kind' is already declared`, the span covering lines 1-10 of the invocation, plus "this error originates in the macro `bumbledb::schema`". No statement or field is identified.
- No compile-fail test covers this: `crates/bumbledb-macros` has no `tests/` directory and no trybuild dependency.

### Failure scenario

A user declares an FD with a repeated determinant field (a plausible typo when a relation has similarly named fields). Instead of a diagnostic naming the statement and the duplicated field, they get a rustc error about a struct field they never wrote, pointing at the whole schema block, with no hint that the FD projection is the offender. In a large schema with many statements this is a hunt.

### Design note (representation-first lens)

The doc contract splits judgments cleanly: names → spanned expansion errors; semantics → typed `SchemaError`. Duplicate projection is a name-shape issue the macro itself reifies into a Rust item, so it must be judged before the item is minted — the parse already carries `projection: Vec<(String, Span)>` (lib.rs:2600, 1443-1448), i.e. the representation needed for a precise diagnostic exists and is discarded.

### Suggested fix

Detect the duplicate where the span is in hand: in `lower_statements` (lib.rs:1443) or at `emit_key_struct` entry, scan the projection for a repeated name and emit a spanned `compile_error!` at the second occurrence's `Span`, phrased in the FD form's vocabulary (e.g. "`kind` appears twice in the determinant of `Task(kind, kind) -> Task`"). For strict macro/runtime agreement (the "ONE shared lowering" doctrine), the cleaner variant is a new `SpecIssue::DuplicateProjectionField { statement, field }` raised in the FD arm of `SchemaSpec::descriptor` (spec.rs:869-874), which the existing span table already knows how to place — and which also upgrades the runtime spec path from deferred `Db::create` rejection to lowering-time rejection. Add a compile-fail (trybuild) test either way.
