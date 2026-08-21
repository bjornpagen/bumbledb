# one-representation — the collection transport, the exact count, and the admission parity

The PRD set that closes both Primer upstream reports —
`bumbledb-bulk-write-and-count-performance` and
`bumbledb-containment-target-key-admission-parity` — as **one** program of
representation changes. Not a performance sprint and not a bug sweep: every
defect in both reports is the same defect, a value that exists in more than
one physical or semantic form, and every fix in this set is the same fix,
collapsing it to one.

These documents are **normative** in the same sense as
`docs/architecture/`: the contract the implementation is verified against,
written before the code, updated in lockstep with it. Read
[00-doctrine.md](00-doctrine.md) first — it is the ruling the rest of the
set applies, and it is the seed of the `audit/REQUIRED-READING.md` the
top-level proposals README already cites. [80-acceptance.md](80-acceptance.md)
is the gate: both upstream acceptance lists, mapped condition by condition.
[90-rollout.md](90-rollout.md) is the self-contained dispatch plan for an
agent fleet building all of it as release **0.16.0** — representation
first, three waves, deletion-forward.

| Doc | Contract |
| --- | --- |
| [00-doctrine.md](00-doctrine.md) | The governing principle: representation over control flow; one way to do each thing; the violation register |
| [10-measurement.md](10-measurement.md) | Attribution first: native phase spans, the allocation census, the Primer-shaped lane, the decision gates |
| [20-accepted-collection.md](20-accepted-collection.md) | THE write representation: one parse, one crossing, one apply — and the transports it deletes |
| [30-string-ownership.md](30-string-ownership.md) | One copy per string, one probe per distinct string; dictionary semantics untouched |
| [40-exact-count.md](40-exact-count.md) | The exact-cardinality read: `count` at engine, bridge, and SDK; `bigint` by law |
| [50-generic-binding.md](50-generic-binding.md) | The `v(relation)` ⇄ `match` law: verified root cause, the one signature that states it |
| [60-containment-parity.md](60-containment-parity.md) | One containment contract at every boundary; names in every diagnostic |
| [70-deletions.md](70-deletions.md) | The deletion ledger: every second spelling and every hot-path waste, each with its one replacement |
| [80-acceptance.md](80-acceptance.md) | Both upstream acceptance lists, the ten preserved laws, digest invariance, the three waves, the 0.16.0 release definition |
| [90-rollout.md](90-rollout.md) | The fleet build: ground rules, lanes, file ownership, gates as merge blockers, receipts |

House laws apply throughout: representation over control flow, **one way to
do each thing** (one meaning, one spelling — the canonical-utterance law the
SDK already enforces for statements and selections now applies to its own
API), zero `dyn` in our own Rust, no allocation on steady-state paths,
attribution-first for any performance claim, and every requirement names its
consumer.

## Drivers (the two upstream reports, summarized)

1. **Collection transport and exact count.** Primer persists 3,993,828
   facts across 39 ordinary relations in 27.61 s at a 7.22 GiB peak RSS;
   the full Learning Commons verifier is 58.02 s. One `insert`/`load` call
   currently materializes the same collection in **six or more physical
   forms** between the caller's fact objects and the delta. Exact relation
   cardinality — maintained transactionally in storage since format 8 — is
   unreachable from every public surface, so Primer counted 4 M facts by
   decoding them, then by full-relation aggregate queries, and carries a
   type suppression for a generic-composition defect in the query SDK.
2. **Containment target-key admission parity.** The TypeScript `schema()`
   boundary accepts (and `lower()` emits) a `contained` statement whose
   target projection resolves no declared key; the engine rejects the same
   schema at `Db.create()` with an id-speak diagnostic. Two admission
   boundaries, one law, two answers.

## Non-goals

- No second public write algebra, no `ArrayBuffer` API, no caller-visible
  packing. The public algebra stays *typed facts in, one mutation report
  out* — this set makes that one spelling also the fastest one.
- No approximate statistics. The count is an exact structural read.
- No general inclusion dependencies into non-key target projections. The
  target-key rule is architected and Lean-priced
  (`docs/architecture/30-dependencies.md` § IND;
  `lean/Bumbledb/Oracle.lean: accepted_target_key_prices_the_probe`);
  this set enforces it everywhere rather than weakening it anywhere.
- No change to fact semantics, interning equality, fresh order, admission,
  or violations. Primer's three canonical digests are a stop-ship
  invariant ([80-acceptance.md](80-acceptance.md)).
