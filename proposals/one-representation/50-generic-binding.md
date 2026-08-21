# 50 — The generic binding law: `v(R)` is the full binding of `R`, stated once

This lawful expression compiles for every concrete relation and fails for a
generic member of the schema (V7):

```ts
const bindings = v(Relation)
rules.match(Relation, bindings)
// TS2345: Type 'VarsOf<R>' is not assignable to 'MatchShape<MatchFields<R>>'
```

Hosts writing schema-generic helpers (Primer's per-relation count queries)
carry a localized type suppression for it. The defect is in the SDK's type
representation of a law that holds by construction; the fix is to state the
law as a signature, not to weaken any judgment.

## Root cause, verified by repro (tsc 7.0.2 against the real SDK types)

Three probes were run; their results are the ruling's evidence and are
reproduced as pinned type tests below.

- **Probe A — the reported failure, isolated.** `VarsOf<R>`
  (`ts/src/query/scope.ts:146`) maps over `keyof MatchFields<R> & string`;
  `MatchShape<F>` (`ts/src/query/atom.ts:241`) maps over bare `keyof F`.
  For generic `R`, `MatchFields<R>` is a deferred conditional type, so tsc
  must relate two *generic* mapped types through their key constraints —
  and it cannot prove `keyof MatchFields<R>` assignable to
  `keyof MatchFields<R> & string` (an unresolved `keyof` may contain
  `number`/`symbol`). The error chain bottoms out at literally
  `Type 'number' is not assignable to type 'string'`. Concrete relations
  evaluate to string-literal keys, hence the concrete/generic asymmetry.
- **Probe B — the "small structural fix" is refused.** Aligning the key
  domains (a `MatchShape` variant mapping over `keyof F & string`) still
  fails: the template relation
  `Var<MatchFields<R>[K], R["name"], K>` →
  `BindingInput<MatchFields<R>[K]>` cannot be proved for generic `R`
  (`BindingInput` is a union of conditionals over the unresolved field
  type, and `MatchFields<R>[K] extends AnyField` is not derivable). And
  behind both stands a third blocker: `CheckBindings`/`JoinOk` must
  resolve the class record by the literal relation name
  (`ClassRecordOf<Classes, R["name"]>`), unresolvable for generic
  `R["name"]`. **Gravestone:** do not attempt to make the general
  signature generically self-evident — three independent deferrals stand
  in the way, and loosening any of them weakens concrete checking.
- **Probe C — identity instantiation holds.** A signature typed
  `bindings: VarsOf<R>` admits `v(rel)` for generic `R` with zero
  conditional evaluation: `VarsOf<R>` unifies with `VarsOf<R>` by
  identity. This compiles today with no SDK change — the signature just
  does not exist yet.

## The ruling

Each of the six `match` sites in `ts/src/query/lower.ts` —
`QueryRuleScope`, `QueryRuleChain`, `InteriorRuleScope`,
`InteriorRuleChain`, `RecRuleScope`, `RecRuleChain` — gains the
full-binding form, declared **before** the general form:

```ts
/** The full binding: every column of R bound to its own v(R) mint — the identity atom. */
match<R extends QueryRelation<Rels>>(
    relation: R,
    bindings: VarsOf<R>
): QueryRuleChain<Rels, Record<never, never>, Classes>          // scope forms
): QueryRuleChain<Rels, P, Classes>                              // chain forms (P unchanged)
```

A `VarsOf<R>` record contributes no params (every position is a variable),
so the scope forms return the paramless chain and the chain forms pass `P`
through — `BindParamsShape` of an all-var record is exactly
`Record<never, never>` today; the signature states the computed fact
instead of recomputing it.

**This is one way, not two.** The utterance is unchanged —
`r.match(relation, bindings)` — and the added form admits *exactly* the
value the concrete path already admits, with identical runtime semantics
(the runtime iterates own enumerable properties; nothing branches on which
signature matched). It is the type-level statement of an invariant `v()`
already enforces at mint: every variable's mint slot **is** its position
slot (same owner, same column), so the join judgment `CheckBindings`
exists to make is discharged by construction — there is nothing left for
it to judge on this shape. Everything else — partial records, literals,
params, mixed records, a full record carrying another relation's vars
(refused by the `owner: { name: R["name"] }` and `column: K` literals) —
fails `VarsOf<R>` structurally and falls through to the general signature,
where `CheckBindings` judges precisely as today.

**Deliberate exclusion:** `not()` does **not** gain the form. A negated
atom's variables must be positively bound elsewhere in the rule; a
full-fresh-var negation is a boundness refusal at construction, and a
signature that blesses the spelling generically would type-admit a
guaranteed construction error. Recorded here so it is a ruling, not an
omission.

**Host-facing bound:** the owner bound for generic helpers is the SDK's
own exported `QueryRelation<Rels>` (already public via `#index.ts`).
Primer's helper failed first on its hand-rolled
`Extract<Rels[keyof Rels], AnyRelation>` (which omits closed owners) —
the SDK name is the one spelling of "a relation this schema can match".

## Pinned type tests (`ts/test/`, alongside the law-typing suite)

1. **The Primer shape compiles:** a schema-generic
   `countQueryOf<Rels, R extends QueryRelation<Rels>>(theory, rel)`
   building `r.match(rel, v(rel)).find({ n: r.count() })` — no casts, no
   suppressions.
2. **Concrete full binding unchanged:** same call on a concrete relation
   infers the paramless chain.
3. **Cross-class join still refused:** the existing `CheckBindings` walls
   (`@ts-expect-error` pins in the law-typing suite) still hold on the
   general path — the added form must not have widened it.
4. **Partial records still infer params:** a record with one param binding
   flows through the general form with `BindParams` intact.
5. **Wrong-relation full record refused:** `r.match(A, v(B))` is a
   compile error for concrete and generic owners alike.
6. **Error-quality guard:** a wrong concrete record (a misspelled field)
   still names the offending property, not an opaque overload failure —
   pinned with an expected-diagnostic test.

## Consumers

Primer (deletes its suppression — D12 in [70-deletions.md](70-deletions.md);
its temporary count queries then die entirely to
[40-exact-count.md](40-exact-count.md)), and any host writing
schema-generic tooling over `query()`.
