# tests/cookbook/

The C++ ports of the 32 engine-cookbook recipes (TODO_CPP §33). Each recipe
represents the same theory as its TypeScript and Rust siblings — same
relation order, field order, structural kinds, fresh marks, closed
extension, statements, canonical descriptor — and must lower through Rust to
the same fingerprint pinned in `fixtures/cookbook-fingerprints.txt` at the
repository root (the fixtures path arrives as the test's argument).

Landed: `r01_uptime` — the §39 vertical slice's theory through the real
`bdb::schema<>` elaborator, fingerprint-matched against the `r01` golden —
and `r01_queries` — the recipe's three queries (downAt / overlapping /
downtime) built through `bdb::query(...).rule(...)`, prepared through the
engine's IR validator, executed against the recipe's example data, and
asserted against the recipe's own answers, plus the §23 `execute_into`
reuse lane and a joined string-answer query (the §22 borrowed-view lane,
audited by the asan-ubsan preset). The remaining recipes land as their
surface (closed relations, σ/ψ selections, recs/negation) arrives.
