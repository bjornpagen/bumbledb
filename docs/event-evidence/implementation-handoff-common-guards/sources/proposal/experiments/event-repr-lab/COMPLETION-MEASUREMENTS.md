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

## Incomplete processes

| Carrier | Normalization | Domain | Readout | Order | Status | Log |
| --- | --- | --- | --- | --- | --- | --- |
| retraction64 | staged | fibred | non-suffix/x | bit-major | resource_cap | [Retained log](results/completion-caps-logs/job-002-readouts-retraction64----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-non-suffix-faces-x-normalize-staged.log) |
| retraction64 | equal | fibred | suffix-1/x | bit-major | resource_cap | [Retained log](results/completion-caps-logs/job-003-readouts-retraction64----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-suffix-1-faces-x-normalize-equal.log) |
| retraction512 | equal | fibred | suffix-1/x | bit-major | resource_cap | [Retained log](results/completion-caps-logs/job-004-readouts-retraction512----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-suffix-1-faces-x-normalize-equal.log) |
| prefix64 | staged | fibred | non-suffix/x | bit-major | resource_cap | [Retained log](results/completion-caps-logs/job-006-readouts-prefix64----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-non-suffix-faces-x-normalize-staged.log) |
| retraction64 | equal | fibred | non-suffix/x | bit-major | resource_cap | [Retained log](results/completion-caps-logs/job-007-readouts-retraction64----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-non-suffix-faces-x-normalize-equal.log) |
| prefix64 | equal | fibred | non-suffix/x | bit-major | resource_cap | [Retained log](results/completion-caps-logs/job-009-readouts-prefix64----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-non-suffix-faces-x-normalize-equal.log) |
| retraction64 | staged | fibred | suffix-1/x | bit-major | resource_cap | [Retained log](results/completion-caps-logs/job-011-readouts-retraction64----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-suffix-1-faces-x-normalize-staged.log) |
| retraction512 | staged | fibred | suffix-1/x | bit-major | resource_cap | [Retained log](results/completion-caps-logs/job-012-readouts-retraction512----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-suffix-1-faces-x-normalize-staged.log) |

## fibred, width 4, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 1.438 | 23.398 | 0.173 | 23.205 | 0.018 | 23.398–23.398 | 0.107 | 4553.0 | 33845 | 1 |
| prefix512/slab/staged | faces | 1.435 | 23.381 | 0.238 | 23.112 | 0.018 | 23.381–23.381 | 0.136 | 4553.0 | 33845 | 1 |
| retraction512/slab/equal | faces | 1.365 | 25.475 | 0.167 | 25.287 | 0.019 | 25.475–25.475 | 0.115 | 5443.2 | 37423 | 1 |
| retraction512/slab/staged | faces | 1.379 | 25.415 | 0.168 | 25.226 | 0.020 | 25.415–25.415 | 0.097 | 5443.2 | 37423 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 1.438 | 22.111 | 0.161 | 21.938 | 0.011 | 22.111–22.111 | 0.074 | 4561.9 | 33845 | 1 |
| prefix512/slab/staged | faces | 1.435 | 22.458 | 0.161 | 22.285 | 0.011 | 22.458–22.458 | 0.073 | 4561.9 | 33845 | 1 |
| retraction512/slab/equal | faces | 1.365 | 24.505 | 0.164 | 24.326 | 0.015 | 24.505–24.505 | 0.073 | 5452.2 | 37423 | 1 |
| retraction512/slab/staged | faces | 1.379 | 24.517 | 0.157 | 24.347 | 0.013 | 24.517–24.517 | 0.075 | 5452.2 | 37423 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, suffix-1/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix64/slab/equal | faces | 4.115 | 0.675 | 0.422 | 0.130 | 0.122 | 0.675–0.675 | 0.253 | 993.7 | 8238 | 1 |
| prefix64/slab/staged | faces | 4.096 | 0.679 | 0.424 | 0.130 | 0.123 | 0.679–0.679 | 0.251 | 993.7 | 8238 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, suffix-1/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix64/slab/equal | faces | 4.115 | 0.648 | 0.410 | 0.124 | 0.114 | 0.648–0.648 | 0.231 | 1002.7 | 8238 | 1 |
| prefix64/slab/staged | faces | 4.096 | 0.644 | 0.403 | 0.122 | 0.118 | 0.644–0.644 | 0.226 | 1002.7 | 8238 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 24.889 | 1695.139 | 1.092 | 1693.836 | 0.210 | 1695.139–1695.139 | 0.429 | 162947.4 | 1363847 | 1 |
| prefix512/slab/staged | faces | 25.016 | 1693.416 | 1.105 | 1692.098 | 0.212 | 1693.416–1693.416 | 0.433 | 162947.4 | 1363847 | 1 |
| retraction512/slab/equal | faces | 26.830 | 2249.057 | 1.556 | 2247.277 | 0.223 | 2249.057–2249.057 | 0.507 | 156759.6 | 1415883 | 1 |
| retraction512/slab/staged | faces | 27.928 | 2269.457 | 1.495 | 2267.734 | 0.224 | 2269.457–2269.457 | 0.499 | 156759.6 | 1415883 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 24.889 | 1742.380 | 1.057 | 1741.126 | 0.196 | 1742.380–1742.380 | 0.395 | 162956.4 | 1363847 | 1 |
| prefix512/slab/staged | faces | 25.016 | 1660.009 | 1.110 | 1658.702 | 0.195 | 1660.009–1660.009 | 0.387 | 162956.4 | 1363847 | 1 |
| retraction512/slab/equal | faces | 26.830 | 2334.404 | 1.436 | 2332.742 | 0.225 | 2334.404–2334.404 | 0.471 | 156768.6 | 1415883 | 1 |
| retraction512/slab/staged | faces | 27.928 | 2282.229 | 1.460 | 2280.521 | 0.248 | 2282.229–2282.229 | 0.442 | 156768.6 | 1415883 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, suffix-1/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix64/slab/equal | faces | 30.942 | 2.185 | 1.432 | 0.389 | 0.365 | 2.185–2.185 | 0.768 | 8832.8 | 85138 | 1 |
| prefix64/slab/staged | faces | 30.429 | 2.121 | 1.407 | 0.358 | 0.355 | 2.121–2.121 | 0.746 | 8832.8 | 85138 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, suffix-1/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix64/slab/equal | faces | 30.942 | 2.281 | 1.505 | 0.403 | 0.373 | 2.281–2.281 | 0.740 | 8841.8 | 85138 | 1 |
| prefix64/slab/staged | faces | 30.429 | 2.393 | 1.591 | 0.435 | 0.363 | 2.393–2.393 | 0.765 | 8841.8 | 85138 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## full, width 4, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 0.086 | 0.193 | 0.149 | 0.041 | 0.002 | 0.193–0.193 | 0.046 | 21.5 | 169 | 1 |
| prefix512/slab/staged | faces | 0.070 | 0.177 | 0.136 | 0.037 | 0.002 | 0.177–0.177 | 0.045 | 21.5 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 0.086 | 0.153 | 0.120 | 0.033 | 0.000 | 0.153–0.153 | 0.039 | 30.4 | 169 | 1 |
| prefix512/slab/staged | faces | 0.070 | 0.152 | 0.119 | 0.032 | 0.000 | 0.152–0.152 | 0.047 | 30.4 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 0.553 | 0.761 | 0.573 | 0.182 | 0.001 | 0.761–0.761 | 0.213 | 137.3 | 1030 | 1 |
| prefix512/slab/staged | faces | 0.576 | 0.686 | 0.506 | 0.179 | 0.001 | 0.686–0.686 | 0.227 | 137.3 | 1030 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 0.553 | 0.657 | 0.490 | 0.166 | 0.000 | 0.657–0.657 | 0.195 | 146.3 | 1030 | 1 |
| prefix512/slab/staged | faces | 0.576 | 0.688 | 0.510 | 0.177 | 0.000 | 0.688–0.688 | 0.193 | 146.3 | 1030 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 0.812 | 0.185 | 0.138 | 0.032 | 0.014 | 0.185–0.185 | 0.076 | 150.7 | 1024 | 1 |
| prefix512/slab/staged | faces | 0.787 | 0.174 | 0.128 | 0.032 | 0.013 | 0.174–0.174 | 0.058 | 150.7 | 1024 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 0.812 | 0.160 | 0.122 | 0.030 | 0.008 | 0.160–0.160 | 0.056 | 159.7 | 1024 | 1 |
| prefix512/slab/staged | faces | 0.787 | 0.159 | 0.120 | 0.031 | 0.008 | 0.159–0.159 | 0.047 | 159.7 | 1024 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 11.507 | 0.814 | 0.535 | 0.156 | 0.123 | 0.814–0.814 | 0.315 | 2917.2 | 17665 | 1 |
| prefix512/slab/staged | faces | 11.770 | 0.810 | 0.530 | 0.155 | 0.125 | 0.810–0.810 | 0.317 | 2917.2 | 17665 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix512/slab/equal | faces | 11.507 | 0.760 | 0.488 | 0.150 | 0.121 | 0.760–0.760 | 0.290 | 2926.2 | 17665 | 1 |
| prefix512/slab/staged | faces | 11.770 | 0.798 | 0.514 | 0.158 | 0.125 | 0.798–0.798 | 0.307 | 2926.2 | 17665 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

40 configurations; 28 matched comparisons; 24 retained processes.

- [Raw evidence](results/completion-first-smoke.json).
- [Raw evidence](results/completion-caps.json).
- [Raw evidence](results/completion-controls.json).
