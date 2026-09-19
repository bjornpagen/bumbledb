# Prefix-preserving completion: native Free Join comparison

Construction is measured separately. Fresh and warm queries include exact legal
population checksums over all 80 outputs. Every output is also checked pointwise
outside timing. Retained KB includes construction residue and owner caches; temporary
observation memo and allocator overhead are excluded. No raw alias counts are used.
Executable: `84ffdedfc491a5b326dd10f3e14c69ff091e0ace5b966ba00183e74135625047`.

## below, 12 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.076 | 0.150 | 0.150 | 0.001 | 0.148–0.172 | 0.139 | 101.1 | 173 | 5 |
| essential512/slab | views | certified | words | 0.397 | 1.451 | 1.451 | 0.000 | 1.422–1.528 | 0.652 | 173.9 | 1408 | 5 |
| packed512/enum | materialized | materialized | words | 1.408 | 1.161 | 1.137 | 0.023 | 1.122–1.215 | 1.025 | 330.6 | 2427 | 5 |
| prefix512/slab | views | certified | words | 0.519 | 0.537 | 0.472 | 0.064 | 0.525–0.596 | 0.381 | 109.5 | 824 | 5 |
| retraction512/slab | views | certified | words | 0.477 | 1.018 | 0.919 | 0.100 | 0.992–1.039 | 0.829 | 126.7 | 959 | 5 |

## below, 12 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.076 | 0.071 | 0.070 | 0.001 | 0.068–0.082 | 0.013 | 114.1 | 173 | 5 |
| essential512/slab | views | certified | words | 0.397 | 1.172 | 1.171 | 0.000 | 1.130–1.208 | 0.012 | 185.8 | 1408 | 5 |
| packed512/enum | materialized | materialized | words | 1.408 | 0.474 | 0.451 | 0.024 | 0.472–0.497 | 0.038 | 343.6 | 2427 | 5 |
| prefix512/slab | views | certified | words | 0.519 | 0.375 | 0.310 | 0.064 | 0.367–0.407 | 0.076 | 121.4 | 824 | 5 |
| retraction512/slab | views | certified | words | 0.477 | 0.602 | 0.503 | 0.098 | 0.576–0.650 | 0.114 | 138.6 | 959 | 5 |

## below, 12 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.077 | 0.150 | 0.149 | 0.001 | 0.147–0.173 | 0.141 | 101.1 | 173 | 5 |
| essential512/slab | views | certified | words | 0.276 | 2.246 | 2.246 | 0.000 | 2.147–2.313 | 0.759 | 306.8 | 2528 | 5 |
| packed512/enum | materialized | materialized | words | 0.098 | 2.181 | 2.170 | 0.010 | 2.143–2.281 | 1.837 | 802.8 | 4803 | 5 |
| prefix512/slab | views | certified | words | 0.528 | 0.693 | 0.651 | 0.044 | 0.670–0.733 | 0.451 | 125.6 | 1141 | 5 |
| retraction512/slab | views | certified | words | 0.510 | 0.919 | 0.888 | 0.031 | 0.867–0.991 | 0.600 | 180.5 | 1359 | 5 |

## below, 12 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.077 | 0.071 | 0.071 | 0.001 | 0.069–0.080 | 0.013 | 114.1 | 173 | 5 |
| essential512/slab | views | certified | words | 0.276 | 1.781 | 1.780 | 0.000 | 1.743–1.863 | 0.013 | 318.7 | 2528 | 5 |
| packed512/enum | materialized | materialized | words | 0.098 | 0.877 | 0.867 | 0.010 | 0.856–0.905 | 0.023 | 815.8 | 4803 | 5 |
| prefix512/slab | views | certified | words | 0.528 | 0.484 | 0.443 | 0.042 | 0.463–0.582 | 0.054 | 137.5 | 1141 | 5 |
| retraction512/slab | views | certified | words | 0.510 | 0.607 | 0.575 | 0.031 | 0.573–0.652 | 0.042 | 192.4 | 1359 | 5 |

## below, 18 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 6.441 | 7.387 | 7.330 | 0.044 | 7.120–7.434 | 6.995 | 7418.7 | 224 | 5 |
| essential512/slab | views | certified | words | 2.592 | 8.305 | 8.305 | 0.000 | 8.274–8.474 | 3.503 | 1175.3 | 8419 | 5 |
| packed512/enum | materialized | materialized | words | 40.533 | 3.702 | 3.505 | 0.196 | 3.426–5.096 | 2.405 | 2465.2 | 17618 | 5 |
| prefix512/slab | views | certified | words | 5.879 | 4.830 | 3.240 | 1.579 | 4.804–5.059 | 3.653 | 1324.1 | 8461 | 5 |
| retraction512/slab | views | certified | words | 7.948 | 7.730 | 5.854 | 1.875 | 7.585–7.800 | 5.690 | 1704.9 | 13037 | 5 |

## below, 18 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 6.441 | 2.911 | 2.866 | 0.044 | 2.865–3.032 | 0.059 | 7431.7 | 224 | 5 |
| essential512/slab | views | certified | words | 2.592 | 7.095 | 7.094 | 0.000 | 6.832–7.327 | 0.027 | 1188.8 | 8419 | 5 |
| packed512/enum | materialized | materialized | words | 40.533 | 2.248 | 2.072 | 0.177 | 2.214–2.515 | 0.210 | 2478.1 | 17618 | 5 |
| prefix512/slab | views | certified | words | 5.879 | 3.966 | 2.365 | 1.618 | 3.930–4.054 | 1.700 | 1337.6 | 8461 | 5 |
| retraction512/slab | views | certified | words | 7.948 | 5.710 | 3.852 | 1.842 | 5.530–5.810 | 1.912 | 1718.4 | 13037 | 5 |

## below, 18 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 6.306 | 7.082 | 7.041 | 0.041 | 6.887–7.140 | 6.749 | 7418.7 | 224 | 5 |
| essential512/slab | views | certified | words | 1.276 | 25.366 | 25.365 | 0.001 | 25.041–26.370 | 7.884 | 4275.4 | 33993 | 5 |
| packed512/enum | materialized | materialized | words | 0.501 | 11.072 | 11.027 | 0.045 | 10.748–11.400 | 7.348 | 6857.2 | 60584 | 5 |
| prefix512/slab | views | certified | words | 5.932 | 10.030 | 9.899 | 0.133 | 9.877–10.532 | 4.266 | 2523.8 | 19046 | 5 |
| retraction512/slab | views | certified | words | 6.767 | 11.375 | 11.278 | 0.097 | 11.006–11.568 | 4.708 | 3090.0 | 22406 | 5 |

## below, 18 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 6.306 | 2.789 | 2.748 | 0.040 | 2.755–2.887 | 0.052 | 7431.7 | 224 | 5 |
| essential512/slab | views | certified | words | 1.276 | 22.002 | 22.001 | 0.001 | 21.063–56.122 | 0.030 | 4288.8 | 33993 | 5 |
| packed512/enum | materialized | materialized | words | 0.501 | 6.725 | 6.678 | 0.047 | 6.481–8.281 | 0.079 | 6870.2 | 60584 | 5 |
| prefix512/slab | views | certified | words | 5.932 | 8.172 | 8.041 | 0.136 | 7.949–8.526 | 0.167 | 2537.3 | 19046 | 5 |
| retraction512/slab | views | certified | words | 6.767 | 9.008 | 8.910 | 0.097 | 8.874–9.484 | 0.135 | 3103.5 | 22406 | 5 |

## fibred, 13 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 1.370 | 0.263 | 0.261 | 0.001 | 0.261–0.291 | 0.257 | 317.0 | 287 | 5 |
| essential512/slab | views | certified | words | 1.911 | 3.506 | 3.506 | 0.000 | 3.452–3.733 | 1.447 | 484.9 | 4420 | 5 |
| packed512/enum | materialized | materialized | words | 1.280 | 1.572 | 1.528 | 0.045 | 1.476–1.644 | 1.127 | 1034.7 | 6535 | 5 |
| prefix512/slab | views | certified | words | 1.227 | 1.219 | 0.944 | 0.274 | 1.157–1.291 | 0.873 | 306.8 | 1833 | 5 |
| retraction512/slab | views | certified | words | 1.200 | 2.015 | 1.726 | 0.292 | 1.963–2.079 | 1.711 | 312.9 | 2070 | 5 |

## fibred, 13 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 1.370 | 0.130 | 0.128 | 0.002 | 0.129–0.144 | 0.016 | 330.0 | 287 | 5 |
| essential512/slab | views | certified | words | 1.911 | 2.736 | 2.736 | 0.000 | 2.662–2.840 | 0.013 | 498.4 | 4420 | 5 |
| packed512/enum | materialized | materialized | words | 1.280 | 0.823 | 0.783 | 0.040 | 0.798–0.854 | 0.059 | 1047.7 | 6535 | 5 |
| prefix512/slab | views | certified | words | 1.227 | 0.882 | 0.625 | 0.256 | 0.867–0.902 | 0.272 | 320.3 | 1833 | 5 |
| retraction512/slab | views | certified | words | 1.200 | 1.278 | 0.988 | 0.296 | 1.267–1.352 | 0.301 | 326.4 | 2070 | 5 |

## fibred, 13 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 1.289 | 0.247 | 0.246 | 0.001 | 0.242–0.268 | 0.230 | 317.0 | 287 | 5 |
| essential512/slab | views | certified | words | 1.347 | 6.079 | 6.078 | 0.000 | 5.985–6.211 | 1.834 | 1088.7 | 7971 | 5 |
| packed512/enum | materialized | materialized | words | 0.367 | 5.535 | 5.500 | 0.034 | 4.961–5.627 | 4.224 | 2327.3 | 16261 | 5 |
| prefix512/slab | views | certified | words | 1.333 | 1.318 | 1.186 | 0.130 | 1.257–1.334 | 0.851 | 306.9 | 2453 | 5 |
| retraction512/slab | views | certified | words | 1.271 | 1.831 | 1.714 | 0.115 | 1.782–2.049 | 1.278 | 433.2 | 3010 | 5 |

## fibred, 13 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 1.289 | 0.115 | 0.113 | 0.001 | 0.114–0.127 | 0.014 | 330.0 | 287 | 5 |
| essential512/slab | views | certified | words | 1.347 | 5.070 | 5.070 | 0.000 | 5.041–5.357 | 0.013 | 1102.2 | 7971 | 5 |
| packed512/enum | materialized | materialized | words | 0.367 | 2.678 | 2.641 | 0.034 | 2.501–2.814 | 0.047 | 2340.3 | 16261 | 5 |
| prefix512/slab | views | certified | words | 1.333 | 0.935 | 0.808 | 0.128 | 0.891–0.964 | 0.142 | 320.4 | 2453 | 5 |
| retraction512/slab | views | certified | words | 1.271 | 1.300 | 1.162 | 0.118 | 1.271–1.344 | 0.143 | 446.7 | 3010 | 5 |

## fibred, 19 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 475.637 | 14.887 | 14.795 | 0.101 | 14.694–15.610 | 14.730 | 45001.8 | 684 | 5 |
| essential512/slab | views | certified | words | 28.563 | 30.466 | 30.465 | 0.001 | 29.035–31.683 | 10.740 | 6853.3 | 59905 | 5 |
| packed512/enum | materialized | materialized | words | 60.418 | 10.918 | 10.067 | 0.862 | 10.797–11.288 | 6.675 | 13305.1 | 98153 | 5 |
| prefix512/slab | views | certified | words | 26.381 | 14.064 | 7.228 | 6.831 | 13.495–14.158 | 11.193 | 6462.3 | 43167 | 5 |
| retraction512/slab | views | certified | words | 34.163 | 43.904 | 35.113 | 8.753 | 43.519–44.000 | 38.653 | 9943.3 | 59803 | 5 |

## fibred, 19 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 475.637 | 6.601 | 6.506 | 0.106 | 6.533–6.663 | 0.101 | 45018.8 | 684 | 5 |
| essential512/slab | views | certified | words | 28.563 | 24.936 | 24.935 | 0.001 | 24.706–24.985 | 0.028 | 6867.1 | 59905 | 5 |
| packed512/enum | materialized | materialized | words | 60.418 | 9.321 | 8.404 | 0.894 | 8.964–9.581 | 0.958 | 13322.2 | 98153 | 5 |
| prefix512/slab | views | certified | words | 26.381 | 11.850 | 5.044 | 6.794 | 11.747–11.891 | 6.857 | 6476.1 | 43167 | 5 |
| retraction512/slab | views | certified | words | 34.163 | 29.087 | 20.365 | 8.757 | 28.571–29.198 | 8.789 | 9957.0 | 59803 | 5 |

## fibred, 19 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 462.796 | 15.504 | 15.413 | 0.085 | 14.664–15.680 | 14.517 | 45001.8 | 684 | 5 |
| essential512/slab | views | certified | words | 7.218 | 70.243 | 70.242 | 0.001 | 69.681–77.914 | 17.091 | 12039.0 | 109616 | 5 |
| packed512/enum | materialized | materialized | words | 2.427 | 42.042 | 41.917 | 0.116 | 37.973–44.381 | 25.759 | 25957.2 | 218715 | 5 |
| prefix512/slab | views | certified | words | 23.133 | 19.589 | 19.230 | 0.360 | 19.213–20.291 | 8.738 | 7073.3 | 57659 | 5 |
| retraction512/slab | views | certified | words | 34.750 | 27.807 | 27.527 | 0.283 | 26.796–42.949 | 11.248 | 9112.8 | 80302 | 5 |

## fibred, 19 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 462.796 | 6.479 | 6.392 | 0.082 | 6.289–6.974 | 0.089 | 45018.8 | 684 | 5 |
| essential512/slab | views | certified | words | 7.218 | 61.384 | 61.383 | 0.001 | 60.417–64.258 | 0.027 | 12052.8 | 109616 | 5 |
| packed512/enum | materialized | materialized | words | 2.427 | 25.384 | 25.262 | 0.116 | 24.701–25.999 | 0.156 | 25974.3 | 218715 | 5 |
| prefix512/slab | views | certified | words | 23.133 | 16.150 | 15.784 | 0.361 | 15.501–16.578 | 0.410 | 7087.0 | 57659 | 5 |
| retraction512/slab | views | certified | words | 34.750 | 22.184 | 21.899 | 0.281 | 22.064–22.544 | 0.332 | 9126.5 | 80302 | 5 |

## full, 12 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.053 | 0.191 | 0.189 | 0.001 | 0.191–0.191 | 0.147 | 83.5 | 140 | 1 |
| essential512/slab | views | certified | words | 0.088 | 0.555 | 0.554 | 0.000 | 0.555–0.555 | 0.335 | 24.9 | 215 | 1 |
| packed512/enum | materialized | materialized | words | 0.541 | 0.938 | 0.929 | 0.009 | 0.938–0.938 | 0.791 | 75.3 | 482 | 1 |
| prefix512/slab | views | certified | words | 0.092 | 0.555 | 0.554 | 0.000 | 0.555–0.555 | 0.317 | 26.1 | 219 | 1 |
| retraction512/slab | views | certified | words | 0.086 | 0.536 | 0.535 | 0.000 | 0.536–0.536 | 0.320 | 26.1 | 219 | 1 |

## full, 12 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.053 | 0.078 | 0.077 | 0.001 | 0.078–0.078 | 0.014 | 96.5 | 140 | 1 |
| essential512/slab | views | certified | words | 0.088 | 0.345 | 0.345 | 0.000 | 0.345–0.345 | 0.013 | 36.7 | 215 | 1 |
| packed512/enum | materialized | materialized | words | 0.541 | 0.283 | 0.274 | 0.009 | 0.283–0.283 | 0.031 | 88.3 | 482 | 1 |
| prefix512/slab | views | certified | words | 0.092 | 0.337 | 0.337 | 0.000 | 0.337–0.337 | 0.019 | 37.9 | 219 | 1 |
| retraction512/slab | views | certified | words | 0.086 | 0.334 | 0.334 | 0.000 | 0.334–0.334 | 0.014 | 37.9 | 219 | 1 |

## full, 12 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.040 | 0.174 | 0.172 | 0.001 | 0.174–0.174 | 0.141 | 83.5 | 140 | 1 |
| essential512/slab | views | certified | words | 0.085 | 0.787 | 0.787 | 0.000 | 0.787–0.787 | 0.430 | 56.1 | 593 | 1 |
| packed512/enum | materialized | materialized | words | 0.059 | 1.477 | 1.474 | 0.003 | 1.477–1.477 | 1.292 | 292.1 | 1326 | 1 |
| prefix512/slab | views | certified | words | 0.086 | 0.759 | 0.759 | 0.000 | 0.759–0.759 | 0.428 | 58.5 | 597 | 1 |
| retraction512/slab | views | certified | words | 0.088 | 0.803 | 0.802 | 0.000 | 0.803–0.803 | 0.441 | 58.5 | 597 | 1 |

## full, 12 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.040 | 0.081 | 0.080 | 0.001 | 0.081–0.081 | 0.014 | 96.5 | 140 | 1 |
| essential512/slab | views | certified | words | 0.085 | 0.509 | 0.508 | 0.000 | 0.509–0.509 | 0.013 | 67.9 | 593 | 1 |
| packed512/enum | materialized | materialized | words | 0.059 | 0.515 | 0.512 | 0.003 | 0.515–0.515 | 0.017 | 305.1 | 1326 | 1 |
| prefix512/slab | views | certified | words | 0.086 | 0.479 | 0.479 | 0.000 | 0.479–0.479 | 0.012 | 70.2 | 597 | 1 |
| retraction512/slab | views | certified | words | 0.088 | 0.475 | 0.475 | 0.000 | 0.475–0.475 | 0.013 | 70.2 | 597 | 1 |

## full, 18 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 1.235 | 7.652 | 7.605 | 0.046 | 7.652–7.652 | 7.198 | 5811.8 | 175 | 1 |
| essential512/slab | views | certified | words | 0.682 | 3.224 | 3.223 | 0.000 | 3.224–3.224 | 1.914 | 233.0 | 1776 | 1 |
| packed512/enum | materialized | materialized | words | 27.557 | 1.533 | 1.498 | 0.035 | 1.533–1.533 | 1.291 | 500.5 | 3815 | 1 |
| prefix512/slab | views | certified | words | 0.718 | 3.308 | 3.308 | 0.001 | 3.308–3.308 | 1.901 | 236.1 | 1782 | 1 |
| retraction512/slab | views | certified | words | 0.743 | 3.221 | 3.220 | 0.000 | 3.221–3.221 | 1.917 | 236.1 | 1782 | 1 |

## full, 18 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 1.235 | 2.695 | 2.650 | 0.045 | 2.695–2.695 | 0.060 | 5824.8 | 175 | 1 |
| essential512/slab | views | certified | words | 0.682 | 2.327 | 2.326 | 0.000 | 2.327–2.327 | 0.027 | 244.8 | 1776 | 1 |
| packed512/enum | materialized | materialized | words | 27.557 | 0.775 | 0.739 | 0.035 | 0.775–0.775 | 0.064 | 513.5 | 3815 | 1 |
| prefix512/slab | views | certified | words | 0.718 | 2.301 | 2.300 | 0.001 | 2.301–2.301 | 0.025 | 247.9 | 1782 | 1 |
| retraction512/slab | views | certified | words | 0.743 | 2.277 | 2.276 | 0.001 | 2.277–2.277 | 0.025 | 247.9 | 1782 | 1 |

## full, 18 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 1.185 | 7.325 | 7.284 | 0.040 | 7.325–7.325 | 6.864 | 5811.8 | 175 | 1 |
| essential512/slab | views | certified | words | 0.672 | 10.207 | 10.207 | 0.000 | 10.207–10.207 | 4.175 | 1562.7 | 11800 | 1 |
| packed512/enum | materialized | materialized | words | 0.416 | 5.319 | 5.295 | 0.024 | 5.319–5.319 | 5.017 | 1201.7 | 7882 | 1 |
| prefix512/slab | views | certified | words | 0.675 | 10.234 | 10.233 | 0.000 | 10.234–10.234 | 4.240 | 1323.1 | 11806 | 1 |
| retraction512/slab | views | certified | words | 0.693 | 9.914 | 9.913 | 0.000 | 9.914–9.914 | 4.062 | 1323.1 | 11806 | 1 |

## full, 18 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 1.185 | 2.565 | 2.524 | 0.040 | 2.565–2.565 | 0.056 | 5824.8 | 175 | 1 |
| essential512/slab | views | certified | words | 0.672 | 7.897 | 7.897 | 0.000 | 7.897–7.897 | 0.022 | 1574.4 | 11800 | 1 |
| packed512/enum | materialized | materialized | words | 0.416 | 2.400 | 2.376 | 0.024 | 2.400–2.400 | 0.067 | 1214.7 | 7882 | 1 |
| prefix512/slab | views | certified | words | 0.675 | 7.921 | 7.920 | 0.000 | 7.921–7.921 | 0.030 | 1334.9 | 11806 | 1 |
| retraction512/slab | views | certified | words | 0.693 | 7.828 | 7.827 | 0.001 | 7.828–7.828 | 0.024 | 1334.9 | 11806 | 1 |

## holes, 12 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.577 | 0.164 | 0.163 | 0.001 | 0.156–0.191 | 0.147 | 148.7 | 250 | 5 |
| essential512/slab | views | certified | words | 1.216 | 2.071 | 2.070 | 0.000 | 2.005–2.121 | 0.835 | 385.4 | 2524 | 5 |
| packed512/enum | materialized | materialized | words | 0.682 | 1.211 | 1.188 | 0.023 | 1.159–1.283 | 0.977 | 587.9 | 3459 | 5 |
| prefix512/slab | views | certified | words | 0.559 | 0.678 | 0.537 | 0.137 | 0.647–0.741 | 0.494 | 138.7 | 1009 | 5 |
| retraction512/slab | views | certified | words | 0.555 | 1.226 | 1.057 | 0.169 | 1.197–1.248 | 1.011 | 137.6 | 1126 | 5 |

## holes, 12 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.577 | 0.084 | 0.083 | 0.001 | 0.079–0.096 | 0.013 | 163.9 | 250 | 5 |
| essential512/slab | views | certified | words | 1.216 | 1.659 | 1.659 | 0.000 | 1.609–1.958 | 0.016 | 398.9 | 2524 | 5 |
| packed512/enum | materialized | materialized | words | 0.682 | 0.621 | 0.597 | 0.023 | 0.599–0.660 | 0.036 | 603.1 | 3459 | 5 |
| prefix512/slab | views | certified | words | 0.559 | 0.493 | 0.357 | 0.128 | 0.472–0.545 | 0.146 | 152.2 | 1009 | 5 |
| retraction512/slab | views | certified | words | 0.555 | 0.798 | 0.629 | 0.168 | 0.785–0.806 | 0.192 | 151.1 | 1126 | 5 |

## holes, 12 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.592 | 0.165 | 0.164 | 0.001 | 0.161–0.188 | 0.152 | 148.7 | 250 | 5 |
| essential512/slab | views | certified | words | 0.866 | 3.993 | 3.993 | 0.000 | 3.970–4.122 | 1.074 | 894.9 | 5252 | 5 |
| packed512/enum | materialized | materialized | words | 0.314 | 3.770 | 3.748 | 0.020 | 3.657–3.825 | 2.988 | 1767.5 | 11385 | 5 |
| prefix512/slab | views | certified | words | 0.566 | 0.752 | 0.674 | 0.077 | 0.725–0.793 | 0.476 | 130.8 | 1222 | 5 |
| retraction512/slab | views | certified | words | 0.545 | 1.056 | 0.995 | 0.061 | 1.047–1.103 | 0.693 | 217.2 | 1626 | 5 |

## holes, 12 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.592 | 0.083 | 0.083 | 0.001 | 0.083–0.097 | 0.014 | 163.9 | 250 | 5 |
| essential512/slab | views | certified | words | 0.866 | 3.440 | 3.439 | 0.000 | 3.403–3.499 | 0.012 | 908.4 | 5252 | 5 |
| packed512/enum | materialized | materialized | words | 0.314 | 1.751 | 1.733 | 0.018 | 1.736–1.815 | 0.030 | 1782.8 | 11385 | 5 |
| prefix512/slab | views | certified | words | 0.566 | 0.531 | 0.451 | 0.075 | 0.522–0.558 | 0.091 | 144.3 | 1222 | 5 |
| retraction512/slab | views | certified | words | 0.545 | 0.736 | 0.671 | 0.060 | 0.707–0.771 | 0.072 | 230.7 | 1626 | 5 |

## holes, 18 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 224.497 | 7.087 | 7.044 | 0.041 | 6.883–7.670 | 6.984 | 18947.4 | 575 | 5 |
| essential512/slab | views | certified | words | 22.940 | 20.453 | 20.452 | 0.001 | 19.893–21.396 | 7.068 | 5057.9 | 44045 | 5 |
| packed512/enum | materialized | materialized | words | 32.425 | 9.395 | 8.726 | 0.668 | 9.077–10.767 | 5.433 | 12587.2 | 69602 | 5 |
| prefix512/slab | views | certified | words | 11.285 | 8.133 | 3.807 | 4.307 | 7.973–8.285 | 6.877 | 2982.6 | 18751 | 5 |
| retraction512/slab | views | certified | words | 14.616 | 33.353 | 26.881 | 6.375 | 32.944–33.588 | 31.331 | 3640.4 | 26100 | 5 |

## holes, 18 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 224.497 | 3.285 | 3.232 | 0.042 | 2.983–3.448 | 0.053 | 18964.4 | 575 | 5 |
| essential512/slab | views | certified | words | 22.940 | 17.332 | 17.328 | 0.001 | 16.515–19.587 | 0.027 | 5071.4 | 44045 | 5 |
| packed512/enum | materialized | materialized | words | 32.425 | 6.738 | 6.068 | 0.650 | 6.537–7.088 | 0.688 | 12604.2 | 69602 | 5 |
| prefix512/slab | views | certified | words | 11.285 | 7.162 | 2.736 | 4.392 | 6.995–7.212 | 4.510 | 2996.1 | 18751 | 5 |
| retraction512/slab | views | certified | words | 14.616 | 20.759 | 14.492 | 6.283 | 20.729–21.082 | 7.046 | 3653.9 | 26100 | 5 |

## holes, 18 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 224.700 | 7.051 | 7.008 | 0.042 | 6.934–7.201 | 6.886 | 18947.4 | 575 | 5 |
| essential512/slab | views | certified | words | 4.728 | 44.435 | 44.434 | 0.001 | 43.212–46.203 | 8.766 | 7905.0 | 73266 | 5 |
| packed512/enum | materialized | materialized | words | 1.667 | 25.479 | 25.402 | 0.075 | 24.496–26.160 | 14.864 | 17678.7 | 151260 | 5 |
| prefix512/slab | views | certified | words | 9.063 | 9.139 | 8.908 | 0.232 | 8.983–9.747 | 3.771 | 2548.4 | 21414 | 5 |
| retraction512/slab | views | certified | words | 11.237 | 15.474 | 15.297 | 0.173 | 15.342–15.622 | 5.964 | 4807.4 | 36857 | 5 |

## holes, 18 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 224.700 | 3.093 | 3.052 | 0.041 | 3.013–3.167 | 0.052 | 18964.4 | 575 | 5 |
| essential512/slab | views | certified | words | 4.728 | 40.064 | 40.063 | 0.001 | 39.583–43.717 | 0.027 | 7918.5 | 73266 | 5 |
| packed512/enum | materialized | materialized | words | 1.667 | 15.956 | 15.871 | 0.074 | 15.558–16.127 | 0.100 | 17695.7 | 151260 | 5 |
| prefix512/slab | views | certified | words | 9.063 | 7.516 | 7.275 | 0.234 | 7.328–7.566 | 0.271 | 2561.9 | 21414 | 5 |
| retraction512/slab | views | certified | words | 11.237 | 12.800 | 12.619 | 0.180 | 12.686–13.103 | 0.211 | 4820.9 | 36857 | 5 |

160 configurations; 128 matched completion/control comparisons; 72 retained processes.

- [Raw evidence](results/prefix-smoke.json).
- [Raw evidence](results/prefix-repeat.json).
