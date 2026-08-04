## parse_bound commits to the Duration-measure form without peeking for the paren group, refusing a dependent bound on a field named `Duration` that both runtime surfaces accept

inappropriate-branching | low | CONFIRMED | cross-branching-new
outcome: fixed 5eb216de

### Summary

In the `schema!` capacity-window grammar, `parse_bound` treats the bare ident `Duration` as the start of the interval-measure form `Duration(field)` unconditionally — it calls `take_group` for a parenthesis group without peeking first. A dependent bound naming a TARGET field literally called `Duration` (a legal field ident) therefore panics with the misleading message `expected the Duration bound's field, found None`. Its twin `parse_weight` guards the exact same ambiguity with a peek, and both runtime surfaces (the `SchemaSpec` wire tier and the TS SDK) accept the identical statement — a three-surface expressibility split on one spelling.

### Evidence (all verified against v0.9.0 on `bugbash-perf`)

- **The unguarded branch** — `crates/bumbledb-macros/src/lib.rs:1006-1019`:
  ```rust
  fn parse_bound(tokens: &mut Tokens, what: &str) -> BoundSpec {
      if matches!(tokens.peek(), Some(TokenTree::Ident(_))) {
          let (name, _) = spanned_ident(tokens, what);
          if name == "Duration" {
              let group = take_group(tokens, Delimiter::Parenthesis, "the Duration bound's field");
              ...
              return BoundSpec::Duration(field.into());
          }
          return BoundSpec::Field(name.into());
  ```
  No peek before `take_group` — the ident alone commits the parse.
- **The twin that does it right** — `lib.rs:1046`: `parse_weight`'s arm is `if name == "Duration" && matches!(tokens.peek(), Some(TokenTree::Group(_)))`, so `[Duration]` as a weight parses as `WeightSpec::Field("Duration")` while `{0..Duration}` as a bound cannot.
- **`Duration` is a legal field name** — `lib.rs:373-385`: `reject_deleted_word` bans only `unique`/`fk`/`enum`; `parse_relation` (lib.rs:390-408) accepts any other ident.
- **Empirical repro (macro tier)** — a test schema with target field `Duration: u64` and statement `Pool(id) <=[watts]{0..Duration} Device(pool);` fails compilation:
  ```
  error: proc macro panicked
    = help: message: schema!: expected the Duration bound's field, found None
  ```
- **Empirical repro (wire tier)** — the identical theory built as a `SchemaSpec` with `hi: BoundSpec::Field("Duration".into())` passes `spec.descriptor()` (test ran green). The resolver at `crates/bumbledb-theory/src/schema/spec.rs:876-892` resolves `BoundSpec::Field` purely by name against the target's sealed roster — no reserved word.
- **TS tier** — `ts/src/capacity.ts:439-441`: `ref()` applies only `PathBan` (no dot) and `assertRowLocal`, so `ref("Duration")` mints a `FieldRef` fine.

### Failure scenario / impact

A schema declares a TARGET relation with a u64 field named `Duration` (legal everywhere) and states a dependent bound on it, e.g. `Pool(id) <=[watts]{0..Duration} Device(pool);`. The macro panics at parse time with a message pointing at nothing the author wrote wrong, while the same statement submitted through the TS SDK or a `SchemaSpec` document validates and runs. (Note: the original finding's scenario placed the `Duration` field on the SOURCE relation; the dependent bound resolves against the TARGET roster — spec.rs:870-892, ruled C1 — but the macro panic fires at parse, before any resolution, so the split is real either way.) Compile-time only, no runtime consequence; severity low, but it is a genuine grammar/misleading-diagnostic defect and a cross-surface expressibility inconsistency.

### Suggested fix

Mirror `parse_weight`'s guard in `parse_bound`: treat `Duration` as the measure form only when a `TokenTree::Group` (parenthesis) immediately follows, otherwise fall through to `BoundSpec::Field(name)` — or extract the shared `ident-or-Duration(field)` parse that both call, so the disambiguation rule exists once. Land with a test pinning `{0..Duration}` on a target that declares a `Duration: u64` field (both the macro parse and the descriptor lowering).