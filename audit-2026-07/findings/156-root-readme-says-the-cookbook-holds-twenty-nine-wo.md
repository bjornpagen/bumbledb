## Root README undercounts the cookbook: "twenty-nine worked schemas" vs the pinned 30

category: incoherence | severity: low | verdict: CONFIRMED | finder: r2:docs-vs-code-drift

### Summary

README.md describes `docs/cookbook.md` as "twenty-nine worked schemas (unions, vocabularies, trees, calendars, tax brackets, ledgers, maintained derived facts, host-driven closures), each rot-proofed by a compile test." The cookbook actually holds 30 numbered recipes — recipe 30 is "The keyed read" — and both compile-test roster pins assert exactly 30. The README is the one estate doc without a count pin, so the drift went uncaught.

### Evidence

- `README.md:525-527` — "twenty-nine worked schemas (unions, vocabularies, trees, calendars, tax brackets, ledgers, maintained derived facts, host-driven closures), each rot-proofed by a compile test" (verified in file).
- `docs/cookbook.md` — `grep -c '^## [0-9]'` returns 30; the final heading is `## 30. The keyed read` at line 1468.
- `crates/bumbledb-query/tests/cookbook.rs:901-909` — `the_doc_roster_is_exactly_this_roster` asserts `recipe numbering is 1..=30 in order`; the file header (lines 3, 717) declares an exhaustive roster with a count assertion.
- `ts/test/cookbook-doc.test.ts:34` — `const RECIPE_COUNT = 30`; line 87 asserts `"the cookbook holds all 30 recipes"`.
- The README parenthetical roster names eight recipe classes and includes no point-read/keyed-read entry, so the missing recipe is exactly the one the stale count omits.

### Failure scenario

The npm/GitHub front page undercounts the cookbook and omits the keyed-read (point-read) recipe class from the advertised roster. Any consumer — human or tooling — that counts recipes against the README number sees a mismatch with both compile pins, which authoritatively assert 30. This is documentation-vs-code drift of exactly the kind the repo's rot-proofing discipline (every doc claim pinned by a compile test) exists to prevent; the README is the unpinned gap.

### Suggested fix

In README.md:526, change "twenty-nine worked schemas" to "thirty worked schemas" and add point reads (the keyed read) to the parenthetical roster, e.g. "(unions, vocabularies, trees, calendars, tax brackets, ledgers, maintained derived facts, host-driven closures, keyed reads)". Optionally, extend one of the existing roster tests (the TS doc test already parses `docs/cookbook.md`) to grep the root README for the spelled-out count so the front page can never drift again.
