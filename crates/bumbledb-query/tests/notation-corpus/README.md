# The notation conformance corpus (PRD-M4) — one grammar, two replayers

One JSON document per case: a (notation ⇄ `ProgramIr` JSON) pair that BOTH
hosts replay mechanically. The Rust side
(`crates/bumbledb-query/tests/notation_corpus.rs`) `query!`-compiles each
case's notation, proves it real against a `Db` of the corpus theory,
round-trips it through `ir::render`, and byte-pins the whole document; the
TS side (`ts/test/notation-corpus.test.ts`) constructs the same query in
the builder (or writes the `ProgramIr` by hand where the builder cannot
spell it), asserts `JSON.stringify` equality against the pinned `program`,
and `dbPrepare`s every case's program against a store of the same theory.

**The law: a disagreement is a trophy, not a merge conflict.** If the two
replayers ever pin different bytes for one case, that is a macro bug, a
builder bug, an encoder bug, or a spec bug — report it prominently and
triage; never "fix" the corpus to make a disagreement go away.

## The document shape

```jsonc
{
  "name": "mandate-window",             // = the file name, corpus-unique
  "builder": true,                      // false: the TS BUILDER cannot spell it
                                        //   (the TS replayer hand-writes the IR
                                        //   and counts the case as skipped from
                                        //   builder construction — the skip
                                        //   count is asserted exactly)
  "productions": ["allen-literal-mask"],// the grammar productions this case
                                        //   is coverage for (enumerated below)
  "notation": "(org) | Mandate(…);",    // the query! source, whitespace-tied
                                        //   to the compiled tokens
  "normalized": "(v0) | Mandate(…);",   // render(lower(notation)) — reparses
                                        //   to the identical IR (the fixed point)
  "program": { "predicates": […],       // the ProgramIr JSON (below), compact —
               "output": 0 }            //   the exact bytes both sides compare
}
```

## The `ProgramIr` wire shape

The interchange format is the napi bridge's `ProgramIr` — `ts/src/native.ts`
declares it; `ts/crate/src/marshal.rs::program_in` mirrors `ir::Program`
1:1. A plain query is its degenerate one-predicate program
(`From<Query> for Program`). Normatively, with key order exactly as the TS
lowering's object literals insert it (`ts/src/query/lower.ts`):

- program — `{"predicates": [predicate…], "output": N}`
- predicate — `{"head": [headTerm…], "rules": [rule…]}`
- headTerm — `{"kind":"var"}` |
  `{"kind":"aggregate","op":"sum"|"min"|"max"|"count"|"countDistinct"|"argMax"|"argMin"|"pack"}`
- rule — `{"finds":[…],"atoms":[…],"negated":[…],"conditions":[…]}`
- find — `{"kind":"var","var":N}` | `{"kind":"measure","var":N}` |
  `{"kind":"aggregate","op":AGG}` (nullary `Count`: no `over` key) |
  `{"kind":"aggregate","op":AGG,"over":N}` |
  `{"kind":"aggregateMeasure","op":AGG,"over":N}`
- AGG — `{"kind":"sum"}` … `{"kind":"argMax","key":N}` |
  `{"kind":"argMin","key":N}` | `{"kind":"pack"}`
- atom — `{"source":{"kind":"edb","relation":N}|{"kind":"idb","pred":N},
  "bindings":[[fieldId, term]…]}` (an idb atom's field ids address head
  POSITIONS; binding order is written order)
- term — `{"kind":"var","var":N}` | `{"kind":"param","param":N}` |
  `{"kind":"paramSet","param":N}` | `{"kind":"literal","value":V}` |
  `{"kind":"measure","var":N}`
- condition — `{"kind":"leaf","cmp":CMP}` |
  `{"kind":"and"|"or","children":[…]}`
- CMP — `{"op":OP,"lhs":term,"rhs":term}`; OP —
  `{"kind":"eq"|"ne"|"lt"|"le"|"gt"|"ge"|"pointIn"}` |
  `{"kind":"allen","mask":{"kind":"literal","mask":BITS}|{"kind":"param","param":N}}`
  (`PointIn` is stored interval-left, point-right)
- V (tagged value) — `{"kind":"bool","value":true}` |
  `{"kind":"u64","value":"18446744073709551615"}` |
  `{"kind":"i64","value":"-3"}` | `{"kind":"string","value":"…"}` |
  `{"kind":"intervalU64","start":"3","end":"10"}` |
  `{"kind":"intervalI64","start":"-3","end":"10"}` |
  `{"kind":"allenMask","mask":BITS}`

**Integer normalization**: every id (relation, field, predicate, variable,
param, output) and every mask is a JSON NUMBER; every `Value` scalar
(u64/i64 payloads, interval endpoints) is a decimal STRING — the TS side
carries them as `bigint` and the corpus normalization is
`JSON.stringify(programIr, (k, v) => typeof v === "bigint" ? v.toString() : v)`,
which renders a bigint as its decimal string. This was verified against
the running TS lowering, not assumed. `bytes<N>` literals are refused
representation (a `Uint8Array` does not `JSON.stringify` canonically);
no case may use one until the corpus rules on a spelling.

## The corpus theory

One schema, the benchmark ledger, declared once per host:

- Rust: `tests/notation_corpus.rs` (`schema!`, declared newtypes — the
  macro's untouched notation);
- TS: `ts/test/notation-corpus.test.ts` (structural fields + the same
  thirteen statements — the laws type the columns).

`schema-fingerprint.txt` pins BOTH constructions to one engine-computed
schema fingerprint (blake3 over canonical descriptor bytes,
`bumbledb-schema-v4` — never syntax, never spellings, never domain
labels; the T5 mechanism, one line), so the corpus schemas cannot drift.

## The production enumeration

Every grammar production below is witnessed by at least one case; the
Rust suite asserts this list (`REQUIRED_PRODUCTIONS`) — an uncovered
production fails the test, as does a case naming an unknown production.

| production | meaning |
| --- | --- |
| `punning` | a bare field name binds a same-named variable |
| `field-var` | `field: var` explicit binding |
| `eq-literal` | in-atom `field == literal` selection |
| `eq-handle` | in-atom selection through a closed handle (`currency == Usd`) |
| `eq-param` | in-atom `field == ?param` selection |
| `ne` `lt` `le` `gt` `ge` | the scalar comparison operators |
| `in-param` | `field in ?param` — set-param membership |
| `point-in` | `?t in v` — point membership in an interval variable |
| `allen-literal-mask` | `Allen(a, MASK, b)` with a named mask |
| `allen-mask-union` | a `\|`-united mask (`BEFORE\|MEETS`) |
| `allen-mask-param` | `Allen(a, ?p, b)` — the mask as a param |
| `negation` | `!atom` — the anti-join |
| `agg-sum` `agg-min` `agg-max` `agg-count` `agg-count-distinct` `agg-arg-max` `agg-arg-min` `agg-pack` | the eight aggregates |
| `duration` | the measure: `Duration(v)` finds, folds, and comparisons |
| `named-columns` | `name: Agg(…)` head naming (call-site only; the IR is positional) |
| `multi-rule-union` | several rules, one head — set union |
| `program-recursion` | named heads + `rec` rules (`reach(…) \| …`) |
| `idb-ordered-dense` | dense in-order predicate bindings written BARE (`reach(m, a)`) |
| `idb-sparse` | a sparse predicate position (`2: x`) |
| `idb-position-selection` | a position selection / membership (`1 == …`, `0 in ?p`) |

## Regeneration and replay

- Replay (runs in the plain suite):
  `cargo test -p bumbledb-query --test notation_corpus` — every document
  byte-identical from the case table; every case validated against a real
  store; the render fixed point; the production enumeration.
- TS replay: `node --test test/notation-corpus.test.ts` from `ts/` —
  builder-constructed `ProgramIr` equality, the exact skipped-count
  assertion, `dbPrepare` acceptance for every case, the fingerprint pin.
- Regenerate (after editing the case table):
  `cargo test -p bumbledb-query regenerate_the_notation_corpus -- --ignored`
  — deterministic: identical bytes from identical source, forever. This
  README is hand-written and never regenerated.
