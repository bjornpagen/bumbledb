# PRD 21 — The cookbook: modeling intuition as schemas (doc unit)

**Depends on:** the set's surface vocabulary (01 `fresh`, 02 rays, 03 Allen
notation, 10/12 `Duration`/`Pack`, 17 `bytes<N>`, 18 the witness, 19 derived
relations, 20 the render notation) — lands last, written against the post-set
grammar.
**Modules:** `docs/cookbook.md` (new, single file), one test module that
compiles every cookbook schema (rot-proofing).
**Authority:** `10-data-model.md`, `30-dependencies.md`; the cookbook is
**illustrative, never normative** — where it and a chapter disagree, the
chapter wins and the recipe is amended (README rule 5 applies to it fully).

## Context (decided shape)

The engine's docs say what the system *is*; nothing says how to *think in
it*. The cookbook is the intuition transfer: a dense file of worked schemas —
comments carry the reasoning, one schema block per problem, a few queries in
the render notation where they teach something — for the owner and for agents
writing theories. Format law: **dense and concise** — each recipe is one
`schema!` block ≤ ~40 lines with terse comments naming which statement buys
which theorem, plus at most three render-notation queries. No prose essays;
the comments are the prose.

## The recipe roster (exhaustive for v1; grow by census)

**Foundations**
1. *The minimal interval schema* — outages: pointwise key = per-group
   disjointness; membership, `INTERSECTS`, `Sum(Duration)`, `MEETS`.
2. *Discriminated unions* — the grading example: `==` arms, totality / arm
   validity / exclusivity as comments on the statements that prove them.
3. *0..1 optional attributes* — the no-nulls idiom: one-way `<=` plus the
   child key; absence is the fact that isn't.
4. *Money* — i64 minor units + newtype; multi-currency as
   `(currency: enum, minor: i64)`; i128-checked `Sum` noted; proration is
   host arithmetic.
5. *Content addressing* — `bytes<32>` digests inline (intern what repeats;
   inline what identifies); large payloads as refs; the dict stays for
   names.

**Structure**
6. *Ordered collections* — position columns, never successor pointers (the
   linked-list verdict, one comment); gapped positions for cheap insertion.
7. *Trees and ASTs* — node header + per-kind arm relations; every edge
   resolves, every arm total; the paths-or-cycles theorem from
   functional+injective successor; acyclicity is host discipline (recorded).
8. *Typed graphs* — edge relations per kind, endpoint containments pinning
   which node kinds each edge may touch; a closed, checked edge vocabulary.
9. *Entity-component (ECS)* — entity header + component sidecars = the 0..1
   idiom at scale; "every Renderable has a Transform" is one containment.
10. *State machines* — states as a DU, transitions as a relation whose
    target containment selects a *kind* (the conditional reference target
    SQL cannot state).

**Time and tilings**
11. *The calendar core* — rooms cannot double-book (pointwise key), working
    hours coverage, RSVP↔claim `==`; soft-vs-hard double-booking as the
    presence or absence of one statement (policy as schema).
12. *Effective-dated configuration* — rule versions: pointwise key (no
    overlapping versions) + coverage (no gaps); "in force on date t" is one
    membership probe; clean successions via `MEETS`.
13. *Tilings* — pay periods, shifts, estimated-tax quarters: disjoint +
    covering = no overlaps, no holes, two statements.
14. *Federal income tax* — brackets as intervals over MONEY with the ray
    top bracket; regime per (year, status); residency exclusion; the
    tile-at-write proration lesson (split facts at year boundaries — the
    representation move that deletes clip-at-query).
15. *Free time / coalescing* — `Pack` per group; gaps as the two-line host
    walk; `Sum∘Duration` insights as the two-query composition.

**The write side**
16. *The ledger* — accounts, postings, journal entries (the census
    workload); balance is host arithmetic over `Sum` (the
    arithmetic-agreement refusal, cited inline).
17. *Conditional writes* — the three witness idioms as recipes:
    update-where (snapshot query → delete+insert → `write_from`),
    insert-select, and read-modify-write with point-read guards for
    key-shaped premises.
18. *Derived relations* — a Pack-fed rollup under `<=`, maintained by
    witnessed writes; staleness uncommittable (cross-reference
    `10-data-model.md` § derived relations — PRD 19's chapter, landed —
    one worked block here).
19. *Union reads* — the DU whole-read as rules (one head, one rule per
    arm); the exclusivity elision noted as the free lunch.

**The anti-recipes (one block, five gravestones)**
20. What NOT to model, each with its one-line replacement: successor
    pointers (→ positions); floats for scores (→ basis points); conditional
    keys (→ relation splits); clip-at-query intervals (→ tile at write);
    uuid keys (→ `fresh` + explicit time).

## Technical direction

1. Write `docs/cookbook.md` in roster order; recipes copy the session's
   worked examples where they exist (uptime, calendar, tax) and are written
   fresh where they don't, all against the post-set grammar.
2. **Rot-proofing:** every cookbook schema block lives verbatim in a test
   module (include-or-duplicate, whichever the macro's item position
   allows) and must validate — a grammar change that breaks a recipe breaks
   the build, and the recipe is amended in the same change (the cookbook
   obeys rule 5 mechanically, not aspirationally). Queries are written in
   **the query notation** — PRD 23's set-builder grammar, the schema
   grammar's own query side (`(head) | body;`) — and double as PRD 23's
   round-trip golden corpus: each cookbook query must expand through
   `query!` and render back byte-exactly.

## Passing criteria

- `[test]` The cookbook-compiles test: every recipe's schema validates
  against the current engine; the test enumerates exactly the roster (a
  recipe added to the doc without a test entry fails a count assertion).
- `[shape]` All 20 recipes present, each ≤ ~40 schema lines, comments naming
  their theorems; the five gravestones each cite their replacement.
- `[shape]` The cookbook carries the illustrative-never-normative sentence
  and a pointer to the chapters it defers to.
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

The cookbook *is* the amendment; architecture README's document table gains
its row (reader: the owner and any agent writing a theory).
