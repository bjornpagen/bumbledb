# Closed-relation 256-axiom cap is engine law; Lean GroundExtension is unbounded
- id: 217
- severity: low
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: unspecified
- components: lean/Bumbledb/Schema.lean, crates/bumbledb-theory/src/schema.rs, crates/bumbledb/src/schema.rs, docs/architecture/30-dependencies.md
- status: open (do not fix)

## Summary
Lean `GroundExtension` is an arbitrary fact list. Rust compiles closed membership into a 256-bit `MemberSet` and rejects extensions above `MAX_EXTENSION_ROWS = 256`. Docs present the cap as what "fixes this width." A Lean theory with 257 ground axioms is a model; it is not a sealable schema.

## Lean spec
`Schema.lean:521-523`: "the ≤256 roster cap is mechanism, not modeled." `den_closed_finite` witnesses finiteness by the list, with no 256 bound. `Oracle` member-test plans do not mention bitset width.

## Normative docs
`30-dependencies.md:457-458`: "≤256 roster cap exists exactly to fix this width" (compiled member-set bitset).

## Rust implementation
`bumbledb-theory/src/schema.rs`: `MAX_EXTENSION_ROWS = 256`. `schema.rs` `MemberSet`: "four words encode the declaration-time 256-axiom bound." Validate rejects oversized extensions.

## Why this matters
O(1) member-set tests and closed-to-closed validate-time refutation assume a width Lean proofs do not. A spec-faithful frontend that omits the cap would produce schemas the engine cannot compile, or a bitset overflow if the cap were only a comment.

## Related
- 207, 208 (other closed-relation acceptance gaps)
