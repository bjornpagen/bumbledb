# Should primer's graph-shaped postgres workloads move to bumbledb read models?

READ-ONLY strategy investigation, 2026-07-17. Three parallel tracks: primer's postgres
estate, bumbledb's operational envelope, and the sync/consistency question. This is an
adoption question, not a feature proposal. Verdict up front: **REJECT for the runtime
read model; the authoring-side direction already exists and is already correct.**

---

## 1. THE INVENTORY

Primer is a Next.js (App Router) app on Vercel (`iad1`), postgres on AWS RDS
(drizzle-orm + `pg`), 3 queue-triggered functions and 7 crons (`vercel.json`). All
heavy graph *writes* happen in a long-lived **local** bun TUI process
(`scripts/tui.ts`: graph-builder, course-manager, demand-factory) against the same
postgres. There is **zero `WITH RECURSIVE` anywhere in the repo — by explicit design.**

### Workload 1 — course prerequisite DAG + materialized closure (the exhibit). RECURSION-SHAPED WIN, ALREADY SOLVED.

- Tables: `course_prerequisites` (edge table, two FKs to `course_lessons`),
  `course_prerequisite_closure` (`lesson_id, ancestor_id`; schema doc: "keeps ancestor
  lookups free of recursive SQL: a lesson's full prerequisite ancestry is one flat read").
  Files: `/Users/bjorn/Documents/primer/src/db/schemas/course/{prerequisites,prerequisite-closure}.ts`.
- Algorithm: `/Users/bjorn/Documents/primer/src/db/queries/course-closure.ts` —
  `computeAncestorClosure`, an in-process memoized DFS with cycle detection
  (`ErrPrerequisiteCycle` fails closed), then scoped delete + batch reinsert.
  **Called in the same postgres transaction as the edge writes.** Exactly two callers:
  - `/Users/bjorn/Documents/primer/src/tools/graph-builder/etl/etl.ts:511` (TUI pipeline quiescence, local)
  - `/Users/bjorn/Documents/primer/src/db/queries/course-delivery/frame-backed/insert-mapped-prerequisites.ts:49` (TUI course-clone, local)
- Readers (these run **on Vercel**):
  - Onboarding placement: `src/db/queries/onboarding/course-prerequisite-edges.ts` →
    `src/lib/onboarding/engine/resolve-course.ts` → `simulateFrontier` — needs the whole
    ancestor chain; runs in the `onboarding-requests` queue function and the
    `onboarding-launch`/`onboarding-complete` every-minute crons.
  - Graph API + studio: `src/db/queries/course-graph-dto/**` loads closure rows into one
    flat DTO; `src/app/api/v0/graph/**` does per-request in-memory BFS/walks
    (`query/{closure,subgraph,strand,search}.ts`) over it. `deriveCourseGraphNodeDepths`
    deliberately has no cycle guard because "the closure table is its materialized
    acyclicity proof."
- The **student-facing frontier hotpath does NOT read the closure**
  (`src/db/queries/learning-run-frontier/build-run-frontier-candidate-ctes.ts`):
  prerequisite satisfaction there is a one-hop `NOT EXISTS` over direct edges against
  write-time-settled mastery. The closure exists precisely so this path never recurses.
- Profile: written in rare authoring bursts (a few per day at most, local, cold path);
  read constantly (every placement job, every graph API hit). Readers tolerate
  minutes of staleness *in principle* — but today they get transactional freshness for free.

### Workload 2 — the graph-builder itself. ALREADY BUMBLEDB. Local only.

`/Users/bjorn/Documents/primer/src/tools/graph-builder/**` (~31.6K LOC): one durable
bumbledb store per run (`store/run-store.ts`, `out/graph-builder/runs/<runId>/store`,
6–18 MB each), topological-rank gates, in-degree scheduling, ~50 prepared queries;
at quiescence `etl/etl.ts` materializes the judged store into postgres in one read +
one drizzle transaction per sheet, with receipt facts for idempotency. **The existing
data flow is bumbledb → postgres — the exact inverse of the proposed read model.**
Smaller consumers: `src/tools/{visual,science-toy}-benchmark/seed.ts`.
`@bjornpagen/bumbledb: 0.4.0` is already in `package.json`.

### Workload 3 — demand niche merge union-find. BORDERLINE, not a candidate.

`src/server/demand/synthesis/merge-fold.ts`: in-memory union-find at write time (TUI,
local); cycle-closing edges deferred to the next sweep. Persisted flat
(`demand_niche_transitions` + winner columns); evidence repointed eagerly so **no
reader ever follows chains**. All reads are plain joins.

### Workload 4 — evidence duplicate canonical pointers. PLAIN JOIN by construction.

`src/db/schemas/demand/evidence-duplicates.ts`: PK on `evidence_id` + orientation CHECK
(`evidence_id > canonical_evidence_id`) + lowest-uuidv7 canonical means chains collapse
to one hop; dedupe re-derives whole groups per run. Sole read: an anti-join
(`src/db/queries/demand/duplicates.ts`).

### False positives

`demand_seo_task_closures`, `learning_run_dynamic_offer_batch_miss_closures`, etc. —
"closure" = closing/tombstone events, not transitive closure. `ledger-offer-cascade.ts`
is FK fan-out delete, no traversal. The frontier/journey/settlement CTE estates are
large but strictly non-recursive. The course→unit→lesson→frame hierarchy is fixed-depth
joins. Other self-ref/double-FK hits are provenance links never walked transitively.

**Inventory verdict: exactly ONE recursion-shaped postgres workload exists, and primer
has already architected it out of postgres** — host DFS at write time, materialized
closure, uuidv7-minted-in-topological-order lesson ids, per-request in-memory walks
over a flat DTO on the read side.

---

## 2. FEASIBILITY

### Can a bumbledb store run where the readers run? No — three independent blockers.

The closure's runtime readers are Vercel serverless functions (queue consumers, crons,
API routes). Against that substrate:

1. **No linux binary.** `ts/package.json` optionalDependencies ship exactly
   `@bjornpagen/bumbledb-darwin-arm64`; `ts/src/native.ts:477`
   (`SHIPPED_PLATFORMS = "darwin-arm64"`) throws at module load on any other host.
   The loader comment calls linux "pure addition," but today it does not exist, and CI
   runs macos-latest only (x86_64-linux is a cross-compile check, no test lane).
2. **No read-only open mode.** No `MDB_RDONLY` path anywhere in `crates/`; every open
   is read-write. Even `exhume` — the "read-only, theory-less open"
   (`docs/architecture/70-api.md:362-401`) — takes the same exclusive advisory lock by
   **creating and writing** `bumbledb.lock`
   (`crates/bumbledb/src/storage/env/acquire_lock.rs:12-16`). A store file inside a
   read-only deploy bundle cannot be opened in place; every cold start would copy to `/tmp`.
3. **Single-process by recorded decision.** `00-product.md:88-96`: multi-process access
   is out of the envelope in v0; a second handle on the same path fails loudly
   (`EnvironmentLocked`). Per-instance `/tmp` copies dodge the lock but instantiate
   failure mode 6 below (cross-instance skew). Also: 4 GiB fixed map
   (`storage/env.rs:168`, `const MAP_SIZE: usize = 4 << 30`, no resize path) — a
   non-issue at primer's 6–18 MB graphs, but part of the envelope.

**Long-lived worker: the designed shape — but primer has no such worker in production.**
One process, filesystem, many reader threads, single writer: this is exactly what
`00-product.md` promises, and exactly what the graph-builder TUI already is — locally,
at authoring time. Primer's production is Vercel-only; there is no container to put
the store in. **Build-time artifact:** technically viable (`next.config.ts:83` already
ships a binary artifact via `outputFileTracingIncludes`), but course graphs change via
TUI ETL against the live DB **without a deploy**, so a build-time snapshot goes stale
until the next deploy. Dead on arrival for this data.

### What the derivation pipeline would look like (if built anyway)

Full rebuild, never incremental — everything in primer points that way: two
transaction-scoped write chokepoints, operator-cadence writes, single-digit-MB data,
and an explicitly anti-diff house style (closure = delete-and-reinsert; rebirth =
"idempotent-by-refusal … not by diffing", `src/tools/graph-builder/store/rebirth.ts`).
Best available shape: outbox row written in the same postgres transaction as the graph
write → swept by the every-minute maintenance cron (existing pattern) → rebuild a fresh
store (inverse of `etl.ts`, likely smaller) → upload to S3/Blob (S3 already wired) →
flip a version pointer → functions download to `/tmp` on cold start / pointer change.
Staleness window ~1 minute; requires atomic version-flip publishing, bumbledb moved to
prod dependencies, a linux native build, and `serverExternalPackages`.

### What breaks when postgres and the read model disagree

1. Edge added, store stale → student offered a lesson whose new prerequisite chain
   isn't satisfied; the mastery ledger records honest misses against a dishonest offer.
2. Edge removed, store stale → lesson stays locked; a frontier can read as empty.
3. Course published, store not rebuilt → course invisible to placement; onboarding
   launch fails or places at a wrong/empty entry set.
4. Course deleted, store stale → dangling lesson ids; FK-backed reads become 404/500.
5. Torn store (rebuild dies mid-write without atomic swap) → a partial graph that is
   **not any state postgres ever held** — worse than stale.
6. Cross-instance skew (instance A on v42, B on v41) → one student sees different
   frontiers on consecutive requests. Postgres-only primer cannot exhibit this today;
   it is a wholly new failure class.
7. **The scope trap:** runtime "prerequisites satisfied" joins the closure against
   per-student mastery state that changes on every interaction. If mastery leaked into
   the read model, staleness becomes per-answer and every mode above fires constantly.
   The read model could only ever hold the authoring-time graph.

And the bar it's measured against: today, staleness is **unrepresentable by
construction** — every writer funnels through `populateCoursePrerequisiteClosure` in
the same transaction. Any out-of-process pipeline is a strict consistency regression.

---

## 3. GOAL ALIGNMENT

`docs/architecture/00-product.md` is unambiguous about what bumbledb is:

- "built by and for one user (Bjorn Pagen) and his applications … Not a product, not a
  server, no external API-stability obligations." Non-goals, verbatim: "Server mode.
  Network protocol … Async API. Multiple writers. Multi-process access."
- "Compatibility is never a design input" — the on-disk format breaks each release and
  data is "ETL'd forward or regenerated." (Tolerable for a derived store, but it chains
  every bumbledb upgrade to a primer rebuild+redeploy.)
- Apple Silicon is the only performance target; other platforms get scalar fallback
  "with no performance promises." A linux read model runs on the exact hardware the
  perf thesis excludes.

**Honest case FOR in-scope:** primer *is* one of the owner's applications — its 74-table
postgres schema was one of the two censused workloads that drove bumbledb's feature set,
and the workload description ("reference walks … read-heavy") fits. Engine recursion
(stratified fixpoints, `MAX_PREDICATES = 16`, `20-query-ir.md`) genuinely covers the
closure — `ts/COOKBOOK.md` recipe 24 is a ~22-line reachability `program()`, exercised
in `ts/test/cookbook.test.ts`. The graph-builder shows the intended integration already
working: a durable authoring store, judged, then ETL'd into postgres.

**Honest case AGAINST:** "read model derived from postgres, downloaded by serverless
fleet instances" is a distributed-systems role — replication, versioned publish,
multi-instance readers, linux prod packaging, read-only opens. Every one of those
contradicts a recorded decision or non-goal. Bumbledb serving primer looks like the
graph-builder: **bumbledb upstream of postgres at authoring time**, one process, one
machine, ETL at the boundary. Bumbledb *downstream* of postgres at serving time is
mission creep — it would drag the roadmap toward multi-process access, a linux perf
posture, and format stability, i.e. toward being the product the doc says it is not.

---

## 4. COST

**The pipeline (build cost):** a postgres→bumbledb derive module (inverse `etl.ts`),
outbox trigger rows at both write chokepoints, an S3 versioned-publish protocol with
atomic pointer flip, a `/tmp` download-and-open path in every consuming function, linux
napi packaging and a linux test lane for bumbledb, prod-dependency promotion. Primer
owns most idioms (outbox+cron sweep, S3, bumbledb schemas, receipt-based idempotency) —
this is weeks, not months. But two items are **bumbledb engine changes reversing
recorded rulings**: a genuine read-only/lockless open, and multi-process (or blessed
copy-per-instance) semantics.

**The ops burden (carry cost):** a second store to monitor, a staleness window to
alarm on, a publish protocol whose failure is a torn or skewed graph, rebuild chained
to every bumbledb format break, and failure modes 1–6 above as permanent operational
surface — versus postgres's transactional recompute, which has **zero** such surface.

**The two-stores tax:** the invariant "closure ≡ transitive closure of edges" is today
enforced inside one ACID boundary. Split across stores it becomes an eventually-
consistent property that must be monitored instead of guaranteed — a strict downgrade
purchased with new infrastructure.

**The do-nothing cost:** ~130 lines (`course-closure.ts`) that are correct, cycle-safe,
transactionally consistent, and fast (readers do one flat indexed read; the hotpath
frontier doesn't even use the closure). Recompute cost is per-course-grade DFS over
hundreds of edges at authoring cadence — milliseconds, on a cold path, a few times a
day. **The do-nothing cost is approximately zero.** Even the minimal variant — using
`program()` to compute the closure inside the graph-builder's run store before ETL —
deletes nothing: the course-clone path (`insert-mapped-prerequisites.ts`) has no store,
so the host DFS must remain; you'd carry two closure implementations instead of one.

---

## 5. VERDICT

**REJECT the runtime read model.** Primer has exactly one recursion-shaped postgres
workload; it is already solved with a consistency guarantee (same-transaction recompute)
that a dual-store pipeline can only weaken, at the cost of new infrastructure, new
failure classes, and bumbledb engine changes that reverse recorded v0 rulings
(read-only open, multi-process, linux packaging/perf posture). The exhibit is not
evidence of a gap — it is evidence primer already found the right shape: recursion at
write time on the cold path, flat reads on the hot path. Bumbledb's honest role in
primer is the one it already plays: the authoring-side judged store **upstream** of
postgres.

**Conditions that would reopen the question (all must hold):**
1. A long-lived linux worker/container enters primer's production topology for
   independent reasons (the store then has a legitimate one-process home).
2. Bumbledb ships a linux npm binary with a tested lane, AND a genuine read-only or
   multi-process-safe open — each an owner decision reversing a recorded non-goal.
3. A new workload appears that postgres actually struggles with — deep recursive
   queries over graphs too large or too write-hot to materialize (none exists today;
   graphs are single-digit MB and authoring-cadence).

**Recommend instead (no action required):** keep the bumbledb→postgres direction and
let it absorb more authoring-time graph judgment (gates, closure verification,
acyclicity proofs inside the run store, where `program()` is native). If the owner
wants a bumbledb win from this investigation, it is a validation one: the graph-builder
estate is a live production census sighting for engine recursion — evidence for the
closure idiom's design, not a reason to build a replication story.
