export const meta = {
  name: 'bugbash-perf-campaign-v2',
  description: 'Resume of the paused bug bash + perf campaign, from-scratch trust posture: the landed opus-authored instrument estate is re-reviewed as untrusted, then gate, traced baseline + tally, trace-driven perf targets, fresh bug audit, fixes, re-bench. All agents fable.',
  phases: [
    { title: 'Review', detail: 'from-scratch review of the landed instrument estate (d7c111a8..ccd0de8d) — untrusted until verified against spec + doctrine, fixed where short' },
    { title: 'Gate', detail: 'zero-cost-off proven, estate green' },
    { title: 'Baseline', detail: 'sequential: full suite + owed bench debt (C17, capacity lanes) + traced/alloc passes + TALLY' },
    { title: 'Hunt', detail: 'trace-attribution readers + fresh bug-bash finders, concurrent' },
    { title: 'Verify', detail: 'adversarial verification of every finding' },
    { title: 'Fix', detail: 'file-disjoint fix lanes driven by attribution + verified bugs' },
    { title: 'Rebench', detail: 'targeted lanes vs baseline, honest deltas' },
    { title: 'Close', detail: 'ledger, doc re-pins, tally, push' },
  ],
}

// MODEL RULING (owner, 2026-08-01, supersedes the 2026-07-25 opus ruling): every
// agent in this workflow pins model 'fable' — leaf work at effort 'low', the
// orchestrating session is fable on high. No opus anywhere.

const ROOT = '/Users/bjorn/Documents/bumbledb'
const OUT = ROOT + '/audit-2026-07-25'
const BASE = ROOT + '/bench-out/baseline-2026-07-25'

const POLICY = `
CONTEXT (read before anything): repo ${ROOT}, current release v0.9.0 (post capacity-cutover, post GJ-split). This campaign lives on branch bugbash-perf (standing PR #14) — main holds only released states; NEVER push to main. Doctrine files: docs/design/representation-first.md (representation over control flow), docs/architecture/40-execution.md §introspection+measured-mechanisms (observability doctrine: zero-cost-off, "no per-tuple labels, no always-on counters, no diagnostics allocation anywhere in the join loops"; introspection is a representation, not a mode). The obs estate: crates/bumbledb/src/obs.rs (trace feature, ZST-off spans, raw-tick stamps, drain-time conversion), exec/run/counters.rs (per-(node,phase) timers), bumbledb-bench/src/trace_out.rs (Chrome JSON + flame summary).
POLICY: maximal churn, maximal elegance, zero backwards compat. Every change lands with its test. Repo commit voice, commits end with:
  Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
GIT: own only your assigned paths, never add -A; commit per milestone; push each commit: git push origin bugbash-perf || (git pull --rebase origin bugbash-perf && git push origin bugbash-perf). No flock on this machine.
CONTEXT HYGIENE: redirect long build/bench output to files, grep tails; your FINAL action is always the structured summary.
`

const LANE_SCHEMA = { type: 'object', required: ['lane', 'completed', 'deferred', 'notes'], properties: { lane: { type: 'string' }, completed: { type: 'array', items: { type: 'object', required: ['item', 'commit'], properties: { item: { type: 'string' }, commit: { type: 'string' }, test: { type: 'string' } } } }, deferred: { type: 'array', items: { type: 'object', required: ['item', 'reason'], properties: { item: { type: 'string' }, reason: { type: 'string' } } } }, handoffs: { type: 'array', items: { type: 'string' } }, notes: { type: 'string' } } }
const GATE_SCHEMA = { type: 'object', required: ['green', 'failures'], properties: { green: { type: 'boolean' }, failures: { type: 'array', items: { type: 'object', required: ['gate', 'excerpt', 'suspect_paths'], properties: { gate: { type: 'string' }, excerpt: { type: 'string' }, suspect_paths: { type: 'array', items: { type: 'string' } } } } } } }
const FINDINGS_SCHEMA = { type: 'object', required: ['findings'], properties: { findings: { type: 'array', items: { type: 'object', required: ['title', 'category', 'file', 'summary', 'evidence', 'severity'], properties: { title: { type: 'string' }, category: { type: 'string', enum: ['bug', 'perf', 'incoherence', 'missing-free-feature', 'unification', 'inelegance', 'inappropriate-branching', 'lean-rust-drift', 'observability'] }, file: { type: 'string' }, line: { type: 'integer' }, summary: { type: 'string' }, evidence: { type: 'string' }, failure_scenario: { type: 'string' }, suggestion: { type: 'string' }, severity: { type: 'string', enum: ['critical', 'high', 'medium', 'low'] } } } } } }
const VERDICT_SCHEMA = { type: 'object', required: ['verdict', 'reasoning', 'report_markdown'], properties: { verdict: { type: 'string', enum: ['CONFIRMED', 'PLAUSIBLE', 'REFUTED'] }, reasoning: { type: 'string' }, corrected_summary: { type: 'string' }, report_markdown: { type: 'string' } } }
const ANALYSIS_SCHEMA = { type: 'object', required: ['lane_group', 'targets', 'notes'], properties: { lane_group: { type: 'string' }, targets: { type: 'array', items: { type: 'object', required: ['lane', 'sink', 'attributed_us', 'mechanism_hypothesis'], properties: { lane: { type: 'string' }, sink: { type: 'string', description: 'span/phase name + node where the time goes' }, attributed_us: { type: 'number' }, share: { type: 'string' }, mechanism_hypothesis: { type: 'string' }, flame: { type: 'string', description: 'path to the flamegraph SVG evidencing this' } } } }, notes: { type: 'string' } } }

const tryAgent = async (prompt, opts) => {
  try { return await agent(prompt, opts) } catch (e) { log(`${opts.label} threw: ${String(e).slice(0, 120)}`) }
  try { return await agent(prompt + '\n(RETRY: prior attempt died or could not emit its summary — check git log/status for its work first, absorb what is sound, redo only the missing, keep the transcript lean, END with the structured summary.)', { ...opts, label: opts.label + ':retry' }) }
  catch (e2) { log(`${opts.label}:retry threw`); return null }
}
const lane = (label, phaseName, prompt) => tryAgent(POLICY + '\n' + prompt, { label, phase: phaseName, schema: LANE_SCHEMA, model: 'fable', effort: 'low' })

// ================ Phase 1: Review (the landed instrument estate is UNTRUSTED) ================
phase('Review')
log('From-scratch review of the landed instrument estate (d7c111a8..ccd0de8d)')
const REVIEW = `
REVIEW FROM SCRATCH. The Instrument phase already landed (commits d7c111a8..ccd0de8d) under a model tier the owner no longer trusts for difficult work. Owner directive: treat your scope as an UNTRUSTED SUBMISSION — review it as a stranger's PR against the original spec below, never assume anything is right because it compiled or its tests pass. Method: git show every commit in your scope, then read every touched file WHOLE (a diff hides what it should have changed and didn't); check each doctrine wall directly (zero-cost-off trace ZSTs, NO per-row/per-tuple spans, no allocation in join loops, alloc×trace exclusivity, the measure.rs traced-sample discipline: timed batches untraced, ONE traced sample after); check the tests PIN real behavior (artifact contents / span nesting / golden shapes — not mere file-exists smoke where the spec demands more); check the spec was DELIVERED, not approximated. Everything that falls short: fix or rewrite freely (maximal churn, maximal elegance), each fix landing with its test, commit per fix. What survives review: record as verified in notes WITH the evidence (what you checked and how). Do not churn what you cannot improve.
THE ORIGINAL SPEC YOUR SCOPE MUST MEET:
`
const rev = (await parallel([
  () => lane('R1:lane-traces', 'Review', REVIEW + `LANE I1 — every bench lane learns to trace. You own crates/bumbledb-bench/** (driver, harness, scenarios, lanes, trace_out). Spec: (1) the scenarios driver takes --trace and emits per-QUERY warm+cold Chrome traces under <out>/trace/scenarios/<family>/<query>.{warm,cold}.json; (2) the write/commit lanes (writes families, crud ops, capacity/lawful judgment lanes) take --trace and emit per-lane traces — the JUDGMENT_*/LMDB_COMMIT spans reach an artifact; (3) every traced artifact ALSO lands as .folded (fold the span tree to stacks by enclosure) beside the .json; (4) the report embeds per-query flame top-10 like read_family does; (5) alloc/trace exclusivity stands — --alloc is a separate pass with the same per-query scoping in scenarios; (6) timed batches stay untraced, the traced sample is separate (measure.rs:95-115 discipline). Tests: a smoke test per traced path asserting artifacts exist and parse.`),
  () => lane('R2:dark-subsystems', 'Review', REVIEW + `LANE I2 — the dark subsystems lit. You own crates/bumbledb/src/** obs call sites (spans/events only; logic is NOT restructured — verify the landed commits didn't). Spec: spans/point-events in plan/planner.rs + plan/selectivity.rs (per DP rung + pruned-candidate COUNT point events, not per candidate), storage/read.rs, encoding/decode.rs + encode.rs (per-image-build batch decode span with a0=rows), exec/kernel.rs + exec/swar.rs (per-kernel-invocation-batch named events with a0=lanes), ir/normalize + ir/validate internals, verify_store/** — ALL at pass/batch/subproblem granularity under the HARD doctrine: zero-cost-off (trace feature ZSTs, obs.rs patterns exactly), NO per-row/per-tuple spans, no allocation in join loops. Names follow obs.rs's registry style. Every new span pinned in api/db/trace_tests.rs-style tests (event presence + nesting). check-asm + the alloc gate green for these files — run them yourself; this lane is the likeliest zero-cost-off hole.`),
  () => lane('R3:flamegraph-tooling', 'Review', REVIEW + `LANE I3 — flamegraph investigation DEAD SIMPLE. You own scripts/flame*.{sh,py}, bumbledb-bench trace_out export submodules, and the doc section. Spec: (1) scripts/flame.sh <family-or-lane> [query]: builds with trace, runs one traced sample under scripts/measure.sh, emits <out>/flame/<name>.folded + <name>.svg via a SELF-CONTAINED renderer (no network, no flamegraph.pl), prints the top-10 self-time table — one command, SVG on disk; (2) scripts/flamediff.sh <foldedA> <foldedB>: differential folded output + red/blue diff SVG (grow/shrink per frame); (3) Chrome JSONs still load in speedscope/chrome; (4) usage documented where the doc estate wants it, spec-census green; (5) a golden mini-trace → folded → SVG snapshot test. ACTUALLY RUN both scripts end to end on a real lane — reviewed tooling that was never executed is not reviewed.`),
])).filter(Boolean)
log(`Review done: ${rev.length}/3 lanes — ${rev.flatMap(l => l.completed || []).length} fixes landed`)

// ================ Phase 2: Gate ================
phase('Gate')
let gate = null
for (let round = 1; round <= 3; round++) {
  gate = await tryAgent(POLICY + `\nGATE (round ${round}) — the instrumentation must be free when off. Run: (1) cargo build --release -p bumbledb (NO trace feature) and prove zero-cost-off: scripts/check-asm.sh green, the alloc gate green, and spot-diff a hot symbol (probe_pass) disassembly against the pre-instrument commit (9ecba41b) if check-asm does not already pin it; (2) cargo test --workspace --all-targets (default features) then -p bumbledb --features trace; (3) scripts/check.sh; (4) ts suite untouched-check (should be zero ts changes — verify); (5) scripts/lean.sh (should be untouched — verify); (6) the I1/I3 smoke + golden tests. Failures: excerpt + suspect paths.`,
    { label: `gate:r${round}`, phase: 'Gate', schema: GATE_SCHEMA, model: 'fable', effort: 'low' })
  if (!gate || gate.green) break
  const byArea = {}
  for (const f of gate.failures) { const k = (f.suspect_paths && f.suspect_paths[0] || f.gate).split('/').slice(0, 3).join('/'); (byArea[k] = byArea[k] || []).push(f) }
  await parallel(Object.entries(byArea).map(([area, fs]) => () =>
    tryAgent(POLICY + `\nGATE FIXER for ${area}: fix so the seam meets (never weaken zero-cost-off to pass), commit.\nFAILURES:\n${JSON.stringify(fs, null, 1)}`, { label: `gfix:${area.split('/').pop()}`, phase: 'Gate', schema: LANE_SCHEMA, model: 'fable', effort: 'low' })
  ))
}
if (!gate || !gate.green) throw new Error('instrumentation gate not green — halting before the baseline would measure a broken estate')

// ================ Phase 3: Baseline (STRICTLY SEQUENTIAL — nothing else runs) ================
phase('Baseline')
const BENCH_COMMON = POLICY + `\nBASELINE BENCH RULES: wall power verified (pmset) before and after every lane; measurement mutex (scripts/measure.sh) held; NOTHING else may run during timed windows — build everything FIRST; each Bash invocation under ~8 minutes (drive lane-by-lane); oracle-gate everywhere the protocol gates; DNFs reported excluded-and-counted; output under ${BASE}/ mirroring the campaign-2026-07-23 layout; commit per segment.`
const benchSegs = [
  { label: 'bench:debt', task: `First retire the FROZEN BENCH DEBT (owner unfroze for this campaign): (1) the C17 measured choice — run the power-budget capacity lane under BOTH measure_children arms (CAPACITY_WEIGHT_SLOT in storage/commit/judgment.rs), same protocol; LAND THE WINNER: flip or keep the constant, DELETE the losing arm and the flag (zero traces), record the numbers as the CONSTRAINT comment at the site; if the slot arm wins, REPORT the ray-at-write-time corner for owner ruling in your notes (do not rule it); (2) the calendar capacity lane (fresh twin world) oracle-gated; (3) windowed/lawful re-pins under the capacity spelling. Commit code changes + bench artifacts.` },
  { label: 'bench:suite', task: `The full-suite baseline into ${BASE}: scenarios (all families), report-class durable+ephemeral x3, writes, churn (3 profiles), crud, curves, lawful, storage. Produce the vs-campaign-2026-07-23 delta table per suite (plus vs-night where campaign lanes rode night pins). MANIFEST with provenance.` },
  { label: 'bench:attribution', task: `The ATTRIBUTION passes: (1) traced pass — every scenario family per-query warm+cold + every write/judgment lane, via the new I1 machinery; generate .folded + .svg flamegraphs for ALL of them via the I3 tooling into ${BASE}/flame/; (2) alloc pass (separate, per doctrine) — per-query allocation footprints; (3) write ${BASE}/TALLY.md: every lane's number, delta vs prior estate, its top-5 span/phase attribution (absolute µs + share), its alloc footprint, and the flamegraph path — THE document the perf fanout reads. Honest gaps marked (e.g. sub-500ns lanes where attribution is below clock resolution).` },
]
const benchNotes = []
for (const seg of benchSegs) {
  const r = await tryAgent(BENCH_COMMON + '\n' + seg.task, { label: seg.label, phase: 'Baseline', schema: LANE_SCHEMA, model: 'fable', effort: 'low' })
  if (!r) throw new Error(`baseline segment ${seg.label} failed twice — halting rather than hunting without a baseline`)
  benchNotes.push(`[${seg.label}] ${r.notes}`)
}
log('Baseline complete with tally')

// ================ Phase 4: Hunt (analysis + bug bash, concurrent — machine is free) ================
phase('Hunt')
const READER_GROUPS = ['rings+graph (r1-r6, g1-g6)', 'olap+joins (o1-o6, j1-j6)', 'points+temporal (p1-p5, t1-t5)', 'writes+commit+judgment lanes (writes families, crud, lawful, capacity lanes)']
const readers = parallel(READER_GROUPS.map(g => () =>
  tryAgent(POLICY + `\nTRACE READER for ${g}. Read ${BASE}/TALLY.md, then the flamegraphs (.folded + .svg under ${BASE}/flame/) and Chrome traces for your lanes. Produce the attribution ranking: for each lane, where do the worst ABSOLUTE microseconds go (span/phase/node), what share, and the mechanism hypothesis tied to the actual code (open the cited spans' source sites — hypotheses must name file:line mechanisms, not vibes). Flag anomalies: phases with unexpected shares, cold-vs-warm divergences, alloc footprints out of line with row counts, sinks that grew vs the 2023-07 campaign estate. Rank by (absolute µs x fixability).`,
    { label: `read:${g.split(' ')[0]}`, phase: 'Hunt', schema: ANALYSIS_SCHEMA, model: 'fable', effort: 'low' })
))
const BUG_MISSIONS = [
  { k: 'capacity-judge', s: 'crates/bumbledb/src/storage/commit/{plan,judgment}.rs + verify_store capacity sweeps — the NEW capacity judgment: dependent-bound resolution, clipped walk, u128 measures, ray refusals, R16 fresh-row interplay, the post-C17 single arm' },
  { k: 'capacity-surface', s: 'validate_capacity + theory spec + both macro grammars capacity arms + ts capacity()/weigh()/within()/ref() + FFI capacity marshal + scripts/pin.ts — the NEW surface estate end to end' },
  { k: 'gj-split-live', s: 'plan/fj/gj_split + fold_split + derive_nodes + the production cyclic paths — the GJ split has been live under real load since 0.7.0: plan-shape correctness, second-cover behavior, interaction with fold pushdown' },
  { k: 'overlap-join-live', s: 'the order-based overlap join (exec/run + interval sweep) + const-operand Allen routing — crossover constant honesty (the provisional 16), mask coverage, converse orientations' },
  { k: 'storage-v7', s: 'storage/env (R17 lockless readers, R18 wipe, meta taxonomy), the one-allocator R16 paths, format v7 refusals — crash-shaped and concurrent-reader edge cases' },
  { k: 'ts-surface-fresh', s: 'ts/src changes since 0.8.0: by()/desc() zero-key, bool order tier split (NumericVarOk/OrderVarOk), capacity builder types, dispose lifetimes, explain() — type-tier soundness vs engine walls' },
  { k: 'lean-capacity-drift', s: 'lean/Bumbledb/Capacity.lean + Decide/Oracle capacity arms vs the engine judgment — drift hunt on the NEWEST proof estate incl. the C11 Admission form and C12 clip lemma vs the shipped clip' },
  { k: 'cross-branching-new', s: 'inappropriate-branching + missing-free-feature + unification sweep over ALL code added since commit fc0631a0 (the audit) — the three campaigns wrote fast; hunt what they special-cased' },
  { k: 'obs-estate', s: 'the instrumentation from this campaign (I1/I2/I3), POST-REVIEW state — zero-cost-off holes, span nesting bugs, folded/SVG generation correctness, plus observability gaps still standing (category: observability)' },
]
const finders = parallel(BUG_MISSIONS.map(m => () =>
  tryAgent(POLICY + `\nBUG-BASH FINDER: ${m.k}. Scope: ${m.s}. Read the actual code deeply. Hunt real bugs (logic errors, edge cases, race/durability holes), inefficiencies with a mechanism, incoherences (doc-vs-code, engine-vs-TS-vs-Lean walls), missing-for-free features, unifications, inappropriate branching. Quality bar: every finding cites code you read (file:line) with real evidence; no style nits, no speculation without mechanism. The prior audit's findings are in ${ROOT}/audit-2026-07 — do not re-report stamped items.`,
    { label: `find:${m.k}`, phase: 'Hunt', schema: FINDINGS_SCHEMA, model: 'fable', effort: 'low' })
))
const [readerResults, finderResults] = await Promise.all([readers, finders])
const attributions = readerResults.filter(Boolean)
let findings = finderResults.filter(Boolean).flatMap((r, i) => (r.findings || []).map(f => ({ ...f, finder: BUG_MISSIONS[i] ? BUG_MISSIONS[i].k : 'unknown' })))
const seen = new Set()
findings = findings.filter(f => { const k = ((f.file || '') + '|' + (f.title || '').toLowerCase()).slice(0, 120); if (seen.has(k)) return false; seen.add(k); return true })
log(`Hunt: ${attributions.length} attribution groups, ${findings.length} deduped findings`)

// ================ Phase 5: Verify ================
phase('Verify')
const verified = (await parallel(findings.map(f => () =>
  tryAgent(POLICY + `\nADVERSARIAL VERIFIER. Try to REFUTE this finding by reading the actual code (and in-repo research/docs where they are the spec — Free Join paper for joins, docs/architecture for contracts, Capacity.lean for capacity semantics). CONFIRMED = holds against real code; PLAUSIBLE = coherent but unpinned; REFUTED = misreads code / already handled. Default REFUTED for uncertain bug claims, PLAUSIBLE for coherent perf/design claims. If not refuted, write self-contained report_markdown ('## title', 'category | severity | verdict | finder', Summary, Evidence with verified file:line, Failure scenario / impact, Suggested fix).\nFINDING:\n${JSON.stringify(f, null, 1)}`,
    { label: 'verify:' + (f.title || '').slice(0, 40), phase: 'Verify', schema: VERDICT_SCHEMA, model: 'fable', effort: 'low' }).then(v => (v ? { ...f, ...v } : null))
))).filter(Boolean)
const confirmed = verified.filter(v => v.verdict !== 'REFUTED')
log(`Verify: ${confirmed.length} survive of ${verified.length}`)

// ================ Phase 6: Fix ================
phase('Fix')
const attributionDigest = JSON.stringify(attributions, null, 1).slice(0, 60000)
const plan = await tryAgent(POLICY + `\nFIX PLANNER (structure only, no code). Inputs: (a) attribution rankings:\n${attributionDigest}\n(b) ${confirmed.length} verified findings (titles): ${JSON.stringify(confirmed.map(c => ({ t: c.title, f: c.file, sev: c.severity, cat: c.category })), null, 0).slice(0, 20000)}\nProduce FILE-DISJOINT fix lanes (max 7): each lane = owned path globs + its work items (perf targets from the attribution ranking with the mechanism to implement + verified findings whose files fall in its territory). Perf items ranked by absolute-µs impact. Items whose fix would cross lanes get ONE owner + handoff notes. Return as the lane list in your notes as JSON: [{lane, paths, items:[...]}]. Skip perf targets whose mechanism is speculative — this campaign is data-driven or nothing.`,
  { label: 'fix:plan', phase: 'Fix', schema: LANE_SCHEMA, model: 'fable', effort: 'low' })
let fixLanes = []
try { fixLanes = JSON.parse((plan.notes.match(/\[[\s\S]*\]/) || ['[]'])[0]) } catch (e) { log('fix plan parse failed — falling back to per-file grouping') }
if (!fixLanes.length) {
  const byArea = {}
  for (const c of confirmed) { const k = c.file.split('/').slice(0, 4).join('/'); (byArea[k] = byArea[k] || []).push(c) }
  fixLanes = Object.entries(byArea).map(([k, items]) => ({ lane: k, paths: [k], items: items.map(i => i.title) }))
}
const fixResults = (await parallel(fixLanes.map(l => () =>
  lane('fix:' + (l.lane || '?').slice(0, 24), 'Fix', `FIX LANE. You own ONLY: ${JSON.stringify(l.paths)}. Work items (perf mechanisms are pre-ranked from trace attribution — implement the named mechanism; bugs carry their verified reports in ${OUT} once written, but the full verified list with evidence is available from the verifier outputs — re-derive detail from the code itself):\n${JSON.stringify(l.items, null, 1)}\nEvery fix lands with its test; perf fixes note which baseline lane + flamegraph sink they target (the Rebench phase measures exactly those). Targeted tests only; commit per item.`)
))).filter(Boolean)
log(`Fix: ${fixResults.flatMap(r => r.completed).length} items landed`)

// gates after fixes
let g2 = null
for (let round = 1; round <= 3; round++) {
  g2 = await tryAgent(POLICY + `\nPOST-FIX GATE (round ${round}): scripts/check.sh, cargo test --workspace, --features trace tests, ts suite, scripts/lean.sh, check-asm, alloc gate. Failures with excerpts + suspect paths.`,
    { label: `gate2:r${round}`, phase: 'Fix', schema: GATE_SCHEMA, model: 'fable', effort: 'low' })
  if (!g2 || g2.green) break
  await parallel((g2.failures || []).slice(0, 8).map((f, i) => () =>
    tryAgent(POLICY + `\nGATE FIXER: ${JSON.stringify(f, null, 1)} — fix, don't revert landed work, commit.`, { label: `g2fix:${i}`, phase: 'Fix', schema: LANE_SCHEMA, model: 'fable', effort: 'low' })))
}

// ================ Phase 7: Rebench ================
phase('Rebench')
const rebench = await tryAgent(BENCH_COMMON + `\nREBENCH: rerun exactly the lanes the fix lanes targeted (their summaries name them: ${JSON.stringify(fixResults.flatMap(r => (r.notes || '').slice(0, 300)))}) plus any lane whose code paths the fixes touched, same protocol as the baseline, into ${BASE}-post/. Produce the DELTA table vs ${BASE} (p50, attribution shift — re-trace the improved lanes and flamediff them via scripts/flamediff.sh, embedding the diff SVGs). Honest: regressions reported as measured; a fix whose lane did not move is called out as NOT CASHED.`,
  { label: 'rebench:targeted', phase: 'Rebench', schema: LANE_SCHEMA, model: 'fable', effort: 'low' })

// ================ Phase 8: Close ================
phase('Close')
const sevRank = { critical: 0, high: 1, medium: 2, low: 3 }
const pub = confirmed.sort((a, b) => (sevRank[a.severity] ?? 9) - (sevRank[b.severity] ?? 9)).map((f, i) => ({ ...f, path: `${OUT}/findings/${String(i + 1).padStart(3, '0')}-${(f.title || 'finding').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 50)}.md` }))
const batches = []
for (let i = 0; i < pub.length; i += 8) batches.push(pub.slice(i, i + 8))
await parallel(batches.map((b, bi) => () =>
  tryAgent(`Write these finding reports verbatim from report_markdown (compose from fields if empty) to their absolute paths with the Write tool, then return the list written.\nITEMS:\n${JSON.stringify(b.map(f => ({ path: f.path, report_markdown: f.report_markdown, title: f.title, category: f.category, severity: f.severity, verdict: f.verdict, file: f.file, summary: f.corrected_summary || f.summary, evidence: f.evidence, suggestion: f.suggestion })), null, 1)}`,
    { label: `write:batch-${bi + 1}`, phase: 'Close', schema: LANE_SCHEMA, model: 'fable', effort: 'low' })))
await tryAgent(POLICY + `\nCLOSE-OUT. (1) Write ${OUT}/README.md: the campaign ledger — findings tally (by verdict/severity/category) with outcome column (cross-reference the fix-lane commits: fixed <commit> / open), the perf campaign story (review → baseline → attribution → fixes → rebench deltas, with flamediff SVG paths), the C17 resolution, the observability upgrades shipped. (2) Stamp each ${OUT}/findings/* with its outcome. (3) True TODO.md + docs re-pins for any doc-cited number the rebench moved (regenerate README graphs from ${BASE}-post where headline lanes moved). (4) Final gates one last time: scripts/check.sh + scripts/lean.sh + ts suite; verdicts plainly. (5) Commit + push everything (branch bugbash-perf). Rebench summary for your context: ${JSON.stringify((rebench && rebench.notes || '').slice(0, 3000))}`,
  { label: 'close:ledger', phase: 'Close', schema: LANE_SCHEMA, model: 'fable', effort: 'low' })

return {
  review: rev.map(l => l.lane),
  reviewFixes: rev.flatMap(l => l.completed || []).length,
  gateGreen: gate ? gate.green : false,
  benchNotes,
  attributionTargets: attributions.flatMap(a => (a.targets || []).slice(0, 5).map(t => `${t.lane}: ${t.sink} ${t.attributed_us}us`)),
  findings: { total: verified.length, confirmed: confirmed.length },
  fixed: fixResults.flatMap(r => r.completed).length,
  rebench: rebench && rebench.notes,
  postFixGatesGreen: g2 ? g2.green : false,
}
