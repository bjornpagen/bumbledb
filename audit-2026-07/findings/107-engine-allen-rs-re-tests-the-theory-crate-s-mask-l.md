## Engine allen.rs re-tests the theory crate's mask laws verbatim while its own module doc says they live theory-side

category: incoherence | severity: low | verdict: CONFIRMED | finder: theory
outcome: fixed ef9efbaa

### Summary

The engine's Allen test module opens with a doc comment claiming "Classification tests only — the mask vocabulary's own laws (constants, parse boundary, the exhaustive converse involution) are pinned in `bumbledb-theory`, next to the definitions" (crates/bumbledb/src/allen.rs:63-65). But the test `converse_is_an_involution_and_dualizes_classification` in that same module then runs a mask-only sweep over all 8,192 masks (lines 180-190) — a duplicate of two theory-crate pins, involving no `classify()` call and therefore no classification content. The doc claim is false, and the duplication is exactly the ownership split the two crates' doc comments say does not exist.

### Evidence

- crates/bumbledb/src/allen.rs:63-65 — module doc: "Classification tests only — the mask vocabulary's own laws (constants, parse boundary, the exhaustive converse involution) are pinned in `bumbledb-theory`".
- crates/bumbledb/src/allen.rs:180-190 — inside `converse_is_an_involution_and_dualizes_classification`, after the legitimate classification-duality loop (174-179), a second loop: `for bits in 0..=0x1FFF_u16 { let mask = AllenMask::new(bits)…; assert_eq!(mask.converse().converse(), mask); for basic in Basic::ALL { assert_eq!(mask.contains(basic), mask.converse().contains(basic.converse())); } }`. No `classify` anywhere in this loop.
- crates/bumbledb-theory/src/allen.rs:281-296 — `exhaustive_converse_involution_over_all_8192_masks`: the same involution sweep, strictly stronger (also asserts `mask.converse().popcount() == mask.popcount()` and the counted loop bound `visited == 8_192`, per the crucible packet's "the loop bound is the claim" doctrine cited in its doc comment).
- crates/bumbledb-theory/src/allen.rs:302-312 — `mask_converse_agrees_with_basic_converse`: byte-for-byte the same law as the engine loop's inner assertion. Its doc comment (:298-300) explicitly records the intended split: "the classification-duality half of this law lives engine-side, with `classify`" — i.e., theory expects the engine to hold only lines 174-179, not the mask half.
- crates/bumbledb/src/allen.rs:15 — `pub use bumbledb_theory::allen::{AllenMask, Basic};` — the same type re-exported, so the engine sweep is literally the same test on the same values, not a check of any engine-specific surface.

### Failure scenario

No runtime failure. The cost is (a) a false module-doc claim about where the mask laws are pinned, (b) a redundant 8,192 × 13 sweep in the engine's test lane, and (c) drift between twins — which has in fact already begun: the engine copy lacks the theory pin's popcount-preservation and counted-loop-bound assertions, so a reader trusting the engine copy sees a weaker law than the one the theory crate actually pins. Any future tightening of the theory tests leaves a stale duplicate the module doc claims does not exist.

### Suggested fix

Delete the mask-only sweep at crates/bumbledb/src/allen.rs:180-190, keeping the classification-duality assertions (lines 174-179: `classify(a,b).converse() == classify(b,a)` and the per-pair involution) that genuinely belong engine-side per both crates' doc comments. This restores the module doc to truth and leaves single ownership of the mask laws in bumbledb-theory, next to the definitions.
