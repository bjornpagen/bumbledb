# Same-binary completion normalization after Free Join

Every query returns 64 Events: original, Possible, Guaranteed and Ambiguous
for sixteen groups. Fresh/warm totals include exact original-support counts.
Every returned Event is checked pointwise and canonically reimported outside
timing. Construction is separate. Bytes estimate retained carrier/cache memory.
They exclude allocator overhead, verification clones, result vectors and the join engine.
Executable: `79c66a42020bd736d0ba9b53426d84d6c24b0b09392b73ddf0a37540f9a6b8fa`.

Only entirely successful processes contribute timing rows. Partial rows from
capped or failed processes remain in the raw evidence but cannot supply a
complete-query comparison. A later incomplete process invalidates an older
successful process for that exact job; no failed result is a fast answer.

## fibred, width 4, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 1.438 | 23.398 | 0.173 | 23.205 | 0.018 | 23.398–23.398 | 0.107 | 4553.0 | 33845 | 1 |
| prefix512/slab/staged | faces | 1.435 | 23.381 | 0.238 | 23.112 | 0.018 | 23.381–23.381 | 0.136 | 4553.0 | 33845 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 1.438 | 22.111 | 0.161 | 21.938 | 0.011 | 22.111–22.111 | 0.074 | 4561.9 | 33845 | 1 |
| prefix512/slab/staged | faces | 1.435 | 22.458 | 0.161 | 22.285 | 0.011 | 22.458–22.458 | 0.073 | 4561.9 | 33845 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 24.889 | 1695.139 | 1.092 | 1693.836 | 0.210 | 1695.139–1695.139 | 0.429 | 162947.4 | 1363847 | 1 |
| prefix512/slab/staged | faces | 25.016 | 1693.416 | 1.105 | 1692.098 | 0.212 | 1693.416–1693.416 | 0.433 | 162947.4 | 1363847 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 24.889 | 1742.380 | 1.057 | 1741.126 | 0.196 | 1742.380–1742.380 | 0.395 | 162956.4 | 1363847 | 1 |
| prefix512/slab/staged | faces | 25.016 | 1660.009 | 1.110 | 1658.702 | 0.195 | 1660.009–1660.009 | 0.387 | 162956.4 | 1363847 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

8 configurations; 4 matched comparisons; 4 retained processes.

- [Raw evidence](results/completion-first-smoke.json).
