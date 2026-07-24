## Commas between body items and head terms are silently optional — the parser accepts a superset of the sacred grammar

category: incoherence | severity: medium | verdict: CONFIRMED | finder: query:crates

### Summary

The notation is normative and pinned in two places — docs/architecture/20-query-ir.md:965-1008 ("The query notation (normative — the render grammar)") and the `query!` macro's own doc header (crates/bumbledb-query-macros/src/lib.rs:10-56, which states "`ir::render` emits it, this macro parses it, round-trip goldens pin the two together"). Both spell the separators as mandatory: `head := headterm (',' headterm)*`, `body := item (',' item)*`. But the macro's head and body list loops treat the comma as skip-if-present, so comma-less rules parse and lower silently. The same production shape — a comma-separated list — is parsed under two strictness regimes inside one file: the atom-bindings loop strictly refuses a missing comma, the head and item loops do not. The lax regime admits text the renderer never emits, so the round-trip discipline (`render(lower(text)) == normalize(text)`) no longer proves the pinned notation is the only writable spelling.

### Evidence

All verified directly against the working tree (commit 89086d4f) plus a probe compile.

- Lax head loop — crates/bumbledb-query-macros/src/lib.rs:637-646:
  ```rust
  fn parse_head(mut tokens: Tokens) -> Parse<Vec<HeadTerm>> {
      let mut head = Vec::new();
      while tokens.peek().is_some() {
          head.push(parse_head_term(&mut tokens)?);
          if peek_punct(&mut tokens, ',') {
              tokens.next();
          }
      }
      Ok(head)
  }
  ```
- Lax body-item loop — crates/bumbledb-query-macros/src/lib.rs:1005-1008: `items.push(parse_item(tokens)?); if peek_punct(tokens, ',') { tokens.next(); }` — no else-branch, no error.
- Strict sibling — parse_bindings, crates/bumbledb-query-macros/src/lib.rs:733-740: after each binding, `,` or end-of-group is required; anything else is `query!: expected \`,\`, found …`.
- Normative grammar — docs/architecture/20-query-ir.md:980 (`head := headterm (',' headterm)*`) and :985 (`body := item (',' item)*`); identically at lib.rs:19 and :24 in the macro's header, alongside the round-trip-goldens claim (lib.rs:7-8).
- Probe compile (temporary test, since deleted): against a two-relation schema,
  ```rust
  query!(Org { (c p) | Parent(child: c, parent: p) Node(id: p) c < p; })
  ```
  compiled, lowered, and `ir::render` printed the canonical form back **with** commas: `(v0, v1) | Parent(child: v0, parent: v1), Node(id: v1), v0 < v1;` — the accepted input is provably not the renderer's fixed point.
- No countervailing intent: no comment in lib.rs documents optional commas (`git log -S` shows the loops arrived unremarked in the crate-split commit 438d63f8), and the compile-fail refusal suite (crates/bumbledb-query/tests/compile-fail/: datalog.rs, explicit_dense_positions.rs, mixed_predicate_bindings.rs, …) has no dropped-comma probe. The corpus goldens (crates/bumbledb-query/tests/notation-corpus/) only prove canonical text round-trips; they cannot detect that non-canonical text is also accepted.

### Failure scenario

A hand-edited golden or cookbook example with a dropped comma compiles silently instead of failing, so the anti-drift discipline (one grammar, three consumers) no longer guarantees the pinned notation is the only writable spelling. A forgotten comma between a comparison and a following atom (`c < p Node(id: p)`-style typo) is also plausible real-world input the grammar intends to reject; today it parses to a meaning the author may not have noticed they wrote. This is precisely the class of laxity the macro elsewhere refuses at cost — the explicit-dense respelling refusal (compile-fail/explicit_dense_positions.rs) exists to keep one canonical spelling per utterance.

### Suggested fix

Make the separator mandatory as parse_bindings already does: after a head term, require `,` or end-of-head; after a body item, require `,` or `;`. Representation-first version: extract one shared separated-list cursor helper (item-parser + terminator predicate) and route all three loops through it, so the strictness regime is a single representation rather than three hand-rolled loops that can drift — the drift is exactly what happened here. Add a compile-fail probe (`missing_comma.rs`) beside the other refusals so the regime is pinned.
