# Product-fibre counting: native Free Join comparison

Construction is measured separately. Fresh and warm queries include exact legal
population checksums over all 80 outputs. Every output is also checked pointwise
outside timing. Retained KB includes construction residue and owner caches; temporary
observation memo and allocator overhead are excluded. No raw alias counts are used.
Executable: `95c0134c4eafbd775d0c8de0df93967cab03660d1135e2981aeda95f9fcf7305`.

## below, 12 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.101 | 0.180 | 0.180 | 0.001 | 0.180–0.180 | 0.136 | 101.1 | 173 | 1 |
| essential512/slab | views | certified | words | joint | 0.510 | 1.607 | 1.606 | 0.000 | 1.607–1.607 | 0.636 | 173.9 | 1408 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.598 | 1.156 | 1.132 | 0.024 | 1.156–1.156 | 0.896 | 330.6 | 2427 | 1 |
| prefix512/slab | views | certified | words | faces | 0.606 | 0.518 | 0.501 | 0.017 | 0.518–0.518 | 0.329 | 109.9 | 827 | 1 |
| prefix512/slab | views | certified | words | joint | 0.625 | 0.602 | 0.522 | 0.080 | 0.602–0.602 | 0.388 | 109.9 | 827 | 1 |
| retraction512/slab | views | certified | words | faces | 0.584 | 0.957 | 0.945 | 0.011 | 0.957–0.957 | 0.740 | 127.1 | 962 | 1 |
| retraction512/slab | views | certified | words | joint | 0.642 | 1.032 | 0.926 | 0.106 | 1.032–1.032 | 0.826 | 127.1 | 962 | 1 |

## below, 12 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.101 | 0.080 | 0.080 | 0.001 | 0.080–0.080 | 0.013 | 114.1 | 173 | 1 |
| essential512/slab | views | certified | words | joint | 0.510 | 1.162 | 1.162 | 0.000 | 1.162–1.162 | 0.049 | 185.8 | 1408 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.598 | 0.555 | 0.531 | 0.024 | 0.555–0.555 | 0.036 | 343.6 | 2427 | 1 |
| prefix512/slab | views | certified | words | faces | 0.606 | 0.339 | 0.326 | 0.012 | 0.339–0.339 | 0.026 | 121.8 | 827 | 1 |
| prefix512/slab | views | certified | words | joint | 0.625 | 0.396 | 0.325 | 0.070 | 0.396–0.396 | 0.086 | 121.8 | 827 | 1 |
| retraction512/slab | views | certified | words | faces | 0.584 | 0.503 | 0.496 | 0.007 | 0.503–0.503 | 0.018 | 139.0 | 962 | 1 |
| retraction512/slab | views | certified | words | joint | 0.642 | 0.623 | 0.526 | 0.097 | 0.623–0.623 | 0.109 | 139.0 | 962 | 1 |

## below, 12 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.168 | 0.530 | 0.526 | 0.003 | 0.530–0.530 | 0.362 | 101.1 | 173 | 1 |
| essential512/slab | views | certified | words | joint | 0.364 | 2.359 | 2.359 | 0.000 | 2.359–2.359 | 0.817 | 306.8 | 2528 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.129 | 2.275 | 2.264 | 0.011 | 2.275–2.275 | 1.826 | 802.8 | 4803 | 1 |
| prefix512/slab | views | certified | words | faces | 0.617 | 0.689 | 0.672 | 0.017 | 0.689–0.689 | 0.431 | 126.1 | 1146 | 1 |
| prefix512/slab | views | certified | words | joint | 0.687 | 0.750 | 0.698 | 0.052 | 0.750–0.750 | 0.559 | 126.1 | 1146 | 1 |
| retraction512/slab | views | certified | words | faces | 0.633 | 0.928 | 0.916 | 0.012 | 0.928–0.928 | 0.598 | 181.3 | 1364 | 1 |
| retraction512/slab | views | certified | words | joint | 0.599 | 0.917 | 0.881 | 0.036 | 0.917–0.917 | 0.589 | 181.3 | 1364 | 1 |

## below, 12 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.168 | 0.284 | 0.281 | 0.003 | 0.284–0.284 | 0.041 | 114.1 | 173 | 1 |
| essential512/slab | views | certified | words | joint | 0.364 | 1.886 | 1.885 | 0.000 | 1.886–1.886 | 0.015 | 318.7 | 2528 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.129 | 0.941 | 0.930 | 0.011 | 0.941–0.941 | 0.024 | 815.8 | 4803 | 1 |
| prefix512/slab | views | certified | words | faces | 0.617 | 0.451 | 0.438 | 0.012 | 0.451–0.451 | 0.024 | 138.0 | 1146 | 1 |
| prefix512/slab | views | certified | words | joint | 0.687 | 0.558 | 0.514 | 0.044 | 0.558–0.558 | 0.067 | 138.0 | 1146 | 1 |
| retraction512/slab | views | certified | words | faces | 0.633 | 0.601 | 0.593 | 0.007 | 0.601–0.601 | 0.030 | 193.2 | 1364 | 1 |
| retraction512/slab | views | certified | words | joint | 0.599 | 0.599 | 0.565 | 0.034 | 0.599–0.599 | 0.044 | 193.2 | 1364 | 1 |

## below, 18 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 6.308 | 7.187 | 7.145 | 0.042 | 7.187–7.187 | 6.750 | 7418.7 | 224 | 1 |
| essential512/slab | views | certified | words | joint | 2.677 | 8.728 | 8.727 | 0.001 | 8.728–8.728 | 3.431 | 1175.3 | 8419 | 1 |
| packed512/enum | materialized | materialized | words | joint | 28.094 | 3.374 | 3.192 | 0.182 | 3.374–3.374 | 2.386 | 2465.2 | 17618 | 1 |
| prefix512/slab | views | certified | words | faces | 6.045 | 3.370 | 3.282 | 0.088 | 3.370–3.370 | 2.303 | 1324.4 | 8465 | 1 |
| prefix512/slab | views | certified | words | joint | 6.230 | 4.944 | 3.327 | 1.617 | 4.944–4.944 | 3.728 | 1324.4 | 8465 | 1 |
| retraction512/slab | views | certified | words | faces | 7.913 | 5.736 | 5.674 | 0.062 | 5.736–5.736 | 3.950 | 1705.2 | 13041 | 1 |
| retraction512/slab | views | certified | words | joint | 7.999 | 7.682 | 5.770 | 1.911 | 7.682–7.682 | 5.957 | 1705.2 | 13041 | 1 |

## below, 18 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 6.308 | 2.784 | 2.742 | 0.042 | 2.784–2.784 | 0.056 | 7431.7 | 224 | 1 |
| essential512/slab | views | certified | words | joint | 2.677 | 6.480 | 6.479 | 0.000 | 6.480–6.480 | 0.033 | 1188.8 | 8419 | 1 |
| packed512/enum | materialized | materialized | words | joint | 28.094 | 2.107 | 1.931 | 0.176 | 2.107–2.107 | 0.211 | 2478.1 | 17618 | 1 |
| prefix512/slab | views | certified | words | faces | 6.045 | 2.481 | 2.394 | 0.086 | 2.481–2.481 | 0.149 | 1337.9 | 8465 | 1 |
| prefix512/slab | views | certified | words | joint | 6.230 | 3.958 | 2.332 | 1.625 | 3.958–3.958 | 1.635 | 1337.9 | 8465 | 1 |
| retraction512/slab | views | certified | words | faces | 7.913 | 3.848 | 3.788 | 0.060 | 3.848–3.848 | 0.096 | 1718.7 | 13041 | 1 |
| retraction512/slab | views | certified | words | joint | 7.999 | 5.778 | 3.914 | 1.864 | 5.778–5.778 | 1.947 | 1718.7 | 13041 | 1 |

## below, 18 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 7.507 | 8.590 | 8.537 | 0.053 | 8.590–8.590 | 8.194 | 7418.7 | 224 | 1 |
| essential512/slab | views | certified | words | joint | 1.430 | 26.172 | 26.171 | 0.001 | 26.172–26.172 | 8.035 | 4275.4 | 33993 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.523 | 11.300 | 11.254 | 0.046 | 11.300–11.300 | 7.161 | 6857.2 | 60584 | 1 |
| prefix512/slab | views | certified | words | faces | 6.065 | 9.728 | 9.629 | 0.098 | 9.728–9.728 | 4.168 | 2525.7 | 19056 | 1 |
| prefix512/slab | views | certified | words | joint | 6.464 | 10.344 | 10.202 | 0.142 | 10.344–10.344 | 18.345 | 2525.7 | 19056 | 1 |
| retraction512/slab | views | certified | words | faces | 7.260 | 12.159 | 12.094 | 0.066 | 12.159–12.159 | 5.106 | 3091.9 | 22416 | 1 |
| retraction512/slab | views | certified | words | joint | 6.710 | 10.824 | 10.728 | 0.096 | 10.824–10.824 | 4.716 | 3091.9 | 22416 | 1 |

## below, 18 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 7.507 | 3.340 | 3.287 | 0.053 | 3.340–3.340 | 0.068 | 7431.7 | 224 | 1 |
| essential512/slab | views | certified | words | joint | 1.430 | 22.538 | 22.537 | 0.001 | 22.538–22.538 | 0.031 | 4288.8 | 33993 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.523 | 6.893 | 6.839 | 0.054 | 6.893–6.893 | 0.086 | 6870.2 | 60584 | 1 |
| prefix512/slab | views | certified | words | faces | 6.065 | 8.189 | 8.094 | 0.095 | 8.189–8.189 | 0.131 | 2539.2 | 19056 | 1 |
| prefix512/slab | views | certified | words | joint | 6.464 | 8.177 | 8.045 | 0.131 | 8.177–8.177 | 0.165 | 2539.2 | 19056 | 1 |
| retraction512/slab | views | certified | words | faces | 7.260 | 9.475 | 9.416 | 0.058 | 9.475–9.475 | 0.093 | 3105.4 | 22416 | 1 |
| retraction512/slab | views | certified | words | joint | 6.710 | 9.161 | 9.061 | 0.099 | 9.161–9.161 | 0.141 | 3105.4 | 22416 | 1 |

## fibred, 13 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 1.300 | 0.286 | 0.284 | 0.001 | 0.286–0.286 | 0.233 | 317.0 | 287 | 1 |
| essential512/slab | views | certified | words | joint | 2.057 | 3.495 | 3.495 | 0.000 | 3.495–3.495 | 1.415 | 484.9 | 4420 | 1 |
| packed512/enum | materialized | materialized | words | joint | 1.333 | 1.695 | 1.634 | 0.061 | 1.695–1.695 | 1.105 | 1034.7 | 6535 | 1 |
| prefix512/slab | views | certified | words | faces | 1.340 | 1.012 | 0.987 | 0.025 | 1.012–1.012 | 0.655 | 308.9 | 1850 | 1 |
| prefix512/slab | views | certified | words | joint | 1.408 | 1.387 | 1.099 | 0.288 | 1.387–1.387 | 1.002 | 308.9 | 1850 | 1 |
| retraction512/slab | views | certified | words | faces | 1.345 | 1.818 | 1.792 | 0.026 | 1.818–1.818 | 1.568 | 315.2 | 2088 | 1 |
| retraction512/slab | views | certified | words | joint | 1.336 | 2.095 | 1.786 | 0.308 | 2.095–2.095 | 1.792 | 315.2 | 2088 | 1 |

## fibred, 13 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 1.300 | 0.127 | 0.126 | 0.001 | 0.127–0.127 | 0.015 | 330.0 | 287 | 1 |
| essential512/slab | views | certified | words | joint | 2.057 | 2.705 | 2.705 | 0.000 | 2.705–2.705 | 0.014 | 498.4 | 4420 | 1 |
| packed512/enum | materialized | materialized | words | joint | 1.333 | 0.845 | 0.804 | 0.041 | 0.845–0.845 | 0.053 | 1047.7 | 6535 | 1 |
| prefix512/slab | views | certified | words | faces | 1.340 | 0.653 | 0.632 | 0.021 | 0.653–0.653 | 0.053 | 322.4 | 1850 | 1 |
| prefix512/slab | views | certified | words | joint | 1.408 | 0.950 | 0.666 | 0.284 | 0.950–0.950 | 0.307 | 322.4 | 1850 | 1 |
| retraction512/slab | views | certified | words | faces | 1.345 | 1.122 | 1.107 | 0.015 | 1.122–1.122 | 0.028 | 328.7 | 2088 | 1 |
| retraction512/slab | views | certified | words | joint | 1.336 | 1.310 | 1.009 | 0.301 | 1.310–1.310 | 0.330 | 328.7 | 2088 | 1 |

## fibred, 13 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 1.284 | 0.267 | 0.266 | 0.001 | 0.267–0.267 | 0.236 | 317.0 | 287 | 1 |
| essential512/slab | views | certified | words | joint | 1.511 | 6.298 | 6.297 | 0.000 | 6.298–6.298 | 1.898 | 1088.7 | 7971 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.447 | 5.412 | 5.374 | 0.037 | 5.412–5.412 | 4.219 | 2327.3 | 16261 | 1 |
| prefix512/slab | views | certified | words | faces | 1.415 | 1.242 | 1.216 | 0.026 | 1.242–1.242 | 0.759 | 309.3 | 2474 | 1 |
| prefix512/slab | views | certified | words | joint | 1.394 | 1.323 | 1.185 | 0.139 | 1.323–1.323 | 0.861 | 309.3 | 2474 | 1 |
| retraction512/slab | views | certified | words | faces | 1.423 | 1.819 | 1.801 | 0.017 | 1.819–1.819 | 1.087 | 436.8 | 3031 | 1 |
| retraction512/slab | views | certified | words | joint | 1.446 | 1.876 | 1.757 | 0.119 | 1.876–1.876 | 1.203 | 436.8 | 3031 | 1 |

## fibred, 13 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 1.284 | 0.128 | 0.126 | 0.001 | 0.128–0.128 | 0.015 | 330.0 | 287 | 1 |
| essential512/slab | views | certified | words | joint | 1.511 | 5.160 | 5.159 | 0.000 | 5.160–5.160 | 0.014 | 1102.2 | 7971 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.447 | 2.514 | 2.481 | 0.033 | 2.514–2.514 | 0.049 | 2340.3 | 16261 | 1 |
| prefix512/slab | views | certified | words | faces | 1.415 | 0.858 | 0.836 | 0.022 | 0.858–0.858 | 0.034 | 322.8 | 2474 | 1 |
| prefix512/slab | views | certified | words | joint | 1.394 | 0.913 | 0.782 | 0.130 | 0.913–0.913 | 0.150 | 322.8 | 2474 | 1 |
| retraction512/slab | views | certified | words | faces | 1.423 | 1.148 | 1.134 | 0.014 | 1.148–1.148 | 0.040 | 450.3 | 3031 | 1 |
| retraction512/slab | views | certified | words | joint | 1.446 | 1.236 | 1.122 | 0.113 | 1.236–1.236 | 0.125 | 450.3 | 3031 | 1 |

## fibred, 19 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 465.563 | 15.493 | 15.406 | 0.086 | 15.493–15.493 | 14.522 | 45001.8 | 684 | 1 |
| essential512/slab | views | certified | words | joint | 29.138 | 29.744 | 29.743 | 0.001 | 29.744–29.744 | 10.531 | 6853.3 | 59905 | 1 |
| packed512/enum | materialized | materialized | words | joint | 60.700 | 11.349 | 10.500 | 0.849 | 11.349–11.349 | 6.700 | 13305.1 | 98153 | 1 |
| prefix512/slab | views | certified | words | faces | 27.689 | 7.607 | 7.426 | 0.181 | 7.607–7.607 | 5.082 | 6471.9 | 43265 | 1 |
| prefix512/slab | views | certified | words | joint | 29.066 | 14.487 | 7.586 | 6.900 | 14.487–14.487 | 11.631 | 6471.9 | 43265 | 1 |
| retraction512/slab | views | certified | words | faces | 34.452 | 34.799 | 34.654 | 0.144 | 34.799–34.799 | 30.364 | 9952.9 | 59894 | 1 |
| retraction512/slab | views | certified | words | joint | 35.420 | 44.782 | 35.766 | 9.015 | 44.782–44.782 | 40.702 | 9952.9 | 59894 | 1 |

## fibred, 19 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 465.563 | 6.479 | 6.378 | 0.101 | 6.479–6.479 | 0.115 | 45018.8 | 684 | 1 |
| essential512/slab | views | certified | words | joint | 29.138 | 26.026 | 26.025 | 0.001 | 26.026–26.026 | 0.031 | 6867.1 | 59905 | 1 |
| packed512/enum | materialized | materialized | words | joint | 60.700 | 8.366 | 7.511 | 0.855 | 8.366–8.366 | 0.879 | 13322.2 | 98153 | 1 |
| prefix512/slab | views | certified | words | faces | 27.689 | 5.359 | 5.188 | 0.171 | 5.359–5.359 | 0.204 | 6485.7 | 43265 | 1 |
| prefix512/slab | views | certified | words | joint | 29.066 | 11.986 | 5.004 | 6.981 | 11.986–11.986 | 6.880 | 6485.7 | 43265 | 1 |
| retraction512/slab | views | certified | words | faces | 34.452 | 19.988 | 19.856 | 0.131 | 19.988–19.988 | 0.175 | 9966.6 | 59894 | 1 |
| retraction512/slab | views | certified | words | joint | 35.420 | 28.563 | 19.796 | 8.766 | 28.563–28.563 | 8.937 | 9966.6 | 59894 | 1 |

## fibred, 19 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 464.328 | 15.445 | 15.357 | 0.088 | 15.445–15.445 | 14.226 | 45001.8 | 684 | 1 |
| essential512/slab | views | certified | words | joint | 7.178 | 68.599 | 68.599 | 0.001 | 68.599–68.599 | 17.046 | 12039.0 | 109616 | 1 |
| packed512/enum | materialized | materialized | words | joint | 2.551 | 44.896 | 44.771 | 0.125 | 44.896–44.896 | 26.587 | 25957.2 | 218715 | 1 |
| prefix512/slab | views | certified | words | faces | 24.386 | 20.798 | 20.566 | 0.232 | 20.798–20.798 | 8.510 | 7077.6 | 57705 | 1 |
| prefix512/slab | views | certified | words | joint | 23.577 | 19.359 | 19.003 | 0.356 | 19.359–19.359 | 8.680 | 7077.6 | 57705 | 1 |
| retraction512/slab | views | certified | words | faces | 29.132 | 27.860 | 27.712 | 0.148 | 27.860–27.860 | 11.609 | 9117.2 | 80348 | 1 |
| retraction512/slab | views | certified | words | joint | 29.095 | 27.665 | 27.378 | 0.287 | 27.665–27.665 | 11.652 | 9117.2 | 80348 | 1 |

## fibred, 19 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 464.328 | 6.603 | 6.512 | 0.085 | 6.603–6.603 | 0.108 | 45018.8 | 684 | 1 |
| essential512/slab | views | certified | words | joint | 7.178 | 68.226 | 68.225 | 0.001 | 68.226–68.226 | 0.038 | 12052.8 | 109616 | 1 |
| packed512/enum | materialized | materialized | words | joint | 2.551 | 26.670 | 26.534 | 0.136 | 26.670–26.670 | 0.164 | 25974.3 | 218715 | 1 |
| prefix512/slab | views | certified | words | faces | 24.386 | 15.341 | 15.145 | 0.195 | 15.341–15.341 | 0.243 | 7091.4 | 57705 | 1 |
| prefix512/slab | views | certified | words | joint | 23.577 | 16.323 | 15.948 | 0.375 | 16.323–16.323 | 0.402 | 7091.4 | 57705 | 1 |
| retraction512/slab | views | certified | words | faces | 29.132 | 22.011 | 21.886 | 0.125 | 22.011–22.011 | 0.167 | 9130.9 | 80348 | 1 |
| retraction512/slab | views | certified | words | joint | 29.095 | 21.867 | 21.594 | 0.273 | 21.867–21.867 | 0.343 | 9130.9 | 80348 | 1 |

## holes, 12 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.607 | 0.215 | 0.214 | 0.001 | 0.215–0.215 | 0.147 | 148.7 | 250 | 1 |
| essential512/slab | views | certified | words | joint | 1.258 | 2.095 | 2.094 | 0.000 | 2.095–2.095 | 0.817 | 385.4 | 2524 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.752 | 1.328 | 1.301 | 0.027 | 1.328–1.328 | 0.974 | 587.9 | 3459 | 1 |
| prefix512/slab | views | certified | words | faces | 0.702 | 0.615 | 0.599 | 0.015 | 0.615–0.615 | 0.413 | 139.5 | 1017 | 1 |
| prefix512/slab | views | certified | words | joint | 0.692 | 0.751 | 0.608 | 0.143 | 0.751–0.751 | 0.515 | 139.5 | 1017 | 1 |
| retraction512/slab | views | certified | words | faces | 0.629 | 1.024 | 1.014 | 0.010 | 1.024–1.024 | 0.809 | 138.5 | 1135 | 1 |
| retraction512/slab | views | certified | words | joint | 0.635 | 1.195 | 1.030 | 0.165 | 1.195–1.195 | 1.029 | 138.5 | 1135 | 1 |

## holes, 12 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.607 | 0.095 | 0.094 | 0.001 | 0.095–0.095 | 0.015 | 163.9 | 250 | 1 |
| essential512/slab | views | certified | words | joint | 1.258 | 1.705 | 1.705 | 0.000 | 1.705–1.705 | 0.017 | 398.9 | 2524 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.752 | 0.623 | 0.598 | 0.024 | 0.623–0.623 | 0.044 | 603.1 | 3459 | 1 |
| prefix512/slab | views | certified | words | faces | 0.702 | 0.430 | 0.419 | 0.011 | 0.430–0.430 | 0.028 | 153.0 | 1017 | 1 |
| prefix512/slab | views | certified | words | joint | 0.692 | 0.588 | 0.445 | 0.142 | 0.588–0.588 | 0.150 | 153.0 | 1017 | 1 |
| retraction512/slab | views | certified | words | faces | 0.629 | 0.575 | 0.568 | 0.006 | 0.575–0.575 | 0.018 | 152.0 | 1135 | 1 |
| retraction512/slab | views | certified | words | joint | 0.635 | 0.804 | 0.638 | 0.166 | 0.804–0.804 | 0.170 | 152.0 | 1135 | 1 |

## holes, 12 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.613 | 0.189 | 0.188 | 0.001 | 0.189–0.189 | 0.150 | 148.7 | 250 | 1 |
| essential512/slab | views | certified | words | joint | 0.997 | 4.271 | 4.271 | 0.000 | 4.271–4.271 | 1.105 | 894.9 | 5252 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.365 | 3.823 | 3.802 | 0.021 | 3.823–3.823 | 3.228 | 1767.5 | 11385 | 1 |
| prefix512/slab | views | certified | words | faces | 2.034 | 1.164 | 1.142 | 0.022 | 1.164–1.164 | 0.516 | 146.9 | 1232 | 1 |
| prefix512/slab | views | certified | words | joint | 0.669 | 0.786 | 0.705 | 0.081 | 0.786–0.786 | 0.474 | 146.9 | 1232 | 1 |
| retraction512/slab | views | certified | words | faces | 0.635 | 1.022 | 1.011 | 0.011 | 1.022–1.022 | 0.635 | 219.0 | 1636 | 1 |
| retraction512/slab | views | certified | words | joint | 0.625 | 1.101 | 1.035 | 0.066 | 1.101–1.101 | 0.715 | 219.0 | 1636 | 1 |

## holes, 12 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.613 | 0.092 | 0.091 | 0.001 | 0.092–0.092 | 0.014 | 163.9 | 250 | 1 |
| essential512/slab | views | certified | words | joint | 0.997 | 3.687 | 3.687 | 0.000 | 3.687–3.687 | 0.019 | 908.4 | 5252 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.365 | 1.944 | 1.924 | 0.019 | 1.944–1.944 | 0.033 | 1782.8 | 11385 | 1 |
| prefix512/slab | views | certified | words | faces | 2.034 | 0.558 | 0.546 | 0.012 | 0.558–0.558 | 0.033 | 160.4 | 1232 | 1 |
| prefix512/slab | views | certified | words | joint | 0.669 | 0.540 | 0.466 | 0.074 | 0.540–0.540 | 0.088 | 160.4 | 1232 | 1 |
| retraction512/slab | views | certified | words | faces | 0.635 | 0.669 | 0.662 | 0.007 | 0.669–0.669 | 0.018 | 232.5 | 1636 | 1 |
| retraction512/slab | views | certified | words | joint | 0.625 | 0.717 | 0.658 | 0.060 | 0.717–0.717 | 0.071 | 232.5 | 1636 | 1 |

## holes, 18 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 228.023 | 7.395 | 7.351 | 0.043 | 7.395–7.395 | 6.856 | 18947.4 | 575 | 1 |
| essential512/slab | views | certified | words | joint | 21.920 | 20.200 | 20.199 | 0.000 | 20.200–20.200 | 7.147 | 5057.9 | 44045 | 1 |
| packed512/enum | materialized | materialized | words | joint | 31.836 | 9.125 | 8.487 | 0.637 | 9.125–9.125 | 4.970 | 12587.2 | 69602 | 1 |
| prefix512/slab | views | certified | words | faces | 12.375 | 3.802 | 3.701 | 0.100 | 3.802–3.802 | 2.547 | 2991.5 | 18837 | 1 |
| prefix512/slab | views | certified | words | joint | 11.816 | 8.069 | 3.696 | 4.374 | 8.069–8.069 | 6.556 | 2991.5 | 18837 | 1 |
| retraction512/slab | views | certified | words | faces | 15.011 | 26.736 | 26.671 | 0.065 | 26.736–26.736 | 25.560 | 3649.2 | 26179 | 1 |
| retraction512/slab | views | certified | words | joint | 15.134 | 35.132 | 28.809 | 6.323 | 35.132–35.132 | 32.879 | 3649.2 | 26179 | 1 |

## holes, 18 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 228.023 | 3.062 | 3.020 | 0.041 | 3.062–3.062 | 0.056 | 18964.4 | 575 | 1 |
| essential512/slab | views | certified | words | joint | 21.920 | 17.225 | 17.224 | 0.001 | 17.225–17.225 | 0.027 | 5071.4 | 44045 | 1 |
| packed512/enum | materialized | materialized | words | joint | 31.836 | 6.626 | 5.981 | 0.644 | 6.626–6.626 | 0.687 | 12604.2 | 69602 | 1 |
| prefix512/slab | views | certified | words | faces | 12.375 | 2.768 | 2.671 | 0.096 | 2.768–2.768 | 0.128 | 3005.0 | 18837 | 1 |
| prefix512/slab | views | certified | words | joint | 11.816 | 7.353 | 2.982 | 4.369 | 7.353–7.353 | 4.346 | 3005.0 | 18837 | 1 |
| retraction512/slab | views | certified | words | faces | 15.011 | 14.761 | 14.699 | 0.061 | 14.761–14.761 | 0.125 | 3662.7 | 26179 | 1 |
| retraction512/slab | views | certified | words | joint | 15.134 | 20.547 | 14.293 | 6.253 | 20.547–20.547 | 6.523 | 3662.7 | 26179 | 1 |

## holes, 18 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 227.272 | 7.642 | 7.595 | 0.047 | 7.642–7.642 | 7.070 | 18947.4 | 575 | 1 |
| essential512/slab | views | certified | words | joint | 4.851 | 44.655 | 44.654 | 0.001 | 44.655–44.655 | 8.625 | 7905.0 | 73266 | 1 |
| packed512/enum | materialized | materialized | words | joint | 1.685 | 25.915 | 25.842 | 0.072 | 25.915–25.915 | 14.849 | 17678.7 | 151260 | 1 |
| prefix512/slab | views | certified | words | faces | 10.599 | 11.057 | 10.915 | 0.141 | 11.057–11.057 | 4.233 | 2551.3 | 21442 | 1 |
| prefix512/slab | views | certified | words | joint | 8.784 | 8.921 | 8.693 | 0.228 | 8.921–8.921 | 3.746 | 2551.3 | 21442 | 1 |
| retraction512/slab | views | certified | words | faces | 11.713 | 15.816 | 15.747 | 0.069 | 15.816–15.816 | 6.128 | 4811.6 | 36885 | 1 |
| retraction512/slab | views | certified | words | joint | 11.775 | 16.466 | 16.283 | 0.183 | 16.466–16.466 | 6.817 | 4811.6 | 36885 | 1 |

## holes, 18 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 227.272 | 3.129 | 3.087 | 0.042 | 3.129–3.129 | 0.056 | 18964.4 | 575 | 1 |
| essential512/slab | views | certified | words | joint | 4.851 | 43.511 | 43.510 | 0.001 | 43.511–43.511 | 0.030 | 7918.5 | 73266 | 1 |
| packed512/enum | materialized | materialized | words | joint | 1.685 | 16.198 | 16.123 | 0.074 | 16.198–16.198 | 0.112 | 17695.7 | 151260 | 1 |
| prefix512/slab | views | certified | words | faces | 10.599 | 8.570 | 8.438 | 0.132 | 8.570–8.570 | 0.162 | 2564.8 | 21442 | 1 |
| prefix512/slab | views | certified | words | joint | 8.784 | 8.045 | 7.797 | 0.247 | 8.045–8.045 | 0.289 | 2564.8 | 21442 | 1 |
| retraction512/slab | views | certified | words | faces | 11.713 | 13.302 | 13.234 | 0.068 | 13.302–13.302 | 0.132 | 4825.1 | 36885 | 1 |
| retraction512/slab | views | certified | words | joint | 11.775 | 13.293 | 13.111 | 0.182 | 13.293–13.293 | 0.230 | 4825.1 | 36885 | 1 |

168 configurations; 384 matched completion/control comparisons; 43 retained processes.

- [Raw evidence](results/factor-smoke.json).
