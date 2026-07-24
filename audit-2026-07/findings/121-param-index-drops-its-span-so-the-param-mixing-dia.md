## Param::Index drops its span, so the param-mixing diagnostic lands at call_site

category: inelegance | severity: low | verdict: CONFIRMED | finder: query:crates

### Summary

The `query!` macro's module doc promises span-carrying diagnostics ("every parse error points at its token", `crates/bumbledb-query-macros/src/lib.rs:106-107`), and every other surface-AST node carries a `Name { text, span }` (:168-172). `Param::Index(u16)` (:176-179) is the lone node that discards its token's span. Consequence: when a positional `?0` follows a named `?window`, the mixing refusal in `Params::resolve` has no span to point at and falls back to `Span::call_site()` (:1090-1091), so the error underlines the entire macro invocation — while its exact twin, the named-after-positional refusal, points at the offending `?name` token via `name.span` (:1069-1075). This is a representation gap, not a control-flow bug: the span exists at parse time and the type throws it away.

### Evidence (verified)

- `crates/bumbledb-query-macros/src/lib.rs:176-179` — `enum Param { Named(Name), Index(u16) }`; `Name` (:168-172) is `{ text: String, span: Span }`.
- `crates/bumbledb-query-macros/src/lib.rs:1088-1095` — the Index arm of `Params::resolve`:
  ```rust
  Param::Index(index) => {
      if self.saw_named {
          return fail(
              Span::call_site(),
              "query!: named and positional ?params cannot mix — ...
  ```
- `crates/bumbledb-query-macros/src/lib.rs:1068-1075` — the Named arm spans `name.span` for the identical mixing error, and again at :1086 for the too-many-params error. The asymmetry is real and one-sided.
- `crates/bumbledb-query-macros/src/lib.rs:497-506` — `parse_param` binds `let span = lit.span();` (:498), uses it only for the malformed-index error (:500-503), then builds `Param::Index(index)` and drops it. The fix's raw material is already in hand.
- Grep of the crate: `Param::Index` has exactly two sites — construction (:506) and consumption (:1088) — so carrying the span touches two lines plus the variant.
- Test coverage: no compile-fail test exercises the param-mixing diagnostic at all. The only "cannot mix" UI test (`crates/bumbledb-query/tests/compile-fail/mixed_predicate_bindings.rs`) covers the bare-ident/indexed-label mixing error (:1200), a different diagnostic. Nothing pins the current call_site behavior, so the fix is unblocked.

### Failure scenario

In a multi-rule query — `query!(T { (x) | A(f: x), x == ?limit; (x) | B(g: x), x == ?0; })` — the mixing refusal is emitted at the macro's call site, underlining the whole invocation. In a large query with many rules and several params, the user must hunt for which token is the positional offender; the symmetric case (named after positional) points at the exact token.

### Suggested fix

Carry the span in the variant — `Index { value: u16, span: Span }` (or `Index(u16, Span)`), constructed in `parse_param` from the `span` already bound at :498, and spanned in the :1090 `fail` call. This is the representation-first fix per docs/design/representation-first.md: the diagnostic degradation is a state the type permits; widening the variant erases it, and only two sites change. Optionally add a compile-fail test for the mixing error in both directions to pin the span.
