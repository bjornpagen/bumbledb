# Native information readouts after Free Join

Every query returns 64 Events: original, Possible, Guaranteed and Ambiguous
for sixteen groups. Fresh/warm totals include exact original-support counts.
Every returned Event is checked pointwise and canonically reimported outside
timing. Construction is separate. Bytes estimate retained carrier/cache memory.
They exclude allocator overhead, verification clones, result vectors and the join engine.
Executable: `9e02aec91a23ad8f7b5902ff8e959b9fbda29e2f3291560063d5a6c6cd309214`.

## fibred, width 4, suffix-1/x, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab | faces | 1.464 | 0.292 | 0.237 | 0.023 | 0.019 | 0.292–0.292 | 0.059 | 285.2 | 1723 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, suffix-1/x, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab | faces | 1.464 | 0.201 | 0.159 | 0.025 | 0.016 | 0.201–0.201 | 0.052 | 294.2 | 1723 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, suffix-1/x, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab | faces | 26.875 | 1.517 | 1.096 | 0.262 | 0.159 | 1.517–1.517 | 0.445 | 6126.3 | 39709 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, suffix-1/x, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab | faces | 26.875 | 1.464 | 1.061 | 0.253 | 0.149 | 1.464–1.464 | 0.401 | 6135.2 | 39709 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

4 configurations; 0 matched comparisons; 2 retained processes.

- [Raw evidence](results/readout-first-smoke.json).
