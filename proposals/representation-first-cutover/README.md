# representation-first-cutover — 141 findings, six decisions

A bug bash against `bumbledb-log` returned **141 defects** (10 critical,
64 major, 67 minor). This subdirectory does not triage them. It reads
them as one signal and moves the **six data representations** whose wrong
form makes each family of bugs writable in the first place. Zero
backwards compat, hard cutover, one version number on the far side.

The thesis: the 141 are not 141 independent mistakes. They are the
shadows of six representational decisions made the easy way — a nullable
`pending` beside the chain, a `bool` for process liveness, a mutable
`prev` on a checkpoint, a floor consulted only by readers, a prose format
hand-parsed twice, a protocol hand-compiled into two drivers that take
opposite arms on identical bytes. Patch the shadows and you get 141
patches and a 142nd next week; move the decision and whole families stop
being expressible. Roughly half the corpus is Rust/TS divergence — the
same clause compiled twice — so the cure is not more review but making
the protocol an **artifact both drivers execute** rather than prose each
driver reimplements.

These documents are **normative** in the same sense the numbered PRD set
(`../00`–`../90`) is: they bind the build. The product laws (L1–L10), the
braids, the five deployment cases, and the "recovery is replay" thesis all
stand — this is how the implementation is made to *be* that law rather
than to approximate it twice. It changes how the protocol is represented
in code, not what it promises.

Read [00-thesis.md](00-thesis.md) first — the doctrine and the map from
141 findings to six decisions. [70-cutover.md](70-cutover.md) is the
deletion table and the order of operations;
[90-traceability.md](90-traceability.md) proves every finding is resolved
by a landed representation, not a patch.

| Doc | Decision | Dissolves |
| --- | --- | --- |
| [00-thesis.md](00-thesis.md) | Fix the representation, delete the bugs — doctrine, SPOV1/2/3, the meta-cause | — |
| [10-protocol-machine.md](10-protocol-machine.md) | The protocol is one transition table both drivers execute; arms are states, not code paths one driver forgot | 24 |
| [20-store-contract.md](20-store-contract.md) | The store is one contract: total-sum outcomes, success = durable + visible, keys a grammar, the lock a fenced CAS lease | 45 |
| [30-pending-chain.md](30-pending-chain.md) | Pending is a chain constructor, not a side-flag; the generation is a total function of the chain | 17 |
| [40-checkpoint-chain.md](40-checkpoint-chain.md) | The checkpoint chain is immutable and content-addressed; `prev` is inside the hash | 7 |
| [50-retention.md](50-retention.md) | The gc floor is a write-path invariant; the sweep is a resumable bottom segment; every resource has an owner | 23 |
| [60-codec-grammar.md](60-codec-grammar.md) | Parse, don't validate, at one codec: exact numbers, canonical bytes, bounded rows, half-open intervals | 25 |
| [70-cutover.md](70-cutover.md) | Hard cutover: `v:3`, the deletion table, the dependency order, the proof obligation | — |
| [90-traceability.md](90-traceability.md) | Every finding id 0–140 → the decision that dissolves it | 141 |

Each decision doc has the same shape: the current representation (with
finding ids), the target representation as a concrete delta against real
code, and the invariant that makes its bug family unrepresentable. The
counts sum to 141 across the six decisions; the split is the argument —
no lever moves fewer than seven bugs, the store contract alone resolves
nearly a third, and it and the codec grammar together account for half the
corpus.
