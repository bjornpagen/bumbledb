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
| dense-dispatched/enum/staged/joint | joint | 1.289 | 0.080 | 0.066 | 0.013 | 0.001 | 0.077–0.136 | 0.075 | 305.2 | 277 | 3 |
| packed512/enum/staged/joint | joint | 1.297 | 0.245 | 0.140 | 0.071 | 0.034 | 0.233–0.373 | 0.077 | 629.8 | 4401 | 3 |
| prefix512/slab/ite/active | faces | 0.908 | 0.593 | 0.152 | 0.426 | 0.015 | 0.578–0.604 | 0.038 | 223.7 | 1736 | 3 |
| prefix512/slab/ite/joint | faces | 0.912 | 14.425 | 0.147 | 14.266 | 0.012 | 13.816–14.493 | 0.080 | 3110.9 | 20158 | 3 |
| prefix512/slab/ite/witness | faces | 0.952 | 0.587 | 0.169 | 0.398 | 0.018 | 0.566–1.318 | 0.060 | 223.7 | 1669 | 3 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 1.289 | 0.041 | 0.029 | 0.010 | 0.001 | 0.040–0.044 | 0.019 | 314.1 | 277 | 3 |
| packed512/enum/staged/joint | joint | 1.297 | 0.225 | 0.128 | 0.065 | 0.033 | 0.225–0.239 | 0.073 | 638.7 | 4401 | 3 |
| prefix512/slab/ite/active | faces | 0.908 | 0.576 | 0.148 | 0.416 | 0.012 | 0.573–0.605 | 0.037 | 232.6 | 1736 | 3 |
| prefix512/slab/ite/joint | faces | 0.912 | 13.809 | 0.139 | 13.644 | 0.012 | 13.692–13.905 | 0.071 | 3119.8 | 20158 | 3 |
| prefix512/slab/ite/witness | faces | 0.952 | 1.349 | 0.818 | 0.516 | 0.014 | 1.235–1.876 | 0.053 | 232.6 | 1669 | 3 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 1.262 | 0.082 | 0.066 | 0.015 | 0.001 | 0.075–0.090 | 0.075 | 305.2 | 277 | 3 |
| packed512/enum/staged/joint | joint | 0.272 | 0.228 | 0.156 | 0.043 | 0.029 | 0.208–0.269 | 0.063 | 638.9 | 3967 | 3 |
| prefix512/slab/ite/active | faces | 0.928 | 0.619 | 0.157 | 0.450 | 0.012 | 0.601–0.642 | 0.037 | 262.8 | 1727 | 3 |
| prefix512/slab/ite/joint | faces | 0.920 | 1.653 | 0.141 | 1.501 | 0.012 | 1.592–1.694 | 0.231 | 458.4 | 3653 | 3 |
| prefix512/slab/ite/witness | faces | 0.888 | 0.525 | 0.146 | 0.368 | 0.012 | 0.523–0.553 | 0.038 | 210.5 | 1639 | 3 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 1.262 | 0.041 | 0.029 | 0.010 | 0.001 | 0.039–0.043 | 0.019 | 314.1 | 277 | 3 |
| packed512/enum/staged/joint | joint | 0.272 | 0.210 | 0.141 | 0.041 | 0.029 | 0.204–0.256 | 0.057 | 647.8 | 3967 | 3 |
| prefix512/slab/ite/active | faces | 0.928 | 0.588 | 0.143 | 0.437 | 0.012 | 0.588–0.606 | 0.034 | 271.7 | 1727 | 3 |
| prefix512/slab/ite/joint | faces | 0.920 | 1.583 | 0.145 | 1.423 | 0.011 | 1.562–1.671 | 0.221 | 467.3 | 3653 | 3 |
| prefix512/slab/ite/witness | faces | 0.888 | 0.513 | 0.141 | 0.361 | 0.011 | 0.512–0.525 | 0.038 | 219.5 | 1639 | 3 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 460.791 | 3.793 | 3.114 | 0.612 | 0.063 | 3.565–4.420 | 3.694 | 41852.6 | 636 | 3 |
| packed512/enum/staged/joint | joint | 64.869 | 1.465 | 0.675 | 0.568 | 0.216 | 1.432–1.470 | 0.790 | 6651.9 | 47190 | 3 |
| prefix512/slab/ite/active | faces | 14.682 | 12.469 | 1.001 | 11.384 | 0.132 | 12.269–12.730 | 0.216 | 4318.9 | 37611 | 3 |
| prefix512/slab/ite/joint | faces | 14.142 | 371.689 | 0.990 | 370.520 | 0.185 | 371.054–387.981 | 0.275 | 83841.7 | 708405 | 3 |
| prefix512/slab/ite/witness | faces | 136.722 | 12.018 | 1.118 | 10.840 | 0.144 | 11.819–31.848 | 0.228 | 4318.9 | 35380 | 3 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 460.791 | 1.446 | 0.909 | 0.480 | 0.062 | 1.445–1.470 | 0.515 | 41861.5 | 636 | 3 |
| packed512/enum/staged/joint | joint | 64.869 | 1.467 | 0.673 | 0.570 | 0.224 | 1.452–1.662 | 0.779 | 6660.8 | 47190 | 3 |
| prefix512/slab/ite/active | faces | 14.682 | 12.402 | 0.974 | 11.298 | 0.130 | 12.097–12.590 | 0.205 | 4327.8 | 37611 | 3 |
| prefix512/slab/ite/joint | faces | 14.142 | 392.599 | 1.020 | 391.363 | 0.177 | 368.885–404.967 | 0.275 | 83850.6 | 708405 | 3 |
| prefix512/slab/ite/witness | faces | 136.722 | 10.758 | 1.022 | 9.606 | 0.132 | 10.462–11.159 | 0.198 | 4327.8 | 35380 | 3 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 463.251 | 3.518 | 2.888 | 0.567 | 0.064 | 3.343–4.087 | 3.717 | 41852.6 | 636 | 3 |
| packed512/enum/staged/joint | joint | 2.159 | 0.573 | 0.395 | 0.105 | 0.075 | 0.558–0.603 | 0.227 | 2402.4 | 18095 | 3 |
| prefix512/slab/ite/active | faces | 10.217 | 7.166 | 1.198 | 5.837 | 0.130 | 7.037–7.427 | 0.537 | 2764.0 | 26325 | 3 |
| prefix512/slab/ite/joint | faces | 10.507 | 8.604 | 1.185 | 7.288 | 0.131 | 8.539–8.684 | 0.724 | 4092.3 | 31114 | 3 |
| prefix512/slab/ite/witness | faces | 10.305 | 6.254 | 1.231 | 4.871 | 0.135 | 6.076–6.290 | 0.471 | 2642.1 | 24270 | 3 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 463.251 | 1.480 | 0.923 | 0.472 | 0.062 | 1.414–1.731 | 0.513 | 41861.5 | 636 | 3 |
| packed512/enum/staged/joint | joint | 2.159 | 0.595 | 0.418 | 0.103 | 0.074 | 0.574–0.687 | 0.208 | 2411.3 | 18095 | 3 |
| prefix512/slab/ite/active | faces | 10.217 | 7.346 | 1.241 | 5.957 | 0.129 | 7.321–7.386 | 0.553 | 2772.9 | 26325 | 3 |
| prefix512/slab/ite/joint | faces | 10.507 | 8.584 | 1.191 | 7.276 | 0.131 | 8.578–8.819 | 0.719 | 4101.3 | 31114 | 3 |
| prefix512/slab/ite/witness | faces | 10.305 | 5.985 | 1.182 | 4.667 | 0.127 | 5.962–6.062 | 0.508 | 2651.1 | 24270 | 3 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

40 configurations; 92 matched comparisons; 15 retained processes.

- [Raw evidence](results/projection-first-smoke.json).
- [Raw evidence](results/projection-readout-repeat.json).
