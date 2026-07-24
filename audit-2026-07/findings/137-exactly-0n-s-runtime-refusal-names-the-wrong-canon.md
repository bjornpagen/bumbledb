## exactly(0n)'s runtime refusal names the wrong canonical shape (`{0..0}` instead of `{0}`)

category: incoherence | severity: low | verdict: CONFIRMED | finder: ts:types

### Summary

The five count constructors in `ts/src/count.ts` enforce the canonical-utterance ban table in two tiers, and the module header (count.ts:13-18) promises the construction tier judges computed bounds "with the same canonical-naming errors" as the type tier. For `exactly()`, the two tiers disagree: the type-tier verdict for a literal `exactly(0n)` names the `{0}` shape, but the runtime throw for a computed `0n` claims the caller wrote a "`{0..0}`-shaped spelling" — a shape `exactly()` cannot produce. The message is a copy from the wrong row of the ban table (the `atMost(0n)` / `between(0n,0n)` row).

### Evidence

All verified in `ts/src/count.ts`:

- **count.ts:135-137** — the defective runtime guard inside `exactly()`:
  ```ts
  if (n === 0n) {
      throw errors.new("`{0..0}`-shaped spelling: the exclusion is written `{0}` — use none")
  }
  ```
- **count.ts:72-73** — the type-tier `ExactlyBan` verdict for the same call: `BannedWindow<"`{0}` is the exclusion — write none">`. `exactly(n)` produces the `{n}` exact-count window (count.ts:138 admits `{ kind: "exact", n }`), so `exactly(0n)` is the `{0}` utterance, never `{0..0}`.
- **count.ts:93, 121, 164, 205** — the `{0..0}` wording used correctly, in the constructors whose argument shape actually spells `{0..0}`: `AtMostBan`, `BetweenBan`, `between`'s runtime guard, `atMost`'s runtime guard (all read "`{0..0}` — the exclusion is written `{0}`: use none").
- **docs/architecture/70-api.md:143-151** — the canonical-utterance ban table (owner-ruled 2026-07-15): the row `X <={0..0} Y` → "the exclusion is written `{0}`". exactly's runtime message quotes this row, but the judgment exactly(0) triggers is the different one — `{0}` is itself the exclusion's canonical spelling and the constructor spelling must be `none`. The law's whole rationale (70-api.md:135-141) is that each error names the canonical form of *the utterance the author wrote*, in pasteable spelling; naming an utterance the author did not and cannot write through this constructor breaks that.
- **ts/test/statements.test.ts:410-413** — the construction-tier test asserts only `assert.throws(..., /use none/)` for `exactly(computed(0n))`, so the wrong shape name is not pinned by any test and slips through green.

### Failure scenario

A computed `bigint` (literal identity erased) reaching `exactly(count)` with `count === 0n` throws a diagnostic asserting the caller spelled `{0..0}`. The same call written with a literal (`exactly(0n)`) fails at compile time with a verdict naming `{0}`. The two tiers of the same constructor contradict each other about which banned utterance was spelled, and the runtime one is wrong: `exactly()` has no argument shape that spells `{0..0}`.

### Suggested fix

Replace the message at count.ts:136 with the type tier's exact sentence, one meaning one spelling across both tiers:

```ts
throw errors.new("`{0}` is the exclusion — write none")
```

Optionally tighten the test regex at statements.test.ts:411-413 to pin the full sentence so tier drift cannot recur silently.
