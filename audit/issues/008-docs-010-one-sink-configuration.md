# docs-010: prepared-query section says "one sink configuration, owned by the head" — the one-list coordinate

- **Severity:** medium
- **Tree:** docs
- **Status:** FIXED(b87f3ad9)
- **Source:** audit/docs.md F10
- **Depends on:** none (prose; same file as docs-001..010; describes the shape engine-001 formalizes — safe to fix now, the SENTENCE is wrong against shipping code too)

## The bug

`docs/architecture/20-query-ir.md:1147-1149`:

> the prepared query holds one validated plan per rule and **one** sink configuration, owned by the head — execution is the rule loop driving every rule's plan into that sink

The SAME chapter at `:81-82` already states the truth: "**sink per rule-list**: one sink per `Interior`, one sink for the `Rec`, one sink for the main query — not one sink for the whole `Query`."

## Why it's wrong

The chapter contradicts itself, and the wrong half is written in the old one-list coordinate (CQuery/program output — Insight 1): "the head's sink" was true when the whole prepared object WAS one rule-list. Interiors and rec are not "the head's sink"; a reader landing on §prepared learns the deleted architecture.

## The fix

Per `audit/CONTRACT.md §C7`: rewrite to agree with `:81-82`: "Each rule-list has its own sink: interiors in declaration order, then the rec, then main. The prepared object holds one plan per rule of each list and one sink per list. Main's sink is the answer." The seen-set clause attaches to each list's sink.

## Acceptance criteria

- [ ] Gone: `rg -n 'one.{0,3}sink configuration' docs/architecture/20-query-ir.md` → no matches.
- [ ] The two sections agree verbatim on the sink-per-list model; `:81-82` unchanged.

## Constraints

- Prose only; describes shipping behavior (the per-list sinks exist today), so it does NOT wait for engine-001.
