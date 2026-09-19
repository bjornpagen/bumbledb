# Same-binary dependency-directed projection after Free Join

Every query returns 64 Events: original, Possible, Guaranteed and Ambiguous
for sixteen groups. Fresh/warm totals include exact original-support counts.
Every returned Event is checked pointwise and canonically reimported outside
timing. Construction is separate. Bytes estimate retained carrier/cache memory.
They exclude allocator overhead, verification clones, result vectors and the join engine.
Executable: `e09c99985bf2e46b2c0dfd9a3d05323794440dda01b50f709913b0c837c08afa`.

Only entirely successful processes contribute timing rows. Partial rows from
capped or failed processes remain in the raw evidence but cannot supply a
complete-query comparison. A later incomplete process invalidates an older
successful process for that exact job; no failed result is a fast answer.

## fibred, width 4, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/ite/active | faces | 0.993 | 0.596 | 0.152 | 0.427 | 0.015 | 0.596–0.596 | 0.042 | 223.7 | 1736 | 1 |
| prefix512/slab/ite/joint | faces | 1.041 | 14.785 | 0.230 | 14.518 | 0.020 | 14.785–14.785 | 0.111 | 3110.9 | 20158 | 1 |
| prefix512/slab/ite/witness | faces | 0.994 | 0.551 | 0.158 | 0.375 | 0.017 | 0.551–0.551 | 0.064 | 223.7 | 1669 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/ite/active | faces | 0.993 | 0.586 | 0.145 | 0.428 | 0.011 | 0.586–0.586 | 0.035 | 232.6 | 1736 | 1 |
| prefix512/slab/ite/joint | faces | 1.041 | 14.761 | 0.155 | 14.592 | 0.013 | 14.761–14.761 | 0.105 | 3119.8 | 20158 | 1 |
| prefix512/slab/ite/witness | faces | 0.994 | 0.541 | 0.156 | 0.372 | 0.012 | 0.541–0.541 | 0.040 | 232.6 | 1669 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/ite/active | faces | 14.188 | 12.755 | 1.008 | 11.612 | 0.134 | 12.755–12.755 | 0.221 | 4318.9 | 37611 | 1 |
| prefix512/slab/ite/joint | faces | 14.839 | 484.695 | 1.044 | 483.468 | 0.182 | 484.695–484.695 | 0.293 | 83841.7 | 708405 | 1 |
| prefix512/slab/ite/witness | faces | 15.261 | 11.610 | 1.047 | 10.407 | 0.152 | 11.610–11.610 | 0.208 | 4318.9 | 35380 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/ite/active | faces | 14.188 | 12.329 | 0.982 | 11.213 | 0.134 | 12.329–12.329 | 0.209 | 4327.8 | 37611 | 1 |
| prefix512/slab/ite/joint | faces | 14.839 | 363.419 | 0.988 | 362.246 | 0.185 | 363.419–363.419 | 0.268 | 83850.6 | 708405 | 1 |
| prefix512/slab/ite/witness | faces | 15.261 | 10.648 | 1.006 | 9.509 | 0.132 | 10.648–10.648 | 0.203 | 4327.8 | 35380 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

12 configurations; 12 matched comparisons; 4 retained processes.

- [Raw evidence](results/projection-first-smoke.json).
