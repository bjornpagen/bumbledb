## The Rust text notation cannot spell the or()/and() condition trees the IR accepts and the TS SDK ships

category: unification | severity: low | verdict: CONFIRMED | finder: cross:free-features
outcome: fixed 727c3b1d (R9)

### Summary

The engine's input condition grammar admits arbitrary boolean trees of positive comparisons — `ConditionTree::Leaf/And/Or` — and validation distributes them to DNF answer-preservingly (Lean-proved), with typed blowup and nesting caps and duplicate-rule collapse. The TypeScript SDK exposes this first-class: `and()`/`or()` are exported builders, condition trees travel on the wire, and tests execute nested or-of-and. The blessed Rust text surface (`query!`) cannot spell any of it: its grammar has no tree production and its lowering emits only `ConditionTree::Leaf`. One condition language, two unequal host surfaces — and the render→parse round-trip discipline the macro advertises holds only because the notation refuses to spell what the IR accepts (Or-carrying trees render "functionally" as diagnostic pictures that do not reparse).

### Evidence (all verified against the working tree)

- **IR accepts trees:** `crates/bumbledb/src/ir.rs:403-408` — `ConditionTree { Leaf(Comparison), And(Vec), Or(Vec) }`; the rustdoc (ir.rs:386-402) calls this "the one place the surface admits a nested OR". `Rule.conditions: Vec<ConditionTree>` at ir.rs:440.
- **DNF machinery is real and engine-owned:** `crates/bumbledb/src/ir/normalize/dnf.rs:89` (`pub fn distribute(rule: &Rule) -> Vec<LoweredRule>`); Lean proof `dnf_preserves_denotation` at `lean/Bumbledb/Query/Denotation.lean:746`; spec at `docs/architecture/20-query-ir.md` § "The input condition grammar and DNF lowering" — blowup cap (`DnfExceedsRules`), nesting cap (`ConditionNestingTooDeep`, 64), duplicate-rule collapse, `And([])`/`Or([])` algebraic readings.
- **TS surface ships it first-class:** `ts/src/query/atom.ts:450` (`and` builder), `:459` (`or` builder), `:660` (exported); `ts/src/index.ts:127` (public re-export); `ts/src/native.ts:172-173` (wire IR carries `{kind:"and"|"or"; children}` trees); `ts/test/query.test.ts:631` (`where(r.or(r.eq(k,"Checking"), r.eq(k,"Savings")))`) and `:807` (nested `r.or(r.and(...), ...)` executed).
- **Rust text surface cannot:** `crates/bumbledb-query-macros/src/lib.rs:24-29` — the normative grammar block: `item := atom | '!' atom | term 'in' term | Allen(...) | term cmp term`; no tree production. Grep confirms the macro's only `ConditionTree` construction site is `lib.rs:1468`, which emits `ConditionTree::Leaf(...)` exclusively. The doc-side normative grammar (`docs/architecture/20-query-ir.md`, the Normalization grammar block) matches.
- **The renderer already fixed the spelling:** `crates/bumbledb/src/ir/render.rs:344-345` renders `ConditionTree::And/Or` as `and(..)`/`or(..)`, but render.rs:41-44 scopes this to diagnostics only: "validated queries are Or-free downstream, so grammar-pure output holds for every query written in the notation; the functional forms appear only when diagnostics picture an input tree." That sentence is the finding in the code's own words: the round-trip guarantee is quantified over "queries written in the notation," which excludes exactly the IR inputs the notation cannot write.

### Failure scenario

A Rust host wanting `(kind == A && x > 4) || kind == B` inside one rule has two options, both worse than the TS host's one-liner `where(or(and(...), ...))`:
1. Hand-distribute the DNF in text — clone the entire rule per disjunct (atoms, finds, and every future edit duplicated), doing by hand what `distribute` does provably and with duplicate-collapse.
2. Abandon the text notation for that query and build the `ir::Query` value as raw data (`Rule.conditions` is `pub`, ir.rs:440, and the macro doc blesses raw IR for "the dynamic tail", lib.rs:79-83) — a real escape hatch, which is why the finder's "must clone by hand" is slightly overstated, but it forfeits the blessed notation, the compile-time name checking, and the goldens for a *static* query that the sibling SDK writes in one expression.

Additionally, any diagnostic that pictures an Or-carrying input rule emits text (`or(...)`) that `query!` refuses — the one class of rendered input queries that does not reparse.

### Suggested fix

Admit `and(item...)`/`or(item...)` into the `query!` item grammar, restricted to comparison leaves exactly as the IR's `ConditionTree` is (comparisons only — no atoms, negation, or membership under a tree, matching ir.rs:394-396 "negated atoms and membership stay leaf-level"). Lowering is direct: parse to nested `ConditionTree::And/Or/Leaf` and push one tree per item, engine-side DNF unchanged. The renderer's existing functional spelling (render.rs:344-345) becomes real notation for free, the render→parse round trip closes over the full input grammar, and lowercase `and`/`or` cannot collide with atoms (relations are UpperCamel; macro-local predicates would need `and`/`or` reserved, a one-line check). Update the normative grammar blocks in lib.rs and docs/architecture/20-query-ir.md together, per the "one notation, everywhere" discipline.
