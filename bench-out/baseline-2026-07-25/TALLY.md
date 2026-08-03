# TALLY — baseline 2026-07-25 attribution (the document the perf fanout reads)

date: 2026-08-03 · code rev of every traced/alloc/sweep run: `484c3871` (the last code
commit; the artifact commits since are bench-out-only) · obs binary: `target/bench-obs`
built at HEAD, oracle re-earned on that binary (2889 cases, stamp `7a1a4951…`, zero
mismatches) · wall power verified before/after every lane (pmset), measurement mutex
held, `BUMBLEDB_BENCH_BOOST=1` shared-machine boost throughout.

How to read a row: **number** = the untraced full-protocol baseline (the committed lane
JSONs under `bench-out/baseline-2026-07-25/…` — the traced/alloc runs are separate solo passes and never source
a timing); **delta** = baseline vs campaign-2026-07-23 (ratio-of-ratios where both runs
are twinned; `<1` = our standing improved); **top-5** = self-time attribution of the
traced sample's engine span tree (absolute µs of the solo traced execute + share of its
traced wall — NOT of the lane p50; traced executes run instrumented and un-batched, so
compare shares, trust the lane JSON for time); **alloc** = the alloc pass's counting
window (`allocs/alloc_bytes`; scenarios: exactly one execute per window); **flame** =
`bench-out/baseline-2026-07-25/flame/<name>.svg` (folded twin beside it; Chrome JSONs under `bench-out/baseline-2026-07-25/traced/…`,
on disk, not in git).

## Honest gaps (marked once, so no row lies by omission)

- **p5_keyed_get** traces empty: the KeyedGet surface is the determinant probe with no
  query machinery — zero engine spans by design. Its number stands; attribution is
  below span coverage (the whole op is sub-µs).
- **closure/displaced read families** (closure_depth, closure_fanout, disp_*) have no
  traced path — `bench --trace` covers the ledger+calendar prepared-query families
  only. Alloc footprints exist for all 33; attribution for these 8 is a machinery gap
  (observability finding material, not a measurement).
- **Sub-µs lanes** (ours p50 < ~500ns, flagged `⚠sub-µs` below): the traced sample's
  span stamps sit at raw-tick resolution and the timed protocol re-batches above the
  quantum floor — per-span µs on these rows carry clock-resolution noise; shares are
  still directionally sound.
- **bench --alloc windows span the whole measured batch** (8 samples × quantum batch),
  so read-family alloc is reported as window totals; quantum-floor families divide
  inexactly. Scenario alloc windows are exactly one execute (samples 1).
- **Write worlds refuse `--alloc` by design** (no counter near a commit loop): crud/
  lawful/writes/judgment alloc = n/a, doctrine, not gap.
- **churn / curves / storage** have no traced path (series/metric lanes); their numbers
  and deltas live in `bench-out/baseline-2026-07-25/SUMMARY.md` and are not re-tallied here.
- **Read-family traces are warm-only** under `bench --trace`; scenario queries carry
  true warm+cold pairs. Cold read-family attribution: use `bumbledb-bench trace
  --family <f>` ad hoc (the quick-look tool) — not part of this estate.
- **Ephemeral reps are untraced** — the traced estate rides the durable store; the
  ephemeral deltas in SUMMARY.md are flat-to-noise vs durable so the attribution
  transfers.

## Scenario queries (34; per-query warm+cold traced, per-query alloc)

| scenario/query | ours p50 | vs sqlite | Δ vs campaign | top-5 warm self-time (traced solo) | cold total | alloc (1 exec) | flame |
|---|---:|---:|---:|---|---:|---|---|
| joins/j1_filmography ⚠sub-µs | 250ns | 0.0423 | 1.15 | finalize 36.7µs 82%; join 7.5µs 17%; execute 0.3µs 1%; rule_0 0.2µs 0%; views 0.1µs 0% | 7612µs | 2a/32B | flame/scenarios.joins.j1_filmography.warm.svg |
| joins/j2_costars | 875ns | 0.0724 | 0.71 | join 22.8µs 91%; finalize 1.4µs 6%; execute 0.3µs 1%; rule_0 0.2µs 1%; views 0.2µs 1% | 10340µs | 2a/32B | flame/scenarios.joins.j2_costars.warm.svg |
| joins/j3_keyword_kind | 3.6µs | 0.2544 | 1.25 | finalize 56.4µs 77%; join 15.9µs 22%; bind_params 0.5µs 1%; execute 0.3µs 0%; rule_0 0.2µs 0% | 4487µs | 2a/56B | flame/scenarios.joins.j3_keyword_kind.warm.svg |
| joins/j4_five_way | 1369.1µs | 0.3428 | 1.05 | finalize 626.5µs 53%; join 563.8µs 47%; execute 0.9µs 0%; rule_0 0.5µs 0%; views 0.2µs 0% | 11413µs | 2a/104B | flame/scenarios.joins.j4_five_way.warm.svg |
| joins/j5_country_rollup | 4778.8µs | 0.1670 | 1.00 | join 5150.8µs 100%; execute 3.3µs 0%; rule_0 0.3µs 0%; finalize 0.2µs 0%; views 0.2µs 0% | 7120µs | 2a/16B | flame/scenarios.joins.j5_country_rollup.warm.svg |
| joins/j6_keyword_neighborhood | 26.6µs | 0.0210 | 0.76 | join 689.0µs 96%; finalize 28.8µs 4%; execute 0.3µs 0%; rule_0 0.3µs 0%; views 0.2µs 0% | 11616µs | 2a/32B | flame/scenarios.joins.j6_keyword_neighborhood.warm.svg |
| graph/g1_neighbors ⚠sub-µs | 250ns | 0.0822 | 0.82 | join 4.8µs 75%; finalize 1.0µs 16%; execute 0.3µs 4%; rule_0 0.2µs 3%; views 0.1µs 1% | 13945µs | 2a/32B | flame/scenarios.graph.g1_neighbors.warm.svg |
| graph/g2_two_hop ⚠sub-µs | 250ns | 0.0245 | 1.12 | join 347.5µs 91%; finalize 33.9µs 9%; execute 0.5µs 0%; rule_0 0.3µs 0%; views 0.2µs 0% | 20324µs | 2a/32B | flame/scenarios.graph.g2_two_hop.warm.svg |
| graph/g3_three_hop_count | 1.3µs | 0.0682 | 1.88 | join 3003.4µs 100%; execute 4.7µs 0%; views 0.3µs 0%; rule_0 0.3µs 0%; finalize 0.1µs 0% | 34856µs | 2a/32B | flame/scenarios.graph.g3_three_hop_count.warm.svg |
| graph/g4_mutual | 2938.8µs | 0.1126 | 0.79 | join 3414.2µs 100%; execute 1.2µs 0%; rule_0 0.5µs 0%; views 0.5µs 0%; finalize 0.2µs 0% | 36990µs | 2a/32B | flame/scenarios.graph.g4_mutual.warm.svg |
| graph/g5_triangles_from | 625ns | 0.0459 | 1.32 | join 44.8µs 98%; execute 0.5µs 1%; views 0.3µs 1%; rule_0 0.2µs 0%; finalize 0.0µs 0% | 29848µs | 2a/32B | flame/scenarios.graph.g5_triangles_from.warm.svg |
| graph/g6_weighted_hop | 750ns | 0.1071 | 2.20 | join 14.7µs 64%; execute 7.1µs 31%; rule_0 0.5µs 2%; finalize 0.3µs 1%; views 0.3µs 1% | 7896µs | 2a/80B | flame/scenarios.graph.g6_weighted_hop.warm.svg |
| olap/o1_revenue_by_region | 458.3µs | 0.0022 | 1.10 | join 475.9µs 100%; execute 0.4µs 0%; rule_0 0.2µs 0%; finalize 0.1µs 0%; views 0.1µs 0% | 8542µs | 2a/16B | flame/scenarios.olap.o1_revenue_by_region.warm.svg |
| olap/o2_category_window | 405.0µs | 0.0226 | 1.18 | join 386.1µs 100%; execute 0.6µs 0%; rule_0 0.4µs 0%; finalize 0.3µs 0%; views 0.2µs 0% | 2055µs | 3a/64B | flame/scenarios.olap.o2_category_window.warm.svg |
| olap/o3_promo_split | 321.1µs | 0.0036 | 1.06 | join 314.3µs 100%; rule_0 0.2µs 0%; execute 0.1µs 0%; finalize 0.1µs 0%; views 0.1µs 0% | 4879µs | 2a/16B | flame/scenarios.olap.o3_promo_split.warm.svg |
| olap/o4_segment_category | 25720.5µs | 0.0750 | 1.01 | join 26574.8µs 100%; finalize 1.1µs 0%; execute 0.7µs 0%; rule_0 0.4µs 0%; views 0.2µs 0% | 35655µs | 2a/24B | flame/scenarios.olap.o4_segment_category.warm.svg |
| olap/o5_store_extremes | 669.2µs | 0.0047 | 1.12 | join 687.7µs 100%; finalize 2.6µs 0%; execute 0.3µs 0%; rule_0 0.3µs 0%; views 0.1µs 0% | 6028µs | 1a/8B | flame/scenarios.olap.o5_store_extremes.warm.svg |
| olap/o6_brand_drill | 1.8µs | 0.0036 | 0.90 | join 3.6µs 81%; execute 0.3µs 7%; views 0.2µs 5%; rule_0 0.2µs 4%; resolve_filters 0.1µs 2% | 1941µs | 2a/56B | flame/scenarios.olap.o6_brand_drill.warm.svg |
| points/p1_by_id ⚠sub-µs | 292ns | 0.2696 | 1.00 | key_probe 0.2µs 75%; execute 0.0µs 13%; bind_params 0.0µs 12% | 15µs | 2a/32B | flame/scenarios.points.p1_by_id.warm.svg |
| points/p2_by_key | 917ns | 0.6879 | 1.00 | key_probe 0.6µs 48%; bind_params 0.6µs 45%; execute 0.1µs 6% | 6µs | 2a/32B | flame/scenarios.points.p2_by_key.warm.svg |
| points/p3_bucket_fetch | 11.0µs | 0.0522 | 1.00 | join 15.0µs 85%; finalize 1.8µs 10%; execute 0.4µs 2%; views 0.2µs 1%; rule_0 0.2µs 1% | 1337µs | 2a/56B | flame/scenarios.points.p3_bucket_fetch.warm.svg |
| points/p4_size_band ⚠sub-µs | 209ns | 0.0019 | 0.86 | execute 0.3µs 26%; rule_0 0.3µs 26%; join 0.2µs 22%; views 0.2µs 17%; bind_params 0.0µs 4% | 2248µs | 2a/56B | flame/scenarios.points.p4_size_band.warm.svg |
| points/p5_keyed_get | 1.0µs | 0.7571 | 0.99 | — | — | 5a/340B | — (empty trace) |
| rings/r1_wash_ring | 9655.2µs | 0.0947 | 0.97 | join 10522.5µs 100%; execute 3.3µs 0%; rule_0 0.3µs 0%; views 0.2µs 0%; finalize 0.1µs 0% | 19784µs | 2a/32B | flame/scenarios.rings.r1_wash_ring.warm.svg |
| rings/r2_temporal_ring | 31499.2µs | 0.2161 | 1.08 | join 34618.9µs 100%; bind_params 3.5µs 0%; execute 0.8µs 0%; rule_0 0.5µs 0%; views 0.2µs 0% | 39832µs | 2a/32B | flame/scenarios.rings.r2_temporal_ring.warm.svg |
| rings/r3_bomb_t1 | 3708.1µs | 0.1176 | 0.99 | join 3771.0µs 100%; execute 0.3µs 0%; rule_0 0.2µs 0%; views 0.2µs 0%; bind_params 0.1µs 0% | 4017µs | 1a/8B | flame/scenarios.rings.r3_bomb_t1.warm.svg |
| rings/r4_bomb_t2 | 1705207.3µs | exceeded_cap | (exceeded_cap both runs) | join 1704551.1µs 100%; views 3.0µs 0%; execute 1.8µs 0%; rule_0 1.1µs 0%; finalize 0.7µs 0% | 1734430µs | 1a/8B | flame/scenarios.rings.r4_bomb_t2.warm.svg |
| rings/r5_reciprocal | 514.8µs | 0.1571 | 1.00 | join 523.3µs 99%; finalize 5.9µs 1%; execute 0.4µs 0%; rule_0 0.3µs 0%; views 0.2µs 0% | 4669µs | 2a/32B | flame/scenarios.rings.r5_reciprocal.warm.svg |
| rings/r6_two_path_count | 131327.4µs | 0.1999 | 0.87 | join 124636.6µs 97%; execute 3400.0µs 3%; views 1.5µs 0%; rule_0 1.0µs 0%; finalize 0.5µs 0% | 389514µs | 1a/8B | flame/scenarios.rings.r6_two_path_count.warm.svg |
| temporal/t1_stab | 1.1µs | 0.2015 | 1.86 | join 23.4µs 78%; finalize 6.0µs 20%; execute 0.2µs 1%; rule_0 0.2µs 1%; views 0.1µs 0% | 2183µs | 2a/32B | flame/scenarios.temporal.t1_stab.warm.svg |
| temporal/t2_overlap_join | 58165.2µs | exceeded_cap | (exceeded_cap both runs) | join 56955.5µs 100%; rule_0 3.6µs 0%; execute 1.1µs 0%; views 0.9µs 0%; finalize 0.3µs 0% | 58564µs | 1a/8B | flame/scenarios.temporal.t2_overlap_join.warm.svg |
| temporal/t3_mixed_mask | 42.7µs | 0.0179 | 0.41 | join 39681.4µs 99%; finalize 382.1µs 1%; execute 3.7µs 0%; rule_0 0.5µs 0%; views 0.2µs 0% | 47561µs | 2a/32B | flame/scenarios.temporal.t3_mixed_mask.warm.svg |
| temporal/t4_ray_stab | 41.0µs | 0.0092 | 0.92 | join 32.1µs 77%; finalize 9.0µs 22%; execute 0.2µs 1%; rule_0 0.2µs 0%; views 0.1µs 0% | 320µs | 2a/32B | flame/scenarios.temporal.t4_ray_stab.warm.svg |
| temporal/t5_pack_key | 2.2µs | — | — | join 44.6µs 55%; finalize 35.2µs 44%; execute 0.2µs 0%; rule_0 0.2µs 0%; views 0.1µs 0% | 2108µs | 2a/32B | flame/scenarios.temporal.t5_pack_key.warm.svg |

## Ledger/calendar/closure/displaced read families (33; min-of-3 durable reps)

| family | ours p50 (min r1-r3) | ratio_p50 | Δ vs campaign | top-5 warm self-time | alloc window (8-sample batch) | flame |
|---|---:|---:|---:|---|---|---|
| point ⚠sub-µs | 255ns | 0.1824 | 1.01 | bind_params 8.1µs 87%; execute 0.8µs 9%; key_probe 0.3µs 4% | 129a/3136B | flame/point.warm.svg |
| containment_walk | 2.2µs | 0.0444 | 0.34 | join 6.9µs 74%; finalize 0.8µs 9%; execute 0.7µs 7%; rule_0 0.6µs 6%; views 0.2µs 3% | 9a/256B | flame/containment_walk.warm.svg |
| chain | 196.5µs | 0.1050 | 0.98 | join 86.4µs 89%; finalize 6.0µs 6%; execute 3.7µs 4%; views 0.6µs 1%; rule_0 0.5µs 1% | 9a/256B | flame/chain.warm.svg |
| range | 19.3µs | 0.1393 | 0.99 | join 12.9µs 60%; finalize 5.7µs 26%; execute 2.2µs 10%; rule_0 0.3µs 1%; views 0.2µs 1% | 9a/448B | flame/range.warm.svg |
| balance | 1.0µs | 0.0040 | 1.00 | join 48.5µs 95%; execute 1.0µs 2%; rule_0 0.7µs 1%; views 0.5µs 1%; finalize 0.2µs 0% | 9a/256B | flame/balance.warm.svg |
| stats | 1334.1µs | 0.0174 | 0.98 | join 1238.6µs 95%; execute 63.8µs 5%; views 3.1µs 0%; rule_0 0.5µs 0%; finalize 0.2µs 0% | 9a/128B | flame/stats.warm.svg |
| string | 2.5µs | 0.0418 | 0.98 | join 5.2µs 66%; execute 1.2µs 15%; rule_0 0.5µs 6%; finalize 0.5µs 6%; bind_params 0.2µs 3% | 9a/256B | flame/string.warm.svg |
| skew | 1537.4µs | 0.2064 | 1.00 | join 1483.9µs 93%; finalize 91.3µs 6%; execute 27.7µs 2%; rule_0 0.5µs 0%; views 0.3µs 0% | 9a/256B | flame/skew.warm.svg |
| spread | 10486.3µs | 0.0821 | 0.97 | join 9878.2µs 95%; finalize 445.1µs 4%; execute 73.2µs 1%; rule_0 0.6µs 0%; views 0.6µs 0% | 1a/64B | flame/spread.warm.svg |
| triangle | 2584.3µs | 0.0697 | 0.98 | join 2672.0µs 100%; execute 9.4µs 0%; rule_0 0.4µs 0%; views 0.3µs 0%; finalize 0.1µs 0% | 9a/448B | flame/triangle.warm.svg |
| entries_for_account_set | 1.3µs | 0.1240 | 0.88 | execute 3.4µs 57%; join 1.5µs 26%; rule_0 0.4µs 7%; views 0.3µs 6%; finalize 0.1µs 2% | 9a/256B | flame/entries_for_account_set.warm.svg |
| postings_without_tag | 6.8µs | 0.1553 | 2.18 | join 4.6µs 80%; execute 0.4µs 6%; rule_0 0.3µs 6%; finalize 0.2µs 3%; views 0.2µs 3% | 9a/256B | flame/postings_without_tag.warm.svg |
| latest_posting_per_account | 2254.9µs | 0.0545 | 1.00 | join 2269.8µs 100%; finalize 6.8µs 0%; execute 1.5µs 0%; rule_0 0.3µs 0%; views 0.2µs 0% | 1a/64B | flame/latest_posting_per_account.warm.svg |
| mandate_at_instant ⚠sub-µs | 281ns | 0.0346 | 0.97 | join 3.2µs 73%; execute 0.5µs 10%; rule_0 0.4µs 10%; views 0.2µs 5%; bind_params 0.0µs 1% | 129a/6208B | flame/mandate_at_instant.warm.svg |
| mandate_overlap | 15.8µs | 0.0381 | 1.03 | join 12.4µs 87%; execute 0.8µs 6%; finalize 0.5µs 4%; views 0.2µs 2%; rule_0 0.1µs 1% | 9a/256B | flame/mandate_overlap.warm.svg |
| deep_chain | 373.8µs | 0.1164 | 1.09 | join 163.2µs 80%; finalize 34.2µs 17%; execute 6.6µs 3%; views 0.3µs 0%; rule_0 0.3µs 0% | 9a/256B | flame/deep_chain.warm.svg |
| busy_scan | 7.7µs | 0.0023 | 1.00 | join 7.8µs 72%; finalize 1.7µs 15%; execute 1.0µs 9%; rule_0 0.2µs 2%; views 0.2µs 2% | 9a/256B | flame/busy_scan.warm.svg |
| meets_chain | 3.1µs | 0.1754 | 1.00 | join 805.5µs 100%; execute 1.6µs 0%; finalize 0.5µs 0%; rule_0 0.1µs 0%; views 0.1µs 0% | 9a/448B | flame/meets_chain.warm.svg |
| rsvp_union | 934.8µs | 0.0514 | 1.00 | join 642.0µs 68%; finalize 254.0µs 27%; execute 48.7µs 5%; views 0.6µs 0%; rule_0 0.3µs 0% | 1a/64B | flame/rsvp_union.warm.svg |
| conflict_pairs | 23.6µs | 0.0083 | 0.84 | join 101.9µs 97%; execute 2.1µs 2%; rule_0 0.7µs 1%; views 0.2µs 0%; finalize 0.2µs 0% | 9a/256B | flame/conflict_pairs.warm.svg |
| conflict_free | 583ns | 0.0245 | 0.99 | join 4.5µs 60%; execute 2.2µs 29%; rule_0 0.4µs 5%; views 0.3µs 4%; bind_params 0.1µs 1% | 9a/448B | flame/conflict_free.warm.svg |
| free_busy | 3.1µs | 0.0110 | 0.97 | join 24.7µs 51%; finalize 21.2µs 44%; execute 1.4µs 3%; rule_0 0.4µs 1%; views 0.2µs 0% | 9a/448B | flame/free_busy.warm.svg |
| claim_hours | 436.0µs | 0.0691 | 1.01 | join 444.2µs 100%; execute 1.0µs 0%; views 0.2µs 0%; rule_0 0.2µs 0%; finalize 0.1µs 0% | 9a/128B | flame/claim_hours.warm.svg |
| slot_scan | 30.3µs | 0.0108 | 0.98 | join 25.0µs 75%; finalize 6.9µs 21%; execute 1.0µs 3%; rule_0 0.3µs 1%; views 0.2µs 1% | 9a/256B | flame/slot_scan.warm.svg |
| slot_booking_overlap | 6.7µs | 0.0101 | 1.09 | join 63.6µs 97%; execute 1.0µs 1%; finalize 0.7µs 1%; rule_0 0.3µs 0%; views 0.2µs 0% | 9a/256B | flame/slot_booking_overlap.warm.svg |
| closure_depth | 2.8µs | 0.2152 | 1.76 | — | 9a/256B | — (no traced path) |
| closure_fanout | 1.0µs | 0.1165 | 1.62 | — | 9a/256B | — (no traced path) |
| disp_probe | 79186.8µs | 0.1245 | 0.87 | — | 1a/64B | — (no traced path) |
| disp_probe_d24 | 83261.4µs | 0.1313 | 0.97 | — | 1a/64B | — (no traced path) |
| disp_probe_d96 | 87127.4µs | 0.1382 | 1.07 | — | 1a/64B | — (no traced path) |
| disp_stream | 131.6µs | 0.0034 | 1.03 | — | 1a/64B | — (no traced path) |
| disp_stream_d24 | 143.7µs | 0.0036 | 1.00 | — | 1a/64B | — (no traced path) |
| disp_stream_d96 | 155.5µs | 0.0039 | 1.00 | — | 1a/64B | — (no traced path) |

## Windowed/capacity judgment lanes (6; 0.8.0 additions — no campaign twin)

| lane | ours p50 (min r1-r3) | note | top-5 self-time (traced solo commit) | flame |
|---|---:|---|---|---|
| commit_window_baseline | 4495.1µs | engine-only (judged admission has no SQL twin) | lmdb_commit 5002.0µs 96%; apply_inserts 46.1µs 1%; commit 41.4µs 1%; apply_deletes 35.8µs 1%; write_txn 31.6µs 1% | flame/commit_window_baseline.svg |
| commit_window_admission | 5064.5µs | engine-only (judged admission has no SQL twin) | lmdb_commit 5239.4µs 97%; commit 56.2µs 1%; apply_inserts 54.0µs 1%; write_txn 36.5µs 1%; judgment_capacities 16.8µs 0% | flame/commit_window_admission.svg |
| commit_window_exclusion | 5111.9µs | engine-only (judged admission has no SQL twin) | lmdb_commit 4790.1µs 97%; apply_inserts 61.4µs 1%; commit 42.9µs 1%; write_txn 36.4µs 1%; counters_flush 7.2µs 0% | flame/commit_window_exclusion.svg |
| commit_capacity_baseline | 4254.5µs | engine-only (judged admission has no SQL twin) | lmdb_commit 4936.5µs 99%; write_txn 30.0µs 1%; apply_inserts 19.4µs 0%; commit 15.1µs 0%; counters_flush 4.3µs 0% | flame/commit_capacity_baseline.svg |
| commit_capacity_sum | 4378.5µs | engine-only (judged admission has no SQL twin) | lmdb_commit 4863.8µs 97%; apply_inserts 56.5µs 1%; write_txn 37.6µs 1%; commit 25.8µs 1%; counters_flush 12.3µs 0% | flame/commit_capacity_sum.svg |
| commit_capacity_duration | 5011.7µs | engine-only (judged admission has no SQL twin) | lmdb_commit 4234.5µs 97%; apply_inserts 40.0µs 1%; write_txn 38.3µs 1%; commit 31.5µs 1%; counters_flush 16.0µs 0% | flame/commit_capacity_duration.svg |

## crud (22 = 2 durability lanes × 11 ops; alloc n/a by doctrine)

| lane/op | ours p50 | ratio_p50 | Δ vs campaign | top-5 self-time (traced twin sample) | flame |
|---|---:|---:|---:|---|---|
| durable/crud_read_point | 791ns | 0.413 | 0.91 | validate_lower 10.2µs 56%; prepare 3.5µs 19%; validate_rules 1.8µs 10%; key_probe 1.0µs 5%; normalize 0.5µs 3% | flame/crud.durable.crud_read_point.svg |
| durable/crud_insert | 5030.9µs | 1.106 | 1.10 | lmdb_commit 4430.0µs 98%; apply_inserts 27.5µs 1%; write_txn 24.5µs 1%; commit 9.6µs 0%; counters_flush 5.8µs 0% | flame/crud.durable.crud_insert.svg |
| durable/crud_insert_10 | 5239.8µs | 1.174 | 1.15 | lmdb_commit 6210.3µs 94%; write_txn 164.7µs 2%; apply_inserts 160.5µs 2%; commit 71.6µs 1%; counters_flush 11.8µs 0% | flame/crud.durable.crud_insert_10.svg |
| durable/crud_insert_100 | 8540.3µs | 1.903 | 1.12 | lmdb_commit 8174.4µs 84%; write_txn 789.5µs 8%; apply_inserts 651.9µs 7%; commit 79.7µs 1%; counters_flush 5.9µs 0% | flame/crud.durable.crud_insert_100.svg |
| durable/crud_insert_1k | 20057.4µs | 3.912 | 0.83 | lmdb_commit 21016.5µs 79%; apply_inserts 2833.2µs 11%; write_txn 2491.9µs 9%; commit 154.1µs 1%; counters_flush 3.5µs 0% | flame/crud.durable.crud_insert_1k.svg |
| durable/crud_update | 5138.8µs | 1.201 | 1.11 | lmdb_commit 4955.7µs 99%; apply_deletes 15.7µs 0%; write_txn 12.1µs 0%; apply_inserts 8.0µs 0%; commit 3.9µs 0% | flame/crud.durable.crud_update.svg |
| durable/crud_update_hot | 5143.2µs | 1.223 | 1.10 | lmdb_commit 4663.4µs 99%; apply_deletes 27.2µs 1%; write_txn 25.9µs 1%; commit 7.3µs 0%; apply_inserts 7.3µs 0% | flame/crud.durable.crud_update_hot.svg |
| durable/crud_upsert | 5124.3µs | 1.205 | 1.24 | lmdb_commit 4584.8µs 97%; write_txn 53.0µs 1%; apply_inserts 40.8µs 1%; commit 27.7µs 1%; counters_flush 7.5µs 0% | flame/crud.durable.crud_upsert.svg |
| durable/crud_rmw | 5140.9µs | 1.214 | 1.21 | lmdb_commit 5448.9µs 97%; write_txn 72.8µs 1%; apply_deletes 42.1µs 1%; commit 29.1µs 1%; apply_inserts 15.5µs 0% | flame/crud.durable.crud_rmw.svg |
| durable/crud_delete | 5125.6µs | 1.195 | 1.01 | lmdb_commit 6134.4µs 98%; apply_deletes 49.6µs 1%; write_txn 45.5µs 1%; commit 28.5µs 0%; counters_flush 17.5µs 0% | flame/crud.durable.crud_delete.svg |
| durable/crud_mixed_90_10 | 5134.3µs | 1.198 | 1.17 | lmdb_commit 5244.2µs 94%; key_probe 77.1µs 1%; prepare 56.3µs 1%; apply_inserts 45.0µs 1%; write_txn 38.2µs 1% | flame/crud.durable.crud_mixed_90_10.svg |
| nosync/crud_read_point | 500ns | 0.414 | 1.00 | prepare 1.9µs 30%; validate 1.8µs 28%; validate_rules 1.0µs 15%; key_probe 0.5µs 9%; validate_lower 0.4µs 6% | flame/crud.nosync.crud_read_point.svg |
| nosync/crud_insert | 28.5µs | 1.716 | 0.94 | lmdb_commit 21.3µs 63%; apply_inserts 4.8µs 14%; write_txn 4.2µs 13%; commit 1.9µs 6%; counters_flush 1.2µs 4% | flame/crud.nosync.crud_insert.svg |
| nosync/crud_insert_10 | 82.2µs | 3.749 | 1.15 | lmdb_commit 35.3µs 43%; apply_inserts 25.1µs 31%; write_txn 14.5µs 18%; commit 6.0µs 7%; counters_flush 1.2µs 2% | flame/crud.nosync.crud_insert_10.svg |
| nosync/crud_insert_100 | 521.2µs | 4.502 | 0.86 | apply_inserts 235.5µs 43%; lmdb_commit 170.8µs 31%; write_txn 131.2µs 24%; commit 12.3µs 2%; counters_flush 1.3µs 0% | flame/crud.nosync.crud_insert_100.svg |
| nosync/crud_insert_1k | 4402.2µs | 6.291 | 0.86 | apply_inserts 2128.2µs 47%; write_txn 1227.9µs 27%; lmdb_commit 1002.5µs 22%; commit 130.1µs 3%; counters_flush 2.0µs 0% | flame/crud.nosync.crud_insert_1k.svg |
| nosync/crud_update | 30.5µs | 2.972 | 0.91 | lmdb_commit 20.5µs 60%; apply_deletes 5.7µs 17%; write_txn 2.8µs 8%; apply_inserts 2.4µs 7%; commit 1.9µs 6% | flame/crud.nosync.crud_update.svg |
| nosync/crud_update_hot | 27.2µs | 2.839 | 0.81 | lmdb_commit 17.0µs 62%; apply_deletes 3.5µs 13%; commit 2.4µs 9%; write_txn 2.3µs 8%; apply_inserts 1.9µs 7% | flame/crud.nosync.crud_update_hot.svg |
| nosync/crud_upsert | 24.7µs | 1.600 | 0.71 | lmdb_commit 19.2µs 68%; apply_inserts 3.7µs 13%; write_txn 2.5µs 9%; commit 1.7µs 6%; counters_flush 1.0µs 4% | flame/crud.nosync.crud_upsert.svg |
| nosync/crud_rmw | 28.0µs | 2.681 | 0.83 | lmdb_commit 21.2µs 62%; apply_deletes 4.3µs 13%; write_txn 3.6µs 11%; commit 2.6µs 8%; apply_inserts 2.2µs 6% | flame/crud.nosync.crud_rmw.svg |
| nosync/crud_delete | 23.9µs | 1.487 | 0.83 | lmdb_commit 20.1µs 70%; apply_deletes 4.0µs 14%; write_txn 2.0µs 7%; commit 1.6µs 5%; counters_flush 1.2µs 4% | flame/crud.nosync.crud_delete.svg |
| nosync/crud_mixed_90_10 | 31.5µs | 1.044 | 0.82 | lmdb_commit 23.8µs 51%; key_probe 5.0µs 11%; apply_inserts 4.3µs 9%; prepare 3.0µs 7%; write_txn 2.5µs 5% | flame/crud.nosync.crud_mixed_90_10.svg |

## lawful (12 = 2 lanes × 6 law ops; alloc n/a by doctrine)

| lane/op | ours p50 | ratio_p50 | Δ vs campaign | top-5 self-time (traced twin sample) | flame |
|---|---:|---:|---:|---|---|
| durable/law_commit_attempt | 5053.6µs | 0.980 | 0.98 | lmdb_commit 4650.2µs 99%; apply_inserts 20.3µs 0%; write_txn 14.9µs 0%; apply_deletes 12.8µs 0%; commit 8.0µs 0% | flame/lawful.durable.law_commit_attempt.svg |
| durable/law_commit_cluster | 5206.6µs | 1.113 | 1.08 | lmdb_commit 4875.6µs 93%; apply_inserts 134.8µs 3%; write_txn 108.2µs 2%; commit 91.6µs 2%; counters_flush 14.1µs 0% | flame/lawful.durable.law_commit_cluster.svg |
| durable/law_reject_key | 4562.5µs | 311.966 | 0.24 | apply_inserts 55.0µs 43%; commit 42.6µs 33%; write_txn 29.5µs 23%; apply_deletes 0.1µs 0% | flame/lawful.durable.law_reject_key.svg |
| durable/law_reject_containment | 41.2µs | 1.544 | 0.87 | apply_inserts 26.5µs 41%; commit 22.7µs 35%; write_txn 12.3µs 19%; judgment_source 2.2µs 3%; judgment_capacities 1.0µs 2% | flame/lawful.durable.law_reject_containment.svg |
| durable/law_reject_window | 45.9µs | 3.277 | 1.03 | commit 27.5µs 48%; apply_inserts 17.8µs 31%; write_txn 7.4µs 13%; judgment_capacities 3.0µs 5%; judgment_source 1.5µs 3% | flame/lawful.durable.law_reject_window.svg |
| durable/law_reject_scope | 20.4µs | 2.899 | 1.09 | apply_inserts 18.5µs 46%; commit 13.4µs 33%; write_txn 6.6µs 16%; judgment_source 1.3µs 3%; judgment_target 0.2µs 1% | flame/lawful.durable.law_reject_scope.svg |
| nosync/law_commit_attempt | 21.8µs | 1.203 | 1.14 | lmdb_commit 11.7µs 52%; apply_inserts 4.3µs 19%; commit 2.5µs 11%; write_txn 1.9µs 8%; counters_flush 0.9µs 4% | flame/lawful.nosync.law_commit_attempt.svg |
| nosync/law_commit_cluster | 41.4µs | 0.789 | 0.88 | lmdb_commit 23.6µs 47%; apply_inserts 13.5µs 27%; write_txn 3.9µs 8%; counters_flush 3.3µs 7%; commit 3.3µs 7% | flame/lawful.nosync.law_commit_cluster.svg |
| nosync/law_reject_key | 14.0µs | 4.434 | 0.98 | apply_inserts 7.4µs 60%; commit 3.3µs 27%; write_txn 1.6µs 13% | flame/lawful.nosync.law_reject_key.svg |
| nosync/law_reject_containment | 8.5µs | 1.586 | 0.98 | apply_inserts 3.8µs 41%; commit 3.3µs 36%; write_txn 1.6µs 18%; judgment_source 0.3µs 4%; judgment_capacities 0.2µs 2% | flame/lawful.nosync.law_reject_containment.svg |
| nosync/law_reject_window | 8.8µs | 2.944 | 0.96 | apply_inserts 3.6µs 40%; commit 3.1µs 34%; write_txn 1.4µs 16%; judgment_capacities 0.7µs 7%; judgment_source 0.2µs 3% | flame/lawful.nosync.law_reject_window.svg |
| nosync/law_reject_scope | 6.8µs | 2.493 | 0.95 | commit 3.4µs 44%; apply_inserts 2.9µs 38%; write_txn 1.0µs 14%; judgment_source 0.3µs 4%; judgment_target 0.0µs 1% | flame/lawful.nosync.law_reject_scope.svg |

## writes ladder (18 cells = 2 lanes × (commit/delete × b1..b1000 + bulk_append); alloc n/a by doctrine)

| lane/cell | ours p50 | ratio_p50 | Δ vs campaign | top-5 self-time (traced twin sample) | flame |
|---|---:|---:|---:|---|---|
| nosync/commit_b1 | 51.2µs | 1.497 | 1.09 | lmdb_commit 34.0µs 49%; apply_inserts 11.5µs 17%; apply_deletes 9.6µs 14%; write_txn 4.8µs 7%; judgment_source 3.5µs 5% | flame/writes.nosync.commit_b1.svg |
| nosync/commit_b10 | 186.0µs | 0.689 | 1.30 | lmdb_commit 98.0µs 47%; apply_inserts 72.0µs 34%; write_txn 16.8µs 8%; judgment_source 15.3µs 7%; commit 5.2µs 2% | flame/writes.nosync.commit_b10.svg |
| nosync/commit_b100 | 1302.0µs | 0.655 | 0.86 | apply_inserts 607.8µs 41%; lmdb_commit 594.8µs 40%; write_txn 134.5µs 9%; judgment_source 119.7µs 8%; commit 27.2µs 2% | flame/writes.nosync.commit_b100.svg |
| nosync/commit_b1000 | 9462.1µs | 0.893 | 1.01 | apply_inserts 4401.7µs 47%; lmdb_commit 2216.9µs 24%; write_txn 1302.3µs 14%; judgment_source 1057.0µs 11%; commit 289.4µs 3% | flame/writes.nosync.commit_b1000.svg |
| nosync/delete_b1 | 43.4µs | 1.070 | 1.01 | lmdb_commit 29.7µs 63%; apply_deletes 8.5µs 18%; commit 4.7µs 10%; write_txn 2.4µs 5%; counters_flush 1.1µs 2% | flame/writes.nosync.delete_b1.svg |
| nosync/delete_b10 | 175.8µs | 0.779 | 0.96 | lmdb_commit 79.1µs 50%; apply_deletes 56.2µs 36%; write_txn 10.5µs 7%; counters_flush 4.5µs 3%; commit 4.4µs 3% | flame/writes.nosync.delete_b10.svg |
| nosync/delete_b100 | 1321.6µs | 0.519 | 0.87 | apply_deletes 612.9µs 46%; lmdb_commit 539.1µs 40%; write_txn 132.7µs 10%; commit 27.4µs 2%; judgment_target 26.3µs 2% | flame/writes.nosync.delete_b100.svg |
| nosync/delete_b1000 | 10201.4µs | 0.723 | 0.90 | apply_deletes 4741.3µs 53%; lmdb_commit 2542.7µs 28%; write_txn 1215.5µs 13%; commit 261.1µs 3%; judgment_target 258.8µs 3% | flame/writes.nosync.delete_b1000.svg |
| nosync/bulk_append | 742829.8µs | 1.711 | 0.99 | — | — (bulk untraced by decision) |
| durable/commit_b1 | 4454.3µs | 0.991 | 1.04 | lmdb_commit 4736.4µs 99%; apply_inserts 30.3µs 1%; write_txn 13.6µs 0%; counters_flush 7.5µs 0%; commit 5.4µs 0% | flame/writes.durable.commit_b1.svg |
| durable/commit_b10 | 6478.9µs | 1.273 | 1.04 | lmdb_commit 5516.0µs 95%; apply_inserts 190.3µs 3%; write_txn 52.9µs 1%; judgment_source 27.8µs 0%; commit 12.4µs 0% | flame/writes.durable.commit_b10.svg |
| durable/commit_b100 | 12968.9µs | 1.687 | 0.98 | lmdb_commit 13926.8µs 92%; apply_inserts 804.2µs 5%; write_txn 202.8µs 1%; judgment_source 129.1µs 1%; commit 29.3µs 0% | flame/writes.durable.commit_b100.svg |
| durable/commit_b1000 | 32011.7µs | 1.967 | 0.96 | lmdb_commit 27023.0µs 77%; apply_inserts 4944.2µs 14%; write_txn 1493.1µs 4%; judgment_source 1076.7µs 3%; commit 430.5µs 1% | flame/writes.durable.commit_b1000.svg |
| durable/delete_b1 | 4197.5µs | 0.998 | 1.00 | lmdb_commit 5303.6µs 99%; apply_deletes 27.2µs 1%; write_txn 8.5µs 0%; commit 5.5µs 0%; counters_flush 3.3µs 0% | flame/writes.durable.delete_b1.svg |
| durable/delete_b10 | 5308.5µs | 1.018 | 1.00 | lmdb_commit 4775.5µs 95%; apply_deletes 171.8µs 3%; write_txn 43.6µs 1%; commit 11.2µs 0%; judgment_target 7.1µs 0% | flame/writes.durable.delete_b10.svg |
| durable/delete_b100 | 13201.3µs | 1.522 | 0.93 | lmdb_commit 11749.2µs 91%; apply_deletes 904.6µs 7%; write_txn 235.0µs 2%; commit 34.0µs 0%; judgment_target 31.0µs 0% | flame/writes.durable.delete_b100.svg |
| durable/delete_b1000 | 41010.4µs | 2.100 | 0.97 | lmdb_commit 32749.3µs 81%; apply_deletes 5346.1µs 13%; write_txn 1758.5µs 4%; judgment_target 273.2µs 1%; commit 272.9µs 1% | flame/writes.durable.delete_b1000.svg |
| durable/bulk_append | 1180086.3µs | 1.719 | 0.98 | — | — (bulk untraced by decision) |

## The untraced lanes (numbers pinned elsewhere, tallied for completeness)

| lane | number | delta | where |
|---|---|---|---|
| churn (3 profiles, 7 lanes) | ours final-probe p50 improved 16-18% every lane; commits/s inside fsync noise | vs night only (no campaign churn) | `bench-out/baseline-2026-07-25/SUMMARY.md`, `bench-out/baseline-2026-07-25/churn/` |
| curves (4 families) | busy_scan 451×, triangle 14.0×, closure_fanout 12.3×, point 4.9× (S) | vs campaign 0.97, 0 capped | `bench-out/baseline-2026-07-25/SUMMARY.md`, `bench-out/baseline-2026-07-25/curves/` |
| storage | ledger 3.487× / calendar 3.608× SQLite bytes | byte-identical to campaign | `bench-out/baseline-2026-07-25/storage/` |
| capacity-c17 | slot arm won; fetch arm deleted at `484c3871` | first pin | `bench-out/baseline-2026-07-25/capacity-c17/SUMMARY.md` |
| windowed / lawful re-pins | segment-1 re-pins under the capacity spelling | see manifest | `bench-out/baseline-2026-07-25/windowed/`, `bench-out/baseline-2026-07-25/lawful/` |
| report-class reps | read geomean(ratio_p50) 0.040–0.046, all_win 6/6 | flat-to-noise vs campaign (0.99–1.07 by rep) | `bench-out/baseline-2026-07-25/SUMMARY.md` |

## sweep-commit (T8 — the formerly PENDING lane, first pin)

Key-sorted source probes beat today's delta-order probes 0.85–0.87× from 256 touched
parents up (4096: 1134µs → 960µs p50); the capacity walk is order-insensitive (both
arms within noise). Full table: `bench-out/baseline-2026-07-25/sweep-commit/sweep.md`. This is a mechanism
with a number — fanout material.

## Where the traced pass says the microseconds are (the cross-lane headline)

Computed from the folded artifacts above; the per-lane tables are authoritative.

Scenario warm executes, all 33 traced queries summed (share of 2021796µs traced wall —
dominated by the heavy rings/olap tails: r4_bomb_t2 alone is ~1.70s of it; the shape,
not the split, is the message — the engine's scenario time IS the join kernel):

- `join` — 2017097.2µs (99%)
- `execute` — 3435.0µs (0%)
- `finalize` — 1231.5µs (0%)
- `rule_0` — 13.7µs (0%)
- `views` — 10.9µs (0%)
- `bind_params` — 5.9µs (0%)
- `resolve_filters` — 1.3µs (0%)
- `key_probe` — 0.9µs (0%)

The graph-world regression cluster (g6_weighted_hop 2.20×, g3_three_hop_count 1.88×,
t1_stab 1.86×, g5_triangles_from 1.32× vs campaign) has its per-query attribution in
the scenario table above — the Hunt readers start there.

