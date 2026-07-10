# PRD 23 — The query notation: set-builder, promoted from the schema grammar

**Phase:** F (the surface). **Index note:** the README row and spine entry are
pending — the index file was in-flight with the executing agent when this PRD
landed; add: "23 requires 05 and 20; coordinates with 21 (the cookbook is
written in this notation)."
**Depends on:** 05 (rules-shaped IR), 20 (`ir::render`, the id constants, the
placement ruling). PRD 21's cookbook queries are written in this notation.
**Modules:** new downstream crate `crates/bumbledb-query` (quarantined),
`ir/render` (the renderer emits this grammar — spec shared with PRD 20), docs.
**Authority:** `30-dependencies.md` ("dependencies and queries share one
representation"), `20-query-ir.md`, PRD 20's rulings.
**Representation move — the notation is derived, not designed.** The schema
grammar already contains a query notation: the two judgments are
parameterized by single-atom queries written `R(X | φ)`, and `|` already
reads *such that* — set-builder's own bar. The query notation is that form
**promoted**: multi-atom, given a head, terminated by the statement's `;`.
Borrowed grammar is refused (owner ruling 2026-07-10): Datalog's `head :-
body` was considered and rejected — this engine's statements are already
statements *about* queries, so the query surface must be the statement
surface's query side, not an import. One notational family, schema to query.

## The grammar (normative; `ir::render` emits it, everything parses it)

```
query   := clause+                     // two or more clauses denote set union
clause  := '(' head ')' '|' body ';'
head    := headterm (',' headterm)*
headterm:= var | [name ':'] agg        // named positions become result columns
agg     := Sum(t) | Min(t) | Max(t) | Count | CountDistinct(v) | Pack(v)
           where t := v | Duration(v)
body    := item (',' item)*
item    := atom                        // positive occurrence
         | '!' atom                    // negation (anti-probe; safety per roster)
         | term 'in' term              // membership: point ∈ interval, value ∈ ?set
         | Allen '(' term ',' mask ',' term ')'
         | term cmp term               // ==  !=  <  <=  >  >=
atom    := Relation '(' binding (',' binding)* ')'
binding := field                       // punning: binds a var named after the field
         | field ':' var               // explicit variable — the join spelling
         | field '==' value            // selection, schema-grammar-verbatim
mask    := MASK ('|' MASK)* | ?param   // masks are sets of basics; '|' is set union
term    := var | ?param | literal
```

Every token is either the schema grammar's own or Rust's: atoms are
`Relation(...)` as statements write them; in-atom selections are the schema's
selections with params admitted; membership is the Rust keyword `in` (∈ is
not a lexable token); negation is `!`; params keep `?`; `;` terminates
clauses as it terminates statements. The two bars are the two the audit
already upheld: clause-level `|` is *such that*; mask-level `|` is set union
over the 13 basics — set-builder and set-union, context-separated exactly as
the two levels of `==` are.

## The punning law — decided (B), recorded with its alternative

A bare field name binds a **clause-local variable named after the field** —
Rust's struct-shorthand instinct, used for projection. **The same punned name
appearing in two atoms of one clause is a macro error** ("ambiguous punning —
rename explicitly"); joins are always written `field: v` on both ends.

**Alternative (A), refused:** set-builder-honest same-name-same-variable
across the clause. It is what the mathematics reads, and it makes joins
silent — which is exactly the problem: every relation names its key `id`, so
a forgotten rename silently unifies a `HolderId` with an `AccountId`, and the
roster cannot object (structurally both u64). Under (A) a *wrong* query
compiles, validates, and runs; under (B) it is unwritable at the call site.
Illegal states unrepresentable outranks notational purism. *Reverses if:*
ambiguity errors prove noisy on real theories AND a schema-aware lint becomes
possible (it is not today: the macro cannot see the theory).

```rust
let unavail = query!(Calendar {
    (person, during) | Busy(person, during), Allen(during, INTERSECTS, ?window);
    (person, during) | Ooo(person, during),  Allen(during, INTERSECTS, ?window);
});
let rate = query!(Tax {
    (rate_bps) | Year(id: y, span), ?today in span,
                 Regime(id: r, year: y, status == ?s),
                 Bracket(regime: r, income, rate_bps), ?taxable in income;
});
let conflicts = query!(Calendar {
    (c1, c2) | Busy(id: c1, person: p, during: d1),
               Busy(id: c2, person: p, during: d2),
               c1 < c2, Allen(d1, INTERSECTS, d2);
});
```

## The macro and its placement

- **`query!(Theory { ... })` lives in a new downstream crate,
  `crates/bumbledb-query`** — the bench-crate quarantine verbatim: it may be
  depended on by hosts, the engine never depends back, and PRD 20's shape
  grep ("no query grammar in the engine") stays true unamended. Parser is
  hand-rolled over the token stream, `syn`-free, on `bumbledb-macros`'
  precedent; the grammar is smaller than the schema's.
- **Name checking without schema visibility — the id-constants trick.** Proc
  macros cannot see each other's output, so `query!` cannot read the theory.
  It doesn't need to: expansion emits paths to PRD 20's emitted id constants
  (`Calendar::BUSY`, `Calendar::BUSY_PERSON`), and ordinary rustc name
  resolution does the checking — a typo'd relation or field is a compile
  error pointing at the query literal. Variable *type* consistency stays the
  roster's (prepare-time, typed, rendered) — the same split the foreign
  surfaces have.
- **Constant text only** (the macro consumes a literal token tree): dynamic
  composition stays on the raw IR layer, which exists regardless — text for
  the static 90%, data for the dynamic tail, both lowering to the same IR.
- Expansion is compile-time lowering: the emitted code constructs the
  `ir::Query` value; no runtime parser, no parse cost, nothing new crosses
  into the engine.

## One notation, everywhere — the anti-drift discipline

`ir::render` (PRD 20) emits **this grammar**; the cookbook (PRD 21) writes
its queries in it; any foreign sugar (the anticipated TS binding) parses it.
The renderer is the spec; every implementation is pinned by **round-trip
goldens**: for each cookbook query, `render(lower(text))` equals the
normalized text, byte-exactly. Three consumers, one grammar, zero drift by
construction.

## Passing criteria

- `[test]` The cookbook's query set, expanded through `query!`, lowers to IR
  that validates and round-trips through `ir::render` byte-exactly.
- `[test]` Compile-fail suite: a typo'd relation, a typo'd field (both via
  id-constant resolution), a punned name in two atoms (the ambiguity error,
  spanned at the second occurrence), a `?param` in a head, `:-` anywhere
  (the refused grammar must not parse).
- `[shape]` `bumbledb-query` is absent from the engine crates' dependency
  graphs (the dependency-law grep, extended); PRD 20's no-query-grammar grep
  over engine crates still passes; the Datalog refusal and the punning
  decision are in the refusals ledger.
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`20-query-ir.md`: the notation section (this grammar, normative, as the
render grammar). `70-api.md`: `bumbledb-query` as the blessed Rust sugar,
downstream; the two-surface framing gains one sentence ("the notation is the
statement grammar's query side, promoted"). Refusals ledger: borrowed query
notation (Datalog `:-`); punning alternative (A). README (set) index: the
pending row noted above.
