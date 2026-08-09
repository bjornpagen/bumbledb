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
audited by the asan-ubsan preset).

Phase F (closed relations, TODO_CPP §8): `r02_grading` — the closed
discriminator, ψ-selected mirrors sources (σ handle literals BY NAME on
the schema wire), the `[[=bdb::named<"operator">]]` wire-name override,
and the host handle-projection switch; `r06_tickets` — the bare-tier
vocabulary, the handle-literal match, and the membership-array ∈-set
match, executed against the engine; `r07_review` — the payload tier,
the ψ-selected containment TARGET, the closed relation as a query atom
with a bool payload literal, and the typed `Kind.axioms` readback;
`r08_oncall` — the sub-vocabulary containment (a nonmember escalation is
commit-rejected, asserted through the typed error) and the same ψ on the
read side.

Phase G (the remaining query/program vocabulary): `r14_calendar` —
ψ-selected mirrors on BOTH sides (accept+claim admitted paired, rejected
unpaired), the ψ-selected multi-column coverage containment, the
Allen-vs-param queries answered live, and the `bdb::pack` coalescing fold
(recipe 18's shape); `r24_closure` — the runtime ∈-set param
(`bdb::set_param`, span-bound at execute), the stratified program
(`bdb::program` / `bdb::rec` / `.idb` / `bdb::pred` / `bdb::bind`),
negation of the finished stratum (`.not_idb`), the EDB anti-join
(`.not_match`, recipe 3's shape), and the host-loop/native-program
agreement; `r29_zone_ledger` — the fixed-width interval family
(`bdb::interval<std::uint64_t, 1>` — the width is TYPE-enforced and a
fingerprint input) under per-kind ψ mirrors at mixed widths, plus
`Db::scan`; `r31_power_budget` — the weighted capacity law
(`weigh`/`within`/`ref`), `bdb::sum` over a plain scalar variable,
`bdb::count`/`bdb::max`/`bdb::arg_max` (Arg terms ride their own head —
the engine refuses Arg beside folds), the over-budget capacity citation,
`Db::write_witnessed`, `Db::execute`, and the `bdb::by`/`bdb::desc`
host comparator.

The dialect's pinned MINT-ON-INSERT spelling: the recipes mint fresh ids
with `tx.alloc(Relation.field)` and insert the FULL row — the TS
`tx.insert(Node, {name}).id` auto-mint is deliberately not ported (an
omitted-field insert has no wire spelling; the alloc+insert pair is the
engine's own split, and the C++ row products carry every field by
construction).
