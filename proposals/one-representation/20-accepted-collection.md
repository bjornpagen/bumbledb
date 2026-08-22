# 20 — The accepted collection: one parse, one crossing, one apply

One `transaction.insert(Relation, facts)` — or one `builder.load(...)` —
currently constructs the same collection in six or more physical forms
(V1, V2 in [00-doctrine.md](00-doctrine.md)). This document replaces them
with **one internal representation**, built once at the bridge, carrying
its own proof, consumed by borrow until the bytes enter the delta. The
public algebra does not move: typed facts in, one mutation report out.

## The chain being deleted (baseline, per collection of N facts, arity A)

| Layer | Form | Cost |
| --- | --- | --- |
| Primer | 16,384-row column transpose (`columnBatch`) | N/16384 × A arrays — host-side packing the acceptance forbids |
| `ts/src/db.ts` | `rowsOf`: one JS array per fact; or `columnsOf`: a second full copy of every column | N arrays + GC pressure (3,107 GC samples in the accepted profile) |
| `ts/crate/src/marshal.rs` | `fact_rows`/`fact_columns`: one `Vec<Value>` per row inside `Vec<Vec<Value>>` — `fact_columns` rebuilds **row-major** anyway | N vecs + N×A `Value` moves |
| same | `schema_value`: one `format!` context string per **cell**, success path included; siblings in `fact_columns` (per cell) and `one_fact_row` (per row) | ~25–30 M alloc/free pairs per Primer run, never read |
| same | `sealed_fields`: fresh `Vec<(Box<str>, ValueType)>` per call | per-call re-derivation of an immutable roster |
| `crates/bumbledb/.../mutation_core.rs` | `parse_dyn_collection`: one `ParsedRow` (`Box<[ParsedCell]>`) per row — a full re-parse of what the bridge already proved | N boxes; validation-not-parsing: the bridge's proof was discarded |
| `encode_dyn` → delta | intern + `encode_fact` into reused `refs`/`scratch`/`parse_bytes` | the one layer that is already right — everything above funnels into it |

## The representation

`AcceptedCollection` (crate `bumbledb`, `pub` but `#[doc(hidden)]` — a
transport type, not embedding API; precedent: the doc-hidden
`introspect`/`profile` harness surfaces):

```
AcceptedCollection {
    relation: RelationId,        // resolved once; closed refusal already judged
    arity:    u16,               // proved equal to the sealed roster, once
    rows:     u64,               // exact row count — MutationReport::submitted's input
    cells:    Vec<Cell>,         // R2, pinned: flat, arity-strided, row-major —
                                 // one fixed-width tagged cell per position; no
                                 // per-row container exists anywhere
    strings:  StringArena,       // 30-string-ownership.md; string cells carry
                                 // (offset, len) spans into it
    bytes:    Vec<u8>,           // fixed-width bytes<N> payloads, same span law
}
```

The physical form is **pinned to R2 now** — the fleet builds it while the
measurement rig comes up in parallel; G1 ([10-measurement.md](10-measurement.md))
is the refutation gate at merge, not a selection ceremony before starting.

Semantic invariants (fixed regardless of G1's verdict on the physical
form):

1. **Construction is the parse.** The ONE constructor performs the whole
   shape judgment — arity per row, type-kind per cell against the sealed
   roster, `bytes<N>` width, interval nonemptiness, UTF-8/well-formedness
   at the marshal seam — and a constructed value *is* the proof. There is
   no other constructor, no unchecked variant, no builder that admits a
   partial collection. Illegal collections are unrepresentable, so every
   downstream arity/type re-check is deleted, not skipped.
2. **Empty is lawful** and constructs without touching the roster
   (`rows: 0`), exactly as `fact_rows` short-circuits today.
3. **Consumption is by borrow.** The engine walks cell views
   (`CellView<'coll>` — the `ParsedCell` vocabulary re-homed) directly
   into the existing `intern → encode_fact → apply_collection` machinery.
   No per-row container is ever allocated between the constructor and the
   encoded fact bytes. `ParsedRow`, `parse_dyn_row`'s per-row boxing, and
   `parse_dyn_collection` are deleted; `encode_dyn.rs` keeps only the
   cell-level type-match rule (`value_matches`) and the intern loop.
4. **One collection, both dispositions.** Insert and delete consume the
   same representation; the delete arm interns nothing (resolve-only),
   exactly as `encode_parsed_resolve` does today.
5. **Ownership is transferable.** The collection is `Send`, built on the
   JS thread, consumable on the transaction's thread — the "move to the
   transaction worker once" requirement. Nothing borrows from V8 after
   the constructor returns.

## The surfaces (one spelling each)

- **TypeScript (the public algebra — unchanged in meaning, narrowed in
  spelling):** `tx.insert(relation, facts)`, `tx.delete(relation, facts)`,
  `builder.load(relation, facts)`, `builder.delete(relation, facts)` with
  `facts: Iterable<Fact<R>>`. **`ColumnBatch` is deleted** — the type, the
  `CollectionWrite` union, `isColumnBatch`, `columnsOf`, and
  `mutateCollection`'s dual arms ([70-deletions.md](70-deletions.md) D1).
  The column spelling existed only because the row spelling was slow; a
  second transport kept "for flexibility" would be a mode, and modes are
  how one meaning grows two behaviors. `rowOf` remains THE semantic
  marshal (closed handle→id through the roster, well-formedness, interval
  shape) — its *output* form is G1's business, its judgment is not.
- **Bridge (one crossing per verb):** `txInsert`, `txDelete`,
  `instanceBuilderLoad`, `instanceBuilderDelete` — each builds ONE
  `AcceptedCollection` in one pass over the incoming JS value and hands it
  to the engine. `txInsertColumns`, `instanceBuilderLoadColumns`,
  `tx_insert_columns`, `instance_builder_load_columns`, and
  `fact_columns` are deleted (D2). The single-fact crossings
  (`tx_contains`, `tx_get`, instance point reads — `fact_row`/
  `one_fact_row`) are not collections and keep their one-row form.
- **Rust embedding API (unchanged):** `WriteTx::insert_dyn` /
  `delete_dyn` / `InstanceBuilder::load_dyn` keep their signatures —
  named consumers: `bumbledb-c`, the entire bench crate, external ETL.
  Their bodies build the same `AcceptedCollection` through the same one
  constructor (from `&[Value]` rows), so there is exactly **one parse
  implementation in the codebase**. The typed lane (`insert<F: Fact>`)
  is a different algebra (compile-time shape) and is out of scope.
- **Engine transport lane:** `#[doc(hidden)] insert_accepted` /
  `delete_accepted` / `load_accepted` taking `&AcceptedCollection`,
  consumed by the bridge (and, when it adopts it, `bumbledb-c` — the C
  marshal has the same six-forms disease in miniature). Doc-hidden
  because the physical form must never become semantic API — the
  upstream report's own words.

## The transport bound (the 4 GiB refusal)

The u32 arena spans that make every cell fixed-width also bound one
collection's variable-width payload: **each arena — strings, and
separately bytes — addresses at most u32::MAX ≈ 4 GiB per call.** A push
that would move an arena past the bound (or a single value longer than
it) refuses with the typed `FactShapeError::PayloadBound` naming the
relation — never a panic (ETL input is data; the no-panics-on-the-import
ruling), enforced at the ONE seam every
variable-width byte lands through (`CollectionBuilder::arena_span`,
`crates/bumbledb/src/api/db/collection.rs`).

The bound is a **transport envelope, not a semantic limit**, and the
refusal is fully recoverable: split the facts across two or more
`insert`/`load` calls **in the same transaction or builder**. The
collection is a transport unit; atomicity lives one level up — a throw
aborts the whole `db.write`, and the builder is all-or-nothing at
`admit` — so chunking costs the caller nothing semantically, and every
per-call law (parse-all-first, exact reports, poison) holds per chunk
exactly as it held per call. Scale reality: the bound is per relation
per call; Primer's entire 3,993,828-fact corpus is 1.68 GB across 39
relations, an order of magnitude under one relation's envelope.

Witnessing the refusal takes a real >4 GiB arena, which no CI harness
can afford — the refusal stands on the typed arm, its variant pin in
`error.rs`, and the comment at `arena_span`; an `#[ignore]`d manual
instrument (the `alloc_census` precedent) is the sanctioned shape if an
executing witness is ever wanted.

## Hot-path hygiene that does not wait for G1

Ships first, on the baseline representation, so the G1 matrix measures
representations and not noise (V3, V4):

- Every `format!`-built error context in the fact lanes (`schema_value`
  line 199, `fact_columns`, `one_fact_row`, fact-lane `req_at` contexts)
  moves inside the error arm. The error text is unchanged byte-for-byte —
  the errors still name relation and field; they are simply not
  constructed for cells that succeed.
- `sealed_fields` becomes a per-handle resident on `Sealed` (computed at
  open/create, borrowed per call).

## The ten laws, write side

| Upstream law | How this representation preserves it |
| --- | --- |
| 1. complete collection passes shape parsing before the first row enters the delta | the constructor IS the shape parse; `insert_accepted` is unreachable without a constructed value — the law goes from *enforced* to *unrepresentable to break* |
| 2. empty collections remain lawful | `rows: 0` constructs and applies as `MutationReport::EMPTY` exactly as today |
| 3. exact `submitted`/`changed` | `submitted` = `rows` (proved at parse); `changed` unchanged (delta effects) |
| 4. failure after applied prefix poisons | `apply_collection`'s phase machine is untouched — this doc changes what feeds it, not what it does |
| 5. closed-relation refusal typed | judged in the constructor path exactly where `parse_dyn_collection` judges it today (`refuse_poisoned`/`refuse_closed` order preserved) |
| 6. interning preserves exact string equality | [30-string-ownership.md](30-string-ownership.md); the dictionary is untouched |
| 7. fresh marks advance under existing rules | `advance_fresh_marks` reads encoded fact bytes — unchanged |
| 8. commit admission and violations unchanged | nothing at or after `apply` moves; the commit pipeline is out of scope here |

## Acceptance (this doc's share; the full gate is 80)

- One parse implementation; `ParsedRow` and `fact_columns` no longer
  exist in the tree.
- Zero per-row heap allocation between the constructor and `encode_fact`,
  proved by an `alloc-counter` window in the primerlane (10).
- The public TS write surface admits exactly one collection spelling, and
  `ts/test/builder-verbs.test.ts`'s column-transport pins are replaced by
  pins that the spelling is gone (a `@ts-expect-error` wall).
- Primer's persist path is `builder.load(Relation, instance.Relation)`
  with no intermediate structure (D10 in 70).
