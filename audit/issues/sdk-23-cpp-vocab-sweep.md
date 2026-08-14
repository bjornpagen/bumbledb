# sdk-23: C++ comment "stratum / program" sweep (wave-2)

Severity: low
Tree: sdk (cpp)
Status: OPEN
Source: wave-2 hunt (grep over cpp/)
Blocked-by: none
Blocks: none

## Bug

- `cpp/src/query/rule.cc:190` — "negates no stratum" (stratum is a
  deleted coordinate).
- `cpp/bridge/src/tests.rs:413,1085,1183,1563` — "The DownAt
  program", closures named `|program|` binding a query view.

NOT findings: "program constant" / "whole-program-optimized"
(`ir.cc:41`, `query_view.cc:672-675`, `cpp/README.md:47`) — C++
compiler senses of the word; `r30_keyed_read.cc`'s `Program`
relation is example data owned by docs-F23.

## Fix

Cites CONTRACT C7: "negates no stratum" → present-tense wall wording
("negation is refused in the rec"); test prose/closure names →
`|query|` / "the DownAt query". Prose-and-names-only.

## Acceptance criteria

- [ ] Grep `stratum|strata` over `cpp/` returns empty.
- [ ] Grep `\|program\|` over `cpp/bridge/src/` returns empty.
- [ ] `cpp/bridge` `cargo test` green; zero behavior change.

## Constraints

The C++-language senses of "program" listed above stay. No IR
Program vocabulary.
