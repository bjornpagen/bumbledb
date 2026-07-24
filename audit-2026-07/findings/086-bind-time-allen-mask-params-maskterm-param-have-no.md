## Bind-time Allen mask params (MaskTerm::Param) have no randomized or parity coverage in the differential apparatus

category: missing-free-feature | severity: low | verdict: CONFIRMED | finder: r2:differential-apparatus-soundness

### Summary

The IR's `MaskTerm::Param` — the temporal relation as a bind-time argument, with its own bind-time vacuous-mask rejection distinct from the validation-time literal one (`crates/bumbledb/src/ir.rs:320-330`: "Both surfaces reject the vacuous ∅/full masks with distinct typed errors — validation for literals, bind for params") — is fully supported by the engine and by the naive oracle, yet the bench differential apparatus never generates it, cannot translate it to SQL, and omits its bind-time errors from the error-parity roster. The only bench-apparatus exercise is one hand-picked conformance case pinning a single mask value. The finder's failure scenario overstates the exposure (engine unit tests already pin the two regression modes it cites — see Corrections), but the apparatus gap itself is real and verified.

### Evidence (all verified against the code)

- **Oracle support exists** — the naive model substitutes param masks like any param: `crates/bumbledb-bench/src/naive/query.rs:755-771` (`substitute_tree` resolves `MaskTerm::Param` to `MaskTerm::Literal` from `ParamValue::Scalar(Value::AllenMask(..))`). So randomized differential coverage is free on the oracle side.
- **querygen never emits it** — `grep -rn "MaskTerm::Param" crates/bumbledb-bench/src/querygen/` returns nothing; the Allen constructor is hardwired literal (`crates/bumbledb-bench/src/querygen/shapes_interval.rs:28-30`: `fn allen(mask: AllenMask) -> CmpOp { CmpOp::Allen { mask: MaskTerm::Literal(mask) } }`), and the coverage contract itself skips non-literal masks (`querygen/coverage.rs:362`).
- **The SQL translator refuses it** — `crates/bumbledb-bench/src/translate/builder.rs:481-486`: `let bumbledb::MaskTerm::Literal(mask) = mask else { return Err("param masks are not translated".to_owned()); }`, with the in-code admission "owed to whichever family first needs one".
- **Error parity is literal-only** — `crates/bumbledb-bench/src/verify/run_algebra.rs:377-399`: the `mask_query` helper builds `CmpOp::Allen { mask: MaskTerm::Literal(mask) }` exclusively, and the roster's `Expected::EmptyMask`/`Expected::FullMask` (run_algebra.rs:358-364) are the validation-time rejections. The bind-time `Error::EmptyAllenMaskParam` / `Error::FullAllenMaskParam` variants appear nowhere in the parity lane.
- **One hand case** — `crates/bumbledb-bench/src/conformance.rs:1409-1434` (`hand-allen-mask-param`, "the mask-param face" named in the roster comment at conformance.rs:1266-1270) binds exactly one mask value, `AllenMask::MEETS`.
- **Spec check (docs/architecture/60-validation.md, the generator feature-coverage contract, ~lines 770-808)**: Allen coverage is chartered as "named composites, all 13 singletons, and random masks (every basic reachable through some **literal** mask per run)"; params are chartered for sets, membership anchors, and interval draws — mask params are absent. So this is a hole in the charter itself, not code diverging from a promise; per the audit lens (representation-first, the coverage contract is "itself asserted"), a supported IR variant invisible to the asserted contract is exactly a missing-free-feature.
- **The plumbing the apparatus skips is real**: `crates/bumbledb/src/exec/run/execute.rs:333` resolves `MaskTerm::Param` from the bound params per execution (the executor's mask residual, `bind_allen_masks`).

### Corrections to the original finding

The finder claimed "No verify lane or differential test compares it against an oracle; only one hand conformance case ... stands between the regression and green." The second half is false: `crates/bumbledb/src/api/prepared/tests/params.rs` carries two dedicated engine unit tests — `a_mask_param_rebinds_the_temporal_relation_per_execution` (six-mask warm rebind on one prepared query, plus the bind-time `EmptyAllenMaskParam`/`FullAllenMaskParam`/`AllenMaskParamExpected`/`ParamScalarExpected` payloads matched by variant and param id) and `a_cross_atom_mask_param_resolves_into_the_executors_residual` (INTERSECTS→DISJOINT→INTERSECTS rebinding through the cross-atom mask residual). These pin precisely the two regression modes the finder hypothesized (stale mask specialization; error-payload drift). The adversarial panic sweep (`crates/bumbledb/tests/adversarial_ir.rs:199`) also draws param masks randomly, but only through validate→normalize→prepare with no execution and no oracle. What genuinely does not exist anywhere: randomized, oracle-compared execution of param masks, and bind-time vacuous-mask rows in the naive-parity roster. Severity accordingly lands at low, not medium.

### Bench/apparatus impact

A regression in bind-time mask resolution that the hand-chosen unit fixtures happen not to trip (e.g. an interaction with a plan shape only querygen reaches — recursive arms, negated gates, aggregate sinks combined with a mask residual) would pass the entire verify fleet, because every randomized Allen mask in the fleet is a literal folded at validation time. Likewise the bind-time error identities are pinned only by unit `matches!` assertions, never by the parity lane that owns error-identity drift.

### Suggested fix

1. Add a mask-param draw to the interval shapes: an `AllenMask` arm in `querygen/oracle.rs`'s anchor machinery beside the existing typed param draws, and a `MaskTerm::Param` branch in `shapes_interval.rs`'s `allen()` path (drawing non-vacuous masks).
2. Route the resulting queries to the naive lane (the SQL translator's refusal at builder.rs:481 makes them SQL-inexpressible; per 60-validation.md the inexpressible set must be "enumerated in the harness, never silently skipped" — or teach the translator to substitute the bound mask per execution like a set param, as its own comment sketches).
3. Add two bind-time rows to `parity_cases` in `verify/run_algebra.rs` beside the literal `EmptyMask`/`FullMask` ones, expecting `Error::EmptyAllenMaskParam` / `Error::FullAllenMaskParam` at execute.
4. Extend the generator's asserted coverage contract (and its 60-validation.md charter paragraph) to name the mask-param cell, so the gap cannot silently reopen.
