# PRD 15 — ParamSpec: three vectors become one

**Depends on:** baseline (independent; if 12 landed, the fields live on
the same `PreparedQuery` either way).
**Modules:** `crates/bumbledb/src/api/prepared.rs` (the three fields),
`api/prepared/build.rs` (their lockstep construction),
`api/prepared/bind.rs` (`bind_scalar_slot`, `bind_set_slot`),
`api/prepared/tests/params.rs` + `sets.rs`.
**Authority:** the audit's finding 6 (~8 sites): `param_types:
Vec<ParamShape>` ∥ `param_is_set: Vec<bool>` ∥ `param_is_point:
Vec<bool>` are three parallel vectors whose agreement is asserted at bind
("validated: a mask param is never a set") and cross-indexed at every
slot. `ParamShape`'s own doc already states the doctrine ("the two-variant
sum keeps the untyped placeholder unrepresentable") and stops one step
short.
**Representation move:** one spec sum per param slot. The mask-set
contradiction, the set-ness bool, and the point-ness bool all become
structure.

## Context (decided shape)

```rust
/// One param slot's complete bind-time contract — dense by ParamId,
/// sealed at prepare from validation's recording.
enum ParamSpec {
    /// A scalar slot: expected element type; `point` = element-typed at
    /// an interval position (the domain-ceiling rejection applies).
    Scalar { ty: ValueType, point: bool },
    /// A set slot: expected ELEMENT type; `point` as above, per element.
    Set { elem: ValueType, point: bool },
    /// An Allen-mask slot: no ValueType, no point-ness, never a set —
    /// the contradictions are now unrepresentable.
    Mask,
}
```

`param_types`/`param_is_set`/`param_is_point` →
`params: Vec<ParamSpec>`. `ParamShape` (Value|AllenMask) dies into the
sum. Dying control flow: `bind_scalar_slot`'s set-check-then-shape-match
becomes one match (`Set{..} => Err(ParamSetExpected)`, `Mask => `the
vacuity checks, `Scalar{ty, point} =>` convert + ceiling check);
`bind_set_slot`'s `param_is_set` check and its "validated: a mask param
is never a set" unreachable die the same way; the point-ness reads at
`bind.rs` (scalar ceiling, set-element ceiling) become field reads on the
matched variant. `begin_bind`'s count check reads `params.len()`.

Construction: `build.rs`'s dense-param derivation (from validation's
`RuleWitness` param recording) emits `ParamSpec` directly — the three
lockstep pushes become one.

Error surface unchanged: `ParamCountMismatch`, `ParamTypeMismatch`
(carries `expected: ValueType` — sourced from the Scalar/Set variant),
`ParamSetExpected`, `ParamScalarExpected`, `ParamElementTypeMismatch`,
`AllenMaskParamExpected`, `Empty/FullAllenMaskParam`,
`PointParamAtCeiling` — same errors, same positions, same payloads.

NOT touched (set refusal, recorded in the README): `resolved_params` /
`missed_params` — the bind-OUTPUT pools are a different mechanism with
their own recorded rationale.

## Technical direction

1. Land `ParamSpec` in `api/prepared.rs`; delete `ParamShape` and the
   two bool vectors.
2. `build.rs`: one construction site; the validation-side recording
   (`RuleWitness`'s param facts) is read-only input — do not reshape
   validation.
3. `bind.rs`: rewrite the two slot binders as single matches per the
   decided shape; `convert_scalar` keeps its signature (it takes the
   expected `ValueType` — now sourced from the variant).
4. Tests: `params.rs`/`sets.rs` assert the same typed errors at the same
   positions — re-anchor any test constructing the old triple directly;
   add the one new pin: a mask param bound with a set arg errors
   `ParamScalarExpected`-or-`AllenMaskParamExpected` (whichever the
   current behavior is — pin CURRENT behavior exactly; this PRD changes
   no verdict).

## Passing criteria

- `[shape]` `grep -rn "param_is_set\|param_is_point\|ParamShape" crates`
  → zero hits; `grep -n "never a set" crates/bumbledb/src/api/prepared`
  → zero hits.
- `[shape]` `bind_scalar_slot` and `bind_set_slot` each contain exactly
  one match on the slot's `ParamSpec` and no cross-vector indexing.
- `[test]` params/sets suites green with unchanged error assertions; the
  mask-set pin of direction 4.
- `[gate]` Workspace gates green at campaign close — and with this PRD
  the set's terminal condition: the full gate suite
  (`fmt`/`clippy`/`test`/`check.sh`) green across the whole workspace,
  plus the campaign-close reconciliation in the commit body — the
  engine-crate `unreachable!`+`expect` census against the audit baseline
  (201 + 353), with the delta itemized per PRD.

## Doc amendments (rule 5)

`70-api.md` § params (if it names the shape recording): one sentence —
each param slot carries one sealed spec; set-ness, point-ness, and
mask-ness are structure, not parallel flags.
