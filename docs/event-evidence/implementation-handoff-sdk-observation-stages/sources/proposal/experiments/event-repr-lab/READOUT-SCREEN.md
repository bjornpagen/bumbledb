# Native information readouts after Free Join

Every query returns 64 Events: original, Possible, Guaranteed and Ambiguous
for sixteen groups. Fresh/warm totals include exact original-support counts.
Every returned Event is checked pointwise and canonically reimported outside
timing. Construction is separate. Bytes estimate retained carrier/cache memory.
They exclude allocator overhead, verification clones, result vectors and the join engine.
Executable: `9e02aec91a23ad8f7b5902ff8e959b9fbda29e2f3291560063d5a6c6cd309214`.

Only entirely successful processes contribute timing rows. Partial rows from
capped or failed processes remain in the raw evidence but cannot supply a
complete-query comparison. A later incomplete process invalidates an older
successful process for that exact job; no failed result is a fast answer.

## Incomplete processes

| Carrier | Domain | Readout | Order | Status | Log |
| --- | --- | --- | --- | --- | --- |
| retraction512 | fibred | suffix-1/x | bit-major | resource_cap | [Retained log](results/readout-smoke-logs/job-014-readouts-retraction512----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-suffix-1-faces-x.log) |
| prefix64 | fibred | non-suffix/x | bit-major | resource_cap | [Retained log](results/readout-cutoff-logs/job-002-readouts-prefix64----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-non-suffix-faces-x.log) |
| retraction64 | fibred | non-suffix/x | bit-major | resource_cap | [Retained log](results/readout-cutoff-logs/job-003-readouts-retraction64----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-non-suffix-faces-x.log) |
| retraction64 | fibred | suffix-1/x | bit-major | resource_cap | [Retained log](results/readout-cutoff-logs/job-004-readouts-retraction64----bit-major-words-identity-native-essential-derived-slab-occupancy-words-product-materialized-domain-fibred-readout-suffix-1-faces-x.log) |

## below, width 4, suffix-half/xy, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.077 | 0.068 | 0.056 | 0.010 | 0.001 | 0.068–0.068 | 0.048 | 92.9 | 160 | 1 |
| essential512/slab | joint | 0.409 | 0.677 | 0.416 | 0.260 | 0.000 | 0.677–0.677 | 0.177 | 161.9 | 1200 | 1 |
| packed512/enum | joint | 0.618 | 0.162 | 0.088 | 0.048 | 0.025 | 0.162–0.162 | 0.055 | 226.6 | 1524 | 1 |
| prefix512/slab | faces | 0.676 | 0.171 | 0.122 | 0.032 | 0.015 | 0.171–0.171 | 0.056 | 123.3 | 895 | 1 |
| retraction512/slab | faces | 0.618 | 0.671 | 0.115 | 0.544 | 0.011 | 0.671–0.671 | 0.182 | 154.2 | 1359 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 40/48.

## below, width 4, suffix-half/xy, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.077 | 0.041 | 0.031 | 0.009 | 0.001 | 0.041–0.041 | 0.017 | 101.8 | 160 | 1 |
| essential512/slab | joint | 0.409 | 0.628 | 0.386 | 0.241 | 0.000 | 0.628–0.628 | 0.176 | 170.8 | 1200 | 1 |
| packed512/enum | joint | 0.618 | 0.127 | 0.072 | 0.035 | 0.020 | 0.127–0.127 | 0.049 | 235.5 | 1524 | 1 |
| prefix512/slab | faces | 0.676 | 0.153 | 0.113 | 0.030 | 0.010 | 0.153–0.153 | 0.047 | 132.3 | 895 | 1 |
| retraction512/slab | faces | 0.618 | 0.649 | 0.115 | 0.529 | 0.005 | 0.649–0.649 | 0.177 | 163.1 | 1359 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 40/48.

## below, width 4, suffix-half/xy, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.081 | 0.065 | 0.052 | 0.010 | 0.001 | 0.065–0.065 | 0.048 | 92.9 | 160 | 1 |
| essential512/slab | joint | 0.259 | 0.540 | 0.328 | 0.210 | 0.000 | 0.540–0.540 | 0.127 | 118.3 | 986 | 1 |
| packed512/enum | joint | 0.063 | 0.117 | 0.076 | 0.029 | 0.010 | 0.117–0.117 | 0.031 | 153.8 | 906 | 1 |
| prefix512/slab | faces | 0.611 | 0.165 | 0.117 | 0.032 | 0.014 | 0.165–0.165 | 0.050 | 111.0 | 789 | 1 |
| retraction512/slab | faces | 0.560 | 0.436 | 0.117 | 0.307 | 0.010 | 0.436–0.436 | 0.136 | 142.1 | 1040 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 40/48.

## below, width 4, suffix-half/xy, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.081 | 0.040 | 0.031 | 0.008 | 0.001 | 0.040–0.040 | 0.017 | 101.8 | 160 | 1 |
| essential512/slab | joint | 0.259 | 0.521 | 0.315 | 0.205 | 0.000 | 0.521–0.521 | 0.120 | 127.2 | 986 | 1 |
| packed512/enum | joint | 0.063 | 0.086 | 0.058 | 0.019 | 0.009 | 0.086–0.086 | 0.027 | 162.8 | 906 | 1 |
| prefix512/slab | faces | 0.611 | 0.160 | 0.122 | 0.029 | 0.009 | 0.160–0.160 | 0.044 | 119.9 | 789 | 1 |
| retraction512/slab | faces | 0.560 | 0.393 | 0.107 | 0.280 | 0.006 | 0.393–0.393 | 0.128 | 151.0 | 1040 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 40/48.

## below, width 4, whole/xy, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.079 | 0.070 | 0.053 | 0.014 | 0.001 | 0.070–0.070 | 0.055 | 88.0 | 151 | 1 |
| essential512/slab | joint | 0.430 | 0.627 | 0.436 | 0.190 | 0.000 | 0.627–0.627 | 0.164 | 161.9 | 1107 | 1 |
| packed512/enum | joint | 0.596 | 0.158 | 0.096 | 0.048 | 0.013 | 0.158–0.158 | 0.044 | 226.6 | 1438 | 1 |
| prefix512/slab | faces | 0.673 | 0.180 | 0.128 | 0.044 | 0.006 | 0.180–0.180 | 0.056 | 123.3 | 886 | 1 |
| retraction512/slab | faces | 0.631 | 0.175 | 0.122 | 0.044 | 0.007 | 0.175–0.175 | 0.058 | 123.7 | 870 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## below, width 4, whole/xy, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.079 | 0.044 | 0.031 | 0.012 | 0.001 | 0.044–0.044 | 0.021 | 97.0 | 151 | 1 |
| essential512/slab | joint | 0.430 | 0.585 | 0.405 | 0.176 | 0.000 | 0.585–0.585 | 0.151 | 170.8 | 1107 | 1 |
| packed512/enum | joint | 0.596 | 0.119 | 0.078 | 0.033 | 0.007 | 0.119–0.119 | 0.039 | 235.5 | 1438 | 1 |
| prefix512/slab | faces | 0.673 | 0.150 | 0.108 | 0.040 | 0.002 | 0.150–0.150 | 0.050 | 132.3 | 886 | 1 |
| retraction512/slab | faces | 0.631 | 0.162 | 0.117 | 0.043 | 0.002 | 0.162–0.162 | 0.074 | 132.7 | 870 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## below, width 4, whole/xy, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.077 | 0.068 | 0.052 | 0.013 | 0.001 | 0.068–0.068 | 0.049 | 88.0 | 151 | 1 |
| essential512/slab | joint | 0.272 | 0.496 | 0.345 | 0.150 | 0.000 | 0.496–0.496 | 0.130 | 102.1 | 876 | 1 |
| packed512/enum | joint | 0.057 | 0.120 | 0.073 | 0.038 | 0.007 | 0.120–0.120 | 0.026 | 145.4 | 826 | 1 |
| prefix512/slab | faces | 0.633 | 0.168 | 0.118 | 0.041 | 0.007 | 0.168–0.168 | 0.055 | 111.0 | 780 | 1 |
| retraction512/slab | faces | 0.576 | 0.162 | 0.113 | 0.042 | 0.006 | 0.162–0.162 | 0.054 | 111.6 | 770 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## below, width 4, whole/xy, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.077 | 0.042 | 0.029 | 0.011 | 0.001 | 0.042–0.042 | 0.020 | 97.0 | 151 | 1 |
| essential512/slab | joint | 0.272 | 0.459 | 0.322 | 0.137 | 0.000 | 0.459–0.459 | 0.124 | 111.1 | 876 | 1 |
| packed512/enum | joint | 0.057 | 0.074 | 0.053 | 0.017 | 0.004 | 0.074–0.074 | 0.022 | 154.4 | 826 | 1 |
| prefix512/slab | faces | 0.633 | 0.149 | 0.108 | 0.040 | 0.002 | 0.149–0.149 | 0.049 | 119.9 | 780 | 1 |
| retraction512/slab | faces | 0.576 | 0.145 | 0.104 | 0.039 | 0.002 | 0.145–0.145 | 0.073 | 120.6 | 770 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## below, width 6, suffix-half/xy, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 6.105 | 2.148 | 1.609 | 0.506 | 0.033 | 2.148–2.148 | 1.933 | 6236.7 | 188 | 1 |
| essential512/slab | joint | 2.114 | 2.758 | 1.866 | 0.892 | 0.000 | 2.758–2.758 | 0.743 | 816.9 | 6299 | 1 |
| packed512/enum | joint | 30.381 | 0.547 | 0.370 | 0.098 | 0.079 | 0.547–0.547 | 0.203 | 999.5 | 6699 | 1 |
| prefix512/slab | faces | 5.361 | 0.755 | 0.538 | 0.154 | 0.063 | 0.755–0.755 | 0.261 | 1195.3 | 7299 | 1 |
| retraction512/slab | faces | 6.257 | 2.307 | 0.575 | 1.686 | 0.045 | 2.307–2.307 | 0.786 | 1363.2 | 9867 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## below, width 6, suffix-half/xy, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 6.105 | 1.146 | 0.571 | 0.543 | 0.030 | 1.146–1.146 | 0.576 | 6245.7 | 188 | 1 |
| essential512/slab | joint | 2.114 | 2.846 | 1.924 | 0.921 | 0.000 | 2.846–2.846 | 0.771 | 825.9 | 6299 | 1 |
| packed512/enum | joint | 30.381 | 0.496 | 0.336 | 0.084 | 0.075 | 0.496–0.496 | 0.179 | 1008.5 | 6699 | 1 |
| prefix512/slab | faces | 5.361 | 0.741 | 0.519 | 0.162 | 0.059 | 0.741–0.741 | 0.239 | 1204.2 | 7299 | 1 |
| retraction512/slab | faces | 6.257 | 2.357 | 0.580 | 1.726 | 0.050 | 2.357–2.357 | 0.789 | 1372.1 | 9867 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## below, width 6, suffix-half/xy, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 5.940 | 2.255 | 1.622 | 0.594 | 0.034 | 2.255–2.255 | 2.150 | 6236.7 | 188 | 1 |
| essential512/slab | joint | 0.924 | 1.360 | 0.982 | 0.378 | 0.000 | 1.360–1.360 | 0.324 | 298.2 | 2809 | 1 |
| packed512/enum | joint | 0.445 | 0.227 | 0.158 | 0.039 | 0.030 | 0.227–0.227 | 0.085 | 368.5 | 2634 | 1 |
| prefix512/slab | faces | 4.151 | 0.852 | 0.586 | 0.204 | 0.062 | 0.852–0.852 | 0.305 | 855.3 | 6772 | 1 |
| retraction512/slab | faces | 4.605 | 1.291 | 0.748 | 0.509 | 0.033 | 1.291–1.291 | 0.350 | 1182.1 | 8067 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## below, width 6, suffix-half/xy, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 5.940 | 1.038 | 0.477 | 0.529 | 0.030 | 1.038–1.038 | 0.567 | 6245.7 | 188 | 1 |
| essential512/slab | joint | 0.924 | 1.461 | 1.058 | 0.401 | 0.000 | 1.461–1.461 | 0.314 | 307.2 | 2809 | 1 |
| packed512/enum | joint | 0.445 | 0.222 | 0.158 | 0.035 | 0.028 | 0.222–0.222 | 0.079 | 377.5 | 2634 | 1 |
| prefix512/slab | faces | 4.151 | 0.889 | 0.623 | 0.201 | 0.064 | 0.889–0.889 | 0.278 | 864.2 | 6772 | 1 |
| retraction512/slab | faces | 4.605 | 1.248 | 0.729 | 0.487 | 0.031 | 1.248–1.248 | 0.354 | 1191.0 | 8067 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## below, width 6, whole/xy, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 5.889 | 2.481 | 1.726 | 0.725 | 0.030 | 2.481–2.481 | 2.133 | 6171.1 | 186 | 1 |
| essential512/slab | joint | 2.067 | 2.651 | 1.875 | 0.775 | 0.000 | 2.651–2.651 | 0.635 | 816.9 | 6234 | 1 |
| packed512/enum | joint | 30.828 | 0.544 | 0.431 | 0.086 | 0.027 | 0.544–0.544 | 0.132 | 999.5 | 6639 | 1 |
| prefix512/slab | faces | 5.375 | 0.714 | 0.540 | 0.151 | 0.022 | 0.714–0.714 | 0.201 | 1195.3 | 7290 | 1 |
| retraction512/slab | faces | 5.936 | 0.877 | 0.609 | 0.241 | 0.027 | 0.877–0.877 | 0.337 | 1332.7 | 8582 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## below, width 6, whole/xy, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 5.889 | 1.161 | 0.444 | 0.685 | 0.031 | 1.161–1.161 | 0.752 | 6180.1 | 186 | 1 |
| essential512/slab | joint | 2.067 | 2.606 | 1.859 | 0.746 | 0.000 | 2.606–2.606 | 0.672 | 825.9 | 6234 | 1 |
| packed512/enum | joint | 30.828 | 0.445 | 0.340 | 0.081 | 0.023 | 0.445–0.445 | 0.117 | 1008.5 | 6639 | 1 |
| prefix512/slab | faces | 5.375 | 0.804 | 0.635 | 0.150 | 0.019 | 0.804–0.804 | 0.182 | 1204.2 | 7290 | 1 |
| retraction512/slab | faces | 5.936 | 0.852 | 0.586 | 0.240 | 0.025 | 0.852–0.852 | 0.297 | 1341.6 | 8582 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## below, width 6, whole/xy, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 6.185 | 2.466 | 1.681 | 0.748 | 0.031 | 2.466–2.466 | 2.262 | 6171.1 | 186 | 1 |
| essential512/slab | joint | 0.929 | 1.288 | 0.963 | 0.325 | 0.000 | 1.288–1.288 | 0.305 | 298.2 | 2758 | 1 |
| packed512/enum | joint | 0.445 | 0.212 | 0.158 | 0.040 | 0.014 | 0.212–0.212 | 0.071 | 368.5 | 2588 | 1 |
| prefix512/slab | faces | 4.243 | 0.915 | 0.617 | 0.271 | 0.026 | 0.915–0.915 | 0.267 | 855.3 | 6761 | 1 |
| retraction512/slab | faces | 4.517 | 0.956 | 0.724 | 0.205 | 0.026 | 0.956–0.956 | 0.265 | 1182.1 | 7668 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## below, width 6, whole/xy, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 6.185 | 1.251 | 0.488 | 0.728 | 0.035 | 1.251–1.251 | 0.751 | 6180.1 | 186 | 1 |
| essential512/slab | joint | 0.929 | 1.365 | 1.026 | 0.338 | 0.001 | 1.365–1.365 | 0.322 | 307.2 | 2758 | 1 |
| packed512/enum | joint | 0.445 | 0.211 | 0.165 | 0.033 | 0.012 | 0.211–0.211 | 0.062 | 377.5 | 2588 | 1 |
| prefix512/slab | faces | 4.243 | 0.798 | 0.584 | 0.195 | 0.018 | 0.798–0.798 | 0.232 | 864.2 | 6761 | 1 |
| retraction512/slab | faces | 4.517 | 0.968 | 0.745 | 0.201 | 0.022 | 0.968–0.968 | 0.238 | 1191.0 | 7668 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## fibred, width 4, non-suffix/x, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 1.274 | 0.096 | 0.079 | 0.014 | 0.001 | 0.096–0.096 | 0.075 | 305.2 | 277 | 1 |
| essential512/slab | joint | 1.925 | 1.468 | 0.973 | 0.493 | 0.000 | 1.468–1.468 | 0.069 | 456.2 | 4103 | 1 |
| essential64/slab | joint | 4.266 | 3.091 | 2.010 | 1.079 | 0.000 | 3.091–3.091 | 0.116 | 1040.5 | 8722 | 1 |
| packed512/enum | joint | 1.369 | 0.323 | 0.184 | 0.094 | 0.044 | 0.323–0.323 | 0.085 | 629.8 | 4401 | 1 |
| prefix512/slab | faces | 1.396 | 22.800 | 0.170 | 22.613 | 0.016 | 22.800–22.800 | 0.093 | 4553.0 | 33845 | 1 |
| retraction512/slab | faces | 1.374 | 25.194 | 0.195 | 24.974 | 0.023 | 25.194–25.194 | 0.125 | 5443.2 | 37423 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 1.274 | 0.045 | 0.033 | 0.011 | 0.001 | 0.045–0.045 | 0.019 | 314.1 | 277 | 1 |
| essential512/slab | joint | 1.925 | 1.437 | 0.956 | 0.480 | 0.000 | 1.437–1.437 | 0.066 | 465.1 | 4103 | 1 |
| essential64/slab | joint | 4.266 | 3.078 | 2.002 | 1.075 | 0.000 | 3.078–3.078 | 0.074 | 1049.4 | 8722 | 1 |
| packed512/enum | joint | 1.369 | 0.257 | 0.152 | 0.070 | 0.034 | 0.257–0.257 | 0.075 | 638.7 | 4401 | 1 |
| prefix512/slab | faces | 1.396 | 22.319 | 0.162 | 22.145 | 0.011 | 22.319–22.319 | 0.076 | 4561.9 | 33845 | 1 |
| retraction512/slab | faces | 1.374 | 24.390 | 0.163 | 24.213 | 0.013 | 24.390–24.390 | 0.081 | 5452.2 | 37423 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 1.293 | 0.098 | 0.081 | 0.015 | 0.001 | 0.098–0.098 | 0.080 | 305.2 | 277 | 1 |
| essential512/slab | joint | 1.267 | 1.586 | 0.989 | 0.594 | 0.000 | 1.586–1.586 | 0.227 | 583.7 | 4209 | 1 |
| essential64/slab | joint | 1.094 | 1.344 | 0.821 | 0.522 | 0.000 | 1.344–1.344 | 0.271 | 533.8 | 5580 | 1 |
| packed512/enum | joint | 0.331 | 0.285 | 0.189 | 0.057 | 0.037 | 0.285–0.285 | 0.066 | 638.9 | 3967 | 1 |
| prefix512/slab | faces | 1.338 | 2.558 | 0.167 | 2.373 | 0.016 | 2.558–2.558 | 0.240 | 684.1 | 5407 | 1 |
| prefix64/slab | faces | 2.417 | 3.037 | 0.446 | 2.505 | 0.085 | 3.037–3.037 | 0.327 | 1105.1 | 11380 | 1 |
| retraction512/slab | faces | 1.286 | 2.650 | 0.168 | 2.462 | 0.018 | 2.650–2.650 | 0.260 | 687.0 | 5648 | 1 |
| retraction64/slab | faces | 2.656 | 3.459 | 0.553 | 2.813 | 0.091 | 3.459–3.459 | 0.346 | 1404.1 | 12278 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 1.293 | 0.052 | 0.040 | 0.011 | 0.001 | 0.052–0.052 | 0.020 | 314.1 | 277 | 1 |
| essential512/slab | joint | 1.267 | 1.524 | 0.936 | 0.588 | 0.000 | 1.524–1.524 | 0.211 | 592.6 | 4209 | 1 |
| essential64/slab | joint | 1.094 | 1.317 | 0.802 | 0.515 | 0.000 | 1.317–1.317 | 0.229 | 542.8 | 5580 | 1 |
| packed512/enum | joint | 0.331 | 0.219 | 0.148 | 0.041 | 0.030 | 0.219–0.219 | 0.059 | 647.8 | 3967 | 1 |
| prefix512/slab | faces | 1.338 | 2.448 | 0.152 | 2.278 | 0.018 | 2.448–2.448 | 0.231 | 693.0 | 5407 | 1 |
| prefix64/slab | faces | 2.417 | 2.830 | 0.414 | 2.337 | 0.078 | 2.830–2.830 | 0.324 | 1114.1 | 11380 | 1 |
| retraction512/slab | faces | 1.286 | 2.679 | 0.177 | 2.486 | 0.012 | 2.679–2.679 | 0.239 | 696.0 | 5648 | 1 |
| retraction64/slab | faces | 2.656 | 3.288 | 0.508 | 2.698 | 0.082 | 3.288–3.288 | 0.305 | 1413.1 | 12278 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, suffix-1/x, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 1.321 | 0.101 | 0.081 | 0.017 | 0.001 | 0.101–0.101 | 0.079 | 305.2 | 277 | 1 |
| essential512/slab | joint | 1.944 | 1.819 | 1.012 | 0.806 | 0.000 | 1.819–1.819 | 0.346 | 668.5 | 4419 | 1 |
| essential64/slab | joint | 4.302 | 3.657 | 1.879 | 1.776 | 0.000 | 3.657–3.657 | 0.976 | 1323.1 | 9588 | 1 |
| packed512/enum | joint | 1.413 | 0.338 | 0.183 | 0.104 | 0.049 | 0.338–0.338 | 0.098 | 629.8 | 4739 | 1 |
| prefix512/slab | faces | 3.722 | 0.496 | 0.400 | 0.055 | 0.038 | 0.496–0.496 | 0.062 | 285.2 | 1723 | 1 |
| prefix64/slab | faces | 4.181 | 0.684 | 0.433 | 0.131 | 0.119 | 0.684–0.684 | 0.258 | 993.7 | 8238 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, suffix-1/x, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 1.321 | 0.049 | 0.036 | 0.011 | 0.001 | 0.049–0.049 | 0.020 | 314.1 | 277 | 1 |
| essential512/slab | joint | 1.944 | 1.738 | 0.950 | 0.787 | 0.000 | 1.738–1.738 | 0.305 | 677.4 | 4419 | 1 |
| essential64/slab | joint | 4.302 | 3.603 | 1.887 | 1.716 | 0.000 | 3.603–3.603 | 0.954 | 1332.1 | 9588 | 1 |
| packed512/enum | joint | 1.413 | 0.270 | 0.150 | 0.079 | 0.040 | 0.270–0.270 | 0.095 | 638.7 | 4739 | 1 |
| prefix512/slab | faces | 3.722 | 0.201 | 0.164 | 0.021 | 0.016 | 0.201–0.201 | 0.053 | 294.2 | 1723 | 1 |
| prefix64/slab | faces | 4.181 | 0.691 | 0.425 | 0.138 | 0.128 | 0.691–0.691 | 0.252 | 1002.7 | 8238 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, suffix-1/x, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 1.292 | 0.105 | 0.083 | 0.015 | 0.001 | 0.105–0.105 | 0.077 | 305.2 | 277 | 1 |
| essential512/slab | joint | 1.222 | 1.594 | 0.980 | 0.613 | 0.000 | 1.594–1.594 | 0.244 | 583.7 | 4308 | 1 |
| essential64/slab | joint | 1.053 | 1.358 | 0.804 | 0.552 | 0.000 | 1.358–1.358 | 0.238 | 533.8 | 5730 | 1 |
| packed512/enum | joint | 0.318 | 0.272 | 0.177 | 0.058 | 0.036 | 0.272–0.272 | 0.063 | 638.9 | 3998 | 1 |
| prefix512/slab | faces | 1.380 | 0.216 | 0.173 | 0.022 | 0.020 | 0.216–0.216 | 0.067 | 281.2 | 1653 | 1 |
| prefix64/slab | faces | 2.409 | 0.705 | 0.438 | 0.150 | 0.116 | 0.705–0.705 | 0.255 | 569.1 | 5416 | 1 |
| retraction512/slab | faces | 1.296 | 2.766 | 0.169 | 2.579 | 0.017 | 2.766–2.766 | 0.262 | 687.0 | 5741 | 1 |
| retraction64/slab | faces | 2.693 | 3.561 | 0.518 | 2.936 | 0.106 | 3.561–3.561 | 0.366 | 1404.1 | 12103 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, suffix-1/x, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 1.292 | 0.046 | 0.034 | 0.011 | 0.001 | 0.046–0.046 | 0.019 | 314.1 | 277 | 1 |
| essential512/slab | joint | 1.222 | 1.637 | 1.003 | 0.630 | 0.000 | 1.637–1.637 | 0.256 | 592.6 | 4308 | 1 |
| essential64/slab | joint | 1.053 | 1.395 | 0.845 | 0.550 | 0.000 | 1.395–1.395 | 0.207 | 542.8 | 5730 | 1 |
| packed512/enum | joint | 0.318 | 0.279 | 0.191 | 0.053 | 0.031 | 0.279–0.279 | 0.066 | 647.8 | 3998 | 1 |
| prefix512/slab | faces | 1.380 | 0.185 | 0.150 | 0.020 | 0.015 | 0.185–0.185 | 0.037 | 290.2 | 1653 | 1 |
| prefix64/slab | faces | 2.409 | 0.689 | 0.426 | 0.152 | 0.111 | 0.689–0.689 | 0.240 | 578.1 | 5416 | 1 |
| retraction512/slab | faces | 1.296 | 2.867 | 0.175 | 2.679 | 0.012 | 2.867–2.867 | 0.265 | 696.0 | 5741 | 1 |
| retraction64/slab | faces | 2.693 | 3.233 | 0.498 | 2.653 | 0.082 | 3.233–3.233 | 0.289 | 1413.1 | 12103 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 469.039 | 4.094 | 3.373 | 0.658 | 0.062 | 4.094–4.094 | 4.122 | 41852.6 | 636 | 1 |
| essential512/slab | joint | 28.058 | 8.204 | 5.755 | 2.449 | 0.000 | 8.204–8.204 | 0.135 | 5722.5 | 48923 | 1 |
| essential64/slab | joint | 26.027 | 12.211 | 8.497 | 3.714 | 0.000 | 12.211–12.211 | 0.151 | 8462.3 | 62232 | 1 |
| packed512/enum | joint | 65.622 | 1.464 | 0.673 | 0.564 | 0.226 | 1.464–1.464 | 0.773 | 6651.9 | 47190 | 1 |
| prefix512/slab | faces | 24.669 | 1753.274 | 1.095 | 1751.968 | 0.209 | 1753.274–1753.274 | 0.466 | 162947.4 | 1363847 | 1 |
| retraction512/slab | faces | 28.844 | 2490.009 | 1.531 | 2488.248 | 0.229 | 2490.009–2490.009 | 0.499 | 156759.6 | 1415883 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 469.039 | 1.453 | 0.902 | 0.484 | 0.064 | 1.453–1.453 | 0.528 | 41861.5 | 636 | 1 |
| essential512/slab | joint | 28.058 | 8.637 | 6.023 | 2.614 | 0.000 | 8.637–8.637 | 0.127 | 5731.5 | 48923 | 1 |
| essential64/slab | joint | 26.027 | 11.549 | 8.000 | 3.548 | 0.000 | 11.549–11.549 | 0.127 | 8471.3 | 62232 | 1 |
| packed512/enum | joint | 65.622 | 1.784 | 0.757 | 0.767 | 0.258 | 1.784–1.784 | 0.757 | 6660.8 | 47190 | 1 |
| prefix512/slab | faces | 24.669 | 1751.653 | 1.028 | 1750.418 | 0.207 | 1751.653–1751.653 | 0.388 | 162956.4 | 1363847 | 1 |
| retraction512/slab | faces | 28.844 | 2343.619 | 1.382 | 2342.014 | 0.223 | 2343.619–2343.619 | 0.464 | 156768.6 | 1415883 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 464.394 | 4.100 | 3.365 | 0.669 | 0.064 | 4.100–4.100 | 3.840 | 41852.6 | 636 | 1 |
| essential512/slab | joint | 5.978 | 3.520 | 2.348 | 1.172 | 0.000 | 3.520–3.520 | 0.588 | 1840.6 | 18642 | 1 |
| essential64/slab | joint | 6.020 | 5.009 | 2.819 | 2.189 | 0.000 | 5.009–5.009 | 1.177 | 2325.7 | 23355 | 1 |
| packed512/enum | joint | 2.146 | 0.633 | 0.449 | 0.107 | 0.077 | 0.633–0.633 | 0.223 | 2402.4 | 18095 | 1 |
| prefix512/slab | faces | 18.529 | 14.664 | 1.261 | 13.265 | 0.138 | 14.664–14.664 | 0.802 | 7010.0 | 59034 | 1 |
| prefix64/slab | faces | 11.948 | 13.311 | 1.780 | 11.120 | 0.405 | 13.311–13.311 | 1.690 | 5968.4 | 67201 | 1 |
| retraction512/slab | faces | 21.236 | 16.827 | 1.507 | 15.146 | 0.173 | 16.827–16.827 | 0.825 | 7419.8 | 65592 | 1 |
| retraction64/slab | faces | 13.339 | 13.885 | 2.084 | 11.347 | 0.453 | 13.885–13.885 | 1.747 | 6341.0 | 77091 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 464.394 | 1.519 | 0.963 | 0.488 | 0.065 | 1.519–1.519 | 0.570 | 41861.5 | 636 | 1 |
| essential512/slab | joint | 5.978 | 3.403 | 2.369 | 1.033 | 0.000 | 3.403–3.403 | 0.544 | 1849.6 | 18642 | 1 |
| essential64/slab | joint | 6.020 | 5.129 | 2.825 | 2.302 | 0.000 | 5.129–5.129 | 1.133 | 2334.6 | 23355 | 1 |
| packed512/enum | joint | 2.146 | 0.567 | 0.386 | 0.105 | 0.075 | 0.567–0.567 | 0.199 | 2411.3 | 18095 | 1 |
| prefix512/slab | faces | 18.529 | 14.464 | 1.244 | 13.079 | 0.140 | 14.464–14.464 | 0.739 | 7018.9 | 59034 | 1 |
| prefix64/slab | faces | 11.948 | 11.755 | 1.773 | 9.581 | 0.399 | 11.755–11.755 | 1.669 | 5977.4 | 67201 | 1 |
| retraction512/slab | faces | 21.236 | 21.001 | 1.569 | 19.253 | 0.179 | 21.001–21.001 | 0.807 | 7428.8 | 65592 | 1 |
| retraction64/slab | faces | 13.339 | 14.939 | 2.121 | 12.356 | 0.458 | 14.939–14.939 | 1.691 | 6349.9 | 77091 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, suffix-1/x, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 545.673 | 7.440 | 3.411 | 3.957 | 0.071 | 7.440–7.440 | 3.936 | 41852.6 | 636 | 1 |
| essential512/slab | joint | 28.254 | 11.216 | 5.739 | 5.477 | 0.000 | 11.216–11.216 | 3.551 | 5722.5 | 51675 | 1 |
| essential64/slab | joint | 24.981 | 15.030 | 8.483 | 6.546 | 0.000 | 15.030–15.030 | 4.098 | 8462.3 | 66445 | 1 |
| packed512/enum | joint | 66.659 | 1.647 | 0.723 | 0.565 | 0.358 | 1.647–1.647 | 0.987 | 6651.9 | 49763 | 1 |
| prefix512/slab | faces | 110.162 | 29.117 | 28.664 | 0.294 | 0.159 | 29.117–29.117 | 0.449 | 6126.3 | 39709 | 1 |
| prefix64/slab | faces | 30.476 | 2.129 | 1.415 | 0.358 | 0.356 | 2.129–2.129 | 0.748 | 8832.8 | 85138 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, suffix-1/x, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 545.673 | 1.521 | 0.941 | 0.504 | 0.071 | 1.521–1.521 | 0.576 | 41861.5 | 636 | 1 |
| essential512/slab | joint | 28.254 | 12.473 | 5.947 | 6.524 | 0.001 | 12.473–12.473 | 3.523 | 5731.5 | 51675 | 1 |
| essential64/slab | joint | 24.981 | 15.061 | 8.316 | 6.744 | 0.000 | 15.061–15.061 | 4.324 | 8471.3 | 66445 | 1 |
| packed512/enum | joint | 66.659 | 1.603 | 0.713 | 0.546 | 0.344 | 1.603–1.603 | 0.902 | 6660.8 | 49763 | 1 |
| prefix512/slab | faces | 110.162 | 1.500 | 1.084 | 0.264 | 0.152 | 1.500–1.500 | 0.425 | 6135.2 | 39709 | 1 |
| prefix64/slab | faces | 30.476 | 2.080 | 1.361 | 0.361 | 0.357 | 2.080–2.080 | 0.749 | 8841.8 | 85138 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, suffix-1/x, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 459.209 | 3.995 | 3.276 | 0.656 | 0.063 | 3.995–3.995 | 3.620 | 41852.6 | 636 | 1 |
| essential512/slab | joint | 6.197 | 3.579 | 2.379 | 1.200 | 0.000 | 3.579–3.579 | 0.541 | 1840.6 | 19306 | 1 |
| essential64/slab | joint | 6.055 | 5.166 | 2.866 | 2.299 | 0.000 | 5.166–5.166 | 0.996 | 2325.7 | 24354 | 1 |
| packed512/enum | joint | 2.145 | 0.626 | 0.429 | 0.114 | 0.083 | 0.626–0.626 | 0.253 | 2402.4 | 18524 | 1 |
| prefix512/slab | faces | 18.616 | 1.775 | 1.269 | 0.319 | 0.186 | 1.775–1.775 | 0.524 | 4922.3 | 33188 | 1 |
| prefix64/slab | faces | 11.261 | 3.133 | 1.766 | 0.856 | 0.511 | 3.133–3.133 | 1.449 | 4002.6 | 38061 | 1 |
| retraction512/slab | faces | 21.953 | 19.886 | 1.602 | 18.106 | 0.177 | 19.886–19.886 | 0.796 | 7419.8 | 67669 | 1 |
| retraction64/slab | faces | 13.454 | 14.825 | 2.058 | 12.268 | 0.499 | 14.825–14.825 | 1.727 | 6341.0 | 78580 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, suffix-1/x, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 459.209 | 1.971 | 1.344 | 0.566 | 0.061 | 1.971–1.971 | 0.515 | 41861.5 | 636 | 1 |
| essential512/slab | joint | 6.197 | 3.424 | 2.253 | 1.171 | 0.000 | 3.424–3.424 | 0.517 | 1849.6 | 19306 | 1 |
| essential64/slab | joint | 6.055 | 5.486 | 3.124 | 2.361 | 0.000 | 5.486–5.486 | 1.104 | 2334.6 | 24354 | 1 |
| packed512/enum | joint | 2.145 | 0.574 | 0.378 | 0.112 | 0.084 | 0.574–0.574 | 0.223 | 2411.3 | 18524 | 1 |
| prefix512/slab | faces | 18.616 | 1.897 | 1.338 | 0.346 | 0.209 | 1.897–1.897 | 0.553 | 4931.2 | 33188 | 1 |
| prefix64/slab | faces | 11.261 | 3.217 | 1.795 | 0.903 | 0.518 | 3.217–3.217 | 1.388 | 4011.6 | 38061 | 1 |
| retraction512/slab | faces | 21.953 | 18.025 | 1.466 | 16.390 | 0.169 | 18.025–18.025 | 0.909 | 7428.8 | 67669 | 1 |
| retraction64/slab | faces | 13.454 | 14.988 | 2.062 | 12.419 | 0.506 | 14.988–14.988 | 1.667 | 6349.9 | 78580 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## full, width 4, non-suffix/x, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.023 | 0.066 | 0.056 | 0.008 | 0.001 | 0.066–0.066 | 0.046 | 93.5 | 161 | 1 |
| essential512/slab | joint | 0.065 | 0.157 | 0.133 | 0.023 | 0.000 | 0.157–0.157 | 0.034 | 22.5 | 161 | 1 |
| packed512/enum | joint | 0.535 | 0.099 | 0.075 | 0.014 | 0.009 | 0.099–0.099 | 0.027 | 105.0 | 591 | 1 |
| prefix512/slab | faces | 0.067 | 0.165 | 0.139 | 0.023 | 0.002 | 0.165–0.165 | 0.032 | 21.5 | 173 | 1 |
| retraction512/slab | faces | 0.066 | 0.165 | 0.137 | 0.024 | 0.003 | 0.165–0.165 | 0.031 | 21.5 | 173 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, non-suffix/x, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.023 | 0.040 | 0.032 | 0.006 | 0.001 | 0.040–0.040 | 0.015 | 102.5 | 161 | 1 |
| essential512/slab | joint | 0.065 | 0.138 | 0.118 | 0.020 | 0.000 | 0.138–0.138 | 0.027 | 31.5 | 161 | 1 |
| packed512/enum | joint | 0.535 | 0.076 | 0.059 | 0.010 | 0.006 | 0.076–0.076 | 0.021 | 114.0 | 591 | 1 |
| prefix512/slab | faces | 0.067 | 0.138 | 0.119 | 0.019 | 0.000 | 0.138–0.138 | 0.027 | 30.4 | 173 | 1 |
| retraction512/slab | faces | 0.066 | 0.141 | 0.121 | 0.019 | 0.000 | 0.141–0.141 | 0.028 | 30.4 | 173 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, non-suffix/x, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.023 | 0.065 | 0.054 | 0.008 | 0.001 | 0.065–0.065 | 0.046 | 93.5 | 161 | 1 |
| essential512/slab | joint | 0.066 | 0.164 | 0.140 | 0.022 | 0.000 | 0.164–0.164 | 0.047 | 22.5 | 161 | 1 |
| packed512/enum | joint | 0.020 | 0.129 | 0.108 | 0.006 | 0.004 | 0.129–0.129 | 0.015 | 36.2 | 161 | 1 |
| prefix512/slab | faces | 0.064 | 0.158 | 0.133 | 0.022 | 0.002 | 0.158–0.158 | 0.032 | 21.5 | 173 | 1 |
| retraction512/slab | faces | 0.078 | 0.164 | 0.135 | 0.025 | 0.003 | 0.164–0.164 | 0.032 | 21.5 | 173 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, non-suffix/x, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.023 | 0.039 | 0.032 | 0.006 | 0.001 | 0.039–0.039 | 0.015 | 102.5 | 161 | 1 |
| essential512/slab | joint | 0.066 | 0.137 | 0.118 | 0.019 | 0.000 | 0.137–0.137 | 0.048 | 31.5 | 161 | 1 |
| packed512/enum | joint | 0.020 | 0.030 | 0.025 | 0.003 | 0.002 | 0.030–0.030 | 0.022 | 45.1 | 161 | 1 |
| prefix512/slab | faces | 0.064 | 0.140 | 0.120 | 0.019 | 0.000 | 0.140–0.140 | 0.033 | 30.4 | 173 | 1 |
| retraction512/slab | faces | 0.078 | 0.152 | 0.132 | 0.020 | 0.000 | 0.152–0.152 | 0.029 | 30.4 | 173 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, suffix-1/x, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.024 | 0.070 | 0.056 | 0.011 | 0.001 | 0.070–0.070 | 0.072 | 102.1 | 177 | 1 |
| essential512/slab | joint | 0.085 | 0.173 | 0.150 | 0.021 | 0.000 | 0.173–0.173 | 0.027 | 22.5 | 177 | 1 |
| packed512/enum | joint | 0.535 | 0.101 | 0.070 | 0.017 | 0.013 | 0.101–0.101 | 0.030 | 105.0 | 693 | 1 |
| prefix512/slab | faces | 0.063 | 0.152 | 0.130 | 0.020 | 0.002 | 0.152–0.152 | 0.024 | 21.5 | 189 | 1 |
| retraction512/slab | faces | 0.070 | 0.155 | 0.131 | 0.020 | 0.002 | 0.155–0.155 | 0.025 | 21.5 | 189 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## full, width 4, suffix-1/x, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.024 | 0.049 | 0.040 | 0.008 | 0.001 | 0.049–0.049 | 0.016 | 111.1 | 177 | 1 |
| essential512/slab | joint | 0.085 | 0.144 | 0.124 | 0.019 | 0.000 | 0.144–0.144 | 0.024 | 31.5 | 177 | 1 |
| packed512/enum | joint | 0.535 | 0.072 | 0.053 | 0.010 | 0.009 | 0.072–0.072 | 0.025 | 114.0 | 693 | 1 |
| prefix512/slab | faces | 0.063 | 0.132 | 0.114 | 0.017 | 0.000 | 0.132–0.132 | 0.018 | 30.4 | 189 | 1 |
| retraction512/slab | faces | 0.070 | 0.136 | 0.118 | 0.017 | 0.000 | 0.136–0.136 | 0.019 | 30.4 | 189 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## full, width 4, suffix-1/x, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.029 | 0.096 | 0.084 | 0.009 | 0.001 | 0.096–0.096 | 0.046 | 102.1 | 177 | 1 |
| essential512/slab | joint | 0.065 | 0.154 | 0.132 | 0.020 | 0.000 | 0.154–0.154 | 0.026 | 22.5 | 177 | 1 |
| packed512/enum | joint | 0.020 | 0.050 | 0.037 | 0.008 | 0.004 | 0.050–0.050 | 0.017 | 36.2 | 177 | 1 |
| prefix512/slab | faces | 0.075 | 0.172 | 0.148 | 0.021 | 0.002 | 0.172–0.172 | 0.023 | 21.5 | 189 | 1 |
| retraction512/slab | faces | 0.066 | 0.178 | 0.150 | 0.023 | 0.003 | 0.178–0.178 | 0.024 | 21.5 | 189 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## full, width 4, suffix-1/x, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.029 | 0.038 | 0.031 | 0.006 | 0.001 | 0.038–0.038 | 0.015 | 111.1 | 177 | 1 |
| essential512/slab | joint | 0.065 | 0.135 | 0.117 | 0.017 | 0.000 | 0.135–0.135 | 0.035 | 31.5 | 177 | 1 |
| packed512/enum | joint | 0.020 | 0.034 | 0.027 | 0.004 | 0.003 | 0.034–0.034 | 0.014 | 45.1 | 177 | 1 |
| prefix512/slab | faces | 0.075 | 0.130 | 0.113 | 0.016 | 0.000 | 0.130–0.130 | 0.018 | 30.4 | 189 | 1 |
| retraction512/slab | faces | 0.066 | 0.134 | 0.117 | 0.017 | 0.000 | 0.134–0.134 | 0.019 | 30.4 | 189 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## full, width 6, non-suffix/x, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.705 | 1.838 | 1.575 | 0.231 | 0.032 | 1.838–1.838 | 1.682 | 5580.8 | 168 | 1 |
| essential512/slab | joint | 0.538 | 0.498 | 0.491 | 0.006 | 0.000 | 0.498–0.498 | 0.029 | 132.0 | 1013 | 1 |
| packed512/enum | joint | 30.277 | 0.246 | 0.180 | 0.028 | 0.035 | 0.246–0.246 | 0.103 | 367.9 | 2285 | 1 |
| prefix512/slab | faces | 0.547 | 0.502 | 0.495 | 0.007 | 0.000 | 0.502–0.502 | 0.070 | 137.3 | 1030 | 1 |
| retraction512/slab | faces | 0.601 | 0.504 | 0.496 | 0.007 | 0.001 | 0.504–0.504 | 0.027 | 137.3 | 1030 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, non-suffix/x, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.705 | 0.702 | 0.444 | 0.226 | 0.031 | 0.702–0.702 | 0.279 | 5589.8 | 168 | 1 |
| essential512/slab | joint | 0.538 | 0.515 | 0.508 | 0.006 | 0.000 | 0.515–0.515 | 0.030 | 141.0 | 1013 | 1 |
| packed512/enum | joint | 30.277 | 0.241 | 0.190 | 0.021 | 0.029 | 0.241–0.241 | 0.082 | 376.8 | 2285 | 1 |
| prefix512/slab | faces | 0.547 | 1.225 | 1.208 | 0.014 | 0.001 | 1.225–1.225 | 0.028 | 146.3 | 1030 | 1 |
| retraction512/slab | faces | 0.601 | 0.497 | 0.491 | 0.005 | 0.000 | 0.497–0.497 | 0.024 | 146.3 | 1030 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, non-suffix/x, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.679 | 1.843 | 1.584 | 0.228 | 0.031 | 1.843–1.843 | 1.730 | 5580.8 | 168 | 1 |
| essential512/slab | joint | 0.545 | 0.812 | 0.659 | 0.148 | 0.000 | 0.812–0.812 | 0.156 | 161.4 | 1308 | 1 |
| packed512/enum | joint | 0.380 | 0.136 | 0.094 | 0.022 | 0.019 | 0.136–0.136 | 0.057 | 215.9 | 1308 | 1 |
| prefix512/slab | faces | 0.585 | 0.786 | 0.643 | 0.138 | 0.004 | 0.786–0.786 | 0.153 | 166.8 | 1324 | 1 |
| retraction512/slab | faces | 0.554 | 0.802 | 0.661 | 0.140 | 0.001 | 0.802–0.802 | 0.155 | 166.8 | 1324 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, non-suffix/x, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.679 | 0.754 | 0.494 | 0.225 | 0.036 | 0.754–0.754 | 0.262 | 5589.8 | 168 | 1 |
| essential512/slab | joint | 0.545 | 0.817 | 0.679 | 0.137 | 0.000 | 0.817–0.817 | 0.152 | 170.4 | 1308 | 1 |
| packed512/enum | joint | 0.380 | 0.139 | 0.107 | 0.014 | 0.015 | 0.139–0.139 | 0.052 | 224.9 | 1308 | 1 |
| prefix512/slab | faces | 0.585 | 0.796 | 0.650 | 0.145 | 0.000 | 0.796–0.796 | 0.154 | 175.7 | 1324 | 1 |
| retraction512/slab | faces | 0.554 | 0.793 | 0.652 | 0.139 | 0.000 | 0.793–0.793 | 0.154 | 175.7 | 1324 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-1/x, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.746 | 2.067 | 1.701 | 0.330 | 0.034 | 2.067–2.067 | 1.838 | 6105.5 | 184 | 1 |
| essential512/slab | joint | 0.548 | 0.626 | 0.491 | 0.134 | 0.000 | 0.626–0.626 | 0.141 | 132.0 | 1172 | 1 |
| packed512/enum | joint | 30.346 | 0.244 | 0.165 | 0.035 | 0.044 | 0.244–0.244 | 0.111 | 367.9 | 2724 | 1 |
| prefix512/slab | faces | 0.538 | 0.630 | 0.491 | 0.138 | 0.001 | 0.630–0.630 | 0.140 | 137.3 | 1189 | 1 |
| retraction512/slab | faces | 0.608 | 0.635 | 0.495 | 0.135 | 0.001 | 0.635–0.635 | 0.144 | 137.3 | 1189 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## full, width 6, suffix-1/x, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.746 | 0.733 | 0.466 | 0.234 | 0.032 | 0.733–0.733 | 0.269 | 6114.4 | 184 | 1 |
| essential512/slab | joint | 0.548 | 0.622 | 0.489 | 0.131 | 0.000 | 0.622–0.622 | 0.134 | 141.0 | 1172 | 1 |
| packed512/enum | joint | 30.346 | 0.255 | 0.180 | 0.030 | 0.041 | 0.255–0.255 | 0.099 | 376.8 | 2724 | 1 |
| prefix512/slab | faces | 0.538 | 0.638 | 0.495 | 0.142 | 0.000 | 0.638–0.638 | 0.136 | 146.3 | 1189 | 1 |
| retraction512/slab | faces | 0.608 | 0.637 | 0.505 | 0.131 | 0.000 | 0.637–0.637 | 0.136 | 146.3 | 1189 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## full, width 6, suffix-1/x, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.734 | 1.959 | 1.621 | 0.306 | 0.031 | 1.959–1.959 | 1.775 | 6105.5 | 184 | 1 |
| essential512/slab | joint | 0.586 | 0.933 | 0.732 | 0.201 | 0.000 | 0.933–0.933 | 0.169 | 161.4 | 1433 | 1 |
| packed512/enum | joint | 0.381 | 0.142 | 0.095 | 0.025 | 0.022 | 0.142–0.142 | 0.068 | 215.9 | 1464 | 1 |
| prefix512/slab | faces | 0.518 | 0.800 | 0.633 | 0.167 | 0.001 | 0.800–0.800 | 0.163 | 166.8 | 1449 | 1 |
| retraction512/slab | faces | 0.537 | 0.833 | 0.657 | 0.176 | 0.000 | 0.833–0.833 | 0.169 | 166.8 | 1449 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## full, width 6, suffix-1/x, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.734 | 0.735 | 0.456 | 0.249 | 0.031 | 0.735–0.735 | 0.265 | 6114.4 | 184 | 1 |
| essential512/slab | joint | 0.586 | 0.823 | 0.650 | 0.172 | 0.000 | 0.823–0.823 | 0.160 | 170.4 | 1433 | 1 |
| packed512/enum | joint | 0.381 | 0.150 | 0.104 | 0.025 | 0.020 | 0.150–0.150 | 0.064 | 224.9 | 1464 | 1 |
| prefix512/slab | faces | 0.518 | 0.861 | 0.688 | 0.172 | 0.000 | 0.861–0.861 | 0.161 | 175.7 | 1449 | 1 |
| retraction512/slab | faces | 0.537 | 0.818 | 0.648 | 0.169 | 0.001 | 0.818–0.818 | 0.160 | 175.7 | 1449 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## holes, width 4, suffix-half/xy, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.602 | 0.063 | 0.051 | 0.010 | 0.001 | 0.063–0.063 | 0.052 | 115.2 | 200 | 1 |
| essential512/slab | joint | 1.190 | 0.991 | 0.552 | 0.437 | 0.000 | 0.991–0.991 | 0.289 | 244.1 | 2001 | 1 |
| packed512/enum | joint | 0.753 | 0.166 | 0.098 | 0.040 | 0.025 | 0.166–0.166 | 0.056 | 290.7 | 1877 | 1 |
| prefix512/slab | faces | 0.757 | 0.181 | 0.136 | 0.031 | 0.013 | 0.181–0.181 | 0.062 | 150.7 | 1024 | 1 |
| retraction512/slab | faces | 0.702 | 1.364 | 0.126 | 1.226 | 0.010 | 1.364–1.364 | 0.298 | 325.3 | 2368 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.602 | 0.039 | 0.029 | 0.008 | 0.001 | 0.039–0.039 | 0.017 | 124.1 | 200 | 1 |
| essential512/slab | joint | 1.190 | 0.959 | 0.535 | 0.424 | 0.000 | 0.959–0.959 | 0.275 | 253.1 | 2001 | 1 |
| packed512/enum | joint | 0.753 | 0.131 | 0.073 | 0.028 | 0.029 | 0.131–0.131 | 0.059 | 299.7 | 1877 | 1 |
| prefix512/slab | faces | 0.757 | 0.153 | 0.116 | 0.029 | 0.008 | 0.153–0.153 | 0.045 | 159.7 | 1024 | 1 |
| retraction512/slab | faces | 0.702 | 1.273 | 0.122 | 1.146 | 0.005 | 1.273–1.273 | 0.279 | 334.2 | 2368 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.579 | 0.064 | 0.051 | 0.011 | 0.001 | 0.064–0.064 | 0.047 | 115.2 | 200 | 1 |
| essential512/slab | joint | 0.795 | 1.107 | 0.725 | 0.381 | 0.000 | 1.107–1.107 | 0.247 | 318.0 | 2239 | 1 |
| packed512/enum | joint | 0.214 | 0.165 | 0.110 | 0.036 | 0.018 | 0.165–0.165 | 0.041 | 331.7 | 2075 | 1 |
| prefix512/slab | faces | 0.644 | 0.175 | 0.130 | 0.031 | 0.013 | 0.175–0.175 | 0.059 | 115.6 | 855 | 1 |
| retraction512/slab | faces | 0.595 | 0.850 | 0.120 | 0.720 | 0.009 | 0.850–0.850 | 0.303 | 186.4 | 1579 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.579 | 0.036 | 0.026 | 0.009 | 0.001 | 0.036–0.036 | 0.017 | 124.1 | 200 | 1 |
| essential512/slab | joint | 0.795 | 1.142 | 0.699 | 0.443 | 0.000 | 1.142–1.142 | 0.245 | 327.0 | 2239 | 1 |
| packed512/enum | joint | 0.214 | 0.119 | 0.083 | 0.023 | 0.013 | 0.119–0.119 | 0.037 | 340.7 | 2075 | 1 |
| prefix512/slab | faces | 0.644 | 0.165 | 0.126 | 0.031 | 0.009 | 0.165–0.165 | 0.048 | 124.6 | 855 | 1 |
| retraction512/slab | faces | 0.595 | 0.828 | 0.116 | 0.707 | 0.004 | 0.828–0.828 | 0.251 | 195.3 | 1579 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, whole/xy, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.581 | 0.067 | 0.053 | 0.013 | 0.001 | 0.067–0.067 | 0.051 | 110.6 | 193 | 1 |
| essential512/slab | joint | 1.230 | 0.893 | 0.557 | 0.334 | 0.000 | 0.893–0.893 | 0.264 | 240.3 | 1858 | 1 |
| packed512/enum | joint | 0.759 | 0.159 | 0.103 | 0.042 | 0.011 | 0.159–0.159 | 0.046 | 290.7 | 1777 | 1 |
| prefix512/slab | faces | 0.750 | 0.175 | 0.126 | 0.041 | 0.006 | 0.175–0.175 | 0.061 | 150.7 | 1017 | 1 |
| retraction512/slab | faces | 0.704 | 0.178 | 0.127 | 0.043 | 0.006 | 0.178–0.178 | 0.063 | 149.5 | 964 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## holes, width 4, whole/xy, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.581 | 0.044 | 0.031 | 0.012 | 0.001 | 0.044–0.044 | 0.021 | 119.6 | 193 | 1 |
| essential512/slab | joint | 1.230 | 0.831 | 0.525 | 0.306 | 0.000 | 0.831–0.831 | 0.265 | 249.3 | 1858 | 1 |
| packed512/enum | joint | 0.759 | 0.113 | 0.077 | 0.028 | 0.007 | 0.113–0.113 | 0.040 | 299.7 | 1777 | 1 |
| prefix512/slab | faces | 0.750 | 0.174 | 0.129 | 0.042 | 0.002 | 0.174–0.174 | 0.056 | 159.7 | 1017 | 1 |
| retraction512/slab | faces | 0.704 | 0.157 | 0.114 | 0.040 | 0.002 | 0.157–0.157 | 0.050 | 158.5 | 964 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## holes, width 4, whole/xy, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.563 | 0.065 | 0.051 | 0.013 | 0.001 | 0.065–0.065 | 0.050 | 110.6 | 193 | 1 |
| essential512/slab | joint | 0.772 | 0.983 | 0.690 | 0.291 | 0.000 | 0.983–0.983 | 0.266 | 316.1 | 2029 | 1 |
| packed512/enum | joint | 0.236 | 0.180 | 0.133 | 0.036 | 0.011 | 0.180–0.180 | 0.035 | 331.7 | 1935 | 1 |
| prefix512/slab | faces | 0.644 | 0.178 | 0.128 | 0.043 | 0.007 | 0.178–0.178 | 0.059 | 115.6 | 848 | 1 |
| retraction512/slab | faces | 0.605 | 0.168 | 0.117 | 0.043 | 0.007 | 0.168–0.168 | 0.059 | 116.3 | 800 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## holes, width 4, whole/xy, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 0.563 | 0.042 | 0.029 | 0.012 | 0.001 | 0.042–0.042 | 0.022 | 119.6 | 193 | 1 |
| essential512/slab | joint | 0.772 | 0.944 | 0.658 | 0.286 | 0.000 | 0.944–0.944 | 0.254 | 325.1 | 2029 | 1 |
| packed512/enum | joint | 0.236 | 0.115 | 0.087 | 0.022 | 0.006 | 0.115–0.115 | 0.035 | 340.7 | 1935 | 1 |
| prefix512/slab | faces | 0.644 | 0.158 | 0.117 | 0.039 | 0.002 | 0.158–0.158 | 0.049 | 124.6 | 848 | 1 |
| retraction512/slab | faces | 0.605 | 0.150 | 0.108 | 0.040 | 0.002 | 0.150–0.150 | 0.050 | 125.3 | 800 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## holes, width 6, suffix-half/xy, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 220.807 | 2.224 | 1.650 | 0.543 | 0.031 | 2.224–2.224 | 2.109 | 17043.7 | 517 | 1 |
| essential512/slab | joint | 20.753 | 7.544 | 4.129 | 3.415 | 0.000 | 7.544–7.544 | 3.192 | 4987.3 | 34080 | 1 |
| packed512/enum | joint | 34.461 | 1.495 | 0.953 | 0.311 | 0.230 | 1.495–1.495 | 0.552 | 5355.9 | 31982 | 1 |
| prefix512/slab | faces | 11.273 | 0.800 | 0.522 | 0.155 | 0.122 | 0.800–0.800 | 0.307 | 2917.2 | 17665 | 1 |
| retraction512/slab | faces | 11.574 | 13.827 | 0.743 | 13.029 | 0.054 | 13.827–13.827 | 3.113 | 4053.3 | 31857 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 220.807 | 1.024 | 0.479 | 0.515 | 0.030 | 1.024–1.024 | 0.552 | 17052.6 | 517 | 1 |
| essential512/slab | joint | 20.753 | 7.438 | 4.048 | 3.388 | 0.000 | 7.438–7.438 | 2.982 | 4996.3 | 34080 | 1 |
| packed512/enum | joint | 34.461 | 1.360 | 0.832 | 0.305 | 0.223 | 1.360–1.360 | 0.520 | 5364.8 | 31982 | 1 |
| prefix512/slab | faces | 11.273 | 0.773 | 0.499 | 0.152 | 0.122 | 0.773–0.773 | 0.347 | 2926.2 | 17665 | 1 |
| retraction512/slab | faces | 11.574 | 14.016 | 0.713 | 13.248 | 0.053 | 14.016–14.016 | 3.324 | 4062.2 | 31857 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 220.280 | 2.232 | 1.636 | 0.564 | 0.030 | 2.232–2.232 | 2.065 | 17043.7 | 517 | 1 |
| essential512/slab | joint | 3.823 | 1.848 | 1.208 | 0.637 | 0.003 | 1.848–1.848 | 0.581 | 1045.3 | 10977 | 1 |
| packed512/enum | joint | 1.356 | 0.355 | 0.248 | 0.060 | 0.046 | 0.355–0.355 | 0.128 | 1282.0 | 10792 | 1 |
| prefix512/slab | faces | 7.370 | 1.001 | 0.667 | 0.206 | 0.127 | 1.001–1.001 | 0.367 | 1588.6 | 11564 | 1 |
| retraction512/slab | faces | 7.959 | 2.323 | 0.974 | 1.286 | 0.062 | 2.323–2.323 | 0.620 | 2235.3 | 14582 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 220.280 | 1.025 | 0.458 | 0.536 | 0.030 | 1.025–1.025 | 0.708 | 17052.6 | 517 | 1 |
| essential512/slab | joint | 3.823 | 1.899 | 1.265 | 0.633 | 0.000 | 1.899–1.899 | 0.554 | 1054.3 | 10977 | 1 |
| packed512/enum | joint | 1.356 | 0.321 | 0.221 | 0.056 | 0.042 | 0.321–0.321 | 0.116 | 1291.0 | 10792 | 1 |
| prefix512/slab | faces | 7.370 | 0.927 | 0.615 | 0.194 | 0.118 | 0.927–0.927 | 0.329 | 1597.5 | 11564 | 1 |
| retraction512/slab | faces | 7.959 | 2.352 | 0.944 | 1.353 | 0.053 | 2.352–2.352 | 0.619 | 2244.3 | 14582 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, whole/xy, bit-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 226.035 | 2.388 | 1.654 | 0.702 | 0.031 | 2.388–2.388 | 2.254 | 16978.1 | 515 | 1 |
| essential512/slab | joint | 21.459 | 7.007 | 4.116 | 2.891 | 0.000 | 7.007–7.007 | 2.552 | 4987.3 | 33882 | 1 |
| packed512/enum | joint | 34.063 | 1.487 | 1.128 | 0.315 | 0.043 | 1.487–1.487 | 0.347 | 5355.9 | 31850 | 1 |
| prefix512/slab | faces | 11.221 | 0.686 | 0.517 | 0.152 | 0.016 | 0.686–0.686 | 0.197 | 2917.2 | 17656 | 1 |
| retraction512/slab | faces | 11.658 | 1.042 | 0.736 | 0.286 | 0.020 | 1.042–1.042 | 0.359 | 3048.0 | 18639 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## holes, width 6, whole/xy, bit-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 226.035 | 1.197 | 0.482 | 0.683 | 0.031 | 1.197–1.197 | 0.814 | 16987.1 | 515 | 1 |
| essential512/slab | joint | 21.459 | 7.153 | 4.110 | 3.042 | 0.001 | 7.153–7.153 | 2.624 | 4996.3 | 33882 | 1 |
| packed512/enum | joint | 34.063 | 1.237 | 0.896 | 0.302 | 0.038 | 1.237–1.237 | 0.322 | 5364.8 | 31850 | 1 |
| prefix512/slab | faces | 11.221 | 0.651 | 0.492 | 0.144 | 0.015 | 0.651–0.651 | 0.176 | 2926.2 | 17656 | 1 |
| retraction512/slab | faces | 11.658 | 1.016 | 0.700 | 0.298 | 0.017 | 1.016–1.016 | 0.324 | 3056.9 | 18639 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## holes, width 6, whole/xy, face-major, memo False

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 218.364 | 2.375 | 1.648 | 0.696 | 0.030 | 2.375–2.375 | 2.153 | 16978.1 | 515 | 1 |
| essential512/slab | joint | 3.795 | 1.821 | 1.255 | 0.566 | 0.000 | 1.821–1.821 | 0.627 | 1045.3 | 10844 | 1 |
| packed512/enum | joint | 1.467 | 0.342 | 0.260 | 0.058 | 0.022 | 0.342–0.342 | 0.107 | 1282.0 | 10703 | 1 |
| prefix512/slab | faces | 7.135 | 0.863 | 0.630 | 0.212 | 0.020 | 0.863–0.863 | 0.244 | 1588.6 | 11555 | 1 |
| retraction512/slab | faces | 8.054 | 1.304 | 0.940 | 0.343 | 0.021 | 1.304–1.304 | 0.409 | 1983.5 | 13261 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

## holes, width 6, whole/xy, face-major, memo True

| Carrier/store | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | joint | 218.364 | 1.171 | 0.463 | 0.677 | 0.030 | 1.171–1.171 | 0.715 | 16987.1 | 515 | 1 |
| essential512/slab | joint | 3.795 | 1.836 | 1.256 | 0.580 | 0.000 | 1.836–1.836 | 0.501 | 1054.3 | 10844 | 1 |
| packed512/enum | joint | 1.467 | 0.337 | 0.264 | 0.055 | 0.017 | 0.337–0.337 | 0.084 | 1291.0 | 10703 | 1 |
| prefix512/slab | faces | 7.135 | 0.823 | 0.610 | 0.195 | 0.017 | 0.823–0.823 | 0.247 | 1597.5 | 11555 | 1 |
| retraction512/slab | faces | 8.054 | 1.438 | 1.063 | 0.356 | 0.018 | 1.438–1.438 | 0.427 | 1992.5 | 13261 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 0/48.

352 configurations; 864 matched comparisons; 97 retained processes.

- [Raw evidence](results/readout-first-smoke.json).
- [Raw evidence](results/readout-smoke.json).
- [Raw evidence](results/readout-domains.json).
- [Raw evidence](results/readout-cutoff.json).
