# engine-035: tests and comments still say "empty program", "multi-rule program"

- **Severity:** low
- **Tree:** engine
- **Status:** DUPLICATE(engine-011)
- **Source:** audit/engine.md F35

engine-011 is the load-bearing vocabulary sweep (false invariants, `stats.strata`, `idb_*`, "whole program" / "fixpoint program"). Its grep covers the test-comment sites this finding named (`statically_empty.rs`, `folded.rs`, `rules.rs`, `adversarial_ir.rs`, `wordmap/clear.rs`). No separate fix lands under this id; the `TooManyCtes`-absence pin stays under engine-011's constraints.
