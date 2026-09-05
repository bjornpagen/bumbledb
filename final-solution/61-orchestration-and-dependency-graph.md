# 61 — Parallel execution, without parallel architectures

This is the execution authority for the next implementation campaign. The
current handoff only preserves source and writes this plan; it does not restart
agents. Start a future campaign with [PROMPT.md](../PROMPT.md).

## Read once, then give every agent a bounded contract

The orchestrator reads the entire proposal and audit. Every implementation
agent reads 00, 01, 02, this chapter, its complete packet in [62](62-work-packets.md),
its contracts in [63](63-shared-interface-contracts.md), its normative chapters,
and the corresponding audit entries and chapter 70 gates. A short dispatch
message is routing, never a substitute for those documents. Independent review
agents must also read adjacent producers/consumers, not only their own diff.

Authority order:

1. The owner's latest explicit instructions, including final-only verification.
2. Chapter 00's selected product and subtraction contract.
3. Detailed semantics: 10–13, 20–22, 30–35, 40–41; chapter 35 owns Effect shape.
4. This chapter's scheduling rules, chapter 62 ownership, chapter 63 interface
   handoffs, chapter 64 verification timing. These supersede old sequencing
   language in milestone narratives, not their semantic requirements.
5. Chapters 50/70 are the complete obligation set; 90 and implementation notes
   record historical evidence, never proof that current code passes.

If two normative contracts genuinely disagree, the orchestrator writes one
bounded decision with affected consumers before they implement divergent APIs.
Do not ask the owner to choose local symbol names. Do ask before expanding the
product, weakening a guarantee, changing the selected numerical semantics or
performing unauthorized deployment/publication.

## Phase machine

| Phase | Work allowed | Exit |
| --- | --- | --- |
| F0 — Inventory and contract handoff | Read source/docs; assign files; publish real type/function declarations and decision records; author fixtures | Each cross-lane interface has one owner, concrete source location and acknowledged consumer |
| F1 — Broad parallel implementation | Implement full lanes, author tests/models/probes, cross-review and integrate continuously | All selected functionality and deletion obligations implemented; no unresolved dependency/stub |
| F2 — Integration freeze | Finish source review, exports, examples, coverage mapping and ownership audit | Orchestrator records the all-lanes-ready barrier described below |
| F3 — Final verification and repair | Fresh installs/builds/types/tests/models/benchmarks/platform qualification; fix failures and requalify | Exact candidate evidence complete, or honestly reported blocked with no release claim |
| F4 — Handoff | Final source/evidence commit and push; report exact remaining external gates | Branch pushed; publication remains separately authorized |

**No tests, typechecks, builds, linters, benchmark executions or executable
probes in F0–F2.** Reading installed dependency docs/types is allowed. Author
tests now; run them in F3. Do not start CI by intermediate pushes. This trades
early feedback for broad implementation, as explicitly requested. It does not
permit skipping the final matrix. Once F3 starts, repairs and repeated checks
remain part of that final verification campaign; there is no one-shot rule.

## Pipeline graph

Dependencies below distinguish **contract-ready** (consumers may implement)
from **implementation-ready** (cross-lane integration can finish). A consumer
does not wait for a producer's test run; there are none until F3. Declaration
availability is not behavior completion, and must not be reported as such.

| Packet | Implement against these contract handoffs | Completion also needs |
| --- | --- | --- |
| P00 Integration | Existing proposal and stopped source | All packets, global hub edits, final evidence |
| P01 Values/admission | C01/C02/C03 | P02 physical candidate/index support |
| P02 LMDB/storage | C01/C02/C04 | P01 canonical parser and candidate judgment |
| P03 Queries | C01/C02/C04/C05 | P01 semantics, P02 cursor/snapshot access |
| P04 History machine | C01/C02/C04/C06/C07 | P01/P02 atomic host adjunct, P05 backend operations |
| P05 Backends/recovery | C04/C06/C07/C08 | P04 authority transitions, P02 snapshot stream |
| P06 Native runtime | C02/C04/C05/C06/C07/C09 | P02/P03/P04/P05 native operations |
| P07 Core TS SDK | C01/C02/C05/C09/C10 | P01/P03/P06 |
| P08 Log TS/tenants | C06/C08/C09/C10 | P04/P05/P06/P07, P09 admin bindings |
| P09 Native migration/admin | C01/C04/C05/C06/C08/C11 | P02/P03/P04/P05 |
| P10 TS migration generator | C01/C05/C10/C11 | P07 shared scalar/schema, P09 canonical plan codec |
| P11 Proofs/models | C01/C03/C05/C06/C11 | Final source correspondence from owning packets |
| P12 Adversarial suite | All relevant contracts | Integrated native/public behavior; execution in F3 |
| P13 Packaging/apps/docs | C09/C10/C11/C12 | P07/P08/P09/P10; exact artifact qualification in F3 |
| P14 Performance/space | C01/C02/C04/C05/C09/C12 | Integrated product and final-only measurements |

P01/P02 have a semantic/physical handshake, not a scheduling deadlock: P01
declares canonical facts and the candidate-judge input; P02 declares transaction
and index access. Both implement after those declarations exist. P04/P05 likewise
agree on authority records and conditional store verbs before either is finished.
P06 publishes native operation/handle ownership before all operations exist;
P07/P08 implement callers against the declarations, never a mock production path.

Suggested broad launch:

- First dispatch P01, P02, P04, P06 and P11 where slots allow. P00 writes shared
  handoffs and maps obligations in parallel, not merely watches agents.
- As contracts arrive, immediately dispatch P03, P05, P07, P08, P09 and P10.
  Do not impose an all-engine-done barrier before SDK or log work begins.
- P12/P13/P14 can author adversarial fixtures, consumer examples and measurement
  harnesses from the beginning. They need integrated code only for execution.
- Review completed portions while other portions are being implemented. A
  producer broadcasts interface changes with the affected contract IDs once;
  consumers acknowledge and change their own files.

This is a dependency pipeline, not 15 simultaneous imaginary workers. Discover
the actual tool capacity. With only three worker slots, use three long-lived
bundles: engine (P01/P02/P03), history (P04/P05/P09), and SDK (P06/P07/P08/P10).
P00 owns integration plus schedules P11–P14 as slots free, or performs their
work itself. With more capacity split the bundles along packet/file boundaries.
Deep fanout is useful for independent proof, regression, codec, packaging and
review work; a child must receive an exclusive subset of its parent's files.
Never spawn children that all edit the same module or simply reread everything.

## File ownership, not optimistic merge conflict recovery

One active writer per file. P00 maintains a small
`implementation/campaign-status.md` when implementation starts: phase, live
agents, packet status, claimed paths, received contracts, blocked consumers and
next dispatch. It is a work ledger, not another release result database.

The path lists in 62 are default allocations. Before dispatch resolve overlaps
to exact files. New sibling modules may be created under a claimed directory;
moving an existing file requires both owners to agree. A file not assigned is
unclaimed, not free for everyone. P00 owns shared root modules, manifests/locks,
central exports/tags/errors and CI wiring until explicitly transferring one.
Subagents send exact small hub patches or requested declarations to that owner.

Agents do not commit, push, reset, stash, delete other agents' edits, change
versions, regenerate shared corpora or run validation. Do not use formatter-wide
rewrites while another agent is editing. Same-checkout edits are already shared;
no cherry-pick fiction. If separate worktrees are deliberately used, record their
base and integrate their diffs before F2 rather than claiming remote completion.

## Handoffs that survive model changes

Each dispatch includes: packet ID; full read list; exact write paths/exclusions;
input contracts and their actual source paths; concrete deliverables; tests to
author; prohibited scope; return format; current phase/no-execution rule.

Each packet maintains a short `implementation/packets/Pxx.md` during execution:

```text
Packet / owner / phase:
Files owned and changed:
Input contracts received (IDs + actual symbols):
Output contracts supplied (IDs + actual symbols):
Implemented behavior:
Deleted mechanisms:
Authored tests (paths/names -> audit/gate IDs):
Unresolved source dependencies and known defects:
Cross-review challenge / resolution:
Verification: NotRun (until F3); then exact evidence references:
```

Do not return only “done” or a count of changed files. A stopped agent leaves
its actual unfinished boundary. Its replacement reads the packet, current diff
and source; it does not repeat completed work from the original prompt. The
present [frozen handoff](../implementation/06-frozen-implementation-handoff.md)
is the first such boundary and must be read before any dispatch.

## The all-lanes-ready barrier

P00 may enter F3 only after every packet's source and authored tests are
integrated, all shared declarations have real implementations, no old public
compatibility path remains, and every chapter 50 obligation/chapter 70 child
has an owner and a designated test or external qualification lane. Read the
source for these claims; an agent's confidence is not a substitute.

P00 records the barrier in the campaign ledger and explicitly announces final
verification. Compilation can discover problems then; repair them without
weakening the contract. P11/P12 supply independent review, not production
self-oracles. Preserve all new counterexamples.

Do not delete `final-solution/` during this handoff or merely to make a completion
check green. In a future fully completed campaign, move the binding contracts,
decisions and obligation mapping into permanent docs before retiring this
folder. If implementation or qualification remains blocked, keep the proposal
as the remaining-work contract. `audit/` is always preserved.
