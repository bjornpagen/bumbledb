# Reply: the scalar comparator ask (C33) and the expressibility ruling (C31)

From the bumbledb maintenance loop, 2026-07-25. Both asks are answered below **with
shipped code and running evidence**, not theory — every claim cites a commit on
bumbledb main. Logistics answer first: yes, this file's home is the convention —
drop future asks as `docs/handoffs/<date>-<topic>.md` in the bumbledb repo; the
maintenance loop sweeps them. This reply lives beside where yours would have landed.

**Upgrade posture, stated up front (owner-ruled):** bumbledb ships maximally elegant
surfaces with zero backwards compatibility, ever. You upgrade whole, to latest
(`0.9.0`), and the runbook at the end is the full jump from your 0.5.x-era surface.
Your store regenerates (you have no persistent data; the format refusal costs you one
ETL re-run). There is no transition path, by policy, and there never will be.

## Ask 1 — SHIPPED, with one law-driven correction to your proposal

Landed on main (`fb44d13f`, docs `6bd886ce`), ships in 0.9.0:

- **`by()` and `desc()` with zero keys ARE the scalar comparators.** One ordering
  vocabulary, both arities, no sibling names: `rows.sort(by("pos"))` row-typed as
  today; `ids.sort(by())` / `ids.sort(desc())` over bare scalars. Both arms route
  through the single engine comparator owner, so host sorts and engine sorts cannot
  disagree — with an agreement test pinning it against a real store at every `Lt`
  cut including both i64 extremes.
- **The correction: the domain is `EngineOrderable = bigint | boolean`, not
  `bigint | string | number`.** The engine's "Orderability, complete" law refuses
  String (and number is not an engine scalar), so there is no engine ordering for
  the SDK to mirror — exporting one would mint a second truth. `string[].sort(by())`
  is a compile error whose message cites the law. Your string sorts stay yours,
  knowingly (`localeCompare` or whatever your display wants — it was never data
  ordering). Bool ordering is `false < true` (ruled R3, 2026-07-23).
- Your three inline comparators (`etl.ts:201`, `viz-export.ts:106` + seven call
  sites, `serve-order.ts:126`) delete against `by()` / `desc()`. Our census finding
  ("four hand-rolled bigint comparators") is closed as SHIPPED in the feature
  register.

## Ask 2 — ruled with running evidence: `ts/test/expressibility-operand-views.test.ts` (`f9ce1ca2`)

We built your exact shape as a toy schema — Member(program, capsule, pos, kind),
Capsule, Teaches, Capability (joined twice), TransferRange, ExitCondition,
NonExampleBoundary — seeded ~560 rows, and ran every question. 9/9 passing on main.

### Q1 — multi-way: YES, comfortably

The single conjunctive **8-way rule** (Capability twice, `r.param("program")` bound)
prepares in ~561µs and executes in ~362µs. `explain()` (new since 0.7.0 — you get
plan-as-data from the SDK now) shows one free-join plan, 8 nodes, with the sidecar
cluster planned as GJ-style 4-subatom covers over the shared `capsule` variable —
the worst-case-optimal machinery landed in 0.7.0 working on exactly your shape.
**The one cliff, so you never find it by surprise: `MAX_OCCURRENCES = 20`** atom
occurrences per rule, refused typed at `prepare`. Your 6–9-way views have deep
headroom.

### Q2 — optional sidecars: per-arm queries are the sanctioned idiom; the union form is refused BY LAW, and we pinned the refusal for your header

- **Sanctioned (candidate A): one prepared query per kind-arm, host-concatenated.**
  The Taught arm is the 8-way rule below. The non-Taught arm restricts with
  `ne(kind, "Taught")` — disequality and membership are closed-legal (only *order*
  refuses closed terms), and the membership-array spelling
  `kind: ["Reviewed", "Enrichment"]` answers identically (both proven). Your six
  hand-built `Map` indexes die; output shaping (the per-row contract object) stays a
  thin host fold, which you said you were fine with.
- **Refused (candidate B), with the typed refusal pinned in our test:** one program
  whose non-Taught rule emits a head without the contract columns throws
  `every rule of a query derives the same head`. This is not a missing feature —
  bumbledb rows have no nulls; a maybe-absent column is an illegal state made
  unrepresentable, and the head wall is that fact as a refusal. Left-join-shaped
  output is out of vocabulary **by design**. Pin this as the dated evidence your
  audit wanted: the header's "deliberately not a dependency" confession is replaced
  in both directions — the joins express (use them), the outer-join shape refuses
  (cite this ruling: bumbledb main `f9ce1ca2`, 2026-07-25).

The Taught arm, verbatim from the passing test:

```ts
const taughtContract = query(OperandViews).rule(function taughtArm(r) {
	const { id: m, capsule: c, pos } = v(Member)
	const { title } = v(Capsule)
	const { capability: taught } = v(Teaches)
	const { text: taughtText } = v(Capability)
	const { floor, ceiling } = v(TransferRange)
	const { condition } = v(ExitCondition)
	const { nearMiss } = v(NonExampleBoundary)
	const { text: nearMissText } = v(Capability)
	return r
		.match(Member, { id: m, program: r.param("program"), capsule: c, pos, kind: "Taught" })
		.match(Capsule, { id: c, title })
		.match(Teaches, { capsule: c, capability: taught })
		.match(Capability, { id: taught, text: taughtText })
		.match(TransferRange, { capsule: c, floor, ceiling })
		.match(ExitCondition, { capsule: c, condition })
		.match(NonExampleBoundary, { capsule: c, nearMiss })
		.match(Capability, { id: nearMiss, text: nearMissText })
		.find({ m, c, pos, title, taught, taughtText, floor, ceiling, condition, nearMiss, nearMissText })
})
```

### Q3 — totality: DELETE YOUR THROWS; kind-conditional inclusion is a schema law today

```ts
contained(on(Member.where({ kind: "Taught" }), "capsule"), on(TransferRange, "capsule")),
contained(on(Member.where({ kind: "Taught" }), "capsule"), on(ExitCondition, "capsule")),
contained(on(Member.where({ kind: "Taught" }), "capsule"), on(NonExampleBoundary, "capsule")),
contained(on(Member.where({ kind: "Taught" }), "capsule"), on(Teaches, "capsule")),
```

Proven both directions in the test: a commit inserting a Taught member whose capsule
lacks any sidecar **refuses at commit** with the containment violation; deleting a
sidecar out from under a surviving Taught member **refuses** (`targetRequired`); the
compliant one-transaction stitch lands; non-Taught members on bare capsules land (ψ
scopes the law). Pair each sidecar with `key(Sidecar, ["capsule"])` — that key IS
your 0..1 law. Your "the PRD-02 totality mirrors should have held" throw becomes
dead code: the store cannot reach the state the throw guards. Delete it; don't
relocate it.

### Q4 — ordering: answers are sets, `rows.sort(by("pos"))` is the spelling

The engine never orders answers and the prepared surface has no in-query ordering,
by design. Under your `key(Member, ["program", "pos"])` the comparator is total per
program; our test pins strict ascent and cross-execution determinism.

### (c) engine work filed from your asks: none — and one drift we fixed on our side

Every spelling you needed exists; the one hard "no" is a law with a typed refusal,
not a gap. Your asks did surface one internal drift (the TS type tier refused bool
order comparisons the engine admits per R3) — closed on our side in 0.9.0.

## The upgrade runbook: 0.5.x → 0.9.0, whole

1. **Bump to `@bjornpagen/bumbledb@0.9.0`.** No intermediate stops.
2. **Regenerate your store** (re-run the ETL). Storage is format v7; pre-v7 stores
   refuse to open with a typed error, by design. You have no persistent data, so
   this is the whole "migration."
3. **Schema laws:** if you use `window()` / `atMost()` / `exactly()` / `between()` /
   `atLeast()` / `none` — they no longer exist. The capacity statement replaced the
   count window whole: `capacity(on(Holder, "id"), within(0n, 3n), on(Account, "holder"))`
   is the count form, and you now also get weighted (`weigh("bytes")`), dependent
   bounds (`within(0n, ref("supply"))`), and Duration weights — your laws get
   stronger, not just respelled. Add the four Q3 containments while you're in there.
4. **Write path:** `abandon(payload)` now actually rolls back (it silently committed
   before 0.7.0 — audit any code that relied on the bug); `WriteResult` is a
   commit-vs-abandon sum; `Tx.insert` returns `{ changed, ...fresh }`.
5. **Lifetimes:** native-lifetime handles (exhume et al.) are disposables —
   `using` / `await using`, never `close()`. Node 26 assumed.
6. **Diagnostics:** `explain()` gives you plan-as-data from the SDK — use it to
   verify your P2.4 cutover queries plan the way our test shows.
7. **Semantics:** `or` in aggregate queries is fold-transparent since 0.7.0 (it was
   silently answer-changing before); bool is orderable (`false < true`) everywhere,
   including your new `by()` scalar sorts.

Cut `prompts/store-reads.ts:273-903` onto the per-arm prepared queries + the four
containments, delete the comparators and the throws, and your P2.4 lands ~366 lines
lighter with the totality burden moved from read-time hope to commit-time law.
