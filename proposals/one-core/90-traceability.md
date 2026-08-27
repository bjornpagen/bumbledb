# 90 — Traceability: every audit finding → the decision that dissolves it

One row per finding across `audit/10`–`80`. **Doc** names the owning
decision; a finding is closed when its representation lands, never by a
site patch. LIVE marks defects shipping today.

## audit/20 — the mirrored-pair inventory

| Finding | Doc | Dissolving move |
| --- | --- | --- |
| LIVE: fs lease bodies mutually unreadable (format, placement, TTL) | [30](30-pin-the-dark.md) | one lock spelling (`LEASE/1`, `~lease/{key}/`, 5 s) + body/placement goldens |
| LIVE: `WAIT_FOR_POLL_MS` 10 vs 20 | [30](30-pin-the-dark.md) | shared machine constants become one inventory table |
| codec stack mirrored (~975 TS lines, best pinned) | [20](20-one-reader.md) | the `LogCodec` handle; TS reader deleted |
| documents/sidecar/scratch grammar mirrored | [20](20-one-reader.md) | grammar crosses the bridge; fs IO stays TS |
| braid union-finds structurally different, reconciled only by goldens | [20](20-one-reader.md) | one derivation riding `DescriptorWire` |
| Vector wire encoder: no golden, no TS caller (grade D) | [20](20-one-reader.md) | TS encoder deleted; Rust form pinned ([30](30-pin-the-dark.md)) |
| unpinned: key grammar, counter body, scratch body, machines | [30](30-pin-the-dark.md) | goldens + the pin-completeness gate |
| `writeCanonicalLiteral` mirrors engine `encode_literal` | [30](30-pin-the-dark.md) | deleted (dead since the seal crossed) |
| keep-mirrored list (stores, fences, loops, tenants, publish choreography) | [50](50-deferred-with-triggers.md) | D3/D4: the essential boundary, stated once |

## audit/30 — type reuse

| Finding | Doc | Dissolving move |
| --- | --- | --- |
| `Value`/`Interval` ≡ `FactValue`/`IntervalValue`, re-declared | [10](10-one-vocabulary.md) | imported; twin unions die |
| four sealed descriptor types restated + identity converters | [10](10-one-vocabulary.md) | imported; converters die |
| `Batch.reserve` returns `bigint[]` vs `FreshRange` — the one subtype blocker | [10](10-one-vocabulary.md) | `FreshRange` everywhere; `Batch` ⊂ `WriteTx` |
| marshal twins (`factOf`/`lowerFact`) + the `as unknown as` cast | [10](10-one-vocabulary.md) | engine exports its marshal helpers |
| napi `ManifestField` drops `fresh`/`newtype` → ~50-line spec re-join | [10](10-one-vocabulary.md) | field widened; the join dies |
| `assembleFromSpec` (~315 lines) ships in src, serves tests | [10](10-one-vocabulary.md) | moved to test support |
| `catalogDigest` duck-probe of an undeclared engine API | [10](10-one-vocabulary.md) | declared on the handle |
| parity corpus loader duplicated intra-package (~225 test lines) | [40](40-the-oracle.md) | the walker dies with the lane restructure |

## audit/40 — the algebra

| Finding | Doc | Dissolving move |
| --- | --- | --- |
| two log drivers picked opposite engine lanes for recorded rows | [10](10-one-vocabulary.md) + [20](20-one-reader.md) | one vocabulary; one decode marshal in engine types |
| `reserve` returns four types across four surfaces | [10](10-one-vocabulary.md) | one `FreshRange` |
| `Commit` restates `Admission`, both languages | [10](10-one-vocabulary.md) | composition |
| "generation" names three coordinates | [10](10-one-vocabulary.md) | sum/`slot`/count — one name each |
| LIVE: `waitFor` on a wedged braid polls forever in TS | [10](10-one-vocabulary.md)/[30](30-pin-the-dark.md) | `Waited` surfaced as the full sum |
| LIVE: `ErrExhausted` spells a cache miss | [10](10-one-vocabulary.md) | refill arm; exhaustion means exhaustion |
| identity strings drift (TS added tail kinds unilaterally) | [40](40-the-oracle.md) | the generated identity table; drift = build error |
| `reserve_capacity` log-Rust-only; empty-commit divergence; judgment timing | [10](10-one-vocabulary.md) | the verb-parity sweep, one ruling each |
| value/interval invariant doubly owned in TS | [10](10-one-vocabulary.md) | one union, one owner |

## audit/10 + audit/50 + audit/70 — the bridge

| Finding | Doc | Dissolving move |
| --- | --- | --- |
| the 6-step bridge recipe | [20](20-one-reader.md) | followed |
| wart: dead head data on the wire | [20](20-one-reader.md) | payloads carry exactly what the core reads |
| wart: payload keys unlocked by goldens | [40](40-the-oracle.md) | keys join the three-way lock |
| wart: comment-enforced ABI generation | [20](20-one-reader.md) | census-enforced |
| B1: tokio/object_store would bloat the cdylib (~146 crates) | [20](20-one-reader.md) | the `store` feature split |
| B2: napi `{kind,message}` loses identity fidelity | [20](20-one-reader.md)+[40](40-the-oracle.md) | the mint-table + FFI identity lane |
| native cost is zero-marginal (codec already unimportable without `.node`) | [20](20-one-reader.md) | the premise of the move |
| LIVE: tilde-lookalike sets differ (15+NFKC vs 10) | [30](30-pin-the-dark.md) | one generated table, TS emitted from Rust |
| DX: codec loses JS stacks, joins the cargo loop | [20](20-one-reader.md) | accepted, priced in the ledger |
| the one surviving mirrored-pair rationale: the two-readers witness | [40](40-the-oracle.md) | replaced, priced, receipted |
| `logApplySlot` one-crossing apply | [50](50-deferred-with-triggers.md) | D1, trigger written |

## audit/60 + audit/80 — the boundary and the adversary

| Finding | Doc | Dissolving move |
| --- | --- | --- |
| log C ABI: defer with trigger | [50](50-deferred-with-triggers.md) | D2, verbatim |
| A: the boundary was deliberate, thrice-renegotiated | [00](00-thesis.md)+[40](40-the-oracle.md) | renegotiated a fourth time, in writing, with the surviving reason priced |
| B: both drivers in production on the same bytes; the oracle is load-bearing | [40](40-the-oracle.md) | the replacement oracle (spec generator + storm + locks + identity lane) |
| C: drift risk already ~zero via the corpus | [30](30-pin-the-dark.md) | true only after the dark surfaces are pinned — now they are |
| D: async poisoning; double-marshal at apply | [20 §3](20-one-reader.md)+[50](50-deferred-with-triggers.md) | nothing async crosses; apply deferred to D1's one-crossing form |
| E: type unification ~90% done, residue is wire coercion | [10](10-one-vocabulary.md)+[20](20-one-reader.md) | the 90% imported; the coercion becomes the core's decode marshal |
| F: protocol law in a workspace-excluded bridge | [20 §5](20-one-reader.md) | the battery gains the bridge lane |
| G: timing against the fresh receipt | [DISPATCH](DISPATCH.md) | this campaign mints its own proof; the 141-row receipt stays historical |
| H: the query pattern is control-plane, the codec is not | [20](20-one-reader.md) | audit/50/70 showed every crossing is cold or fsync-adjacent; the hot pure paths stay TS by D5 |

## Roll-up

| Doc | Findings closed |
| --- | --- |
| [10](10-one-vocabulary.md) | 15 |
| [20](20-one-reader.md) | 14 |
| [30](30-pin-the-dark.md) | 8 |
| [40](40-the-oracle.md) | 6 |
| [50](50-deferred-with-triggers.md) | 4 |

Five LIVE defects, all closed by representation; zero findings closed by
a site patch. That distribution is, as always, the argument.
