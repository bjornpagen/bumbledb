# COLT Allocation Baseline

Captured after PRD 12 using:

```bash
cargo test -p bumbledb-lmdb colt_allocation_benchmark_report --all-features -- --nocapture --test-threads=1
```

This focused fixture is diagnostic and deterministic enough for regression gates. It isolates COLT force, suffix iteration, forced-map lookup, and bounded batch fill without query tracing.

| fixture | intended complexity | rows | distinct keys | key width | filtered | alloc calls | allocated bytes | net bytes |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: |
| `force_duplicate_k8_unfiltered` | force allocates with distinct keys plus table storage, not source rows | 512 | 8 | 8 | no | 18 | 40992 | 37568 |
| `force_distinct_k8_unfiltered` | force allocates with distinct keys plus table storage, not source rows | 512 | 512 | 8 | no | 10 | 246624 | 140352 |
| `force_duplicate_k16_filtered` | force allocates with distinct keys plus table storage, not source rows | 512 | 8 | 16 | yes | 18 | 41048 | 35520 |
| `suffix_iteration_k8_unfiltered` | suffix iteration stays streaming and does not force a map | 512 | 512 | 8 | no | 3 | 104 | 64 |
| `map_lookup_repeated_k8_forced` | borrowed-key lookup is bounded after force | 512 | 8 | 8 | no | 0 | 0 | 0 |
| `batch_fill_k8_size4_unfiltered` | batch fill allocates with batch size, not source rows | 1024 | 1024 | 8 | no | 6 | 192 | 192 |
