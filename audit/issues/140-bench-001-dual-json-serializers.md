# bench-001: two JSON emitters, two query types — the producer of lean-008

- **Severity:** high
- **Tree:** bench
- **Status:** FIXED(8536b3df)
- **Source:** audit/bench.md F1
- **Depends on:** none (encoder-local; lean-008 is the decoder twin — land in either order if corpus stays byte-identical; docs-027 describes this split)

## The bug

`crates/bumbledb-bench/src/conformance.rs:810-864` (`render_case`) serializes a Query as CQuery JSON:

```rust
let mut query_block = String::from("{\"rules\":[\n");
// …
let _ = write!(out, "{{\"relation\":{},\"bindings\":[", atom.relation().0);
```

No `interiors`, no `rec`, no `head`. Atoms are `"relation"`. The reach twin is `conformance/reach.rs:236-261` (`render_reach_case`) — **this issue owns both emitters**. (engine-038 is a DUPLICATE stub of engine-012 about stats shape, not the encoder.) Atoms `edb` / `interior`; document keys `interiors` / `rec` / `arity` / `rules`. One language, two serializers that cannot share a Query renderer. lean-008 is the decoder twin.

## Why it's wrong

Insight 2: two representations of one thing will drift. lean-008 is the decoder that grew `CQuery` + `plainQuery` because this mint produces two documents. docs-027 teaches "a reach case carries a Query … instead of a CQuery" because the files look like two types. The code that writes them is the coordinate.

## The fix

Per `audit/CONTRACT.md` §C1 (corpus JSON frozen — 268 cases do **not** regenerate) and §C4 (one decoder; two JSON *spellings* of one `AtomSource`):

- ONE Query renderer. Two frozen spellings as data: `Seeded` omits empty interiors/rec/`head` and writes `"relation"` (the 246 seeded/hand files stay byte-identical); `Reach` writes `interiors` / `rec` / `rules` and `edb` / `interior` (the 22 reach files stay byte-identical, including the frozen `arity` keys).
- `relation` and `edb` are two keys for `AtomSource::Edb`. Do not add `predicates` / `output` / `strata` / `idb`.
- `EdbAtom::relation()` on the seeded atom path dies with bench-004; the renderer matches `AtomSource`.

## Acceptance criteria

- [ ] One renderer: `rg -n 'let mut query_block = String::from' crates/bumbledb-bench/src/conformance.rs crates/bumbledb-bench/src/conformance/reach.rs` → one construction site (or a shared helper both arms call).
- [ ] Unchanged corpus: `git diff --stat lean/conformance/cases` empty after a regenerate-and-compare (or the regenerate test still asserts byte-identity with the checked-in 268 files). **Do not check in new case bytes.**
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench`; `./scripts/lean.sh` (268-case conformance, three-way comparator).

## Constraints

- C1: 268 cases frozen. Dual *spellings* stay; dual *functions* die. No Program fields. lean-008's decoder merge is a separate commit; this issue does not require it.
