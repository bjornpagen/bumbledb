# Current native and TypeScript findings

Source observations from the moving dirty tree; no build/test was run. TS-001–020 and REVIEW-001–004 remain in [50](50-audit-closure-matrix.md). C7/C8 select the correction; [64](64-native-sdk-lanes.md) assigns exclusive writers.

## TS-008/009 — Session open waits for session death

`ts/crate/src/runtime/session.rs::spawn_read_session` waits on ready_rx. `runtime.rs::run_lane_command` calls host_read_lane, which enters a receive loop until Close, and sends readiness only after that call returns. The returned session is already finished. Write opening has the same shape.

The lane sender does not wake the worker’s ordinary Condvar, its wait loop does not recheck the lane inbox, and an opening pool job may claim its own worker then wait for itself. Fixing one send’s location is not enough. Idle sessions monopolize configured workers, starving unrelated jobs/control.

**Selected correction:** L07 supplies owned pinned read frames; L12 stores resources in fixed worker-local tables and runs one job at a time through the normal event loop. Idle handles do not park stacks. L13/L14 mint the same capability for core/log. Delete the parked read/write reactor and unused JS-driven writer ABI, not the useful snapshot/execution-session product. D24 checks one worker, sleeping worker, multiple idle snapshots, readiness, read and joined close. Source-confirmed deadlock.

## TS-002/003 — Registry insertion, charge and release remain separate lifetimes

`runtime/registry.rs::with_payload` holds a runtime-wide mutex across payload execution. Cursor pull and collection can perform scratch reads/conversion under it. `RegistryAdmission::admit` inserts a payload before retained-byte admission; failed admission leaves an orphan. Revoke retains tombstone rows. The retained guard lives in a JS-held wrapper rather than in the payload.

A runtime revoke call and take_retained method landed during review; do not report them as still absent. They do not alone prove off-thread reclamation, in-flight ownership, handle-count refund or bounded metadata.

L12 makes worker table ownership and payload charges authoritative; short-held shared routing metadata does no heavy work. L13/L14 replace wrapper-side ownership, including abandoned outputs and command resources. Close is a coalesced existing-owner obligation, not rejectable submit_control work. D18/D29 retain JS tokens, fail admission, fill normal queues and repeat long open/close histories; actual memory/locks/rows drain. No permanent tombstone ledger or counter-only green.

## TS-001/004/005/006/007 — Whole native operation boundaries

`db_wire.rs::cursor_pull` can consume earlier rows then throw on a later row’s remaining budget (CORE-008). The direct native paths now call bounded core APIs; that integration is useful, but the unit of atomicity is still wrong.

L05/L13 provide one native delivery ticket, admitted output and final position commit; L16 intersects limits and scopes the Stream. D12/D25 include two individually fitting rows that do not jointly fit, cancellation after the first copy, result lifetime beyond snapshot, and RAM/scratch parity.

Draft ingestion keeps one cumulative budget/deadline across chunks and finish. Host length checks precede conversion; a failed draft is terminal. Each operation receives fresh work without stealing the snapshot’s acquisition deadline. L13/L16 own actual production paths, not just policy-wire shape assertions. D01/D07/D18.

## TS-012/016 — Removing field<T> broke useful symbolic expressions

`ts/src/scalar.ts::field` now returns an unasserted field ref, but judgeMigration throws on every field node, and add/cast/etc recursively call it. Thus normal source-dependent arithmetic/backfill cannot be authored. QueryNode and MigrationNode still duplicate the operator roster despite shared imports.

L15 implements C1’s one scoped grammar: query leaves derive known kinds; migration field names remain unresolved until native schema binding. `Scalar.add(Scalar.field("units"), Scalar.u64(1n))` is valid metadata and is compiled against the verified source snapshot, not rejected for containing a field. A missing/incorrectly typed field refuses before effects even with zero rows. No fake generic field type and no unchecked native execution.

L14/L10 enforce full compile; D19/D20/D27 run field arithmetic through a real generated migration and query analogue, including i64/u64 distinction and wrong-field cases. Delete duplicate constructor/roster judgment, not field-dependent functionality.

## TS-013 — Preserve full-chain compile and prove the append path

`migration_wire.rs::verify_compiled_chain` and mandatory snapshots have landed. Preserve them. L14/L17 must cover verify, append, generation, runtime contract, initialize and freeze with the same source/target chain binding; no optional snapshots or final-hash-only fast path. D20 uses invalid mappings on empty data and checks absence of side effects. A good verification entry point cannot certify a separate unchecked append.

## TS-014/015 — Stale-lock recovery steals a live generator’s lock

`ts-log/src/migrations/fsops.ts::createGenerationLock` creates the lock before writing its PID. Another call sees the empty file, readLockPid returns null, and tryAcquireGenerationLock deletes it as stale. Two generators then proceed. Two reclaimers can likewise unlink a successor; release ignores the written token. stat→readFile still does not bound a growing file.

C8 selects the existing native kernel-held directory lock and persistent inode, not another stale-file algorithm. L14 exposes minimal internal acquisition/drain; L17 scopes it across generation and joined I/O, reads bounded same-descriptor chunks, writes immutable artifacts and commits manifest last. D21/D28 exercise actual same/cross-process exclusion, process death, owner release and manifest recovery. Delete PID/processAlive/readLockPid/stale unlink, not merely add retries.

UUID exclusive temp files and surfaced directory-sync failures are real improvements to preserve. They do not establish repository transaction ownership.

## TS-010/011/017/018/019 — Effect and application semantics still need the real path

Lazy addon acquisition and internal/log exports are useful. L16 reads the installed Effect 4 docs and closes every acquisition/callback/finalizer gap, without restoring Promise twins or superbuilders/errors. L17 imports core primitives literally. L18 updates actual Notes and native-ledger-shaped consumers without touching sibling Edullm.

D22 uses packed artifacts, not workspace aliases, and removes addon availability during pure authoring import. Real command refs, witnessed correction, generated source-field backfill, reopen, backup/restore and close are mandatory. Useful scripted Cause tests remain, but do not stand in for native ownership/publication producers.

## TS-020 / REVIEW-001–004 — Qualification and subtraction

Candidate kind/mode/symlink framing and packed provenance have landed in scripts. Preserve them. Required real S3 and Graviton cells remain unqualified and no release-results manifest exists; that is honest, not a reason to invent one.

L21 repairs remaining evidence/retirement dependencies and runner ordering, L19 current proof correspondence, L20 real performance/storage measurement inputs, L18 public docs/examples. D23 rejects garbage/stale/duplicate evidence. Retire the proposal **before** final candidate identity/qualification, after transferring contracts/checklists. “Checks → delete inputs → commit” certifies the wrong tree.

Delete stale symbol/word census, vacuous smoke tests, obsolete API/format corpora and duplicated expensive setup by responsibility. Preserve independent tiny models, canonical expected bytes, failure schedules and genuine scale qualification. There is no lines-deleted quota and no new fixture/exhaust/implementation hierarchy.
