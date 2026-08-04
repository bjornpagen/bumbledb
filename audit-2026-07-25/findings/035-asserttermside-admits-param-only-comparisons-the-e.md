## assertTermSide admits param-only comparisons the engine convicts as ConstantComparison — the TS wall mirrors the wrong predicate

incoherence | low | CONFIRMED | ts-surface-fresh
outcome: fixed 9241feb3

### Summary

`assertTermSide` (ts/src/query/atom.ts:357-363) documents itself as failing "with the same verdict" as the engine's constant-valued-comparison refusal, but its predicate is `isTerm` on either side — and params ARE terms (scope.ts:149-151 admits any `term`-tagged object; `Param` is tagged `"param"` at scope.ts:105-108). The engine's shape pass requires a `Var` or `Measure` side: the catch-all arm at crates/bumbledb/src/ir/validate/context.rs:893-898 convicts `(Param|ParamSet|Literal, Param|ParamSet|Literal)` as `ValidationError::ConstantComparison`. So `lt(r.param("lo"), r.param("hi"))`, `lt(r.param("p"), 2n)`, and `pointIn(param, param)` all pass the wall, and the refusal lands at prepare — either as the TS anchor error or as an engine error — never at the constructor the doc points to.

### Evidence (verified)

- ts/src/query/atom.ts:352-356 — doc: "Rejects a comparison with no term side: it is constant-valued, the engine's own validation refuses it … fail here with the same verdict." atom.ts:358 tests `!isTerm(lhs) && !isTerm(rhs)`. The message at atom.ts:360 even says "without a variable **or parameter** side" — the wrong predicate encoded in prose.
- ts/src/query/scope.ts:149-151 — `isTerm` is `term in value`; Param/SetParam/MaskParam/Duration all qualify.
- crates/bumbledb/src/ir/validate/context.rs:747-899 — the full shape match read arm by arm: every `Ok` arm has a `Var` or `Measure` side; 893-898 is the ConstantComparison catch-all over param/set/literal pairs. `(Param, Param)` and `(Param, Literal)` both land there.
- Runtime repro (dist build, v0.9.0): `lt(makeParam('lo'), makeParam('hi'))` → **constructed** (`{"cond":"cmp","op":"lt","lhs":{"name":"lo"},"rhs":{"name":"hi"}}`); `lt(makeParam('p'), 2n)` → constructed; `pointIn(param, param)` → constructed; only `lt(1n, 2n)` throws the BAN.
- End-to-end, both predicted failure sites reproduced:
  - Unanchored params: `lowerQuery` throws `query param lo has no field-anchored use — bind it in an atom or compare it against a bound variable` (ts/src/query/lower.ts:1970-1973). The param-param comparison records both uses with `anchor: undefined` (lower.ts:633-654).
  - Params anchored in atom bindings: the query lowers clean — `lowerCmpTerm` (lower.ts:1794-1795) ships `(Param, Param)` IR — and `db.prepare` throws the engine error `bumbledb irError (prepare): comparison 0: neither side is a variable`.
- Supporting incoherence: `cmpAnchorOf` (lower.ts:1820-1822) types a comparison literal by an anchored-**param** sibling — a branch only reachable for `(Param, Literal)` comparisons the engine convicts anyway, i.e. the lowering also believes param-literal comparisons are legal.

### Failure scenario / impact

A parameterized guard written `where(lt(r.param("lo"), r.param("hi")))` compiles (OrderSide includes `Param<string>`) and constructs. The refusal arrives at prepare, as either "query param lo has no field-anchored use" (if the params anchor nowhere else — a misleading message: comparing against a bound variable is not what the user wrote wrong) or the engine's "comparison 0: neither side is a variable" (if they anchor in atom bindings) — far from the call site the construction wall exists to catch. Severity low: the query is still refused, correctness holds; the defect is verdict placement and message.

### Suggested fix

Tighten `assertTermSide` to the engine's predicate: at least one side must be a var or duration term (`isTerm(x) && (x[term] === "var" || x[term] === "duration")`); params count as constants exactly as the engine counts them (context.rs:893-898). Fix the message to say "variable or measure side". Consider deleting the now-unreachable anchored-param branch of `cmpAnchorOf` (lower.ts:1820-1822) in the same change, and land the test pinning `lt(r.param("a"), r.param("b"))` and `pointIn(param, param)` to the constructor throw.