# Same-binary direct completed projection after Free Join

Every query returns 64 Events: original, Possible, Guaranteed and Ambiguous
for sixteen groups. Fresh/warm totals include exact original-support counts.
Every returned Event is checked pointwise and canonically reimported outside
timing. Construction is separate. Bytes estimate retained carrier/cache memory.
They exclude allocator overhead, verification clones, result vectors and the join engine.
Executable: `2ebe0641b45c4e625a8249bb522a62c056c37fb49d14747bbe25e1adc003ea82`.

Only entirely successful processes contribute timing rows. Partial rows from
capped or failed processes remain in the raw evidence but cannot supply a
complete-query comparison. A later incomplete process invalidates an older
successful process for that exact job; no failed result is a fast answer.

## fibred, width 4, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 1.282 | 0.084 | 0.070 | 0.013 | 0.001 | 0.072–0.094 | 0.075 | 305.2 | 277 | 3 |
| packed512/enum/staged/joint/full/staged | joint | 1.303 | 0.245 | 0.140 | 0.072 | 0.033 | 0.235–0.309 | 0.079 | 629.8 | 4401 | 3 |
| prefix512/slab/ite/active/local-needed/fused | faces | 1.028 | 0.829 | 0.155 | 0.656 | 0.016 | 0.829–0.829 | 0.037 | 255.1 | 2125 | 1 |
| prefix512/slab/ite/active/local-needed/staged | faces | 1.036 | 0.537 | 0.160 | 0.359 | 0.016 | 0.537–0.537 | 0.049 | 223.7 | 1718 | 1 |
| prefix512/slab/ite/joint/local-needed/fused | faces | 1.006 | 6.780 | 0.170 | 6.592 | 0.016 | 6.780–6.780 | 0.032 | 1649.3 | 10234 | 1 |
| prefix512/slab/ite/joint/local-needed/staged | faces | 2.855 | 6.996 | 0.387 | 6.586 | 0.020 | 6.996–6.996 | 0.108 | 832.0 | 7028 | 1 |
| prefix512/slab/ite/witness/full/fused | faces | 1.041 | 0.760 | 0.168 | 0.572 | 0.020 | 0.760–0.760 | 0.036 | 257.9 | 1826 | 1 |
| prefix512/slab/ite/witness/full/staged | faces | 1.000 | 0.562 | 0.155 | 0.390 | 0.015 | 0.562–0.562 | 0.047 | 223.7 | 1669 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.953 | 0.377 | 0.147 | 0.218 | 0.012 | 0.375–0.407 | 0.027 | 223.4 | 1552 | 3 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.915 | 0.281 | 0.145 | 0.124 | 0.012 | 0.280–0.298 | 0.040 | 213.5 | 1395 | 3 |
| prefix512/slab/staged/witness/local-needed/fused | faces | 1.415 | 0.577 | 0.173 | 0.386 | 0.017 | 0.577–0.577 | 0.044 | 334.8 | 2202 | 1 |
| prefix512/slab/staged/witness/local-needed/staged | faces | 1.429 | 0.399 | 0.173 | 0.209 | 0.016 | 0.399–0.399 | 0.047 | 332.4 | 1959 | 1 |
| prefix64/slab/ite/witness/local-needed/fused | faces | 2.498 | 1.344 | 0.457 | 0.800 | 0.085 | 1.344–1.344 | 0.144 | 636.0 | 6098 | 1 |
| prefix64/slab/ite/witness/local-needed/staged | faces | 2.566 | 1.055 | 0.428 | 0.541 | 0.085 | 1.055–1.055 | 0.137 | 603.2 | 5659 | 1 |
| retraction512/slab/ite/witness/full/fused | faces | 0.919 | 0.870 | 0.159 | 0.698 | 0.013 | 0.841–0.931 | 0.037 | 319.4 | 1994 | 3 |
| retraction512/slab/ite/witness/full/staged | faces | 0.867 | 0.652 | 0.156 | 0.482 | 0.014 | 0.641–0.653 | 0.063 | 218.2 | 1745 | 3 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.960 | 0.559 | 0.160 | 0.381 | 0.016 | 0.559–0.559 | 0.031 | 220.7 | 1644 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.951 | 0.372 | 0.152 | 0.202 | 0.016 | 0.372–0.372 | 0.044 | 217.0 | 1436 | 1 |
| retraction64/slab/ite/witness/local-needed/fused | faces | 2.651 | 1.592 | 0.546 | 0.954 | 0.091 | 1.592–1.592 | 0.117 | 641.8 | 6699 | 1 |
| retraction64/slab/ite/witness/local-needed/staged | faces | 2.634 | 1.339 | 0.537 | 0.711 | 0.090 | 1.339–1.339 | 0.129 | 638.1 | 6160 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 1.282 | 0.042 | 0.029 | 0.011 | 0.001 | 0.039–0.044 | 0.019 | 314.1 | 277 | 3 |
| packed512/enum/staged/joint/full/staged | joint | 1.303 | 0.236 | 0.135 | 0.068 | 0.034 | 0.233–0.258 | 0.072 | 638.7 | 4401 | 3 |
| prefix512/slab/ite/active/local-needed/fused | faces | 1.028 | 0.868 | 0.161 | 0.692 | 0.013 | 0.868–0.868 | 0.030 | 264.1 | 2125 | 1 |
| prefix512/slab/ite/active/local-needed/staged | faces | 1.036 | 0.520 | 0.150 | 0.358 | 0.011 | 0.520–0.520 | 0.038 | 232.7 | 1718 | 1 |
| prefix512/slab/ite/joint/local-needed/fused | faces | 1.006 | 6.531 | 0.146 | 6.373 | 0.012 | 6.531–6.531 | 0.027 | 1658.3 | 10234 | 1 |
| prefix512/slab/ite/joint/local-needed/staged | faces | 2.855 | 4.387 | 0.157 | 4.217 | 0.013 | 4.387–4.387 | 0.098 | 841.0 | 7028 | 1 |
| prefix512/slab/ite/witness/full/fused | faces | 1.041 | 0.723 | 0.161 | 0.550 | 0.012 | 0.723–0.723 | 0.027 | 266.9 | 1826 | 1 |
| prefix512/slab/ite/witness/full/staged | faces | 1.000 | 0.548 | 0.153 | 0.383 | 0.012 | 0.548–0.548 | 0.042 | 232.7 | 1669 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.953 | 0.381 | 0.148 | 0.221 | 0.012 | 0.373–0.395 | 0.023 | 232.4 | 1552 | 3 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.915 | 0.293 | 0.155 | 0.123 | 0.012 | 0.287–0.297 | 0.041 | 222.5 | 1395 | 3 |
| prefix512/slab/staged/witness/local-needed/fused | faces | 1.415 | 0.536 | 0.158 | 0.365 | 0.012 | 0.536–0.536 | 0.038 | 343.7 | 2202 | 1 |
| prefix512/slab/staged/witness/local-needed/staged | faces | 1.429 | 0.371 | 0.155 | 0.204 | 0.012 | 0.371–0.371 | 0.037 | 341.4 | 1959 | 1 |
| prefix64/slab/ite/witness/local-needed/fused | faces | 2.498 | 1.385 | 0.493 | 0.812 | 0.080 | 1.385–1.385 | 0.094 | 644.9 | 6098 | 1 |
| prefix64/slab/ite/witness/local-needed/staged | faces | 2.566 | 1.047 | 0.437 | 0.529 | 0.080 | 1.047–1.047 | 0.136 | 612.1 | 5659 | 1 |
| retraction512/slab/ite/witness/full/fused | faces | 0.919 | 0.871 | 0.156 | 0.700 | 0.015 | 0.866–0.897 | 0.026 | 328.4 | 1994 | 3 |
| retraction512/slab/ite/witness/full/staged | faces | 0.867 | 0.653 | 0.158 | 0.477 | 0.014 | 0.630–0.663 | 0.044 | 227.2 | 1745 | 3 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.960 | 0.536 | 0.148 | 0.375 | 0.012 | 0.536–0.536 | 0.023 | 229.7 | 1644 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.951 | 0.388 | 0.169 | 0.202 | 0.013 | 0.388–0.388 | 0.043 | 226.0 | 1436 | 1 |
| retraction64/slab/ite/witness/local-needed/fused | faces | 2.651 | 1.651 | 0.546 | 1.017 | 0.088 | 1.651–1.651 | 0.105 | 650.7 | 6699 | 1 |
| retraction64/slab/ite/witness/local-needed/staged | faces | 2.634 | 1.281 | 0.503 | 0.694 | 0.083 | 1.281–1.281 | 0.125 | 647.0 | 6160 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 1.261 | 0.082 | 0.067 | 0.014 | 0.001 | 0.076–0.091 | 0.074 | 305.2 | 277 | 3 |
| packed512/enum/staged/joint/full/staged | joint | 0.270 | 0.214 | 0.142 | 0.048 | 0.030 | 0.211–0.262 | 0.064 | 638.9 | 3967 | 3 |
| prefix512/slab/ite/active/local-needed/fused | faces | 0.950 | 0.766 | 0.162 | 0.585 | 0.017 | 0.766–0.766 | 0.030 | 244.8 | 2063 | 1 |
| prefix512/slab/ite/active/local-needed/staged | faces | 0.980 | 0.520 | 0.167 | 0.335 | 0.016 | 0.520–0.520 | 0.047 | 210.6 | 1667 | 1 |
| prefix512/slab/ite/joint/local-needed/fused | faces | 0.951 | 4.000 | 0.153 | 3.830 | 0.016 | 4.000–4.000 | 0.033 | 824.2 | 6742 | 1 |
| prefix512/slab/ite/joint/local-needed/staged | faces | 0.972 | 2.991 | 0.161 | 2.813 | 0.016 | 2.991–2.991 | 0.234 | 779.4 | 4987 | 1 |
| prefix512/slab/ite/witness/full/fused | faces | 1.012 | 0.683 | 0.160 | 0.502 | 0.019 | 0.683–0.683 | 0.030 | 221.4 | 1778 | 1 |
| prefix512/slab/ite/witness/full/staged | faces | 1.017 | 0.598 | 0.179 | 0.395 | 0.019 | 0.598–0.598 | 0.046 | 210.6 | 1639 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.888 | 0.371 | 0.144 | 0.214 | 0.012 | 0.364–0.401 | 0.029 | 210.3 | 1503 | 3 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.904 | 0.279 | 0.153 | 0.117 | 0.011 | 0.277–0.309 | 0.037 | 200.4 | 1346 | 3 |
| prefix512/slab/staged/witness/local-needed/fused | faces | 1.350 | 0.579 | 0.167 | 0.394 | 0.017 | 0.579–0.579 | 0.031 | 315.6 | 2132 | 1 |
| prefix512/slab/staged/witness/local-needed/staged | faces | 1.377 | 0.395 | 0.170 | 0.207 | 0.017 | 0.395–0.395 | 0.044 | 313.2 | 1889 | 1 |
| prefix64/slab/ite/witness/local-needed/fused | faces | 1.521 | 1.419 | 0.452 | 0.880 | 0.085 | 1.419–1.419 | 0.096 | 444.2 | 4747 | 1 |
| prefix64/slab/ite/witness/local-needed/staged | faces | 1.565 | 1.240 | 0.453 | 0.702 | 0.084 | 1.240–1.240 | 0.257 | 441.8 | 4282 | 1 |
| retraction512/slab/ite/witness/full/fused | faces | 0.963 | 0.832 | 0.161 | 0.652 | 0.018 | 0.832–0.832 | 0.038 | 302.8 | 1883 | 1 |
| retraction512/slab/ite/witness/full/staged | faces | 0.953 | 0.686 | 0.156 | 0.509 | 0.019 | 0.686–0.686 | 0.042 | 258.8 | 1721 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.946 | 0.620 | 0.199 | 0.390 | 0.017 | 0.620–0.620 | 0.029 | 209.9 | 1601 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.924 | 0.381 | 0.155 | 0.207 | 0.017 | 0.381–0.381 | 0.041 | 206.1 | 1393 | 1 |
| retraction64/slab/ite/witness/local-needed/fused | faces | 1.604 | 1.709 | 0.541 | 1.077 | 0.089 | 1.709–1.709 | 0.114 | 643.5 | 5351 | 1 |
| retraction64/slab/ite/witness/local-needed/staged | faces | 1.582 | 1.358 | 0.498 | 0.773 | 0.085 | 1.358–1.358 | 0.253 | 445.4 | 4746 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 1.261 | 0.041 | 0.029 | 0.011 | 0.001 | 0.039–0.043 | 0.019 | 314.1 | 277 | 3 |
| packed512/enum/staged/joint/full/staged | joint | 0.270 | 0.214 | 0.142 | 0.042 | 0.030 | 0.203–0.234 | 0.060 | 647.8 | 3967 | 3 |
| prefix512/slab/ite/active/local-needed/fused | faces | 0.950 | 0.707 | 0.149 | 0.545 | 0.011 | 0.707–0.707 | 0.023 | 253.7 | 2063 | 1 |
| prefix512/slab/ite/active/local-needed/staged | faces | 0.980 | 0.515 | 0.158 | 0.345 | 0.012 | 0.515–0.515 | 0.048 | 219.6 | 1667 | 1 |
| prefix512/slab/ite/joint/local-needed/fused | faces | 0.951 | 3.947 | 0.149 | 3.787 | 0.012 | 3.947–3.947 | 0.022 | 833.2 | 6742 | 1 |
| prefix512/slab/ite/joint/local-needed/staged | faces | 0.972 | 3.040 | 0.147 | 2.880 | 0.012 | 3.040–3.040 | 0.234 | 788.4 | 4987 | 1 |
| prefix512/slab/ite/witness/full/fused | faces | 1.012 | 0.669 | 0.149 | 0.508 | 0.012 | 0.669–0.669 | 0.032 | 230.3 | 1778 | 1 |
| prefix512/slab/ite/witness/full/staged | faces | 1.017 | 0.541 | 0.148 | 0.382 | 0.011 | 0.541–0.541 | 0.046 | 219.6 | 1639 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.888 | 0.388 | 0.154 | 0.222 | 0.012 | 0.371–0.427 | 0.024 | 219.3 | 1503 | 3 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.904 | 0.275 | 0.148 | 0.115 | 0.011 | 0.275–0.276 | 0.033 | 209.3 | 1346 | 3 |
| prefix512/slab/staged/witness/local-needed/fused | faces | 1.350 | 0.518 | 0.149 | 0.357 | 0.012 | 0.518–0.518 | 0.024 | 324.5 | 2132 | 1 |
| prefix512/slab/staged/witness/local-needed/staged | faces | 1.377 | 0.353 | 0.148 | 0.194 | 0.011 | 0.353–0.353 | 0.035 | 322.2 | 1889 | 1 |
| prefix64/slab/ite/witness/local-needed/fused | faces | 1.521 | 1.330 | 0.420 | 0.833 | 0.077 | 1.330–1.330 | 0.088 | 453.1 | 4747 | 1 |
| prefix64/slab/ite/witness/local-needed/staged | faces | 1.565 | 1.160 | 0.433 | 0.641 | 0.086 | 1.160–1.160 | 0.228 | 450.8 | 4282 | 1 |
| retraction512/slab/ite/witness/full/fused | faces | 0.963 | 0.810 | 0.154 | 0.641 | 0.014 | 0.810–0.810 | 0.028 | 311.8 | 1883 | 1 |
| retraction512/slab/ite/witness/full/staged | faces | 0.953 | 0.658 | 0.148 | 0.497 | 0.012 | 0.658–0.658 | 0.044 | 267.8 | 1721 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.946 | 0.528 | 0.146 | 0.370 | 0.012 | 0.528–0.528 | 0.023 | 218.8 | 1601 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.924 | 0.351 | 0.143 | 0.196 | 0.012 | 0.351–0.351 | 0.039 | 215.1 | 1393 | 1 |
| retraction64/slab/ite/witness/local-needed/fused | faces | 1.604 | 1.613 | 0.509 | 1.022 | 0.082 | 1.613–1.613 | 0.105 | 652.5 | 5351 | 1 |
| retraction64/slab/ite/witness/local-needed/staged | faces | 1.582 | 1.322 | 0.485 | 0.756 | 0.081 | 1.322–1.322 | 0.243 | 454.3 | 4746 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 461.576 | 3.938 | 3.237 | 0.640 | 0.060 | 3.379–4.043 | 3.665 | 41852.6 | 636 | 3 |
| packed512/enum/staged/joint/full/staged | joint | 65.211 | 1.463 | 0.653 | 0.584 | 0.226 | 1.423–1.546 | 0.757 | 6651.9 | 47190 | 3 |
| prefix512/slab/ite/active/local-needed/fused | faces | 14.441 | 4.683 | 1.013 | 3.539 | 0.132 | 4.683–4.683 | 0.170 | 3466.9 | 26682 | 1 |
| prefix512/slab/ite/active/local-needed/staged | faces | 14.471 | 3.518 | 1.023 | 2.364 | 0.130 | 3.518–3.518 | 0.206 | 3344.1 | 24799 | 1 |
| prefix512/slab/ite/joint/local-needed/fused | faces | 14.592 | 46.426 | 1.047 | 45.241 | 0.137 | 46.426–46.426 | 0.176 | 11465.4 | 93046 | 1 |
| prefix512/slab/ite/joint/local-needed/staged | faces | 15.041 | 36.032 | 1.045 | 34.848 | 0.139 | 36.032–36.032 | 0.258 | 8271.9 | 70052 | 1 |
| prefix512/slab/ite/witness/full/fused | faces | 14.551 | 13.305 | 1.052 | 12.108 | 0.143 | 13.305–13.305 | 0.184 | 4338.8 | 37926 | 1 |
| prefix512/slab/ite/witness/full/staged | faces | 14.879 | 10.767 | 1.002 | 9.634 | 0.131 | 10.767–10.767 | 0.206 | 4318.9 | 35380 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 14.437 | 2.697 | 1.032 | 1.527 | 0.129 | 2.681–2.713 | 0.175 | 3347.8 | 23351 | 3 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 15.256 | 2.340 | 1.029 | 1.162 | 0.131 | 2.193–2.377 | 0.206 | 3345.5 | 22748 | 3 |
| prefix512/slab/staged/witness/local-needed/fused | faces | 24.764 | 3.457 | 1.102 | 2.219 | 0.135 | 3.457–3.457 | 0.185 | 6130.1 | 42858 | 1 |
| prefix512/slab/staged/witness/local-needed/staged | faces | 25.151 | 2.790 | 1.100 | 1.561 | 0.127 | 2.790–2.790 | 0.222 | 6127.8 | 41745 | 1 |
| prefix64/slab/ite/witness/local-needed/fused | faces | 16.737 | 3.637 | 1.344 | 1.983 | 0.310 | 3.637–3.637 | 0.352 | 4879.2 | 50321 | 1 |
| prefix64/slab/ite/witness/local-needed/staged | faces | 16.974 | 3.362 | 1.391 | 1.657 | 0.313 | 3.362–3.362 | 0.377 | 4876.9 | 49365 | 1 |
| retraction512/slab/ite/witness/full/fused | faces | 15.691 | 21.783 | 1.381 | 20.242 | 0.165 | 21.529–21.886 | 0.229 | 6655.2 | 49307 | 3 |
| retraction512/slab/ite/witness/full/staged | faces | 15.458 | 15.899 | 1.396 | 14.341 | 0.167 | 15.552–17.174 | 0.246 | 6622.9 | 41728 | 3 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 15.562 | 5.761 | 1.415 | 4.183 | 0.163 | 5.761–5.761 | 0.208 | 3456.8 | 28224 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 15.341 | 4.893 | 1.406 | 3.327 | 0.158 | 4.893–4.893 | 0.226 | 3444.7 | 26977 | 1 |
| retraction64/slab/ite/witness/local-needed/fused | faces | 27.149 | 9.332 | 2.140 | 6.718 | 0.474 | 9.332–9.332 | 0.542 | 7305.9 | 64805 | 1 |
| retraction64/slab/ite/witness/local-needed/staged | faces | 24.056 | 7.844 | 2.279 | 5.095 | 0.469 | 7.844–7.844 | 0.589 | 7293.8 | 61543 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 461.576 | 1.448 | 0.909 | 0.474 | 0.065 | 1.428–1.460 | 0.516 | 41861.5 | 636 | 3 |
| packed512/enum/staged/joint/full/staged | joint | 65.211 | 1.531 | 0.695 | 0.593 | 0.222 | 1.484–1.535 | 0.789 | 6660.8 | 47190 | 3 |
| prefix512/slab/ite/active/local-needed/fused | faces | 14.441 | 4.691 | 0.991 | 3.568 | 0.131 | 4.691–4.691 | 0.167 | 3475.8 | 26682 | 1 |
| prefix512/slab/ite/active/local-needed/staged | faces | 14.471 | 3.478 | 0.993 | 2.358 | 0.127 | 3.478–3.478 | 0.202 | 3353.1 | 24799 | 1 |
| prefix512/slab/ite/joint/local-needed/fused | faces | 14.592 | 44.689 | 0.993 | 43.553 | 0.142 | 44.689–44.689 | 0.161 | 11474.3 | 93046 | 1 |
| prefix512/slab/ite/joint/local-needed/staged | faces | 15.041 | 33.716 | 1.013 | 32.570 | 0.132 | 33.716–33.716 | 0.248 | 8280.8 | 70052 | 1 |
| prefix512/slab/ite/witness/full/fused | faces | 14.551 | 13.146 | 1.120 | 11.891 | 0.135 | 13.146–13.146 | 0.169 | 4347.7 | 37926 | 1 |
| prefix512/slab/ite/witness/full/staged | faces | 14.879 | 10.734 | 1.004 | 9.601 | 0.128 | 10.734–10.734 | 0.203 | 4327.9 | 35380 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 14.437 | 2.711 | 1.025 | 1.562 | 0.130 | 2.671–2.852 | 0.159 | 3356.8 | 23351 | 3 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 15.256 | 2.229 | 0.995 | 1.107 | 0.127 | 2.183–2.368 | 0.188 | 3354.5 | 22748 | 3 |
| prefix512/slab/staged/witness/local-needed/fused | faces | 24.764 | 3.376 | 1.029 | 2.217 | 0.129 | 3.376–3.376 | 0.166 | 6139.1 | 42858 | 1 |
| prefix512/slab/staged/witness/local-needed/staged | faces | 25.151 | 2.764 | 1.042 | 1.593 | 0.129 | 2.764–2.764 | 0.199 | 6136.7 | 41745 | 1 |
| prefix64/slab/ite/witness/local-needed/fused | faces | 16.737 | 3.603 | 1.329 | 1.975 | 0.299 | 3.603–3.603 | 0.335 | 4888.2 | 50321 | 1 |
| prefix64/slab/ite/witness/local-needed/staged | faces | 16.974 | 3.688 | 1.502 | 1.873 | 0.312 | 3.688–3.688 | 0.359 | 4885.9 | 49365 | 1 |
| retraction512/slab/ite/witness/full/fused | faces | 15.691 | 21.598 | 1.376 | 20.052 | 0.165 | 21.541–22.135 | 0.209 | 6664.2 | 49307 | 3 |
| retraction512/slab/ite/witness/full/staged | faces | 15.458 | 15.991 | 1.398 | 14.430 | 0.163 | 15.810–17.377 | 0.241 | 6631.8 | 41728 | 3 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 15.562 | 5.926 | 1.384 | 4.368 | 0.173 | 5.926–5.926 | 0.193 | 3465.8 | 28224 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 15.341 | 5.300 | 1.432 | 3.705 | 0.163 | 5.300–5.300 | 0.218 | 3453.7 | 26977 | 1 |
| retraction64/slab/ite/witness/local-needed/fused | faces | 27.149 | 9.446 | 2.052 | 6.916 | 0.477 | 9.446–9.446 | 0.520 | 7314.9 | 64805 | 1 |
| retraction64/slab/ite/witness/local-needed/staged | faces | 24.056 | 7.634 | 2.092 | 5.055 | 0.485 | 7.634–7.634 | 0.572 | 7302.8 | 61543 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 456.957 | 3.365 | 2.757 | 0.545 | 0.064 | 3.356–4.066 | 3.682 | 41852.6 | 636 | 3 |
| packed512/enum/staged/joint/full/staged | joint | 2.142 | 0.574 | 0.401 | 0.104 | 0.076 | 0.555–0.607 | 0.223 | 2402.4 | 18095 | 3 |
| prefix512/slab/ite/active/local-needed/fused | faces | 10.339 | 5.179 | 1.246 | 3.802 | 0.131 | 5.179–5.179 | 0.168 | 2751.6 | 22003 | 1 |
| prefix512/slab/ite/active/local-needed/staged | faces | 10.602 | 3.776 | 1.251 | 2.386 | 0.135 | 3.776–3.776 | 0.540 | 2382.3 | 19938 | 1 |
| prefix512/slab/ite/joint/local-needed/fused | faces | 10.248 | 12.797 | 1.210 | 11.456 | 0.131 | 12.797–12.797 | 0.180 | 4853.1 | 41338 | 1 |
| prefix512/slab/ite/joint/local-needed/staged | faces | 10.466 | 8.645 | 1.236 | 7.278 | 0.130 | 8.645–8.645 | 0.725 | 4336.1 | 31308 | 1 |
| prefix512/slab/ite/witness/full/fused | faces | 10.452 | 6.608 | 1.262 | 5.210 | 0.136 | 6.608–6.608 | 0.184 | 2823.2 | 25167 | 1 |
| prefix512/slab/ite/witness/full/staged | faces | 10.811 | 6.357 | 1.259 | 4.960 | 0.135 | 6.357–6.357 | 0.495 | 2642.2 | 24270 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 10.363 | 3.363 | 1.265 | 1.962 | 0.134 | 3.249–3.398 | 0.175 | 2507.9 | 19323 | 3 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 10.618 | 2.812 | 1.250 | 1.425 | 0.130 | 2.744–2.832 | 0.485 | 2383.7 | 18579 | 3 |
| prefix512/slab/staged/witness/local-needed/fused | faces | 18.400 | 3.862 | 1.278 | 2.457 | 0.128 | 3.862–3.862 | 0.180 | 5048.0 | 36743 | 1 |
| prefix512/slab/staged/witness/local-needed/staged | faces | 18.631 | 3.368 | 1.318 | 1.881 | 0.168 | 3.368–3.368 | 0.487 | 5045.7 | 35515 | 1 |
| prefix64/slab/ite/witness/local-needed/fused | faces | 6.867 | 5.422 | 1.868 | 3.160 | 0.393 | 5.422–5.422 | 0.486 | 2489.0 | 25125 | 1 |
| prefix64/slab/ite/witness/local-needed/staged | faces | 6.896 | 4.991 | 1.877 | 2.709 | 0.405 | 4.991–4.991 | 1.293 | 2337.2 | 23653 | 1 |
| retraction512/slab/ite/witness/full/fused | faces | 11.717 | 10.164 | 1.525 | 8.476 | 0.162 | 10.164–10.164 | 0.219 | 3393.5 | 30412 | 1 |
| retraction512/slab/ite/witness/full/staged | faces | 11.669 | 9.637 | 1.486 | 7.987 | 0.164 | 9.637–9.637 | 0.593 | 3206.3 | 29504 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 11.429 | 5.348 | 1.458 | 3.728 | 0.162 | 5.348–5.348 | 0.199 | 2847.9 | 23676 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 11.045 | 4.499 | 1.453 | 2.883 | 0.163 | 4.499–4.499 | 0.572 | 2713.9 | 22363 | 1 |
| retraction64/slab/ite/witness/local-needed/fused | faces | 7.870 | 8.271 | 2.065 | 5.746 | 0.459 | 8.271–8.271 | 0.501 | 3172.6 | 35854 | 1 |
| retraction64/slab/ite/witness/local-needed/staged | faces | 7.501 | 7.003 | 1.955 | 4.599 | 0.448 | 7.003–7.003 | 1.483 | 3160.5 | 32988 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 456.957 | 1.423 | 0.887 | 0.473 | 0.062 | 1.407–1.447 | 0.525 | 41861.5 | 636 | 3 |
| packed512/enum/staged/joint/full/staged | joint | 2.142 | 0.585 | 0.396 | 0.104 | 0.076 | 0.549–0.702 | 0.206 | 2411.3 | 18095 | 3 |
| prefix512/slab/ite/active/local-needed/fused | faces | 10.339 | 5.162 | 1.245 | 3.784 | 0.132 | 5.162–5.162 | 0.163 | 2760.6 | 22003 | 1 |
| prefix512/slab/ite/active/local-needed/staged | faces | 10.602 | 3.971 | 1.322 | 2.518 | 0.131 | 3.971–3.971 | 0.536 | 2391.3 | 19938 | 1 |
| prefix512/slab/ite/joint/local-needed/fused | faces | 10.248 | 13.775 | 1.199 | 12.448 | 0.127 | 13.775–13.775 | 0.158 | 4862.1 | 41338 | 1 |
| prefix512/slab/ite/joint/local-needed/staged | faces | 10.466 | 9.746 | 1.355 | 8.251 | 0.139 | 9.746–9.746 | 0.703 | 4345.1 | 31308 | 1 |
| prefix512/slab/ite/witness/full/fused | faces | 10.452 | 6.618 | 1.256 | 5.233 | 0.128 | 6.618–6.618 | 0.170 | 2832.1 | 25167 | 1 |
| prefix512/slab/ite/witness/full/staged | faces | 10.811 | 6.480 | 1.275 | 5.068 | 0.137 | 6.480–6.480 | 0.482 | 2651.1 | 24270 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 10.363 | 3.286 | 1.230 | 1.926 | 0.130 | 3.236–3.374 | 0.164 | 2516.9 | 19323 | 3 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 10.618 | 2.964 | 1.303 | 1.525 | 0.134 | 2.901–3.029 | 0.448 | 2392.7 | 18579 | 3 |
| prefix512/slab/staged/witness/local-needed/fused | faces | 18.400 | 3.895 | 1.269 | 2.496 | 0.129 | 3.895–3.895 | 0.170 | 5057.0 | 36743 | 1 |
| prefix512/slab/staged/witness/local-needed/staged | faces | 18.631 | 3.262 | 1.269 | 1.849 | 0.143 | 3.262–3.262 | 0.459 | 5054.6 | 35515 | 1 |
| prefix64/slab/ite/witness/local-needed/fused | faces | 6.867 | 5.934 | 1.945 | 3.573 | 0.415 | 5.934–5.934 | 0.440 | 2497.9 | 25125 | 1 |
| prefix64/slab/ite/witness/local-needed/staged | faces | 6.896 | 5.092 | 1.908 | 2.757 | 0.425 | 5.092–5.092 | 1.359 | 2346.2 | 23653 | 1 |
| retraction512/slab/ite/witness/full/fused | faces | 11.717 | 10.082 | 1.505 | 8.420 | 0.156 | 10.082–10.082 | 0.203 | 3402.5 | 30412 | 1 |
| retraction512/slab/ite/witness/full/staged | faces | 11.669 | 9.551 | 1.445 | 7.909 | 0.197 | 9.551–9.551 | 0.590 | 3215.2 | 29504 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 11.429 | 5.403 | 1.495 | 3.755 | 0.153 | 5.403–5.403 | 0.195 | 2856.9 | 23676 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 11.045 | 4.509 | 1.437 | 2.915 | 0.156 | 4.509–4.509 | 0.578 | 2722.9 | 22363 | 1 |
| retraction64/slab/ite/witness/local-needed/fused | faces | 7.870 | 8.507 | 1.998 | 6.039 | 0.469 | 8.507–8.507 | 0.522 | 3181.6 | 35854 | 1 |
| retraction64/slab/ite/witness/local-needed/staged | faces | 7.501 | 6.853 | 1.895 | 4.514 | 0.444 | 6.853–6.853 | 1.616 | 3169.5 | 32988 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## full, width 4, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 0.021 | 0.064 | 0.053 | 0.008 | 0.001 | 0.064–0.064 | 0.048 | 91.4 | 157 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 0.537 | 0.098 | 0.073 | 0.014 | 0.010 | 0.098–0.098 | 0.027 | 105.0 | 590 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.066 | 0.171 | 0.133 | 0.036 | 0.002 | 0.171–0.171 | 0.045 | 21.6 | 169 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.065 | 0.170 | 0.133 | 0.035 | 0.002 | 0.170–0.170 | 0.043 | 21.6 | 169 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.073 | 0.180 | 0.140 | 0.037 | 0.002 | 0.180–0.180 | 0.049 | 21.6 | 169 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.067 | 0.168 | 0.130 | 0.035 | 0.002 | 0.168–0.168 | 0.042 | 21.6 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 0.021 | 0.040 | 0.031 | 0.008 | 0.001 | 0.040–0.040 | 0.016 | 100.3 | 157 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 0.537 | 0.068 | 0.051 | 0.009 | 0.008 | 0.068–0.068 | 0.023 | 114.0 | 590 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.066 | 0.151 | 0.119 | 0.031 | 0.000 | 0.151–0.151 | 0.039 | 30.6 | 169 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.065 | 0.149 | 0.117 | 0.031 | 0.000 | 0.149–0.149 | 0.039 | 30.6 | 169 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.073 | 0.153 | 0.122 | 0.031 | 0.000 | 0.153–0.153 | 0.041 | 30.6 | 169 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.067 | 0.152 | 0.120 | 0.031 | 0.000 | 0.152–0.152 | 0.038 | 30.6 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 0.022 | 0.065 | 0.055 | 0.009 | 0.001 | 0.065–0.065 | 0.049 | 91.4 | 157 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 0.018 | 0.048 | 0.037 | 0.006 | 0.004 | 0.048–0.048 | 0.016 | 36.2 | 157 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.070 | 0.175 | 0.134 | 0.037 | 0.002 | 0.175–0.175 | 0.047 | 21.6 | 169 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.066 | 0.173 | 0.133 | 0.037 | 0.001 | 0.173–0.173 | 0.043 | 21.6 | 169 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.066 | 0.164 | 0.129 | 0.033 | 0.002 | 0.164–0.164 | 0.042 | 21.6 | 169 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.066 | 0.170 | 0.133 | 0.034 | 0.002 | 0.170–0.170 | 0.043 | 21.6 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 0.022 | 0.040 | 0.032 | 0.008 | 0.001 | 0.040–0.040 | 0.016 | 100.3 | 157 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 0.018 | 0.033 | 0.027 | 0.004 | 0.002 | 0.033–0.033 | 0.014 | 45.1 | 157 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.070 | 0.157 | 0.124 | 0.033 | 0.000 | 0.157–0.157 | 0.041 | 30.6 | 169 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.066 | 0.156 | 0.124 | 0.032 | 0.000 | 0.156–0.156 | 0.053 | 30.6 | 169 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.066 | 0.162 | 0.127 | 0.034 | 0.000 | 0.162–0.162 | 0.040 | 30.6 | 169 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.066 | 0.150 | 0.118 | 0.032 | 0.000 | 0.150–0.150 | 0.040 | 30.6 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 0.718 | 2.144 | 1.597 | 0.516 | 0.030 | 2.144–2.144 | 1.981 | 5351.2 | 161 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 30.600 | 0.222 | 0.162 | 0.030 | 0.030 | 0.222–0.222 | 0.086 | 367.9 | 2289 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.593 | 0.656 | 0.485 | 0.169 | 0.001 | 0.656–0.656 | 0.192 | 137.5 | 1030 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.541 | 0.657 | 0.483 | 0.173 | 0.000 | 0.657–0.657 | 0.185 | 137.5 | 1030 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.606 | 0.721 | 0.551 | 0.169 | 0.000 | 0.721–0.721 | 0.199 | 137.5 | 1030 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.549 | 0.653 | 0.483 | 0.169 | 0.000 | 0.653–0.653 | 0.191 | 137.5 | 1030 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 0.718 | 0.984 | 0.437 | 0.512 | 0.034 | 0.984–0.984 | 0.559 | 5360.2 | 161 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 30.600 | 0.230 | 0.171 | 0.031 | 0.028 | 0.230–0.230 | 0.066 | 376.8 | 2289 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.593 | 0.709 | 0.535 | 0.173 | 0.000 | 0.709–0.709 | 0.180 | 146.4 | 1030 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.541 | 0.655 | 0.487 | 0.167 | 0.001 | 0.655–0.655 | 0.184 | 146.4 | 1030 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.606 | 0.676 | 0.503 | 0.172 | 0.000 | 0.676–0.676 | 0.177 | 146.4 | 1030 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.549 | 0.651 | 0.486 | 0.165 | 0.000 | 0.651–0.651 | 0.193 | 146.4 | 1030 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 0.695 | 2.127 | 1.585 | 0.512 | 0.031 | 2.127–2.127 | 1.979 | 5351.2 | 161 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 0.367 | 0.125 | 0.086 | 0.022 | 0.017 | 0.125–0.125 | 0.058 | 215.9 | 1238 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.563 | 0.853 | 0.638 | 0.214 | 0.001 | 0.853–0.853 | 0.248 | 166.9 | 1254 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.539 | 0.880 | 0.662 | 0.217 | 0.000 | 0.880–0.880 | 0.243 | 166.9 | 1254 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.545 | 0.887 | 0.656 | 0.230 | 0.000 | 0.887–0.887 | 0.226 | 166.9 | 1254 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.546 | 0.867 | 0.642 | 0.225 | 0.000 | 0.867–0.867 | 0.245 | 166.9 | 1254 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 0.695 | 0.985 | 0.442 | 0.512 | 0.031 | 0.985–0.985 | 0.555 | 5360.2 | 161 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 0.367 | 0.142 | 0.108 | 0.018 | 0.015 | 0.142–0.142 | 0.051 | 224.9 | 1238 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.563 | 0.927 | 0.694 | 0.231 | 0.001 | 0.927–0.927 | 0.236 | 175.8 | 1254 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.539 | 0.854 | 0.640 | 0.212 | 0.001 | 0.854–0.854 | 0.266 | 175.8 | 1254 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.545 | 0.900 | 0.681 | 0.218 | 0.000 | 0.900–0.900 | 0.226 | 175.8 | 1254 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.546 | 0.867 | 0.651 | 0.216 | 0.000 | 0.867–0.867 | 0.231 | 175.8 | 1254 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 0.566 | 0.062 | 0.051 | 0.010 | 0.001 | 0.062–0.062 | 0.047 | 115.2 | 200 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 0.723 | 0.157 | 0.094 | 0.038 | 0.024 | 0.157–0.157 | 0.053 | 290.7 | 1877 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.583 | 0.178 | 0.133 | 0.032 | 0.013 | 0.178–0.178 | 0.057 | 107.9 | 759 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.571 | 0.176 | 0.128 | 0.031 | 0.016 | 0.176–0.176 | 0.058 | 107.9 | 759 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.537 | 0.286 | 0.115 | 0.161 | 0.009 | 0.286–0.286 | 0.021 | 126.6 | 906 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.516 | 0.194 | 0.116 | 0.068 | 0.009 | 0.194–0.194 | 0.049 | 104.7 | 751 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 0.566 | 0.043 | 0.033 | 0.008 | 0.001 | 0.043–0.043 | 0.016 | 124.1 | 200 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 0.723 | 0.120 | 0.073 | 0.028 | 0.019 | 0.120–0.120 | 0.050 | 299.7 | 1877 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.583 | 0.159 | 0.120 | 0.030 | 0.009 | 0.159–0.159 | 0.045 | 116.8 | 759 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.571 | 0.157 | 0.119 | 0.029 | 0.008 | 0.157–0.157 | 0.048 | 116.8 | 759 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.537 | 0.268 | 0.111 | 0.152 | 0.005 | 0.268–0.268 | 0.020 | 135.6 | 906 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.516 | 0.176 | 0.107 | 0.064 | 0.005 | 0.176–0.176 | 0.050 | 113.6 | 751 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 0.597 | 0.066 | 0.053 | 0.011 | 0.001 | 0.066–0.066 | 0.052 | 115.2 | 200 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 0.211 | 0.164 | 0.110 | 0.035 | 0.018 | 0.164–0.164 | 0.043 | 331.7 | 2075 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.521 | 0.171 | 0.126 | 0.031 | 0.013 | 0.171–0.171 | 0.050 | 92.8 | 664 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.517 | 0.175 | 0.126 | 0.035 | 0.013 | 0.175–0.175 | 0.055 | 92.8 | 664 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.455 | 0.308 | 0.120 | 0.177 | 0.009 | 0.308–0.308 | 0.018 | 120.3 | 824 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.466 | 0.192 | 0.118 | 0.064 | 0.009 | 0.192–0.192 | 0.047 | 91.0 | 656 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 0.597 | 0.041 | 0.031 | 0.009 | 0.001 | 0.041–0.041 | 0.017 | 124.1 | 200 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 0.211 | 0.124 | 0.086 | 0.024 | 0.014 | 0.124–0.124 | 0.040 | 340.7 | 2075 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 0.521 | 0.152 | 0.115 | 0.029 | 0.008 | 0.152–0.152 | 0.045 | 101.8 | 664 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 0.517 | 0.165 | 0.126 | 0.030 | 0.009 | 0.165–0.165 | 0.047 | 101.8 | 664 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 0.455 | 0.280 | 0.110 | 0.166 | 0.005 | 0.280–0.280 | 0.015 | 129.3 | 824 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 0.466 | 0.172 | 0.107 | 0.061 | 0.005 | 0.172–0.172 | 0.042 | 100.0 | 656 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 222.283 | 2.073 | 1.535 | 0.507 | 0.031 | 2.073–2.073 | 2.307 | 17043.7 | 517 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 33.425 | 1.490 | 0.949 | 0.316 | 0.225 | 1.490–1.490 | 0.546 | 5355.9 | 31982 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 6.734 | 0.783 | 0.510 | 0.149 | 0.124 | 0.783–0.783 | 0.302 | 1431.5 | 9539 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 6.597 | 0.792 | 0.513 | 0.153 | 0.125 | 0.792–0.792 | 0.323 | 1431.5 | 9539 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 6.891 | 1.226 | 0.738 | 0.430 | 0.058 | 1.226–1.226 | 0.086 | 1575.5 | 9973 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 7.042 | 1.257 | 0.735 | 0.466 | 0.055 | 1.257–1.257 | 0.363 | 1561.1 | 9815 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 222.283 | 1.029 | 0.493 | 0.505 | 0.030 | 1.029–1.029 | 0.609 | 17052.6 | 517 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 33.425 | 1.434 | 0.873 | 0.305 | 0.255 | 1.434–1.434 | 0.529 | 5364.8 | 31982 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 6.734 | 0.783 | 0.513 | 0.149 | 0.120 | 0.783–0.783 | 0.291 | 1440.5 | 9539 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 6.597 | 0.791 | 0.519 | 0.151 | 0.121 | 0.791–0.791 | 0.288 | 1440.5 | 9539 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 6.891 | 1.233 | 0.736 | 0.442 | 0.053 | 1.233–1.233 | 0.075 | 1584.5 | 9973 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 7.042 | 1.285 | 0.755 | 0.477 | 0.053 | 1.285–1.285 | 0.345 | 1570.0 | 9815 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 230.018 | 2.257 | 1.681 | 0.543 | 0.033 | 2.257–2.257 | 2.116 | 17043.7 | 517 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 1.439 | 0.413 | 0.294 | 0.070 | 0.049 | 0.413–0.413 | 0.117 | 1282.0 | 10792 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 4.458 | 0.952 | 0.629 | 0.199 | 0.123 | 0.952–0.952 | 0.357 | 914.0 | 6457 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 4.494 | 0.967 | 0.634 | 0.202 | 0.129 | 0.967–0.967 | 0.343 | 914.0 | 6457 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 4.620 | 1.578 | 0.826 | 0.699 | 0.052 | 1.578–1.578 | 0.080 | 1083.9 | 7624 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 4.549 | 1.308 | 0.801 | 0.454 | 0.053 | 1.308–1.308 | 0.368 | 902.7 | 7015 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full/staged | joint | 230.018 | 1.221 | 0.627 | 0.561 | 0.032 | 1.221–1.221 | 0.551 | 17052.6 | 517 | 1 |
| packed512/enum/staged/joint/full/staged | joint | 1.439 | 0.362 | 0.251 | 0.062 | 0.048 | 0.362–0.362 | 0.111 | 1291.0 | 10792 | 1 |
| prefix512/slab/ite/witness/local-needed/fused | faces | 4.458 | 0.970 | 0.647 | 0.200 | 0.122 | 0.970–0.970 | 0.338 | 923.0 | 6457 | 1 |
| prefix512/slab/ite/witness/local-needed/staged | faces | 4.494 | 0.933 | 0.620 | 0.194 | 0.119 | 0.933–0.933 | 0.334 | 923.0 | 6457 | 1 |
| retraction512/slab/ite/witness/local-needed/fused | faces | 4.620 | 1.561 | 0.815 | 0.693 | 0.052 | 1.561–1.561 | 0.074 | 1092.9 | 7624 | 1 |
| retraction512/slab/ite/witness/local-needed/staged | faces | 4.549 | 1.315 | 0.807 | 0.456 | 0.051 | 1.315–1.315 | 0.365 | 911.7 | 7015 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

256 configurations; 856 matched comparisons; 81 retained processes.

- [Raw evidence](results/fused-first-smoke.json).
- [Raw evidence](results/fused-domain-controls.json).
- [Raw evidence](results/fused-fallback-controls.json).
- [Raw evidence](results/fused-cutoff-screen.json).
- [Raw evidence](results/fused-normalization-control.json).
- [Raw evidence](results/fused-readout-repeat.json).
- [Raw evidence](results/fused-exception-repeat.json).
