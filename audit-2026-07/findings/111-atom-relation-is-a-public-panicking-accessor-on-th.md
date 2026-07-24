## Atom::relation() is a public panicking accessor on the pure-data IR; the Option form sits directly below it

category: inappropriate-branching | severity: low | verdict: CONFIRMED | finder: lean:query

### Summary

`Atom` is part of the pure-data Rust IR that hosts construct freely (re-exported from the crate root, `crates/bumbledb/src/lib.rs:158-162`, per the no-text-query-language doctrine). Yet `Atom::relation()` (`crates/bumbledb/src/ir.rs:92-97`) is a public accessor that panics via `unreachable!("caller asserted a stored-relation (Edb) atom")` on the `Idb` arm. The `unreachable!` claim is false at the public boundary: `Atom { source: AtomSource::Idb(..) }` is a legal value any host can hold. The parse-don't-validate form already exists one screen down — `AtomSource::edb(self) -> Option<RelationId>` (`ir.rs:103-108`, `#[must_use]`) — so the panicking twin is a branch re-encoding a caller assertion the type system already carries.

The Lean model agrees with the Option shape: `lean/Bumbledb/Query/Syntax.lean:384-388` defines only `AtomSource.idb?` (Option-valued), and the `PAtom` doc comment at Syntax.lean:390-394 explicitly records **"the design's `Atom.relation → Atom.source` cut"** — the model deliberately retired the total `relation` accessor when the source position widened to a sum. The Rust panicking accessor reintroduces exactly the observer the modeled cut removed.

### Evidence (verified)

- `crates/bumbledb/src/ir.rs:92-97` — the accessor:
  ```rust
  pub fn relation(&self) -> RelationId {
      match self.source {
          AtomSource::Edb(relation) => relation,
          AtomSource::Idb(_) => unreachable!("caller asserted a stored-relation (Edb) atom"),
      }
  }
  ```
  Its own doc comment (ir.rs:82-90) concedes the premise is per-caller: "for consumers whose atoms are stored-relation-only by construction (the bench harness's generators and oracles...)".
- `crates/bumbledb/src/ir.rs:100-119` — `AtomSource::edb` / `AtomSource::idb` Option accessors directly below.
- `crates/bumbledb/src/lib.rs:158-162` — `Atom` and `AtomSource` publicly re-exported from the crate root.
- `lean/Bumbledb/Query/Syntax.lean:384-394` — only `AtomSource.idb?` is modeled; the PAtom comment names the `Atom.relation → Atom.source` cut this accessor reverses.
- **Caller audit (correction to the original finding):** there are *zero* in-crate callers of `Atom::relation()`. Every call site is in `bumbledb-bench` (conformance.rs:710/781/795/801/830/1108, querygen/coverage.rs, querygen/oracle.rs, querygen/builder.rs:41, querygen/contradict.rs:34, querygen/dress.rs:186, naive/query.rs:621) — all Edb-only generators/oracles. The sites the finding cited as in-crate callers (exec/dispatch/classify.rs:129, plan/selectivity.rs:163, api/prepared/run_join.rs:125) call the *separate* `Occurrence::relation()` / `PlanOccurrence::relation()` twins (ir/normalize.rs:159, plan/fj.rs:224) — same panicking pattern, but on `pub(crate)` types (`ir.rs:9: pub(crate) mod normalize`) and each behind a real prior Idb refusal (classify.rs:94 gates via `occurrence.source.edb()?`; run_join.rs:115-124 `continue`s on the Idb arm before line 125).

### Failure scenario

A host assembling or introspecting a recursive `Program` calls `atom.relation()` on an `Idb` atom — plain data it legally constructed — and the process aborts with "internal error: entered unreachable code" instead of receiving an `Option`/typed refusal. No current caller trips it (all bench-side callers are Edb-only by construction), so severity stays low; the defect is the public API shape, not a live crash.

### Suggested fix

Delete `Atom::relation()` in favor of `atom.source.edb()` (already `#[must_use] Option<RelationId>`), migrating the bumbledb-bench call sites — the only users. If the bench harness's Edb-only ergonomics matter, host the convenience accessor in the bench crate (or an extension trait there), where its by-construction premise actually holds. This restores the Lean-modeled `Atom.relation → Atom.source` cut and removes the one panicking branch on the public pure-data IR. The crate-internal `Occurrence::relation()` / `PlanOccurrence::relation()` twins are a lesser instance of the same pattern (guarded, non-public) and could follow the same treatment opportunistically.
