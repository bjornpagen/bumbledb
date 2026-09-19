# Same-binary completion normalization after Free Join

Every query returns 64 Events: original, Possible, Guaranteed and Ambiguous
for sixteen groups. Fresh/warm totals include exact original-support counts.
Every returned Event is checked pointwise and canonically reimported outside
timing. Construction is separate. Bytes estimate retained carrier/cache memory.
They exclude allocator overhead, verification clones, result vectors and the join engine.
Executable: `eafc585b015466065805e6399056acb3780901258178e6b947157d61030a60ce`.

Only entirely successful processes contribute timing rows. Partial rows from
capped or failed processes remain in the raw evidence but cannot supply a
complete-query comparison. A later incomplete process invalidates an older
successful process for that exact job; no failed result is a fast answer.

## Incomplete processes

| Carrier | Normalization | Domain | Readout | Order | Status | Log |
| --- | --- | --- | --- | --- | --- | --- |
| prefix64 | staged | fibred | non-suffix/x | bit-major | resource_cap | [Retained log](results/ite-cap-screen-logs/job-002-readouts-prefix64----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-non-suffix-faces-x-normalize-staged.log) |
| retraction512 | staged | fibred | suffix-1/x | bit-major | resource_cap | [Retained log](results/ite-cap-screen-logs/job-003-readouts-retraction512----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-suffix-1-faces-x-normalize-staged.log) |
| retraction64 | staged | fibred | non-suffix/x | bit-major | resource_cap | [Retained log](results/ite-cap-screen-logs/job-006-readouts-retraction64----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-non-suffix-faces-x-normalize-staged.log) |
| retraction64 | staged | fibred | suffix-1/x | bit-major | resource_cap | [Retained log](results/ite-cap-screen-logs/job-008-readouts-retraction64----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-suffix-1-faces-x-normalize-staged.log) |

## fibred, width 4, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 1.273 | 0.086 | 0.071 | 0.014 | 0.001 | 0.072–0.103 | 0.075 | 305.2 | 277 | 3 |
| packed512/enum/staged | joint | 1.328 | 0.265 | 0.153 | 0.075 | 0.036 | 0.254–0.387 | 0.082 | 629.8 | 4401 | 3 |
| prefix512/slab/ite | faces | 0.932 | 14.250 | 0.144 | 14.094 | 0.012 | 14.231–14.401 | 0.076 | 3110.8 | 20158 | 3 |
| prefix512/slab/staged | faces | 1.319 | 22.904 | 0.168 | 22.731 | 0.012 | 22.592–23.543 | 0.081 | 4553.0 | 33845 | 3 |
| prefix64/slab/ite | faces | 2.548 | 22.599 | 0.428 | 22.078 | 0.091 | 22.599–22.599 | 0.162 | 4228.7 | 41558 | 1 |
| retraction512/slab/ite | faces | 0.989 | 15.051 | 0.160 | 14.871 | 0.018 | 15.051–15.051 | 0.086 | 3022.6 | 21209 | 1 |
| retraction512/slab/staged | faces | 1.345 | 24.280 | 0.175 | 24.086 | 0.018 | 24.280–24.280 | 0.102 | 5443.2 | 37423 | 1 |
| retraction64/slab/ite | faces | 2.672 | 24.600 | 0.550 | 23.947 | 0.101 | 24.600–24.600 | 0.175 | 4310.2 | 42815 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 1.273 | 0.042 | 0.029 | 0.011 | 0.001 | 0.039–0.042 | 0.020 | 314.1 | 277 | 3 |
| packed512/enum/staged | joint | 1.328 | 0.228 | 0.130 | 0.066 | 0.033 | 0.227–0.258 | 0.072 | 638.7 | 4401 | 3 |
| prefix512/slab/ite | faces | 0.932 | 14.993 | 0.156 | 14.815 | 0.015 | 14.411–15.000 | 0.083 | 3119.7 | 20158 | 3 |
| prefix512/slab/staged | faces | 1.319 | 22.186 | 0.156 | 22.025 | 0.011 | 22.052–22.416 | 0.077 | 4561.9 | 33845 | 3 |
| prefix64/slab/ite | faces | 2.548 | 21.773 | 0.434 | 21.256 | 0.082 | 21.773–21.773 | 0.145 | 4237.6 | 41558 | 1 |
| retraction512/slab/ite | faces | 0.989 | 14.628 | 0.150 | 14.466 | 0.012 | 14.628–14.628 | 0.073 | 3031.6 | 21209 | 1 |
| retraction512/slab/staged | faces | 1.345 | 23.640 | 0.153 | 23.475 | 0.012 | 23.640–23.640 | 0.074 | 5452.2 | 37423 | 1 |
| retraction64/slab/ite | faces | 2.672 | 23.776 | 0.575 | 23.115 | 0.085 | 23.776–23.776 | 0.151 | 4319.2 | 42815 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 1.298 | 0.083 | 0.067 | 0.015 | 0.001 | 0.072–0.131 | 0.074 | 305.2 | 277 | 3 |
| packed512/enum/staged | joint | 0.277 | 0.215 | 0.144 | 0.042 | 0.029 | 0.201–0.302 | 0.064 | 638.9 | 3967 | 3 |
| prefix512/slab/ite | faces | 0.827 | 1.615 | 0.151 | 1.446 | 0.012 | 1.590–1.617 | 0.232 | 458.3 | 3653 | 3 |
| prefix512/slab/staged | faces | 1.297 | 2.645 | 0.165 | 2.444 | 0.022 | 2.534–4.224 | 0.279 | 684.1 | 5407 | 3 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 1.298 | 0.041 | 0.029 | 0.011 | 0.001 | 0.041–0.043 | 0.019 | 314.1 | 277 | 3 |
| packed512/enum/staged | joint | 0.277 | 0.196 | 0.127 | 0.039 | 0.029 | 0.189–0.210 | 0.057 | 647.8 | 3967 | 3 |
| prefix512/slab/ite | faces | 0.827 | 1.578 | 0.141 | 1.423 | 0.011 | 1.559–1.591 | 0.234 | 467.3 | 3653 | 3 |
| prefix512/slab/staged | faces | 1.297 | 2.537 | 0.164 | 2.361 | 0.013 | 2.528–2.633 | 0.263 | 693.0 | 5407 | 3 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, suffix-1/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix64/slab/ite | faces | 2.513 | 0.679 | 0.428 | 0.128 | 0.121 | 0.679–0.679 | 0.240 | 510.2 | 4962 | 1 |
| prefix64/slab/staged | faces | 4.157 | 0.675 | 0.423 | 0.131 | 0.120 | 0.675–0.675 | 0.251 | 993.7 | 8238 | 1 |
| retraction512/slab/ite | faces | 1.018 | 14.336 | 0.183 | 14.133 | 0.018 | 14.336–14.336 | 0.350 | 3022.6 | 19937 | 1 |
| retraction64/slab/ite | faces | 2.755 | 24.432 | 0.544 | 23.781 | 0.105 | 24.432–24.432 | 1.145 | 4249.3 | 40087 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, suffix-1/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix64/slab/ite | faces | 2.513 | 0.674 | 0.437 | 0.124 | 0.113 | 0.674–0.674 | 0.225 | 519.2 | 4962 | 1 |
| prefix64/slab/staged | faces | 4.157 | 0.634 | 0.400 | 0.121 | 0.113 | 0.634–0.634 | 0.228 | 1002.7 | 8238 | 1 |
| retraction512/slab/ite | faces | 1.018 | 13.983 | 0.167 | 13.805 | 0.011 | 13.983–13.983 | 0.319 | 3031.6 | 19937 | 1 |
| retraction64/slab/ite | faces | 2.755 | 23.120 | 0.518 | 22.510 | 0.092 | 23.120–23.120 | 1.081 | 4258.2 | 40087 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 461.482 | 3.413 | 2.796 | 0.551 | 0.065 | 3.394–4.052 | 3.733 | 41852.6 | 636 | 3 |
| packed512/enum/staged | joint | 65.565 | 1.481 | 0.670 | 0.583 | 0.237 | 1.448–1.520 | 0.760 | 6651.9 | 47190 | 3 |
| prefix512/slab/ite | faces | 14.474 | 372.666 | 1.034 | 371.396 | 0.176 | 371.281–377.607 | 0.294 | 83841.6 | 708405 | 3 |
| prefix512/slab/staged | faces | 24.815 | 1671.186 | 1.016 | 1669.914 | 0.208 | 1652.080–1713.951 | 0.430 | 162947.4 | 1363847 | 3 |
| prefix64/slab/ite | faces | 16.668 | 303.365 | 1.340 | 301.663 | 0.360 | 303.365–303.365 | 0.486 | 86802.5 | 839852 | 1 |
| retraction512/slab/ite | faces | 15.262 | 410.704 | 1.405 | 409.093 | 0.205 | 410.704–410.704 | 0.308 | 78436.4 | 673536 | 1 |
| retraction512/slab/staged | faces | 26.526 | 2435.365 | 1.473 | 2433.660 | 0.231 | 2435.365–2435.365 | 0.557 | 156759.6 | 1415883 | 1 |
| retraction64/slab/ite | faces | 26.038 | 411.049 | 2.149 | 408.351 | 0.548 | 411.049–411.049 | 0.670 | 92504.5 | 804725 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 461.482 | 1.451 | 0.888 | 0.489 | 0.063 | 1.418–1.502 | 0.542 | 41861.5 | 636 | 3 |
| packed512/enum/staged | joint | 65.565 | 1.486 | 0.681 | 0.594 | 0.220 | 1.482–1.650 | 0.749 | 6660.8 | 47190 | 3 |
| prefix512/slab/ite | faces | 14.474 | 377.951 | 0.988 | 376.797 | 0.175 | 374.044–390.359 | 0.263 | 83850.6 | 708405 | 3 |
| prefix512/slab/staged | faces | 24.815 | 1643.748 | 1.012 | 1642.507 | 0.194 | 1639.810–1661.637 | 0.405 | 162956.4 | 1363847 | 3 |
| prefix64/slab/ite | faces | 16.668 | 371.357 | 1.362 | 369.633 | 0.361 | 371.357–371.357 | 0.459 | 86811.5 | 839852 | 1 |
| retraction512/slab/ite | faces | 15.262 | 486.694 | 1.391 | 485.086 | 0.216 | 486.694–486.694 | 0.297 | 78445.4 | 673536 | 1 |
| retraction512/slab/staged | faces | 26.526 | 2291.654 | 1.419 | 2290.007 | 0.227 | 2291.654–2291.654 | 0.452 | 156768.6 | 1415883 | 1 |
| retraction64/slab/ite | faces | 26.038 | 413.411 | 2.167 | 410.696 | 0.547 | 413.411–413.411 | 0.643 | 92513.5 | 804725 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 462.528 | 3.387 | 2.785 | 0.540 | 0.065 | 3.381–4.147 | 3.760 | 41852.6 | 636 | 3 |
| packed512/enum/staged | joint | 2.131 | 0.586 | 0.413 | 0.105 | 0.075 | 0.560–0.626 | 0.216 | 2402.4 | 18095 | 3 |
| prefix512/slab/ite | faces | 10.331 | 8.743 | 1.203 | 7.374 | 0.129 | 8.563–8.789 | 0.706 | 4092.3 | 31114 | 3 |
| prefix512/slab/staged | faces | 19.352 | 15.358 | 1.286 | 13.925 | 0.146 | 15.334–15.565 | 0.794 | 7010.0 | 59034 | 3 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 462.528 | 1.449 | 0.892 | 0.491 | 0.065 | 1.432–1.478 | 0.532 | 41861.5 | 636 | 3 |
| packed512/enum/staged | joint | 2.131 | 0.564 | 0.388 | 0.103 | 0.074 | 0.562–0.705 | 0.243 | 2411.3 | 18095 | 3 |
| prefix512/slab/ite | faces | 10.331 | 9.062 | 1.241 | 7.686 | 0.133 | 8.687–9.087 | 0.753 | 4101.2 | 31114 | 3 |
| prefix512/slab/staged | faces | 19.352 | 14.269 | 1.256 | 12.890 | 0.126 | 14.038–14.279 | 0.714 | 7018.9 | 59034 | 3 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, suffix-1/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix64/slab/ite | faces | 16.869 | 2.050 | 1.343 | 0.355 | 0.352 | 2.050–2.050 | 0.718 | 4692.6 | 46969 | 1 |
| prefix64/slab/staged | faces | 30.998 | 2.207 | 1.429 | 0.383 | 0.395 | 2.207–2.207 | 0.794 | 8832.8 | 85138 | 1 |
| retraction512/slab/ite | faces | 15.169 | 411.179 | 1.369 | 409.608 | 0.201 | 411.179–411.179 | 3.919 | 60172.4 | 637135 | 1 |
| retraction64/slab/ite | faces | 25.712 | 438.538 | 2.112 | 435.888 | 0.535 | 438.538–438.538 | 5.266 | 70725.9 | 777405 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, suffix-1/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix64/slab/ite | faces | 16.869 | 2.054 | 1.337 | 0.357 | 0.356 | 2.054–2.054 | 0.707 | 4701.6 | 46969 | 1 |
| prefix64/slab/staged | faces | 30.998 | 2.204 | 1.448 | 0.390 | 0.366 | 2.204–2.204 | 0.773 | 8841.8 | 85138 | 1 |
| retraction512/slab/ite | faces | 15.169 | 415.146 | 1.437 | 413.505 | 0.203 | 415.146–415.146 | 4.073 | 60181.3 | 637135 | 1 |
| retraction64/slab/ite | faces | 25.712 | 429.620 | 2.104 | 426.989 | 0.526 | 429.620–429.620 | 4.995 | 70734.9 | 777405 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## full, width 4, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 0.029 | 0.078 | 0.066 | 0.010 | 0.001 | 0.078–0.078 | 0.057 | 91.4 | 157 | 1 |
| packed512/enum/staged | joint | 0.538 | 0.104 | 0.075 | 0.015 | 0.010 | 0.104–0.104 | 0.026 | 105.0 | 590 | 1 |
| prefix512/slab/ite | faces | 0.068 | 0.165 | 0.128 | 0.035 | 0.002 | 0.165–0.165 | 0.044 | 21.5 | 169 | 1 |
| prefix512/slab/staged | faces | 0.069 | 0.170 | 0.132 | 0.035 | 0.002 | 0.170–0.170 | 0.042 | 21.5 | 169 | 1 |
| retraction512/slab/ite | faces | 0.075 | 0.176 | 0.135 | 0.038 | 0.002 | 0.176–0.176 | 0.045 | 21.5 | 169 | 1 |
| retraction512/slab/staged | faces | 0.071 | 0.179 | 0.137 | 0.039 | 0.002 | 0.179–0.179 | 0.048 | 21.5 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 0.029 | 0.046 | 0.036 | 0.009 | 0.001 | 0.046–0.046 | 0.019 | 100.3 | 157 | 1 |
| packed512/enum/staged | joint | 0.538 | 0.068 | 0.052 | 0.009 | 0.008 | 0.068–0.068 | 0.023 | 114.0 | 590 | 1 |
| prefix512/slab/ite | faces | 0.068 | 0.157 | 0.127 | 0.030 | 0.000 | 0.157–0.157 | 0.038 | 30.4 | 169 | 1 |
| prefix512/slab/staged | faces | 0.069 | 0.178 | 0.140 | 0.038 | 0.000 | 0.178–0.178 | 0.038 | 30.4 | 169 | 1 |
| retraction512/slab/ite | faces | 0.075 | 0.155 | 0.123 | 0.032 | 0.001 | 0.155–0.155 | 0.041 | 30.4 | 169 | 1 |
| retraction512/slab/staged | faces | 0.071 | 0.161 | 0.127 | 0.034 | 0.000 | 0.161–0.161 | 0.042 | 30.4 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 0.023 | 0.065 | 0.053 | 0.009 | 0.001 | 0.065–0.065 | 0.049 | 91.4 | 157 | 1 |
| packed512/enum/staged | joint | 0.055 | 0.060 | 0.044 | 0.009 | 0.004 | 0.060–0.060 | 0.069 | 36.2 | 157 | 1 |
| prefix512/slab/ite | faces | 0.072 | 0.177 | 0.136 | 0.038 | 0.002 | 0.177–0.177 | 0.046 | 21.5 | 169 | 1 |
| prefix512/slab/staged | faces | 0.153 | 0.425 | 0.324 | 0.094 | 0.005 | 0.425–0.425 | 0.124 | 21.5 | 169 | 1 |
| retraction512/slab/ite | faces | 0.074 | 0.186 | 0.139 | 0.042 | 0.005 | 0.186–0.186 | 0.043 | 21.5 | 169 | 1 |
| retraction512/slab/staged | faces | 0.075 | 0.180 | 0.139 | 0.039 | 0.002 | 0.180–0.180 | 0.042 | 21.5 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 0.023 | 0.039 | 0.031 | 0.008 | 0.001 | 0.039–0.039 | 0.017 | 100.3 | 157 | 1 |
| packed512/enum/staged | joint | 0.055 | 0.049 | 0.040 | 0.004 | 0.003 | 0.049–0.049 | 0.025 | 45.1 | 157 | 1 |
| prefix512/slab/ite | faces | 0.072 | 0.154 | 0.121 | 0.032 | 0.000 | 0.154–0.154 | 0.041 | 30.4 | 169 | 1 |
| prefix512/slab/staged | faces | 0.153 | 0.166 | 0.132 | 0.033 | 0.000 | 0.166–0.166 | 0.042 | 30.4 | 169 | 1 |
| retraction512/slab/ite | faces | 0.074 | 0.147 | 0.116 | 0.031 | 0.000 | 0.147–0.147 | 0.038 | 30.4 | 169 | 1 |
| retraction512/slab/staged | faces | 0.075 | 0.148 | 0.117 | 0.030 | 0.000 | 0.148–0.148 | 0.038 | 30.4 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 0.831 | 2.598 | 1.933 | 0.622 | 0.040 | 2.598–2.598 | 2.614 | 5351.2 | 161 | 1 |
| packed512/enum/staged | joint | 30.054 | 0.212 | 0.153 | 0.030 | 0.029 | 0.212–0.212 | 0.087 | 367.9 | 2289 | 1 |
| prefix512/slab/ite | faces | 0.535 | 0.681 | 0.506 | 0.174 | 0.000 | 0.681–0.681 | 0.185 | 137.3 | 1030 | 1 |
| prefix512/slab/staged | faces | 0.552 | 0.659 | 0.490 | 0.169 | 0.000 | 0.659–0.659 | 0.188 | 137.3 | 1030 | 1 |
| retraction512/slab/ite | faces | 0.578 | 0.770 | 0.593 | 0.175 | 0.001 | 0.770–0.770 | 0.212 | 137.3 | 1030 | 1 |
| retraction512/slab/staged | faces | 0.578 | 0.693 | 0.516 | 0.176 | 0.001 | 0.693–0.693 | 0.200 | 137.3 | 1030 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 0.831 | 1.167 | 0.535 | 0.593 | 0.038 | 1.167–1.167 | 0.640 | 5360.2 | 161 | 1 |
| packed512/enum/staged | joint | 30.054 | 0.221 | 0.169 | 0.027 | 0.026 | 0.221–0.221 | 0.070 | 376.8 | 2289 | 1 |
| prefix512/slab/ite | faces | 0.535 | 0.643 | 0.473 | 0.169 | 0.000 | 0.643–0.643 | 0.217 | 146.3 | 1030 | 1 |
| prefix512/slab/staged | faces | 0.552 | 0.650 | 0.485 | 0.164 | 0.000 | 0.650–0.650 | 0.185 | 146.3 | 1030 | 1 |
| retraction512/slab/ite | faces | 0.578 | 0.681 | 0.511 | 0.169 | 0.001 | 0.681–0.681 | 0.193 | 146.3 | 1030 | 1 |
| retraction512/slab/staged | faces | 0.578 | 1.261 | 0.512 | 0.747 | 0.001 | 1.261–1.261 | 0.195 | 146.3 | 1030 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 0.691 | 2.190 | 1.641 | 0.517 | 0.031 | 2.190–2.190 | 2.009 | 5351.2 | 161 | 1 |
| packed512/enum/staged | joint | 0.454 | 0.159 | 0.111 | 0.027 | 0.021 | 0.159–0.159 | 0.075 | 215.9 | 1238 | 1 |
| prefix512/slab/ite | faces | 0.571 | 0.901 | 0.674 | 0.226 | 0.001 | 0.901–0.901 | 0.254 | 166.8 | 1254 | 1 |
| prefix512/slab/staged | faces | 0.564 | 0.896 | 0.672 | 0.223 | 0.001 | 0.896–0.896 | 0.255 | 166.8 | 1254 | 1 |
| retraction512/slab/ite | faces | 0.526 | 0.876 | 0.640 | 0.235 | 0.000 | 0.876–0.876 | 0.255 | 166.8 | 1254 | 1 |
| retraction512/slab/staged | faces | 0.525 | 0.852 | 0.632 | 0.220 | 0.000 | 0.852–0.852 | 0.260 | 166.8 | 1254 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 0.691 | 1.004 | 0.463 | 0.510 | 0.031 | 1.004–1.004 | 0.556 | 5360.2 | 161 | 1 |
| packed512/enum/staged | joint | 0.454 | 0.167 | 0.125 | 0.023 | 0.018 | 0.167–0.167 | 0.060 | 224.9 | 1238 | 1 |
| prefix512/slab/ite | faces | 0.571 | 0.883 | 0.660 | 0.222 | 0.000 | 0.883–0.883 | 0.244 | 175.7 | 1254 | 1 |
| prefix512/slab/staged | faces | 0.564 | 0.882 | 0.658 | 0.222 | 0.000 | 0.882–0.882 | 0.241 | 175.7 | 1254 | 1 |
| retraction512/slab/ite | faces | 0.526 | 0.858 | 0.648 | 0.209 | 0.000 | 0.858–0.858 | 0.221 | 175.7 | 1254 | 1 |
| retraction512/slab/staged | faces | 0.525 | 0.860 | 0.641 | 0.218 | 0.000 | 0.860–0.860 | 0.229 | 175.7 | 1254 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 0.574 | 0.074 | 0.060 | 0.012 | 0.001 | 0.074–0.074 | 0.047 | 115.2 | 200 | 1 |
| packed512/enum/staged | joint | 0.769 | 0.175 | 0.111 | 0.040 | 0.023 | 0.175–0.175 | 0.053 | 290.7 | 1877 | 1 |
| prefix512/slab/ite | faces | 0.578 | 0.179 | 0.131 | 0.031 | 0.014 | 0.179–0.179 | 0.055 | 107.7 | 759 | 1 |
| prefix512/slab/staged | faces | 0.760 | 0.182 | 0.135 | 0.032 | 0.013 | 0.182–0.182 | 0.056 | 150.7 | 1024 | 1 |
| retraction512/slab/ite | faces | 0.520 | 1.059 | 0.115 | 0.929 | 0.013 | 1.059–1.059 | 0.291 | 188.2 | 1626 | 1 |
| retraction512/slab/staged | faces | 0.735 | 1.387 | 0.138 | 1.237 | 0.011 | 1.387–1.387 | 0.324 | 325.3 | 2368 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 0.574 | 0.035 | 0.026 | 0.008 | 0.001 | 0.035–0.035 | 0.016 | 124.1 | 200 | 1 |
| packed512/enum/staged | joint | 0.769 | 0.119 | 0.073 | 0.028 | 0.018 | 0.119–0.119 | 0.053 | 299.7 | 1877 | 1 |
| prefix512/slab/ite | faces | 0.578 | 0.156 | 0.118 | 0.030 | 0.008 | 0.156–0.156 | 0.050 | 116.7 | 759 | 1 |
| prefix512/slab/staged | faces | 0.760 | 0.165 | 0.127 | 0.029 | 0.008 | 0.165–0.165 | 0.045 | 159.7 | 1024 | 1 |
| retraction512/slab/ite | faces | 0.520 | 1.101 | 0.115 | 0.980 | 0.006 | 1.101–1.101 | 0.307 | 197.2 | 1626 | 1 |
| retraction512/slab/staged | faces | 0.735 | 1.300 | 0.132 | 1.163 | 0.005 | 1.300–1.300 | 0.295 | 334.2 | 2368 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 0.593 | 0.063 | 0.050 | 0.010 | 0.001 | 0.063–0.063 | 0.049 | 115.2 | 200 | 1 |
| packed512/enum/staged | joint | 0.210 | 0.159 | 0.106 | 0.034 | 0.017 | 0.159–0.159 | 0.041 | 331.7 | 2075 | 1 |
| prefix512/slab/ite | faces | 0.528 | 0.177 | 0.130 | 0.032 | 0.014 | 0.177–0.177 | 0.053 | 92.7 | 664 | 1 |
| prefix512/slab/staged | faces | 0.729 | 0.197 | 0.146 | 0.035 | 0.015 | 0.197–0.197 | 0.065 | 115.6 | 855 | 1 |
| retraction512/slab/ite | faces | 0.469 | 0.680 | 0.116 | 0.554 | 0.009 | 0.680–0.680 | 0.276 | 168.5 | 1080 | 1 |
| retraction512/slab/staged | faces | 1.466 | 2.116 | 0.276 | 1.813 | 0.024 | 2.116–2.116 | 0.281 | 186.4 | 1579 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 0.593 | 0.037 | 0.028 | 0.008 | 0.001 | 0.037–0.037 | 0.017 | 124.1 | 200 | 1 |
| packed512/enum/staged | joint | 0.210 | 0.128 | 0.083 | 0.031 | 0.013 | 0.128–0.128 | 0.036 | 340.7 | 2075 | 1 |
| prefix512/slab/ite | faces | 0.528 | 0.156 | 0.118 | 0.030 | 0.008 | 0.156–0.156 | 0.047 | 101.7 | 664 | 1 |
| prefix512/slab/staged | faces | 0.729 | 0.236 | 0.146 | 0.068 | 0.021 | 0.236–0.236 | 0.062 | 124.6 | 855 | 1 |
| retraction512/slab/ite | faces | 0.469 | 0.630 | 0.107 | 0.518 | 0.005 | 0.630–0.630 | 0.267 | 177.5 | 1080 | 1 |
| retraction512/slab/staged | faces | 1.466 | 0.836 | 0.123 | 0.707 | 0.005 | 0.836–0.836 | 0.270 | 195.3 | 1579 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 226.990 | 2.250 | 1.657 | 0.561 | 0.032 | 2.250–2.250 | 2.073 | 17043.7 | 517 | 1 |
| packed512/enum/staged | joint | 34.416 | 1.529 | 0.985 | 0.318 | 0.225 | 1.529–1.529 | 0.547 | 5355.9 | 31982 | 1 |
| prefix512/slab/ite | faces | 6.690 | 0.785 | 0.505 | 0.150 | 0.130 | 0.785–0.785 | 0.315 | 1431.4 | 9539 | 1 |
| prefix512/slab/staged | faces | 11.402 | 0.822 | 0.524 | 0.167 | 0.130 | 0.822–0.822 | 0.325 | 2917.2 | 17665 | 1 |
| retraction512/slab/ite | faces | 7.075 | 10.665 | 0.736 | 9.867 | 0.061 | 10.665–10.665 | 3.156 | 2139.4 | 16955 | 1 |
| retraction512/slab/staged | faces | 12.296 | 15.039 | 0.779 | 14.194 | 0.066 | 15.039–15.039 | 3.134 | 4053.3 | 31857 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 226.990 | 1.130 | 0.523 | 0.576 | 0.031 | 1.130–1.130 | 0.581 | 17052.6 | 517 | 1 |
| packed512/enum/staged | joint | 34.416 | 1.401 | 0.869 | 0.306 | 0.225 | 1.401–1.401 | 0.549 | 5364.8 | 31982 | 1 |
| prefix512/slab/ite | faces | 6.690 | 0.796 | 0.521 | 0.152 | 0.122 | 0.796–0.796 | 0.302 | 1440.4 | 9539 | 1 |
| prefix512/slab/staged | faces | 11.402 | 0.838 | 0.557 | 0.157 | 0.123 | 0.838–0.838 | 0.303 | 2926.2 | 17665 | 1 |
| retraction512/slab/ite | faces | 7.075 | 10.526 | 0.763 | 9.709 | 0.054 | 10.526–10.526 | 3.263 | 2148.4 | 16955 | 1 |
| retraction512/slab/staged | faces | 12.296 | 14.003 | 0.714 | 13.234 | 0.054 | 14.003–14.003 | 3.111 | 4062.2 | 31857 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 222.797 | 2.161 | 1.574 | 0.555 | 0.031 | 2.161–2.161 | 2.043 | 17043.7 | 517 | 1 |
| packed512/enum/staged | joint | 1.381 | 0.349 | 0.243 | 0.060 | 0.046 | 0.349–0.349 | 0.120 | 1282.0 | 10792 | 1 |
| prefix512/slab/ite | faces | 4.709 | 0.997 | 0.663 | 0.206 | 0.128 | 0.997–0.997 | 0.347 | 913.9 | 6457 | 1 |
| prefix512/slab/staged | faces | 7.518 | 1.060 | 0.719 | 0.212 | 0.129 | 1.060–1.060 | 0.368 | 1588.6 | 11564 | 1 |
| retraction512/slab/ite | faces | 4.615 | 1.878 | 0.816 | 1.006 | 0.055 | 1.878–1.878 | 0.623 | 1062.3 | 7618 | 1 |
| retraction512/slab/staged | faces | 8.298 | 2.328 | 1.017 | 1.248 | 0.063 | 2.328–2.328 | 1.732 | 2235.3 | 14582 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged | joint | 222.797 | 1.114 | 0.558 | 0.523 | 0.031 | 1.114–1.114 | 0.566 | 17052.6 | 517 | 1 |
| packed512/enum/staged | joint | 1.381 | 0.322 | 0.225 | 0.055 | 0.042 | 0.322–0.322 | 0.116 | 1291.0 | 10792 | 1 |
| prefix512/slab/ite | faces | 4.709 | 0.976 | 0.646 | 0.205 | 0.124 | 0.976–0.976 | 0.348 | 922.9 | 6457 | 1 |
| prefix512/slab/staged | faces | 7.518 | 0.983 | 0.639 | 0.206 | 0.137 | 0.983–0.983 | 0.348 | 1597.5 | 11564 | 1 |
| retraction512/slab/ite | faces | 4.615 | 1.933 | 0.819 | 1.061 | 0.053 | 1.933–1.933 | 0.620 | 1071.3 | 7618 | 1 |
| retraction512/slab/staged | faces | 8.298 | 2.354 | 0.988 | 1.309 | 0.057 | 2.354–2.354 | 0.631 | 2244.3 | 14582 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

160 configurations; 364 matched comparisons; 50 retained processes.

- [Raw evidence](results/ite-readout-repeat.json).
- [Raw evidence](results/ite-cap-screen.json).
- [Raw evidence](results/ite-controls.json).
