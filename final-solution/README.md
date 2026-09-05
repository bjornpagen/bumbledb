# Bumbledb 1.0 — convergence, not another rewrite wave

This is the replacement execution packet for the owner’s next swarm, intended for Grok Fast workers under a capable orchestrator. It supersedes every earlier proposal and completion label. It preserves useful current code and the product’s seven commitments; it is **not** evidence of a release.

Source: dirty `codex/bumbledb-1-0`, HEAD `4a0573692431ae7b2f8ef82d8663bd199a4058da`, reviewed 2026-09-05. Production paths were inspected while Cursor was still changing files. Symbol anchors and concrete schedules take precedence over old line numbers. No tests, builds, typechecks, benchmarks, deployment, commit or push ran in this proposal pass.

## Read and execute

1. [North star](00-north-star.md) and [semantics](01-semantic-contract.md): what stays and what must disappear.
2. [Binding interfaces](61-interface-contracts.md): selected representations, constructor authority, lifetime and failure rules. These are architectural decisions, not homework for individual workers.
3. [Core](11-core-findings.md), [log](21-log-findings.md), [SDK](31-sdk-findings.md) findings: current source counterexamples and retained obligations.
4. [Orchestration](60-cursor-execution.md): **21 exclusive execution lanes**, a proven twelve-worker ready frontier, shared-hub ownership and acceptance procedure.
5. Lane instructions: [core](62-core-lanes.md), [log](63-log-lanes.md), [native/SDK](64-native-sdk-lanes.md), [application/qualification](65-product-lanes.md). A lane section is its complete dispatch message; send it with its named contract excerpts.
6. [Discriminators](70-test-and-release-gates.md) and [continuity](50-audit-closure-matrix.md): D01–D29 plus all 68 inherited audit IDs, 220 child behaviors and 78 prior-review IDs.
7. [Retirement and evidence](90-evidence-and-retirement.md): review source, transfer permanent contracts, retire the packet, qualify the actual final input, then make the sole final commit/push when authorized.

Start with [PROMPT.md](PROMPT.md). Only [STATUS.md](STATUS.md) tracks current execution; no implementation, fixtures, wave-report or exhaust folder.

## What this pass changes

- Replace parked session reactors with fixed-worker resource tables: idle snapshots consume memory/read leases, not worker threads.
- Make allocation charges and cache generations belong to the retained allocation, including queued delivery and old readers.
- Separate complete-state admission from incremental judgment with a lawful-parent premise.
- Make paged delivery transactional at the **native call’s** boundary, not just a single internal row.
- Replace Pack’s all-in-RAM final sort with exact scratch group IDs and ordered claim streaming.
- Enforce checkpoint-base-aware direct locators, coherent negative proof and bounded materialization through every caller.
- Permit symbolic migration field expressions; resolve their types against verified source snapshots before any effects.
- Use kernel-held repository exclusion; delete PID/stale-lock guessing.
- Make “integrated” a coordinator source verdict, not a worker’s claim that declarations exist.

## Completion equation

`replacement used in production ∧ every consumer adapted ∧ predecessor deleted ∧ discriminators authored ∧ composed source reviewed ∧ final required evidence passed`

Each conjunct is mandatory. “Future tightening,” “ready except callers,” “mock passed,” and “compiles” are not alternative definitions of completion.

The swarm must run **at least twelve useful execution workers concurrently**, excluding the orchestrator, after the small declaration cut. This packet does not grant permission to ignore tool capacity. A host that cannot do that must report the mismatch before starting. No product checks or interim commits during the swarm; no weakening scope to get green at the end.
