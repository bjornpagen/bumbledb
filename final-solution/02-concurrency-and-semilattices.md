# 02 — Use the algebra; do not invent another database around it

Status: selected 1.0 concurrency contract and required proofs/tests, not a claim that the successor already implements them.

The owner's instinct is right: if a useful multiwriter property follows from the representation, take it. The important distinction is **what merges**, **which laws survive that merge**, and **what a successful application command promises**. A join-semilattice theorem about sets does not settle all three.

## What 1.0 actually supports

| Surface | Concurrent work | Authority and publication |
| --- | --- | --- |
| Core embedded | Multiple caller threads and snapshot readers | One owning process/environment; LMDB serializes write transactions |
| LocalHistory, through TypeScript | Concurrent logical submissions and readers | One LMDB transaction commits facts, receipt and attachment; no network arbitration |
| HostedHistory, through TypeScript | Multiple independent clients/hosts, each with its own materialization | All contend through the tenant's single S3 HEAD; one successful CAS decides each successor |
| Different tenants | Independent owners, readers and write histories | No shared database transaction or cross-tenant atomicity |

One order is not one privileged writer. There is no permanent remote leader lease to acquire before submitting a command. Hosted replicas can compete; a loser refreshes and retries the **same immutable command identity** within its budget. Correctness does not require scheduling contenders one at a time. Fair latency is not guaranteed against unlimited/adversarial contention; bounded attempts return progress or uncertainty honestly.

LMDB's one local write transaction is not a defect to replace. It is the compact substrate that makes final-state admission and snapshot visibility tractable. Parallel read execution, independent tenants, and bounded CPU kernels provide useful concurrency without a new persistent synchronization model.

## The exact free theorem

For finite sets ordered by inclusion, union is a join:

```text
A ∪ B = B ∪ A
(A ∪ B) ∪ C = A ∪ (B ∪ C)
A ∪ A = A
```

For a componentwise collection of relations, the same laws hold. Repeated delivery and reordering of **insert-only set elements** cannot change the final union. This is already useful for deduplication, monotone recursive frontiers, and internal set merging.

Let `Valid(S)` mean every declared database law holds. A coordination-free merge additionally needs closure for the reachable states/transactions in question:

```text
Valid(A) ∧ Valid(B) ∧ CommonValidAncestor(A, B)
    ⇒ Valid(A ∪ B)
```

This is an invariant-confluence-style obligation, not a consequence of associativity. It must cover the actual operation grammar, not every imaginable state nor only a favorable example. Commands with reads, rejection receipts, deletion and external effects have additional observable semantics beyond their final union. Application entity IDs are already concrete when commands are sealed; no allocation semantics need to be merged.

## Three small counterexamples that block unrestricted merge

### A unique key is not union-closed

Start with an empty `User(id, email)` relation whose `id` is a key. Writer A independently admits `(7, "a@example")`; writer B admits `(7, "b@example")`. Each local state is valid. Their union violates the key. Declaring both independently successful and choosing a winner during merge either breaks the law or retracts an acknowledged success.

### Capacity is not union-closed

A class has enrollment capacity one. Two writers independently enroll different students. Each state fits capacity; their union does not. Replacing enrollments with a semilattice of claims can record the conflict, but does not make both admissions valid. Escrow/partitioned rights could coordinate in advance; that is another policy and state machine, not free union. It is not added in 1.0.

The tiny executable fixture should use capacity `Class(c,1)` and distinct facts `Enrollment(c,a)` / `Enrollment(c,b)`, each with unit weight. Their grouped measure is two, so deduplication cannot hide the conflict. A weighted variant replaces unit count with exact nonnegative child weights and has the same issue. Neither example requires a new pointwise temporal-capacity feature.

### Deletion and references need meaning, not just a merge function

Start with parent `P(1)` and no children; `Child.parent ⊆ P.id`. A deletes `P(1)` and is valid. B inserts `Child(c,1)` while retaining the parent and is valid. Plain state union resurrects the parent, erasing A's intended deletion. A merge that instead lets the deletion win leaves the child dangling unless it rejects or changes B's effect.

Containment/foreign-key validity **by itself is closed under union of complete valid states**: a child in the union came from one side, whose witnessing parent remains in that side and hence the union. This third example is therefore not a counterexample to containment's union closure. It exposes the incompatibility between using plain union and preserving an acknowledged delete. Combining the actual two deltas against the common base produces the dangling child; an ordered authority instead rejudges the second command and refuses the incompatible effect.

Observed-remove sets, tombstones and causal dots can define a deterministic result. They do not follow from the current set representation, and a deterministic result does not automatically preserve the relational law or both advertised command outcomes. Choosing those semantics would be a materially different database.

The same distinction applies to blind counter replacement. Two commands deleting `(balance,10)` and adding `(balance,9)` can produce one net decrement. Set idempotence is working; the application failed to encode its read-dependent intent. `ExactState` supplies the missing condition without a general read-set protocol.

## The cheap wins we do take

1. **Normalize effects once.** In one command, additions A and removals D normalize to `(A, D \ A)` and apply as `(S \ D) ∪ A`. Exact duplicate intentions disappear, independent of builder order. This add-wins tie rule is not concurrent-command conflict resolution.
2. **Use set joins inside execution.** Distinctness, finite recursive `seen`/frontier sets, union branches and lawful accumulator merges use their algebra directly. RAM and temporary LMDB are representations of the same set, not separate semantics. For an insert-only fragment containing only union-closed containment laws (including fixed closed vocabulary), combining complete independently valid instances preserves those laws; the engine may exploit that lemma internally without exposing a second publication protocol.
3. **Keep incremental admission where proved.** Mutable-relation support can omit unaffected checks. Closed relation denotations remain fixed. The proof must match the actual support calculation, including shared closed vocabularies; it does not create independent publication authority.
4. **Exploit independent storage access.** LMDB read snapshots, schema templates without tenant data, per-tenant owners and bounded batch kernels allow concurrent work under ordinary ownership.
5. **Recognize commutation without exposing another mode.** Disjoint effects can be reordered internally only when both semantic outcome and applicable read/receipt conditions are proved invariant. In 1.0 the existing authority order remains the externally observable order; no public “probably commutative” flag bypasses admission.

For fixed deltas with normalized `(A1,D1)` and `(A2,D2)`, absence of cross insert/delete conflicts (`A1 ∩ D2 = ∅` and `A2 ∩ D1 = ∅`) is a sufficient condition for their **raw set transformations** to commute. It is not sufficient for their independently observed admission outcomes or exact-state preconditions to commute. For example, disjoint reservations still interact through a shared capacity law. State the theorem at its real strength.

The exact float aggregate is another deliberately useful algebra: bounded finite accumulation plus canonical nonfinite cases can merge **disjoint, already deduplicated binding partitions** independently of grouping/partition order, followed by one rounding. This merge is associative/commutative but **not idempotent**, so it is not a join-semilattice and replaying a partial sum twice doubles its finite contribution/count. Set-level deduplication must precede accumulation. Rounded scalar float arithmetic cannot generally be reassociated. [11](11-floats.md) specifies those distinctions.

## Why not expose a special insert-only multiwriter mode now?

Such a mode would need an accepted-law closure checker/proof roster, distinct receipt/rejection semantics, a read contract, retirement rules and a merge/recovery path. If it writes the same LMDB environment, LMDB still serializes physical writes. If it writes independent replicas without HEAD, the system has acquired another distributed protocol.

That is not free at either layer. The selected small core instead admits every supported final state under one clear transaction rule. Hosted multiwriter already exists through HEAD. A later measured use case could justify a narrowly specified monotone import/merge operation; it must prove its closure and integration, not borrow the database's name as a proof.

Do not use the old braid theorem as an argument for tenant sharding. It concerns a particular decomposition of constraint support, not user rows, all application read dependencies, recovery cuts, or throughput isolation. Retaining it as an admission optimization is useful; promoting it into an unproved distributed guarantee is not.

## Required concurrency obligations

These refine G03/G04/G06/G07/G09, and are **not yet passed**.

| Gate | Required evidence |
| --- | --- |
| `CONC-01` | Independent finite-state checks and Lean lemmas for set normalization/idempotence and the stated raw-delta commutation condition; distinguish one-command tie rules from sequential commands |
| `CONC-02` | Executable key, grouped count/weighted-capacity and parent-delete/child-insert counterexamples; no optimization advertises both conflicting commands as admitted in one invalid state |
| `CONC-03` | Real LMDB parallel readers/writers, held snapshots and resize barriers; one owner and correct visibility with no new local merge layer |
| `CONC-04` | Multiple hosted contenders, lost responses and repeated same IDs; one ordered terminal decision per named command and no candidate visibility; maps to PROTO-01–10 |
| `CONC-05` | Mutable-support theorem premises tested against runtime law analysis, shared closed vocabularies and incremental versus full judgment |
| `CONC-06` | Measure multiwriter contention, failed candidate work and other-tenant progress; no universal low-latency or lock-free claim inferred from algebra |

The judgment is therefore: **keep multiwriter callers, keep LMDB's serialization, keep one hosted publication order, and exploit semilattice laws inside mechanisms that already exist.** This uses the philosophy aggressively without charging the project for another concurrency product.
