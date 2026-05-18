# 05: Direct Selective Query Kernels

**Goal**
- Add direct point/range kernels for tiny selective queries so they do not pay generic Free Join/LFTJ setup costs.

**Trace Evidence**
| Query | Rows | Total | Plan→Exec | Useful Work |
|---|---:|---:|---:|---|
| `sailors/sailor_range_reserves` | 5 | `20.6ms` | `0.3ms` | `10` candidates |
| `joinstress/chain4_from_a` | 1 | `17.9ms` | `5.4ms` | `3` candidates |

The direct kernel target is microsecond-class prepared/cached execution for simple indexed paths.

**Target Shapes**
- Single-relation exact prefix.
- Single-relation equality prefix plus one range predicate.
- Acyclic chains where each later atom is a low-fanout point/prefix lookup from already-bound values.
- Optional existence-only checks.
- Projection and count-only aggregate through existing sinks.

**Design**
- Add an optional `DirectKernelPlan` selected after normal Free Join planning.
- Keep it under the normalized query/Free Join runtime story, not a public second executor.
- Direct kernels consume `QueryImage`, `NormalizedQuery`, `EncodedInputs`, and emit through `OutputSink`.

Kernel kinds:
- `PointLookup`
- `PrefixRange`
- `ChainProbe`
- `CountOnly`

**Examples**
`sailor_range_reserves`:
- Direct prefix/range over `Reserve`.
- Equality prefix: `sailor=$sailor`.
- Range: `day in [$start, $end)`.
- Emit `boat`, `day`.

`chain4_from_a`:
- Check `A.primary($a)`.
- Probe `B.by_a($a)` -> `?b`.
- Probe `C.by_b(?b)` -> `?c`.
- Probe `D.by_c(?c)` -> `?d`.
- Emit `?d`.

**Implementation Steps**
1. Add direct kernel plan structs and explain line.
2. Add planner matcher `try_direct_kernel`.
3. Add rowset iteration helpers for `RowSetRef`.
4. Add direct prefix and range lookup APIs on cached indexes.
5. Implement prefix/range kernel.
6. Implement acyclic chain kernel.
7. Reuse existing output and aggregate sinks.
8. Add counters: `direct_kernel_probes`, `direct_kernel_rows`, `direct_kernel_predicates`.
9. Add benchmark gates for the two target queries.

**Tests**
- Direct planner selects prefix/range for `sailor_range_reserves`.
- Direct planner selects chain probe for `chain4_from_a`.
- Direct planner rejects cyclic joins.
- Direct results match LFTJ fallback and SQLite.
- Direct plans have zero LFTJ trie counters.
- Empty prefix/range and broken chain return zero rows.

**Acceptance Criteria**
- Target queries select direct kernels.
- Target queries return identical rows to existing execution.
- Direct prepared/cached execution is under `50us` initially, stretch `10us`.
- Direct plans show `direct_kernel_rows` near output rows and LFTJ counters zero.
- Unsupported shapes fall back to Free Join/LFTJ.

**Risks**
- Current benchmark includes image/planning time; add prepared/cached timing to enforce microsecond gates fairly.
- Range performance may require a `(sailor, day, boat)` access path rather than primary `(sailor, boat, day)`.
- Direct kernels must not bypass set semantics or aggregate semantics.
