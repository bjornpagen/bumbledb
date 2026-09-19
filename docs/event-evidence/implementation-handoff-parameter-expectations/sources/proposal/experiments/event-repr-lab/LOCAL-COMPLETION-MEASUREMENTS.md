# Same-binary local completion after Free Join

Every query returns 64 Events: original, Possible, Guaranteed and Ambiguous
for sixteen groups. Fresh/warm totals include exact original-support counts.
Every returned Event is checked pointwise and canonically reimported outside
timing. Construction is separate. Bytes estimate retained carrier/cache memory.
They exclude allocator overhead, verification clones, result vectors and the join engine.
Executable: `f2c0c5b44acd678eb5cf51704e510e5c060855216bf4c6b66bfc569f1e9046e1`.

Only entirely successful processes contribute timing rows. Partial rows from
capped or failed processes remain in the raw evidence but cannot supply a
complete-query comparison. A later incomplete process invalidates an older
successful process for that exact job; no failed result is a fast answer.

## fibred, width 4, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 1.314 | 0.087 | 0.072 | 0.014 | 0.001 | 0.075–0.097 | 0.079 | 305.2 | 277 | 3 |
| packed512/enum/staged/joint/full | joint | 1.323 | 0.264 | 0.152 | 0.073 | 0.040 | 0.248–0.303 | 0.080 | 629.8 | 4401 | 3 |
| prefix512/slab/ite/active/full | faces | 1.073 | 0.661 | 0.171 | 0.469 | 0.019 | 0.661–0.661 | 0.051 | 223.7 | 1736 | 1 |
| prefix512/slab/ite/active/local | faces | 1.037 | 0.638 | 0.164 | 0.453 | 0.019 | 0.638–0.638 | 0.051 | 223.7 | 1736 | 1 |
| prefix512/slab/ite/active/local-needed | faces | 1.053 | 0.542 | 0.157 | 0.367 | 0.017 | 0.542–0.542 | 0.051 | 223.7 | 1718 | 1 |
| prefix512/slab/ite/active/needed | faces | 1.043 | 0.549 | 0.164 | 0.366 | 0.017 | 0.549–0.549 | 0.050 | 223.7 | 1718 | 1 |
| prefix512/slab/ite/joint/full | faces | 1.025 | 14.236 | 0.160 | 14.056 | 0.018 | 14.236–14.236 | 0.089 | 3110.9 | 20158 | 1 |
| prefix512/slab/ite/joint/local | faces | 1.031 | 14.817 | 0.159 | 14.636 | 0.020 | 14.817–14.817 | 0.088 | 3110.9 | 20158 | 1 |
| prefix512/slab/ite/joint/local-needed | faces | 1.031 | 4.405 | 0.170 | 4.216 | 0.018 | 4.405–4.405 | 0.102 | 832.0 | 7028 | 1 |
| prefix512/slab/ite/joint/needed | faces | 1.056 | 4.396 | 0.163 | 4.211 | 0.021 | 4.396–4.396 | 0.156 | 832.0 | 7028 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.929 | 0.540 | 0.152 | 0.373 | 0.016 | 0.536–0.557 | 0.047 | 223.7 | 1669 | 3 |
| prefix512/slab/ite/witness/local | faces | 1.515 | 0.713 | 0.276 | 0.460 | 0.014 | 0.684–1.076 | 0.052 | 230.9 | 1700 | 3 |
| prefix512/slab/ite/witness/local-needed | faces | 0.943 | 0.282 | 0.148 | 0.120 | 0.013 | 0.273–0.301 | 0.038 | 213.5 | 1395 | 3 |
| prefix512/slab/ite/witness/needed | faces | 0.924 | 0.498 | 0.152 | 0.333 | 0.013 | 0.478–0.521 | 0.044 | 223.7 | 1713 | 3 |
| prefix512/slab/staged/witness/full | faces | 1.442 | 0.716 | 0.179 | 0.517 | 0.017 | 0.716–0.716 | 0.060 | 335.0 | 2290 | 1 |
| prefix512/slab/staged/witness/local | faces | 1.419 | 0.732 | 0.178 | 0.535 | 0.017 | 0.732–0.732 | 0.057 | 372.7 | 2363 | 1 |
| prefix512/slab/staged/witness/local-needed | faces | 1.376 | 0.402 | 0.170 | 0.214 | 0.017 | 0.402–0.402 | 0.051 | 332.4 | 1959 | 1 |
| prefix512/slab/staged/witness/needed | faces | 1.406 | 0.759 | 0.169 | 0.571 | 0.017 | 0.759–0.759 | 0.057 | 365.5 | 2456 | 1 |
| prefix64/slab/ite/witness/full | faces | 2.528 | 1.748 | 0.427 | 1.229 | 0.087 | 1.748–1.748 | 0.132 | 605.8 | 6831 | 1 |
| prefix64/slab/ite/witness/local-needed | faces | 2.503 | 1.063 | 0.435 | 0.527 | 0.089 | 1.063–1.063 | 0.133 | 603.1 | 5659 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.976 | 0.639 | 0.156 | 0.463 | 0.018 | 0.639–0.639 | 0.044 | 218.2 | 1745 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.954 | 0.632 | 0.155 | 0.459 | 0.017 | 0.632–0.632 | 0.047 | 225.4 | 1777 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 1.001 | 0.387 | 0.160 | 0.209 | 0.017 | 0.387–0.387 | 0.045 | 217.0 | 1436 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.997 | 0.755 | 0.167 | 0.571 | 0.016 | 0.755–0.755 | 0.050 | 308.5 | 1843 | 1 |
| retraction64/slab/ite/witness/full | faces | 2.661 | 2.427 | 0.525 | 1.807 | 0.094 | 2.427–2.427 | 0.133 | 761.1 | 7801 | 1 |
| retraction64/slab/ite/witness/local-needed | faces | 2.796 | 1.430 | 0.558 | 0.772 | 0.098 | 1.430–1.430 | 0.145 | 638.0 | 6160 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 1.314 | 0.044 | 0.031 | 0.011 | 0.001 | 0.041–0.046 | 0.020 | 314.1 | 277 | 3 |
| packed512/enum/staged/joint/full | joint | 1.323 | 0.250 | 0.146 | 0.067 | 0.032 | 0.229–0.250 | 0.074 | 638.7 | 4401 | 3 |
| prefix512/slab/ite/active/full | faces | 1.073 | 0.638 | 0.166 | 0.456 | 0.013 | 0.638–0.638 | 0.049 | 232.7 | 1736 | 1 |
| prefix512/slab/ite/active/local | faces | 1.037 | 0.627 | 0.159 | 0.454 | 0.014 | 0.627–0.627 | 0.048 | 232.7 | 1736 | 1 |
| prefix512/slab/ite/active/local-needed | faces | 1.053 | 0.517 | 0.152 | 0.352 | 0.013 | 0.517–0.517 | 0.040 | 232.7 | 1718 | 1 |
| prefix512/slab/ite/active/needed | faces | 1.043 | 0.514 | 0.150 | 0.352 | 0.012 | 0.514–0.514 | 0.042 | 232.7 | 1718 | 1 |
| prefix512/slab/ite/joint/full | faces | 1.025 | 14.106 | 0.146 | 13.947 | 0.012 | 14.106–14.106 | 0.073 | 3119.8 | 20158 | 1 |
| prefix512/slab/ite/joint/local | faces | 1.031 | 14.296 | 0.150 | 14.133 | 0.013 | 14.296–14.296 | 0.084 | 3119.8 | 20158 | 1 |
| prefix512/slab/ite/joint/local-needed | faces | 1.031 | 4.369 | 0.153 | 4.202 | 0.013 | 4.369–4.369 | 0.081 | 840.9 | 7028 | 1 |
| prefix512/slab/ite/joint/needed | faces | 1.056 | 4.326 | 0.162 | 4.148 | 0.014 | 4.326–4.326 | 0.103 | 840.9 | 7028 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.929 | 0.552 | 0.149 | 0.392 | 0.014 | 0.533–0.568 | 0.034 | 232.7 | 1669 | 3 |
| prefix512/slab/ite/witness/local | faces | 1.515 | 0.648 | 0.163 | 0.471 | 0.013 | 0.578–0.717 | 0.047 | 239.8 | 1700 | 3 |
| prefix512/slab/ite/witness/local-needed | faces | 0.943 | 0.277 | 0.145 | 0.120 | 0.012 | 0.273–0.280 | 0.033 | 222.4 | 1395 | 3 |
| prefix512/slab/ite/witness/needed | faces | 0.924 | 0.496 | 0.152 | 0.327 | 0.012 | 0.486–0.497 | 0.036 | 232.7 | 1713 | 3 |
| prefix512/slab/staged/witness/full | faces | 1.442 | 0.703 | 0.167 | 0.524 | 0.011 | 0.703–0.703 | 0.036 | 344.0 | 2290 | 1 |
| prefix512/slab/staged/witness/local | faces | 1.419 | 0.732 | 0.179 | 0.542 | 0.012 | 0.732–0.732 | 0.046 | 381.6 | 2363 | 1 |
| prefix512/slab/staged/witness/local-needed | faces | 1.376 | 0.362 | 0.152 | 0.198 | 0.012 | 0.362–0.362 | 0.037 | 341.4 | 1959 | 1 |
| prefix512/slab/staged/witness/needed | faces | 1.406 | 0.710 | 0.158 | 0.540 | 0.012 | 0.710–0.710 | 0.055 | 374.5 | 2456 | 1 |
| prefix64/slab/ite/witness/full | faces | 2.528 | 1.670 | 0.408 | 1.181 | 0.080 | 1.670–1.670 | 0.115 | 614.7 | 6831 | 1 |
| prefix64/slab/ite/witness/local-needed | faces | 2.503 | 1.004 | 0.409 | 0.515 | 0.080 | 1.004–1.004 | 0.118 | 612.1 | 5659 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.976 | 0.640 | 0.156 | 0.471 | 0.013 | 0.640–0.640 | 0.044 | 227.2 | 1745 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.954 | 0.620 | 0.149 | 0.458 | 0.013 | 0.620–0.620 | 0.039 | 234.3 | 1777 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 1.001 | 0.369 | 0.155 | 0.202 | 0.012 | 0.369–0.369 | 0.042 | 225.9 | 1436 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.997 | 0.696 | 0.146 | 0.537 | 0.013 | 0.696–0.696 | 0.052 | 317.5 | 1843 | 1 |
| retraction64/slab/ite/witness/full | faces | 2.661 | 2.381 | 0.510 | 1.785 | 0.086 | 2.381–2.381 | 0.135 | 770.1 | 7801 | 1 |
| retraction64/slab/ite/witness/local-needed | faces | 2.796 | 1.334 | 0.526 | 0.719 | 0.088 | 1.334–1.334 | 0.140 | 647.0 | 6160 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 1.309 | 0.083 | 0.067 | 0.015 | 0.001 | 0.075–0.098 | 0.079 | 305.2 | 277 | 3 |
| packed512/enum/staged/joint/full | joint | 0.280 | 0.278 | 0.179 | 0.048 | 0.036 | 0.245–0.287 | 0.070 | 638.9 | 3967 | 3 |
| prefix512/slab/ite/active/full | faces | 1.002 | 0.655 | 0.164 | 0.469 | 0.017 | 0.655–0.655 | 0.044 | 262.8 | 1727 | 1 |
| prefix512/slab/ite/active/local | faces | 0.983 | 0.634 | 0.160 | 0.457 | 0.016 | 0.634–0.634 | 0.040 | 262.8 | 1727 | 1 |
| prefix512/slab/ite/active/local-needed | faces | 1.022 | 0.539 | 0.169 | 0.351 | 0.017 | 0.539–0.539 | 0.047 | 210.6 | 1667 | 1 |
| prefix512/slab/ite/active/needed | faces | 0.999 | 0.514 | 0.160 | 0.336 | 0.017 | 0.514–0.514 | 0.040 | 210.6 | 1667 | 1 |
| prefix512/slab/ite/joint/full | faces | 0.998 | 1.701 | 0.163 | 1.516 | 0.020 | 1.701–1.701 | 0.231 | 458.4 | 3653 | 1 |
| prefix512/slab/ite/joint/local | faces | 1.041 | 1.753 | 0.165 | 1.569 | 0.017 | 1.753–1.753 | 0.248 | 458.4 | 3653 | 1 |
| prefix512/slab/ite/joint/local-needed | faces | 1.015 | 3.025 | 0.161 | 2.844 | 0.018 | 3.025–3.025 | 0.240 | 779.4 | 4987 | 1 |
| prefix512/slab/ite/joint/needed | faces | 1.003 | 2.989 | 0.162 | 2.810 | 0.016 | 2.989–2.989 | 0.243 | 779.4 | 4987 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.921 | 0.570 | 0.157 | 0.400 | 0.012 | 0.549–0.602 | 0.043 | 210.6 | 1639 | 3 |
| prefix512/slab/ite/witness/local | faces | 0.906 | 0.520 | 0.145 | 0.361 | 0.012 | 0.508–0.536 | 0.040 | 217.7 | 1639 | 3 |
| prefix512/slab/ite/witness/local-needed | faces | 0.911 | 0.273 | 0.144 | 0.116 | 0.012 | 0.263–0.293 | 0.037 | 200.3 | 1346 | 3 |
| prefix512/slab/ite/witness/needed | faces | 0.894 | 0.469 | 0.143 | 0.318 | 0.012 | 0.464–0.497 | 0.036 | 210.6 | 1675 | 3 |
| prefix512/slab/staged/witness/full | faces | 1.376 | 0.725 | 0.166 | 0.542 | 0.016 | 0.725–0.725 | 0.047 | 315.8 | 2274 | 1 |
| prefix512/slab/staged/witness/local | faces | 1.393 | 0.699 | 0.174 | 0.506 | 0.018 | 0.699–0.699 | 0.043 | 323.0 | 2283 | 1 |
| prefix512/slab/staged/witness/local-needed | faces | 1.367 | 0.385 | 0.167 | 0.200 | 0.016 | 0.385–0.385 | 0.045 | 313.2 | 1889 | 1 |
| prefix512/slab/staged/witness/needed | faces | 1.353 | 0.718 | 0.171 | 0.528 | 0.017 | 0.718–0.718 | 0.050 | 346.3 | 2394 | 1 |
| prefix64/slab/ite/witness/full | faces | 1.543 | 1.522 | 0.439 | 0.993 | 0.089 | 1.522–1.522 | 0.271 | 444.4 | 5019 | 1 |
| prefix64/slab/ite/witness/local-needed | faces | 1.563 | 1.179 | 0.437 | 0.656 | 0.085 | 1.179–1.179 | 0.250 | 441.8 | 4282 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.956 | 0.682 | 0.156 | 0.504 | 0.020 | 0.682–0.682 | 0.046 | 258.8 | 1721 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.924 | 0.638 | 0.152 | 0.467 | 0.017 | 0.638–0.638 | 0.042 | 214.5 | 1717 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 0.965 | 0.423 | 0.199 | 0.207 | 0.016 | 0.423–0.423 | 0.041 | 206.1 | 1393 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.942 | 0.644 | 0.146 | 0.478 | 0.018 | 0.644–0.644 | 0.042 | 207.3 | 1743 | 1 |
| retraction64/slab/ite/witness/full | faces | 1.649 | 2.079 | 0.526 | 1.460 | 0.091 | 2.079–2.079 | 0.256 | 701.9 | 5731 | 1 |
| retraction64/slab/ite/witness/local-needed | faces | 1.615 | 1.419 | 0.521 | 0.805 | 0.091 | 1.419–1.419 | 0.268 | 445.3 | 4746 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 1.309 | 0.044 | 0.030 | 0.012 | 0.001 | 0.042–0.046 | 0.020 | 314.1 | 277 | 3 |
| packed512/enum/staged/joint/full | joint | 0.280 | 0.222 | 0.144 | 0.044 | 0.029 | 0.215–0.247 | 0.078 | 647.8 | 3967 | 3 |
| prefix512/slab/ite/active/full | faces | 1.002 | 0.611 | 0.152 | 0.447 | 0.012 | 0.611–0.611 | 0.039 | 271.8 | 1727 | 1 |
| prefix512/slab/ite/active/local | faces | 0.983 | 0.602 | 0.145 | 0.444 | 0.012 | 0.602–0.602 | 0.033 | 271.8 | 1727 | 1 |
| prefix512/slab/ite/active/local-needed | faces | 1.022 | 0.480 | 0.147 | 0.321 | 0.012 | 0.480–0.480 | 0.041 | 219.5 | 1667 | 1 |
| prefix512/slab/ite/active/needed | faces | 0.999 | 0.491 | 0.147 | 0.331 | 0.012 | 0.491–0.491 | 0.033 | 219.5 | 1667 | 1 |
| prefix512/slab/ite/joint/full | faces | 0.998 | 1.588 | 0.151 | 1.425 | 0.012 | 1.588–1.588 | 0.225 | 467.4 | 3653 | 1 |
| prefix512/slab/ite/joint/local | faces | 1.041 | 1.755 | 0.163 | 1.575 | 0.016 | 1.755–1.755 | 0.233 | 467.4 | 3653 | 1 |
| prefix512/slab/ite/joint/local-needed | faces | 1.015 | 2.990 | 0.149 | 2.829 | 0.012 | 2.990–2.990 | 0.245 | 788.4 | 4987 | 1 |
| prefix512/slab/ite/joint/needed | faces | 1.003 | 2.997 | 0.163 | 2.818 | 0.015 | 2.997–2.997 | 0.226 | 788.4 | 4987 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.921 | 0.570 | 0.157 | 0.395 | 0.012 | 0.555–0.575 | 0.046 | 219.5 | 1639 | 3 |
| prefix512/slab/ite/witness/local | faces | 0.906 | 0.521 | 0.145 | 0.363 | 0.012 | 0.511–0.554 | 0.044 | 226.7 | 1639 | 3 |
| prefix512/slab/ite/witness/local-needed | faces | 0.911 | 0.267 | 0.144 | 0.112 | 0.012 | 0.263–0.280 | 0.033 | 209.3 | 1346 | 3 |
| prefix512/slab/ite/witness/needed | faces | 0.894 | 0.467 | 0.147 | 0.308 | 0.011 | 0.457–0.489 | 0.034 | 219.5 | 1675 | 3 |
| prefix512/slab/staged/witness/full | faces | 1.376 | 0.700 | 0.151 | 0.537 | 0.012 | 0.700–0.700 | 0.035 | 324.8 | 2274 | 1 |
| prefix512/slab/staged/witness/local | faces | 1.393 | 0.663 | 0.156 | 0.495 | 0.012 | 0.663–0.663 | 0.034 | 332.0 | 2283 | 1 |
| prefix512/slab/staged/witness/local-needed | faces | 1.367 | 0.354 | 0.150 | 0.192 | 0.012 | 0.354–0.354 | 0.039 | 322.2 | 1889 | 1 |
| prefix512/slab/staged/witness/needed | faces | 1.353 | 0.663 | 0.148 | 0.503 | 0.012 | 0.663–0.663 | 0.147 | 355.3 | 2394 | 1 |
| prefix64/slab/ite/witness/full | faces | 1.543 | 1.490 | 0.440 | 0.963 | 0.085 | 1.490–1.490 | 0.241 | 453.4 | 5019 | 1 |
| prefix64/slab/ite/witness/local-needed | faces | 1.563 | 1.141 | 0.427 | 0.634 | 0.079 | 1.141–1.141 | 0.230 | 450.8 | 4282 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.956 | 0.659 | 0.150 | 0.496 | 0.013 | 0.659–0.659 | 0.038 | 267.7 | 1721 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.924 | 0.627 | 0.149 | 0.465 | 0.012 | 0.627–0.627 | 0.035 | 223.5 | 1717 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 0.965 | 0.390 | 0.158 | 0.220 | 0.012 | 0.390–0.390 | 0.033 | 215.1 | 1393 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.942 | 0.622 | 0.142 | 0.467 | 0.012 | 0.622–0.622 | 0.034 | 216.3 | 1743 | 1 |
| retraction64/slab/ite/witness/full | faces | 1.649 | 1.981 | 0.490 | 1.408 | 0.083 | 1.981–1.981 | 0.257 | 710.9 | 5731 | 1 |
| retraction64/slab/ite/witness/local-needed | faces | 1.615 | 1.396 | 0.517 | 0.785 | 0.094 | 1.396–1.396 | 0.249 | 454.3 | 4746 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 1097.248 | 3.512 | 2.880 | 0.567 | 0.065 | 3.505–4.091 | 3.826 | 41852.6 | 636 | 3 |
| packed512/enum/staged/joint/full | joint | 66.663 | 1.430 | 0.654 | 0.561 | 0.222 | 1.424–1.513 | 0.763 | 6651.9 | 47190 | 3 |
| prefix512/slab/ite/active/full | faces | 14.807 | 12.925 | 1.046 | 11.728 | 0.151 | 12.925–12.925 | 0.229 | 4318.9 | 37611 | 1 |
| prefix512/slab/ite/active/local | faces | 14.236 | 12.490 | 1.023 | 11.328 | 0.137 | 12.490–12.490 | 0.217 | 4318.9 | 37611 | 1 |
| prefix512/slab/ite/active/local-needed | faces | 14.496 | 3.522 | 1.025 | 2.364 | 0.132 | 3.522–3.522 | 0.221 | 3344.1 | 24799 | 1 |
| prefix512/slab/ite/active/needed | faces | 14.628 | 3.603 | 1.037 | 2.425 | 0.141 | 3.603–3.603 | 0.209 | 3344.1 | 24799 | 1 |
| prefix512/slab/ite/joint/full | faces | 14.384 | 376.032 | 1.019 | 374.844 | 0.169 | 376.032–376.032 | 0.271 | 83841.7 | 708405 | 1 |
| prefix512/slab/ite/joint/local | faces | 14.433 | 397.420 | 1.033 | 396.198 | 0.188 | 397.420–397.420 | 0.277 | 83841.7 | 708405 | 1 |
| prefix512/slab/ite/joint/local-needed | faces | 15.561 | 33.250 | 1.045 | 32.051 | 0.153 | 33.250–33.250 | 0.268 | 8271.8 | 70052 | 1 |
| prefix512/slab/ite/joint/needed | faces | 15.427 | 35.616 | 1.020 | 34.449 | 0.146 | 35.616–35.616 | 0.263 | 8271.8 | 70052 | 1 |
| prefix512/slab/ite/witness/full | faces | 14.625 | 10.847 | 0.976 | 9.665 | 0.131 | 10.649–11.057 | 0.214 | 4318.9 | 35380 | 3 |
| prefix512/slab/ite/witness/local | faces | 17.344 | 4.473 | 1.197 | 3.175 | 0.140 | 4.379–4.523 | 0.276 | 3366.5 | 24372 | 3 |
| prefix512/slab/ite/witness/local-needed | faces | 14.277 | 2.211 | 0.983 | 1.101 | 0.129 | 2.205–2.285 | 0.200 | 3345.5 | 22748 | 3 |
| prefix512/slab/ite/witness/needed | faces | 14.302 | 3.466 | 1.002 | 2.298 | 0.133 | 3.405–3.507 | 0.197 | 3344.1 | 24840 | 3 |
| prefix512/slab/staged/witness/full | faces | 25.531 | 15.912 | 1.099 | 14.674 | 0.139 | 15.912–15.912 | 0.224 | 8076.1 | 61226 | 1 |
| prefix512/slab/staged/witness/local | faces | 24.558 | 4.126 | 1.089 | 2.901 | 0.136 | 4.126–4.126 | 0.212 | 6148.8 | 43704 | 1 |
| prefix512/slab/staged/witness/local-needed | faces | 24.693 | 2.764 | 1.073 | 1.558 | 0.133 | 2.764–2.764 | 0.207 | 6127.8 | 41745 | 1 |
| prefix512/slab/staged/witness/needed | faces | 24.949 | 4.622 | 1.051 | 3.439 | 0.131 | 4.622–4.622 | 0.217 | 6126.4 | 45167 | 1 |
| prefix64/slab/ite/witness/full | faces | 16.599 | 15.579 | 1.335 | 13.941 | 0.302 | 15.579–15.579 | 0.441 | 6948.9 | 80231 | 1 |
| prefix64/slab/ite/witness/local-needed | faces | 16.841 | 3.127 | 1.298 | 1.523 | 0.305 | 3.127–3.127 | 0.374 | 4876.9 | 49365 | 1 |
| retraction512/slab/ite/witness/full | faces | 15.228 | 15.580 | 1.380 | 14.036 | 0.164 | 15.580–15.580 | 0.270 | 6622.8 | 41728 | 1 |
| retraction512/slab/ite/witness/local | faces | 15.556 | 4.679 | 1.406 | 3.115 | 0.159 | 4.679–4.679 | 0.251 | 3478.3 | 26131 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 15.584 | 4.915 | 1.428 | 3.321 | 0.165 | 4.915–4.915 | 0.236 | 3444.7 | 26977 | 1 |
| retraction512/slab/ite/witness/needed | faces | 15.670 | 10.385 | 1.403 | 8.825 | 0.156 | 10.385–10.385 | 0.234 | 4424.5 | 34132 | 1 |
| retraction64/slab/ite/witness/full | faces | 24.228 | 23.450 | 2.043 | 20.914 | 0.493 | 23.450–23.450 | 0.588 | 9126.5 | 96018 | 1 |
| retraction64/slab/ite/witness/local-needed | faces | 24.994 | 8.001 | 2.108 | 5.397 | 0.495 | 8.001–8.001 | 0.575 | 7293.8 | 61543 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 1097.248 | 1.495 | 0.928 | 0.498 | 0.067 | 1.494–1.495 | 0.558 | 41861.5 | 636 | 3 |
| packed512/enum/staged/joint/full | joint | 66.663 | 1.587 | 0.715 | 0.596 | 0.236 | 1.442–1.599 | 0.757 | 6660.8 | 47190 | 3 |
| prefix512/slab/ite/active/full | faces | 14.807 | 12.614 | 1.038 | 11.438 | 0.137 | 12.614–12.614 | 0.215 | 4327.9 | 37611 | 1 |
| prefix512/slab/ite/active/local | faces | 14.236 | 12.641 | 1.077 | 11.434 | 0.128 | 12.641–12.641 | 0.241 | 4327.9 | 37611 | 1 |
| prefix512/slab/ite/active/local-needed | faces | 14.496 | 3.562 | 1.011 | 2.416 | 0.134 | 3.562–3.562 | 0.217 | 3353.0 | 24799 | 1 |
| prefix512/slab/ite/active/needed | faces | 14.628 | 3.600 | 1.041 | 2.422 | 0.136 | 3.600–3.600 | 0.203 | 3353.0 | 24799 | 1 |
| prefix512/slab/ite/joint/full | faces | 14.384 | 380.162 | 1.052 | 378.924 | 0.185 | 380.162–380.162 | 0.270 | 83850.7 | 708405 | 1 |
| prefix512/slab/ite/joint/local | faces | 14.433 | 389.996 | 1.001 | 388.791 | 0.203 | 389.996–389.996 | 0.275 | 83850.7 | 708405 | 1 |
| prefix512/slab/ite/joint/local-needed | faces | 15.561 | 33.273 | 1.029 | 32.110 | 0.133 | 33.273–33.273 | 0.257 | 8280.8 | 70052 | 1 |
| prefix512/slab/ite/joint/needed | faces | 15.427 | 34.088 | 1.005 | 32.948 | 0.134 | 34.088–34.088 | 0.258 | 8280.8 | 70052 | 1 |
| prefix512/slab/ite/witness/full | faces | 14.625 | 10.659 | 0.986 | 9.548 | 0.130 | 10.648–10.765 | 0.202 | 4327.9 | 35380 | 3 |
| prefix512/slab/ite/witness/local | faces | 17.344 | 3.366 | 1.021 | 2.209 | 0.136 | 3.309–4.417 | 0.198 | 3375.4 | 24372 | 3 |
| prefix512/slab/ite/witness/local-needed | faces | 14.277 | 2.247 | 0.994 | 1.123 | 0.130 | 2.233–2.255 | 0.191 | 3354.4 | 22748 | 3 |
| prefix512/slab/ite/witness/needed | faces | 14.302 | 3.373 | 0.979 | 2.273 | 0.129 | 3.359–3.472 | 0.185 | 3353.0 | 24840 | 3 |
| prefix512/slab/staged/witness/full | faces | 25.531 | 15.844 | 1.051 | 14.658 | 0.134 | 15.844–15.844 | 0.204 | 8085.0 | 61226 | 1 |
| prefix512/slab/staged/witness/local | faces | 24.558 | 4.127 | 1.077 | 2.912 | 0.137 | 4.127–4.127 | 0.197 | 6157.7 | 43704 | 1 |
| prefix512/slab/staged/witness/local-needed | faces | 24.693 | 2.701 | 1.024 | 1.549 | 0.128 | 2.701–2.701 | 0.190 | 6136.7 | 41745 | 1 |
| prefix512/slab/staged/witness/needed | faces | 24.949 | 4.928 | 1.188 | 3.610 | 0.128 | 4.928–4.928 | 0.196 | 6135.3 | 45167 | 1 |
| prefix64/slab/ite/witness/full | faces | 16.599 | 17.201 | 1.371 | 15.496 | 0.333 | 17.201–17.201 | 0.386 | 6957.9 | 80231 | 1 |
| prefix64/slab/ite/witness/local-needed | faces | 16.841 | 3.161 | 1.296 | 1.562 | 0.302 | 3.161–3.161 | 0.373 | 4885.9 | 49365 | 1 |
| retraction512/slab/ite/witness/full | faces | 15.228 | 15.465 | 1.341 | 13.961 | 0.163 | 15.465–15.465 | 0.263 | 6631.8 | 41728 | 1 |
| retraction512/slab/ite/witness/local | faces | 15.556 | 4.668 | 1.364 | 3.131 | 0.173 | 4.668–4.668 | 0.236 | 3487.2 | 26131 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 15.584 | 4.860 | 1.380 | 3.321 | 0.158 | 4.860–4.860 | 0.231 | 3453.6 | 26977 | 1 |
| retraction512/slab/ite/witness/needed | faces | 15.670 | 10.382 | 1.331 | 8.894 | 0.156 | 10.382–10.382 | 0.224 | 4433.4 | 34132 | 1 |
| retraction64/slab/ite/witness/full | faces | 24.228 | 24.527 | 2.139 | 21.895 | 0.492 | 24.527–24.527 | 0.579 | 9135.5 | 96018 | 1 |
| retraction64/slab/ite/witness/local-needed | faces | 24.994 | 7.801 | 2.047 | 5.264 | 0.490 | 7.801–7.801 | 0.560 | 7302.7 | 61543 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 468.321 | 3.384 | 2.784 | 0.554 | 0.061 | 3.372–3.924 | 3.676 | 41852.6 | 636 | 3 |
| packed512/enum/staged/joint/full | joint | 2.243 | 0.581 | 0.401 | 0.108 | 0.077 | 0.563–0.667 | 0.215 | 2402.4 | 18095 | 3 |
| prefix512/slab/ite/active/full | faces | 10.442 | 7.476 | 1.250 | 6.086 | 0.136 | 7.476–7.476 | 0.553 | 2764.0 | 26325 | 1 |
| prefix512/slab/ite/active/local | faces | 10.192 | 7.098 | 1.194 | 5.770 | 0.133 | 7.098–7.098 | 0.549 | 2764.0 | 26325 | 1 |
| prefix512/slab/ite/active/local-needed | faces | 11.031 | 3.861 | 1.292 | 2.435 | 0.133 | 3.861–3.861 | 0.532 | 2382.3 | 19938 | 1 |
| prefix512/slab/ite/active/needed | faces | 10.571 | 3.888 | 1.254 | 2.488 | 0.145 | 3.888–3.888 | 0.568 | 2382.3 | 19938 | 1 |
| prefix512/slab/ite/joint/full | faces | 10.359 | 8.873 | 1.189 | 7.549 | 0.135 | 8.873–8.873 | 0.730 | 4092.4 | 31114 | 1 |
| prefix512/slab/ite/joint/local | faces | 11.313 | 10.473 | 1.218 | 9.102 | 0.152 | 10.473–10.473 | 0.773 | 4092.4 | 31114 | 1 |
| prefix512/slab/ite/joint/local-needed | faces | 10.832 | 8.826 | 1.234 | 7.449 | 0.142 | 8.826–8.826 | 0.752 | 4336.1 | 31308 | 1 |
| prefix512/slab/ite/joint/needed | faces | 10.619 | 8.819 | 1.219 | 7.470 | 0.130 | 8.819–8.819 | 0.740 | 4336.1 | 31308 | 1 |
| prefix512/slab/ite/witness/full | faces | 10.820 | 6.051 | 1.238 | 4.688 | 0.126 | 5.961–6.330 | 0.488 | 2642.1 | 24270 | 3 |
| prefix512/slab/ite/witness/local | faces | 10.441 | 3.153 | 1.199 | 1.826 | 0.129 | 3.118–3.183 | 0.451 | 2404.7 | 19093 | 3 |
| prefix512/slab/ite/witness/local-needed | faces | 10.170 | 2.693 | 1.190 | 1.372 | 0.129 | 2.644–2.716 | 0.452 | 2383.7 | 18579 | 3 |
| prefix512/slab/ite/witness/needed | faces | 10.619 | 3.585 | 1.187 | 2.268 | 0.130 | 3.493–3.657 | 0.458 | 2382.3 | 19986 | 3 |
| prefix512/slab/staged/witness/full | faces | 18.454 | 9.143 | 1.287 | 7.724 | 0.131 | 9.143–9.143 | 0.474 | 5060.4 | 46993 | 1 |
| prefix512/slab/staged/witness/local | faces | 19.473 | 3.944 | 1.318 | 2.491 | 0.134 | 3.944–3.944 | 0.471 | 5066.6 | 36232 | 1 |
| prefix512/slab/staged/witness/local-needed | faces | 18.359 | 3.213 | 1.301 | 1.776 | 0.136 | 3.213–3.213 | 0.465 | 5045.6 | 35515 | 1 |
| prefix512/slab/staged/witness/needed | faces | 18.150 | 4.633 | 1.292 | 3.214 | 0.127 | 4.633–4.633 | 0.470 | 5044.2 | 37832 | 1 |
| prefix64/slab/ite/witness/full | faces | 6.663 | 5.780 | 1.789 | 3.586 | 0.405 | 5.780–5.780 | 1.283 | 2778.4 | 29209 | 1 |
| prefix64/slab/ite/witness/local-needed | faces | 6.765 | 4.791 | 1.805 | 2.591 | 0.395 | 4.791–4.791 | 1.269 | 2337.2 | 23653 | 1 |
| retraction512/slab/ite/witness/full | faces | 12.285 | 10.005 | 1.590 | 8.246 | 0.167 | 10.005–10.005 | 0.586 | 3206.2 | 29504 | 1 |
| retraction512/slab/ite/witness/local | faces | 11.409 | 3.930 | 1.464 | 2.304 | 0.161 | 3.930–3.930 | 0.583 | 2747.5 | 20629 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 11.463 | 4.553 | 1.494 | 2.896 | 0.164 | 4.553–4.553 | 0.578 | 2713.9 | 22363 | 1 |
| retraction512/slab/ite/witness/needed | faces | 11.403 | 7.191 | 1.479 | 5.550 | 0.162 | 7.191–7.191 | 0.635 | 2718.8 | 25826 | 1 |
| retraction64/slab/ite/witness/full | faces | 7.575 | 8.096 | 1.979 | 5.648 | 0.468 | 8.096–8.096 | 1.623 | 3443.7 | 38368 | 1 |
| retraction64/slab/ite/witness/local-needed | faces | 7.557 | 7.334 | 1.920 | 4.944 | 0.469 | 7.334–7.334 | 1.532 | 3160.5 | 32988 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 468.321 | 1.438 | 0.886 | 0.485 | 0.062 | 1.422–1.455 | 0.522 | 41861.5 | 636 | 3 |
| packed512/enum/staged/joint/full | joint | 2.243 | 0.638 | 0.462 | 0.101 | 0.073 | 0.563–0.644 | 0.201 | 2411.3 | 18095 | 3 |
| prefix512/slab/ite/active/full | faces | 10.442 | 7.883 | 1.253 | 6.496 | 0.133 | 7.883–7.883 | 0.545 | 2773.0 | 26325 | 1 |
| prefix512/slab/ite/active/local | faces | 10.192 | 7.000 | 1.178 | 5.696 | 0.126 | 7.000–7.000 | 0.523 | 2773.0 | 26325 | 1 |
| prefix512/slab/ite/active/local-needed | faces | 11.031 | 3.744 | 1.220 | 2.397 | 0.127 | 3.744–3.744 | 0.520 | 2391.3 | 19938 | 1 |
| prefix512/slab/ite/active/needed | faces | 10.571 | 3.724 | 1.209 | 2.385 | 0.129 | 3.724–3.724 | 0.539 | 2391.3 | 19938 | 1 |
| prefix512/slab/ite/joint/full | faces | 10.359 | 8.515 | 1.189 | 7.194 | 0.130 | 8.515–8.515 | 0.707 | 4101.3 | 31114 | 1 |
| prefix512/slab/ite/joint/local | faces | 11.313 | 8.876 | 1.210 | 7.536 | 0.129 | 8.876–8.876 | 0.747 | 4101.3 | 31114 | 1 |
| prefix512/slab/ite/joint/local-needed | faces | 10.832 | 9.178 | 1.283 | 7.755 | 0.139 | 9.178–9.178 | 0.745 | 4345.1 | 31308 | 1 |
| prefix512/slab/ite/joint/needed | faces | 10.619 | 8.670 | 1.228 | 7.314 | 0.127 | 8.670–8.670 | 0.713 | 4345.1 | 31308 | 1 |
| prefix512/slab/ite/witness/full | faces | 10.820 | 6.006 | 1.189 | 4.693 | 0.132 | 5.996–6.264 | 0.454 | 2651.1 | 24270 | 3 |
| prefix512/slab/ite/witness/local | faces | 10.441 | 3.120 | 1.181 | 1.813 | 0.125 | 3.101–3.265 | 0.444 | 2413.7 | 19093 | 3 |
| prefix512/slab/ite/witness/local-needed | faces | 10.170 | 2.655 | 1.173 | 1.356 | 0.125 | 2.641–2.687 | 0.437 | 2392.7 | 18579 | 3 |
| prefix512/slab/ite/witness/needed | faces | 10.619 | 3.634 | 1.220 | 2.281 | 0.130 | 3.509–3.673 | 0.489 | 2391.3 | 19986 | 3 |
| prefix512/slab/staged/witness/full | faces | 18.454 | 9.113 | 1.250 | 7.736 | 0.126 | 9.113–9.113 | 0.468 | 5069.3 | 46993 | 1 |
| prefix512/slab/staged/witness/local | faces | 19.473 | 4.039 | 1.261 | 2.648 | 0.129 | 4.039–4.039 | 0.464 | 5075.6 | 36232 | 1 |
| prefix512/slab/staged/witness/local-needed | faces | 18.359 | 3.107 | 1.231 | 1.750 | 0.126 | 3.107–3.107 | 0.453 | 5054.6 | 35515 | 1 |
| prefix512/slab/staged/witness/needed | faces | 18.150 | 4.486 | 1.237 | 3.124 | 0.125 | 4.486–4.486 | 0.478 | 5053.2 | 37832 | 1 |
| prefix64/slab/ite/witness/full | faces | 6.663 | 5.731 | 1.776 | 3.553 | 0.401 | 5.731–5.731 | 1.285 | 2787.4 | 29209 | 1 |
| prefix64/slab/ite/witness/local-needed | faces | 6.765 | 4.902 | 1.826 | 2.677 | 0.398 | 4.902–4.902 | 1.279 | 2346.2 | 23653 | 1 |
| retraction512/slab/ite/witness/full | faces | 12.285 | 9.466 | 1.418 | 7.894 | 0.154 | 9.466–9.466 | 0.604 | 3215.2 | 29504 | 1 |
| retraction512/slab/ite/witness/local | faces | 11.409 | 3.878 | 1.438 | 2.285 | 0.154 | 3.878–3.878 | 0.585 | 2756.5 | 20629 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 11.463 | 4.508 | 1.442 | 2.915 | 0.151 | 4.508–4.508 | 0.571 | 2722.9 | 22363 | 1 |
| retraction512/slab/ite/witness/needed | faces | 11.403 | 6.956 | 1.427 | 5.375 | 0.153 | 6.956–6.956 | 0.585 | 2727.8 | 25826 | 1 |
| retraction64/slab/ite/witness/full | faces | 7.575 | 8.227 | 1.996 | 5.772 | 0.459 | 8.227–8.227 | 1.590 | 3452.6 | 38368 | 1 |
| retraction64/slab/ite/witness/local-needed | faces | 7.557 | 6.873 | 1.903 | 4.516 | 0.453 | 6.873–6.873 | 1.480 | 3169.4 | 32988 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## full, width 4, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 0.024 | 0.071 | 0.057 | 0.011 | 0.001 | 0.071–0.071 | 0.048 | 91.4 | 157 | 1 |
| packed512/enum/staged/joint/full | joint | 0.558 | 0.112 | 0.085 | 0.014 | 0.011 | 0.112–0.112 | 0.032 | 105.0 | 590 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.071 | 0.178 | 0.140 | 0.035 | 0.002 | 0.178–0.178 | 0.045 | 21.6 | 169 | 1 |
| prefix512/slab/ite/witness/local | faces | 0.072 | 0.179 | 0.138 | 0.038 | 0.002 | 0.179–0.179 | 0.047 | 21.6 | 169 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 0.066 | 0.170 | 0.132 | 0.036 | 0.001 | 0.170–0.170 | 0.043 | 21.6 | 169 | 1 |
| prefix512/slab/ite/witness/needed | faces | 0.072 | 0.180 | 0.140 | 0.037 | 0.002 | 0.180–0.180 | 0.042 | 21.6 | 169 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.067 | 0.170 | 0.134 | 0.034 | 0.001 | 0.170–0.170 | 0.042 | 21.6 | 169 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.068 | 0.181 | 0.142 | 0.036 | 0.002 | 0.181–0.181 | 0.042 | 21.6 | 169 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 0.069 | 0.178 | 0.129 | 0.046 | 0.002 | 0.178–0.178 | 0.042 | 21.6 | 169 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.064 | 0.172 | 0.133 | 0.036 | 0.001 | 0.172–0.172 | 0.043 | 21.6 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 0.024 | 0.041 | 0.032 | 0.008 | 0.001 | 0.041–0.041 | 0.018 | 100.3 | 157 | 1 |
| packed512/enum/staged/joint/full | joint | 0.558 | 0.084 | 0.065 | 0.010 | 0.009 | 0.084–0.084 | 0.025 | 114.0 | 590 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.071 | 0.152 | 0.120 | 0.032 | 0.000 | 0.152–0.152 | 0.040 | 30.5 | 169 | 1 |
| prefix512/slab/ite/witness/local | faces | 0.072 | 0.153 | 0.119 | 0.033 | 0.000 | 0.153–0.153 | 0.042 | 30.5 | 169 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 0.066 | 0.150 | 0.118 | 0.031 | 0.000 | 0.150–0.150 | 0.040 | 30.5 | 169 | 1 |
| prefix512/slab/ite/witness/needed | faces | 0.072 | 0.155 | 0.124 | 0.031 | 0.000 | 0.155–0.155 | 0.038 | 30.5 | 169 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.067 | 0.148 | 0.118 | 0.030 | 0.000 | 0.148–0.148 | 0.042 | 30.5 | 169 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.068 | 0.147 | 0.117 | 0.030 | 0.000 | 0.147–0.147 | 0.038 | 30.5 | 169 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 0.069 | 0.145 | 0.114 | 0.031 | 0.000 | 0.145–0.145 | 0.039 | 30.5 | 169 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.064 | 0.148 | 0.116 | 0.031 | 0.000 | 0.148–0.148 | 0.039 | 30.5 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 0.025 | 0.072 | 0.058 | 0.009 | 0.001 | 0.072–0.072 | 0.048 | 91.4 | 157 | 1 |
| packed512/enum/staged/joint/full | joint | 0.021 | 0.054 | 0.041 | 0.008 | 0.004 | 0.054–0.054 | 0.016 | 36.2 | 157 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.066 | 0.172 | 0.133 | 0.036 | 0.002 | 0.172–0.172 | 0.043 | 21.6 | 169 | 1 |
| prefix512/slab/ite/witness/local | faces | 0.063 | 0.173 | 0.134 | 0.036 | 0.002 | 0.173–0.173 | 0.043 | 21.6 | 169 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 0.066 | 0.179 | 0.138 | 0.037 | 0.002 | 0.179–0.179 | 0.046 | 21.6 | 169 | 1 |
| prefix512/slab/ite/witness/needed | faces | 0.066 | 0.175 | 0.131 | 0.036 | 0.006 | 0.175–0.175 | 0.043 | 21.6 | 169 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.065 | 0.176 | 0.135 | 0.038 | 0.002 | 0.176–0.176 | 0.042 | 21.6 | 169 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.065 | 0.170 | 0.134 | 0.033 | 0.001 | 0.170–0.170 | 0.042 | 21.6 | 169 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 0.075 | 0.178 | 0.138 | 0.034 | 0.002 | 0.178–0.178 | 0.043 | 21.6 | 169 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.065 | 0.168 | 0.132 | 0.033 | 0.001 | 0.168–0.168 | 0.042 | 21.6 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 0.025 | 0.037 | 0.029 | 0.008 | 0.001 | 0.037–0.037 | 0.017 | 100.3 | 157 | 1 |
| packed512/enum/staged/joint/full | joint | 0.021 | 0.034 | 0.027 | 0.004 | 0.003 | 0.034–0.034 | 0.016 | 45.1 | 157 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.066 | 0.150 | 0.118 | 0.031 | 0.000 | 0.150–0.150 | 0.040 | 30.5 | 169 | 1 |
| prefix512/slab/ite/witness/local | faces | 0.063 | 0.153 | 0.121 | 0.032 | 0.000 | 0.153–0.153 | 0.039 | 30.5 | 169 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 0.066 | 0.152 | 0.121 | 0.031 | 0.000 | 0.152–0.152 | 0.039 | 30.5 | 169 | 1 |
| prefix512/slab/ite/witness/needed | faces | 0.066 | 0.152 | 0.120 | 0.032 | 0.000 | 0.152–0.152 | 0.039 | 30.5 | 169 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.065 | 0.154 | 0.123 | 0.030 | 0.000 | 0.154–0.154 | 0.038 | 30.5 | 169 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.065 | 0.150 | 0.118 | 0.031 | 0.000 | 0.150–0.150 | 0.038 | 30.5 | 169 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 0.075 | 0.149 | 0.117 | 0.031 | 0.000 | 0.149–0.149 | 0.038 | 30.5 | 169 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.065 | 0.145 | 0.115 | 0.030 | 0.000 | 0.145–0.145 | 0.038 | 30.5 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 0.694 | 2.176 | 1.623 | 0.521 | 0.031 | 2.176–2.176 | 2.000 | 5351.2 | 161 | 1 |
| packed512/enum/staged/joint/full | joint | 30.518 | 0.230 | 0.169 | 0.030 | 0.030 | 0.230–0.230 | 0.087 | 367.9 | 2289 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.585 | 0.693 | 0.513 | 0.178 | 0.001 | 0.693–0.693 | 0.202 | 137.4 | 1030 | 1 |
| prefix512/slab/ite/witness/local | faces | 0.575 | 0.691 | 0.514 | 0.176 | 0.000 | 0.691–0.691 | 0.199 | 137.4 | 1030 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 0.545 | 0.664 | 0.485 | 0.178 | 0.000 | 0.664–0.664 | 0.182 | 137.4 | 1030 | 1 |
| prefix512/slab/ite/witness/needed | faces | 0.563 | 0.676 | 0.508 | 0.168 | 0.000 | 0.676–0.676 | 0.187 | 137.4 | 1030 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.565 | 0.678 | 0.498 | 0.179 | 0.000 | 0.678–0.678 | 0.190 | 137.4 | 1030 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.553 | 0.655 | 0.486 | 0.168 | 0.000 | 0.655–0.655 | 0.188 | 137.4 | 1030 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 0.608 | 0.700 | 0.503 | 0.194 | 0.000 | 0.700–0.700 | 0.198 | 137.4 | 1030 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.551 | 0.667 | 0.494 | 0.172 | 0.000 | 0.667–0.667 | 0.185 | 137.4 | 1030 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 0.694 | 0.994 | 0.448 | 0.515 | 0.031 | 0.994–0.994 | 0.561 | 5360.2 | 161 | 1 |
| packed512/enum/staged/joint/full | joint | 30.518 | 0.233 | 0.178 | 0.027 | 0.027 | 0.233–0.233 | 0.077 | 376.8 | 2289 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.585 | 0.675 | 0.503 | 0.171 | 0.000 | 0.675–0.675 | 0.193 | 146.4 | 1030 | 1 |
| prefix512/slab/ite/witness/local | faces | 0.575 | 0.692 | 0.517 | 0.174 | 0.001 | 0.692–0.692 | 0.187 | 146.4 | 1030 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 0.545 | 0.685 | 0.518 | 0.166 | 0.000 | 0.685–0.685 | 0.184 | 146.4 | 1030 | 1 |
| prefix512/slab/ite/witness/needed | faces | 0.563 | 0.669 | 0.504 | 0.164 | 0.000 | 0.669–0.669 | 0.182 | 146.4 | 1030 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.565 | 0.654 | 0.487 | 0.165 | 0.000 | 0.654–0.654 | 0.183 | 146.4 | 1030 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.553 | 0.762 | 0.571 | 0.190 | 0.001 | 0.762–0.762 | 0.180 | 146.4 | 1030 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 0.608 | 0.657 | 0.489 | 0.167 | 0.000 | 0.657–0.657 | 0.212 | 146.4 | 1030 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.551 | 0.677 | 0.501 | 0.175 | 0.000 | 0.677–0.677 | 0.181 | 146.4 | 1030 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 0.732 | 2.188 | 1.628 | 0.529 | 0.031 | 2.188–2.188 | 2.113 | 5351.2 | 161 | 1 |
| packed512/enum/staged/joint/full | joint | 0.396 | 0.141 | 0.096 | 0.026 | 0.018 | 0.141–0.141 | 0.056 | 215.9 | 1238 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.530 | 0.857 | 0.637 | 0.219 | 0.000 | 0.857–0.857 | 0.263 | 166.9 | 1254 | 1 |
| prefix512/slab/ite/witness/local | faces | 0.524 | 0.857 | 0.637 | 0.220 | 0.000 | 0.857–0.857 | 0.239 | 166.9 | 1254 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 0.528 | 0.870 | 0.636 | 0.233 | 0.000 | 0.870–0.870 | 0.245 | 166.9 | 1254 | 1 |
| prefix512/slab/ite/witness/needed | faces | 0.535 | 0.863 | 0.644 | 0.219 | 0.000 | 0.863–0.863 | 0.258 | 166.9 | 1254 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.531 | 0.845 | 0.626 | 0.219 | 0.000 | 0.845–0.845 | 0.241 | 166.9 | 1254 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.586 | 0.879 | 0.659 | 0.219 | 0.000 | 0.879–0.879 | 0.239 | 166.9 | 1254 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 0.534 | 0.854 | 0.628 | 0.225 | 0.000 | 0.854–0.854 | 0.240 | 166.9 | 1254 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.534 | 0.870 | 0.654 | 0.215 | 0.000 | 0.870–0.870 | 0.256 | 166.9 | 1254 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 0.732 | 0.974 | 0.448 | 0.496 | 0.029 | 0.974–0.974 | 0.548 | 5360.2 | 161 | 1 |
| packed512/enum/staged/joint/full | joint | 0.396 | 0.141 | 0.104 | 0.020 | 0.016 | 0.141–0.141 | 0.055 | 224.9 | 1238 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.530 | 0.853 | 0.637 | 0.215 | 0.000 | 0.853–0.853 | 0.232 | 175.8 | 1254 | 1 |
| prefix512/slab/ite/witness/local | faces | 0.524 | 0.864 | 0.640 | 0.223 | 0.000 | 0.864–0.864 | 0.228 | 175.8 | 1254 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 0.528 | 0.864 | 0.634 | 0.229 | 0.000 | 0.864–0.864 | 0.231 | 175.8 | 1254 | 1 |
| prefix512/slab/ite/witness/needed | faces | 0.535 | 0.867 | 0.650 | 0.216 | 0.000 | 0.867–0.867 | 0.233 | 175.8 | 1254 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.531 | 0.859 | 0.644 | 0.213 | 0.001 | 0.859–0.859 | 0.244 | 175.8 | 1254 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.586 | 0.857 | 0.642 | 0.214 | 0.000 | 0.857–0.857 | 0.238 | 175.8 | 1254 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 0.534 | 0.855 | 0.639 | 0.215 | 0.000 | 0.855–0.855 | 0.227 | 175.8 | 1254 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.534 | 0.845 | 0.634 | 0.210 | 0.000 | 0.845–0.845 | 0.251 | 175.8 | 1254 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 0.932 | 0.073 | 0.058 | 0.012 | 0.001 | 0.073–0.073 | 0.050 | 115.2 | 200 | 1 |
| packed512/enum/staged/joint/full | joint | 0.752 | 0.168 | 0.101 | 0.039 | 0.026 | 0.168–0.168 | 0.053 | 290.7 | 1877 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.579 | 0.177 | 0.129 | 0.031 | 0.015 | 0.177–0.177 | 0.054 | 107.8 | 759 | 1 |
| prefix512/slab/ite/witness/local | faces | 0.603 | 0.191 | 0.140 | 0.033 | 0.016 | 0.191–0.191 | 0.067 | 107.8 | 759 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 0.572 | 0.176 | 0.130 | 0.031 | 0.014 | 0.176–0.176 | 0.053 | 107.8 | 759 | 1 |
| prefix512/slab/ite/witness/needed | faces | 0.573 | 0.176 | 0.129 | 0.032 | 0.014 | 0.176–0.176 | 0.057 | 107.8 | 759 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.531 | 0.193 | 0.117 | 0.066 | 0.009 | 0.193–0.193 | 0.051 | 103.9 | 751 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.529 | 0.192 | 0.116 | 0.066 | 0.009 | 0.192–0.192 | 0.049 | 104.6 | 751 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 0.536 | 0.196 | 0.116 | 0.068 | 0.010 | 0.196–0.196 | 0.052 | 104.6 | 751 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.551 | 0.193 | 0.117 | 0.065 | 0.010 | 0.193–0.193 | 0.050 | 103.9 | 751 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 0.932 | 0.038 | 0.028 | 0.009 | 0.001 | 0.038–0.038 | 0.017 | 124.1 | 200 | 1 |
| packed512/enum/staged/joint/full | joint | 0.752 | 0.154 | 0.071 | 0.029 | 0.054 | 0.154–0.154 | 0.078 | 299.7 | 1877 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.579 | 0.175 | 0.132 | 0.032 | 0.010 | 0.175–0.175 | 0.060 | 116.8 | 759 | 1 |
| prefix512/slab/ite/witness/local | faces | 0.603 | 0.162 | 0.122 | 0.030 | 0.009 | 0.162–0.162 | 0.054 | 116.8 | 759 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 0.572 | 0.155 | 0.117 | 0.030 | 0.008 | 0.155–0.155 | 0.055 | 116.8 | 759 | 1 |
| prefix512/slab/ite/witness/needed | faces | 0.573 | 0.163 | 0.124 | 0.030 | 0.008 | 0.163–0.163 | 0.047 | 116.8 | 759 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.531 | 0.182 | 0.115 | 0.063 | 0.005 | 0.182–0.182 | 0.043 | 112.9 | 751 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.529 | 0.173 | 0.106 | 0.062 | 0.005 | 0.173–0.173 | 0.043 | 113.6 | 751 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 0.536 | 0.184 | 0.115 | 0.063 | 0.005 | 0.184–0.184 | 0.056 | 113.6 | 751 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.551 | 0.172 | 0.106 | 0.061 | 0.005 | 0.172–0.172 | 0.044 | 112.9 | 751 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 0.587 | 0.065 | 0.052 | 0.011 | 0.001 | 0.065–0.065 | 0.047 | 115.2 | 200 | 1 |
| packed512/enum/staged/joint/full | joint | 0.217 | 0.164 | 0.110 | 0.034 | 0.018 | 0.164–0.164 | 0.042 | 331.7 | 2075 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.538 | 0.175 | 0.126 | 0.032 | 0.015 | 0.175–0.175 | 0.051 | 92.8 | 664 | 1 |
| prefix512/slab/ite/witness/local | faces | 0.519 | 0.182 | 0.135 | 0.032 | 0.014 | 0.182–0.182 | 0.061 | 92.8 | 664 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 0.517 | 0.181 | 0.133 | 0.033 | 0.014 | 0.181–0.181 | 0.051 | 92.8 | 664 | 1 |
| prefix512/slab/ite/witness/needed | faces | 0.511 | 0.173 | 0.126 | 0.032 | 0.014 | 0.173–0.173 | 0.051 | 92.8 | 664 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.525 | 0.229 | 0.138 | 0.078 | 0.012 | 0.229–0.229 | 0.049 | 90.3 | 656 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.464 | 0.192 | 0.117 | 0.064 | 0.009 | 0.192–0.192 | 0.047 | 91.0 | 656 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 0.463 | 0.193 | 0.117 | 0.065 | 0.009 | 0.193–0.193 | 0.048 | 91.0 | 656 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.476 | 0.198 | 0.120 | 0.066 | 0.010 | 0.198–0.198 | 0.049 | 90.3 | 656 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 0.587 | 0.037 | 0.027 | 0.009 | 0.001 | 0.037–0.037 | 0.017 | 124.1 | 200 | 1 |
| packed512/enum/staged/joint/full | joint | 0.217 | 0.121 | 0.084 | 0.023 | 0.014 | 0.121–0.121 | 0.038 | 340.7 | 2075 | 1 |
| prefix512/slab/ite/witness/full | faces | 0.538 | 0.156 | 0.119 | 0.029 | 0.008 | 0.156–0.156 | 0.046 | 101.8 | 664 | 1 |
| prefix512/slab/ite/witness/local | faces | 0.519 | 0.167 | 0.128 | 0.030 | 0.008 | 0.167–0.167 | 0.050 | 101.8 | 664 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 0.517 | 0.158 | 0.119 | 0.030 | 0.008 | 0.158–0.158 | 0.045 | 101.8 | 664 | 1 |
| prefix512/slab/ite/witness/needed | faces | 0.511 | 0.157 | 0.119 | 0.030 | 0.008 | 0.157–0.157 | 0.046 | 101.8 | 664 | 1 |
| retraction512/slab/ite/witness/full | faces | 0.525 | 0.179 | 0.112 | 0.062 | 0.005 | 0.179–0.179 | 0.046 | 99.3 | 656 | 1 |
| retraction512/slab/ite/witness/local | faces | 0.464 | 0.174 | 0.109 | 0.061 | 0.005 | 0.174–0.174 | 0.043 | 100.0 | 656 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 0.463 | 0.174 | 0.108 | 0.061 | 0.005 | 0.174–0.174 | 0.043 | 100.0 | 656 | 1 |
| retraction512/slab/ite/witness/needed | faces | 0.476 | 0.176 | 0.111 | 0.060 | 0.004 | 0.176–0.176 | 0.043 | 99.3 | 656 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 236.957 | 2.295 | 1.684 | 0.576 | 0.034 | 2.295–2.295 | 2.143 | 17043.7 | 517 | 1 |
| packed512/enum/staged/joint/full | joint | 34.949 | 1.573 | 1.009 | 0.337 | 0.226 | 1.573–1.573 | 0.566 | 5355.9 | 31982 | 1 |
| prefix512/slab/ite/witness/full | faces | 7.070 | 0.856 | 0.554 | 0.168 | 0.134 | 0.856–0.856 | 0.304 | 1431.5 | 9539 | 1 |
| prefix512/slab/ite/witness/local | faces | 6.815 | 0.808 | 0.522 | 0.156 | 0.130 | 0.808–0.808 | 0.317 | 1431.5 | 9539 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 6.804 | 0.799 | 0.521 | 0.152 | 0.126 | 0.799–0.799 | 0.305 | 1431.5 | 9539 | 1 |
| prefix512/slab/ite/witness/needed | faces | 6.783 | 0.818 | 0.526 | 0.159 | 0.133 | 0.818–0.818 | 0.328 | 1431.5 | 9539 | 1 |
| retraction512/slab/ite/witness/full | faces | 7.472 | 1.280 | 0.745 | 0.475 | 0.060 | 1.280–1.280 | 0.353 | 1560.7 | 9815 | 1 |
| retraction512/slab/ite/witness/local | faces | 7.135 | 1.242 | 0.718 | 0.465 | 0.058 | 1.242–1.242 | 0.364 | 1561.0 | 9815 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 7.130 | 1.305 | 0.748 | 0.489 | 0.068 | 1.305–1.305 | 0.369 | 1561.0 | 9815 | 1 |
| retraction512/slab/ite/witness/needed | faces | 6.849 | 1.248 | 0.721 | 0.467 | 0.060 | 1.248–1.248 | 0.355 | 1560.7 | 9815 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 236.957 | 1.079 | 0.501 | 0.543 | 0.034 | 1.079–1.079 | 0.642 | 17052.6 | 517 | 1 |
| packed512/enum/staged/joint/full | joint | 34.949 | 1.407 | 0.856 | 0.315 | 0.235 | 1.407–1.407 | 0.523 | 5364.8 | 31982 | 1 |
| prefix512/slab/ite/witness/full | faces | 7.070 | 0.779 | 0.507 | 0.151 | 0.120 | 0.779–0.779 | 0.338 | 1440.5 | 9539 | 1 |
| prefix512/slab/ite/witness/local | faces | 6.815 | 0.848 | 0.563 | 0.157 | 0.127 | 0.848–0.848 | 0.294 | 1440.5 | 9539 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 6.804 | 0.827 | 0.530 | 0.166 | 0.131 | 0.827–0.827 | 0.295 | 1440.5 | 9539 | 1 |
| prefix512/slab/ite/witness/needed | faces | 6.783 | 0.829 | 0.542 | 0.155 | 0.132 | 0.829–0.829 | 0.295 | 1440.5 | 9539 | 1 |
| retraction512/slab/ite/witness/full | faces | 7.472 | 1.231 | 0.706 | 0.470 | 0.054 | 1.231–1.231 | 0.347 | 1569.6 | 9815 | 1 |
| retraction512/slab/ite/witness/local | faces | 7.135 | 1.251 | 0.725 | 0.469 | 0.056 | 1.251–1.251 | 0.365 | 1570.0 | 9815 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 7.130 | 1.258 | 0.731 | 0.468 | 0.058 | 1.258–1.258 | 0.341 | 1570.0 | 9815 | 1 |
| retraction512/slab/ite/witness/needed | faces | 6.849 | 1.219 | 0.704 | 0.460 | 0.054 | 1.219–1.219 | 0.356 | 1569.6 | 9815 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 224.667 | 2.206 | 1.620 | 0.555 | 0.031 | 2.206–2.206 | 2.085 | 17043.7 | 517 | 1 |
| packed512/enum/staged/joint/full | joint | 1.364 | 0.354 | 0.248 | 0.061 | 0.046 | 0.354–0.354 | 0.124 | 1282.0 | 10792 | 1 |
| prefix512/slab/ite/witness/full | faces | 4.582 | 0.958 | 0.631 | 0.204 | 0.123 | 0.958–0.958 | 0.367 | 914.0 | 6457 | 1 |
| prefix512/slab/ite/witness/local | faces | 4.539 | 0.945 | 0.617 | 0.201 | 0.126 | 0.945–0.945 | 0.371 | 914.0 | 6457 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 4.459 | 0.959 | 0.624 | 0.209 | 0.126 | 0.959–0.959 | 0.350 | 914.0 | 6457 | 1 |
| prefix512/slab/ite/witness/needed | faces | 4.501 | 0.962 | 0.634 | 0.201 | 0.127 | 0.962–0.962 | 0.348 | 914.0 | 6457 | 1 |
| retraction512/slab/ite/witness/full | faces | 4.536 | 1.339 | 0.818 | 0.464 | 0.057 | 1.339–1.339 | 0.361 | 902.0 | 7015 | 1 |
| retraction512/slab/ite/witness/local | faces | 4.700 | 1.354 | 0.834 | 0.465 | 0.055 | 1.354–1.354 | 0.376 | 902.7 | 7015 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 4.582 | 1.337 | 0.814 | 0.467 | 0.057 | 1.337–1.337 | 0.349 | 902.7 | 7015 | 1 |
| retraction512/slab/ite/witness/needed | faces | 4.657 | 1.357 | 0.830 | 0.468 | 0.059 | 1.357–1.357 | 0.366 | 902.0 | 7015 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint/full | joint | 224.667 | 1.033 | 0.470 | 0.532 | 0.031 | 1.033–1.033 | 0.559 | 17052.6 | 517 | 1 |
| packed512/enum/staged/joint/full | joint | 1.364 | 0.321 | 0.222 | 0.056 | 0.042 | 0.321–0.321 | 0.112 | 1291.0 | 10792 | 1 |
| prefix512/slab/ite/witness/full | faces | 4.582 | 0.961 | 0.629 | 0.211 | 0.121 | 0.961–0.961 | 0.344 | 923.0 | 6457 | 1 |
| prefix512/slab/ite/witness/local | faces | 4.539 | 0.942 | 0.620 | 0.200 | 0.121 | 0.942–0.942 | 0.343 | 923.0 | 6457 | 1 |
| prefix512/slab/ite/witness/local-needed | faces | 4.459 | 0.941 | 0.611 | 0.200 | 0.128 | 0.941–0.941 | 0.339 | 923.0 | 6457 | 1 |
| prefix512/slab/ite/witness/needed | faces | 4.501 | 0.951 | 0.625 | 0.200 | 0.126 | 0.951–0.951 | 0.338 | 923.0 | 6457 | 1 |
| retraction512/slab/ite/witness/full | faces | 4.536 | 1.305 | 0.804 | 0.449 | 0.052 | 1.305–1.305 | 0.348 | 911.0 | 7015 | 1 |
| retraction512/slab/ite/witness/local | faces | 4.700 | 1.296 | 0.799 | 0.444 | 0.053 | 1.296–1.296 | 0.340 | 911.7 | 7015 | 1 |
| retraction512/slab/ite/witness/local-needed | faces | 4.582 | 1.316 | 0.802 | 0.462 | 0.052 | 1.316–1.316 | 0.344 | 911.7 | 7015 | 1 |
| retraction512/slab/ite/witness/needed | faces | 4.657 | 1.312 | 0.807 | 0.452 | 0.052 | 1.312–1.312 | 0.373 | 911.0 | 7015 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

368 configurations; 1472 matched comparisons; 110 retained processes.

- [Raw evidence](results/local-first-smoke.json).
- [Raw evidence](results/local-domain-controls.json).
- [Raw evidence](results/local-fallback-controls.json).
- [Raw evidence](results/local-cutoff-screen.json).
- [Raw evidence](results/local-normalization-control.json).
- [Raw evidence](results/local-readout-repeat.json).
