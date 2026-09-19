# Checked legal relations through Free Join

Fresh queries include input-role checks, actual Free Join, group union/intersection,
closure, converse, residual, May and nonvacuous Must: 80 complete Event outputs.
Every fresh/warm result is checked pointwise against legal-state matrices outside timing.
Construction/admission are separate phases; timings include output checksums.
Resident KB includes outer caches and retained map certificates, excluding temporary
plane caches and allocator overhead. See [the contract](LEGAL-RELATIONS.md).
Executable: `988d0cbb34c3f2a1087bb895e74e5b1afb7cc95549646a316de628d02d02d082`.

## below, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.183 | 0.183–0.183 | 0.149 | 101.1 | 173 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 1.757 | 1.757–1.757 | 0.747 | 285.7 | 1521 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 2.242 | 2.242–2.242 | 1.056 | 285.7 | 1521 | 1 |
| essential512/slab | materialized | deferred | off | materialized | 2.085 | 2.083–2.086 | 0.640 | 330.9 | 2130 | 2 |
| essential512/slab | views | source | bounded | certified | 1.549 | 1.540–1.558 | 0.691 | 173.9 | 1408 | 2 |
| essential512/slab | views | source | bounded | gated | 2.022 | 1.994–2.050 | 0.981 | 204.2 | 1408 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 1.837 | 1.837–1.837 | 0.746 | 204.2 | 1521 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 2.144 | 2.144–2.144 | 1.125 | 204.2 | 1521 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 3.017 | 3.017–3.017 | 1.523 | 488.3 | 3168 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 3.580 | 3.580–3.580 | 2.358 | 488.3 | 3168 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 3.197 | 3.197–3.197 | 1.468 | 347.5 | 3168 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 3.594 | 3.594–3.594 | 2.257 | 347.5 | 3168 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 1.119 | 1.119–1.119 | 0.893 | 330.6 | 2427 | 1 |

## below, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.086 | 0.086–0.086 | 0.014 | 114.1 | 173 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 1.596 | 1.596–1.596 | 0.024 | 298.3 | 1521 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 1.530 | 1.530–1.530 | 0.013 | 298.3 | 1521 | 1 |
| essential512/slab | materialized | deferred | off | materialized | 1.725 | 1.711–1.740 | 0.025 | 343.9 | 2130 | 2 |
| essential512/slab | views | source | bounded | certified | 1.211 | 1.176–1.245 | 0.020 | 185.8 | 1408 | 2 |
| essential512/slab | views | source | bounded | gated | 1.439 | 1.428–1.449 | 0.013 | 216.1 | 1408 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 1.426 | 1.426–1.426 | 0.020 | 216.8 | 1521 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 1.743 | 1.743–1.743 | 0.023 | 216.8 | 1521 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 2.102 | 2.102–2.102 | 0.019 | 500.9 | 3168 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 2.584 | 2.584–2.584 | 0.023 | 500.9 | 3168 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 2.190 | 2.190–2.190 | 0.018 | 360.1 | 3168 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 2.549 | 2.549–2.549 | 0.015 | 360.1 | 3168 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 0.501 | 0.501–0.501 | 0.047 | 343.6 | 2427 | 1 |

## below, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.174 | 0.166–0.182 | 0.146 | 101.1 | 173 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 3.101 | 3.063–3.140 | 0.949 | 520.5 | 3295 | 2 |
| essential512/slab | views | source | bounded | certified | 2.405 | 2.337–2.474 | 0.840 | 306.8 | 2528 | 2 |
| essential512/slab | views | source | bounded | gated | 2.713 | 2.629–2.797 | 1.063 | 306.6 | 2528 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 2.581 | 2.570–2.591 | 1.096 | 306.6 | 2120 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 3.039 | 2.971–3.108 | 1.279 | 306.6 | 2120 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 2.310 | 2.252–2.368 | 1.935 | 802.8 | 4803 | 2 |

## below, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.080 | 0.079–0.082 | 0.014 | 114.1 | 173 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 2.648 | 2.583–2.713 | 0.022 | 533.5 | 3295 | 2 |
| essential512/slab | views | source | bounded | certified | 1.921 | 1.880–1.962 | 0.015 | 318.7 | 2528 | 2 |
| essential512/slab | views | source | bounded | gated | 2.086 | 2.080–2.093 | 0.013 | 318.5 | 2528 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 2.093 | 2.076–2.110 | 0.023 | 319.2 | 2120 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 2.252 | 2.237–2.266 | 0.045 | 319.2 | 2120 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 0.932 | 0.913–0.951 | 0.027 | 815.8 | 4803 | 2 |

## below, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 7.473 | 7.473–7.473 | 7.005 | 7418.7 | 224 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 10.027 | 10.027–10.027 | 4.374 | 1743.9 | 10032 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 11.401 | 11.401–11.401 | 5.415 | 1987.6 | 10032 | 1 |
| essential512/slab | materialized | deferred | off | materialized | 14.064 | 13.592–14.537 | 3.799 | 2226.7 | 16146 | 2 |
| essential512/slab | views | source | bounded | certified | 8.923 | 8.585–9.262 | 3.432 | 1175.3 | 8419 | 2 |
| essential512/slab | views | source | bounded | gated | 10.198 | 10.190–10.207 | 4.774 | 1175.0 | 8419 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 9.537 | 9.537–9.537 | 3.941 | 1175.0 | 10032 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 11.587 | 11.587–11.587 | 5.190 | 1418.8 | 10032 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 11.641 | 11.641–11.641 | 6.225 | 2556.2 | 15263 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 13.957 | 13.957–13.957 | 7.167 | 2556.2 | 15263 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 11.636 | 11.636–11.636 | 5.669 | 1628.5 | 15263 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 13.153 | 13.153–13.153 | 6.947 | 1628.5 | 15263 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 3.288 | 3.288–3.288 | 2.147 | 2465.2 | 17618 | 1 |

## below, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 2.839 | 2.839–2.839 | 0.066 | 7431.7 | 224 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 7.908 | 7.908–7.908 | 0.029 | 1758.1 | 10032 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 9.195 | 9.195–9.195 | 0.031 | 2001.8 | 10032 | 1 |
| essential512/slab | materialized | deferred | off | materialized | 11.255 | 11.025–11.484 | 0.032 | 2239.7 | 16146 | 2 |
| essential512/slab | views | source | bounded | certified | 7.251 | 6.748–7.754 | 0.028 | 1188.8 | 8419 | 2 |
| essential512/slab | views | source | bounded | gated | 8.150 | 7.645–8.656 | 0.025 | 1188.5 | 8419 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 7.664 | 7.664–7.664 | 0.036 | 1189.2 | 10032 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 8.774 | 8.774–8.774 | 0.026 | 1432.9 | 10032 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 9.000 | 9.000–9.000 | 0.025 | 2570.3 | 15263 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 10.215 | 10.215–10.215 | 0.031 | 2570.3 | 15263 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 8.622 | 8.622–8.622 | 0.026 | 1642.7 | 15263 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 9.775 | 9.775–9.775 | 0.029 | 1642.7 | 15263 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 2.435 | 2.435–2.435 | 0.209 | 2478.1 | 17618 | 1 |

## below, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 7.531 | 7.530–7.532 | 7.191 | 7418.7 | 224 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 35.058 | 33.927–36.190 | 12.527 | 6598.9 | 49714 | 2 |
| essential512/slab | views | source | bounded | certified | 27.683 | 27.076–28.290 | 21.427 | 4275.4 | 33993 | 2 |
| essential512/slab | views | source | bounded | gated | 46.258 | 25.684–66.832 | 8.176 | 4275.1 | 33993 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 26.644 | 26.517–26.770 | 9.025 | 3544.0 | 27489 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 56.606 | 45.466–67.745 | 13.856 | 3544.0 | 27489 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 11.859 | 11.843–11.874 | 7.474 | 6857.2 | 60584 | 2 |

## below, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 3.074 | 3.049–3.100 | 0.056 | 7431.7 | 224 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 134.375 | 96.240–172.511 | 0.029 | 6611.9 | 49714 | 2 |
| essential512/slab | views | source | bounded | certified | 74.911 | 35.415–114.407 | 0.045 | 4288.8 | 33993 | 2 |
| essential512/slab | views | source | bounded | gated | 22.335 | 21.982–22.689 | 0.025 | 4288.6 | 33993 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 21.531 | 20.370–22.691 | 0.025 | 3558.1 | 27489 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 22.511 | 22.506–22.516 | 0.029 | 3558.1 | 27489 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 6.950 | 6.931–6.969 | 0.079 | 6870.2 | 60584 | 2 |

## fibred, 13 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.286 | 0.286–0.286 | 0.236 | 317.0 | 287 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 4.385 | 4.385–4.385 | 2.008 | 972.8 | 4766 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 5.620 | 5.620–5.620 | 2.735 | 1094.6 | 4766 | 1 |
| essential512/slab | materialized | deferred | off | materialized | 4.788 | 4.768–4.808 | 1.615 | 815.7 | 6212 | 2 |
| essential512/slab | views | source | bounded | certified | 3.759 | 3.651–3.867 | 1.521 | 484.9 | 4420 | 2 |
| essential512/slab | views | source | bounded | gated | 5.044 | 4.990–5.097 | 2.734 | 484.7 | 4420 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 4.310 | 4.310–4.310 | 1.762 | 611.8 | 4766 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 5.581 | 5.581–5.581 | 2.994 | 733.7 | 4766 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 10.002 | 10.002–10.002 | 4.768 | 2038.1 | 12084 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 15.076 | 15.076–15.076 | 9.709 | 2038.1 | 12084 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 10.157 | 10.157–10.157 | 5.206 | 1438.4 | 12084 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 14.636 | 14.636–14.636 | 9.327 | 1438.4 | 12084 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 1.661 | 1.661–1.661 | 1.163 | 1034.7 | 6535 | 1 |

## fibred, 13 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.132 | 0.132–0.132 | 0.015 | 330.0 | 287 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 3.486 | 3.486–3.486 | 0.019 | 986.9 | 4766 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 4.154 | 4.154–4.154 | 0.018 | 1108.8 | 4766 | 1 |
| essential512/slab | materialized | deferred | off | materialized | 4.540 | 4.525–4.556 | 0.027 | 828.7 | 6212 | 2 |
| essential512/slab | views | source | bounded | certified | 2.921 | 2.843–2.999 | 0.022 | 498.4 | 4420 | 2 |
| essential512/slab | views | source | bounded | gated | 3.580 | 3.563–3.597 | 0.018 | 498.2 | 4420 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 3.397 | 3.397–3.397 | 0.016 | 626.0 | 4766 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 3.943 | 3.943–3.943 | 0.018 | 747.9 | 4766 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 7.285 | 7.285–7.285 | 0.017 | 2052.2 | 12084 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 9.767 | 9.767–9.767 | 0.020 | 2052.2 | 12084 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 7.857 | 7.857–7.857 | 0.023 | 1452.5 | 12084 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 9.840 | 9.840–9.840 | 0.019 | 1452.5 | 12084 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 0.860 | 0.860–0.860 | 0.063 | 1047.7 | 6535 | 1 |

## fibred, 13 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.263 | 0.263–0.263 | 0.236 | 317.0 | 287 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 8.896 | 8.893–8.898 | 2.554 | 1395.2 | 10831 | 2 |
| essential512/slab | views | source | bounded | certified | 6.416 | 6.349–6.483 | 2.087 | 1088.7 | 7971 | 2 |
| essential512/slab | views | source | bounded | gated | 7.315 | 7.134–7.496 | 2.733 | 1088.5 | 7971 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 7.395 | 7.314–7.477 | 2.789 | 1033.4 | 7236 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 8.375 | 8.304–8.447 | 3.389 | 1033.4 | 7236 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 7.216 | 6.788–7.644 | 4.766 | 2327.3 | 16261 | 2 |

## fibred, 13 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.121 | 0.120–0.122 | 0.016 | 330.0 | 287 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 7.256 | 7.249–7.263 | 0.018 | 1408.2 | 10831 | 2 |
| essential512/slab | views | source | bounded | certified | 5.262 | 5.221–5.303 | 0.016 | 1102.2 | 7971 | 2 |
| essential512/slab | views | source | bounded | gated | 5.965 | 5.890–6.039 | 0.019 | 1102.0 | 7971 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 5.790 | 5.777–5.803 | 0.022 | 1047.6 | 7236 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 6.450 | 6.344–6.556 | 0.021 | 1047.6 | 7236 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 2.794 | 2.691–2.898 | 0.186 | 2340.3 | 16261 | 2 |

## fibred, 19 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 15.217 | 15.217–15.217 | 14.440 | 45001.8 | 684 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 48.903 | 48.903–48.903 | 15.238 | 13076.4 | 71409 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 59.197 | 59.197–59.197 | 27.442 | 13076.4 | 71409 | 1 |
| essential512/slab | materialized | deferred | off | materialized | 60.226 | 58.890–61.563 | 14.970 | 11510.3 | 96826 | 2 |
| essential512/slab | views | source | bounded | certified | 32.393 | 31.561–33.224 | 11.463 | 6853.3 | 59905 | 2 |
| essential512/slab | views | source | bounded | gated | 50.231 | 47.216–53.247 | 24.940 | 8802.8 | 59905 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 45.228 | 45.228–45.228 | 14.277 | 9389.3 | 71409 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 57.909 | 57.909–57.909 | 28.572 | 9389.3 | 71409 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 57.743 | 57.743–57.743 | 28.020 | 16302.0 | 99961 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 78.155 | 78.155–78.155 | 41.426 | 16302.0 | 99961 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 56.467 | 56.467–56.467 | 24.261 | 11311.6 | 99961 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 76.707 | 76.707–76.707 | 41.356 | 11311.6 | 99961 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 11.592 | 11.592–11.592 | 6.680 | 13305.1 | 98153 | 1 |

## fibred, 19 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 6.445 | 6.445–6.445 | 0.110 | 45018.8 | 684 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 33.118 | 33.118–33.118 | 0.035 | 13091.4 | 71409 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 43.411 | 43.411–43.411 | 0.026 | 13091.4 | 71409 | 1 |
| essential512/slab | materialized | deferred | off | materialized | 49.833 | 48.054–51.611 | 0.031 | 11527.3 | 96826 | 2 |
| essential512/slab | views | source | bounded | certified | 28.623 | 25.732–31.513 | 0.029 | 6867.1 | 59905 | 2 |
| essential512/slab | views | source | bounded | gated | 35.574 | 35.231–35.917 | 0.026 | 8816.5 | 59905 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 33.357 | 33.357–33.357 | 0.031 | 9404.4 | 71409 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 42.414 | 42.414–42.414 | 0.037 | 9404.4 | 71409 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 45.710 | 45.710–45.710 | 0.026 | 16317.1 | 99961 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 55.914 | 55.914–55.914 | 0.031 | 16317.1 | 99961 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 42.506 | 42.506–42.506 | 0.029 | 11326.7 | 99961 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 59.184 | 59.184–59.184 | 0.034 | 11326.7 | 99961 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 8.617 | 8.617–8.617 | 1.042 | 13322.2 | 98153 | 1 |

## fibred, 19 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 15.544 | 15.480–15.607 | 14.692 | 45001.8 | 684 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 117.893 | 110.648–125.139 | 21.294 | 19200.9 | 187955 | 2 |
| essential512/slab | views | source | bounded | certified | 72.842 | 72.538–73.145 | 17.714 | 12039.0 | 109616 | 2 |
| essential512/slab | views | source | bounded | gated | 81.060 | 80.256–81.863 | 21.029 | 12038.8 | 109616 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 87.216 | 82.762–91.670 | 23.850 | 11063.9 | 109832 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 83.642 | 82.859–84.424 | 23.978 | 11063.9 | 109832 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 76.804 | 64.510–89.099 | 27.836 | 25957.2 | 218715 | 2 |

## fibred, 19 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 6.385 | 6.339–6.432 | 0.105 | 45018.8 | 684 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 105.916 | 102.585–109.246 | 0.034 | 19217.9 | 187955 | 2 |
| essential512/slab | views | source | bounded | certified | 66.258 | 63.952–68.563 | 0.024 | 12052.8 | 109616 | 2 |
| essential512/slab | views | source | bounded | gated | 73.861 | 71.284–76.438 | 0.026 | 12052.5 | 109616 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 74.427 | 72.853–76.000 | 0.027 | 11079.0 | 109832 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 72.456 | 70.433–74.479 | 0.027 | 11079.0 | 109832 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 49.391 | 26.820–71.961 | 0.149 | 25974.3 | 218715 | 2 |

## full, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.177 | 0.177–0.177 | 0.144 | 83.5 | 140 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 0.512 | 0.512–0.512 | 0.327 | 35.5 | 205 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 0.537 | 0.537–0.537 | 0.329 | 35.5 | 205 | 1 |
| essential512/slab | views-inputs | source | bounded | certified | 0.548 | 0.548–0.548 | 0.342 | 24.9 | 205 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 0.527 | 0.527–0.527 | 0.350 | 24.9 | 205 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 0.935 | 0.935–0.935 | 0.601 | 86.0 | 663 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 0.915 | 0.915–0.915 | 0.598 | 86.0 | 663 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 1.242 | 1.242–1.242 | 0.708 | 54.2 | 663 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 0.898 | 0.898–0.898 | 0.632 | 54.2 | 663 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 0.880 | 0.880–0.880 | 0.769 | 75.3 | 482 | 1 |

## full, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.077 | 0.077–0.077 | 0.013 | 96.5 | 140 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 0.334 | 0.334–0.334 | 0.016 | 47.6 | 205 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 0.305 | 0.305–0.305 | 0.033 | 47.6 | 205 | 1 |
| essential512/slab | views-inputs | source | bounded | certified | 0.330 | 0.330–0.330 | 0.013 | 37.0 | 205 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 0.294 | 0.294–0.294 | 0.013 | 37.0 | 205 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 0.520 | 0.520–0.520 | 0.021 | 98.1 | 663 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 0.541 | 0.541–0.541 | 0.012 | 98.1 | 663 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 0.580 | 0.580–0.580 | 0.017 | 66.3 | 663 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 0.557 | 0.557–0.557 | 0.018 | 66.3 | 663 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 0.296 | 0.296–0.296 | 0.031 | 88.3 | 482 | 1 |

## full, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 7.523 | 7.523–7.523 | 7.138 | 5811.8 | 175 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 3.060 | 3.060–3.060 | 1.964 | 303.5 | 1645 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 3.104 | 3.104–3.104 | 2.284 | 303.5 | 1645 | 1 |
| essential512/slab | views-inputs | source | bounded | certified | 3.008 | 3.008–3.008 | 2.081 | 202.5 | 1645 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 2.901 | 2.901–2.901 | 2.078 | 202.5 | 1645 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 4.224 | 4.224–4.224 | 2.954 | 444.8 | 3294 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 4.059 | 4.059–4.059 | 2.955 | 444.8 | 3294 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 4.103 | 4.103–4.103 | 2.966 | 299.2 | 3294 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 4.075 | 4.075–4.075 | 3.039 | 299.2 | 3294 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 1.559 | 1.559–1.559 | 1.318 | 500.5 | 3815 | 1 |

## full, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 2.573 | 2.573–2.573 | 0.053 | 5824.8 | 175 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 2.016 | 2.016–2.016 | 0.024 | 316.1 | 1645 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 2.170 | 2.170–2.170 | 0.026 | 316.1 | 1645 | 1 |
| essential512/slab | views-inputs | source | bounded | certified | 1.959 | 1.959–1.959 | 0.027 | 215.1 | 1645 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 1.946 | 1.946–1.946 | 0.024 | 215.1 | 1645 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 2.599 | 2.599–2.599 | 0.024 | 457.3 | 3294 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 2.703 | 2.703–2.703 | 0.025 | 457.3 | 3294 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 2.613 | 2.613–2.613 | 0.024 | 311.7 | 3294 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 2.591 | 2.591–2.591 | 0.025 | 311.7 | 3294 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 0.766 | 0.766–0.766 | 0.077 | 513.5 | 3815 | 1 |

## holes, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.182 | 0.182–0.182 | 0.144 | 148.7 | 250 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 2.438 | 2.438–2.438 | 1.042 | 578.1 | 2723 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 3.379 | 3.379–3.379 | 1.834 | 578.1 | 2723 | 1 |
| essential512/slab | views-inputs | source | bounded | certified | 2.438 | 2.438–2.438 | 0.981 | 385.2 | 2723 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 3.502 | 3.502–3.502 | 1.821 | 385.2 | 2723 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 7.515 | 7.515–7.515 | 3.454 | 1705.5 | 8532 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 11.511 | 11.511–11.511 | 7.181 | 1705.5 | 8532 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 7.510 | 7.510–7.510 | 3.538 | 1189.6 | 8532 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 12.139 | 12.139–12.139 | 7.691 | 1189.6 | 8532 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 1.438 | 1.438–1.438 | 0.985 | 587.9 | 3459 | 1 |

## holes, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.091 | 0.091–0.091 | 0.014 | 163.9 | 250 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 1.928 | 1.928–1.928 | 0.025 | 592.3 | 2723 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 2.432 | 2.432–2.432 | 0.021 | 592.3 | 2723 | 1 |
| essential512/slab | views-inputs | source | bounded | certified | 1.912 | 1.912–1.912 | 0.018 | 399.4 | 2723 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 2.476 | 2.476–2.476 | 0.025 | 399.4 | 2723 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 5.354 | 5.354–5.354 | 0.020 | 1719.7 | 8532 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 7.511 | 7.511–7.511 | 0.017 | 1719.7 | 8532 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 5.257 | 5.257–5.257 | 0.019 | 1203.8 | 8532 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 7.802 | 7.802–7.802 | 0.024 | 1203.8 | 8532 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 0.675 | 0.675–0.675 | 0.037 | 603.1 | 3459 | 1 |

## holes, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 7.273 | 7.273–7.273 | 6.818 | 18947.4 | 575 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 27.366 | 27.366–27.366 | 9.806 | 7539.4 | 51470 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 41.160 | 41.160–41.160 | 22.622 | 7539.4 | 51470 | 1 |
| essential512/slab | views-inputs | source | bounded | certified | 31.133 | 31.133–31.133 | 9.700 | 5454.1 | 51470 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 42.273 | 42.273–42.273 | 23.154 | 5454.1 | 51470 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 42.004 | 42.004–42.004 | 18.703 | 14404.5 | 72787 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 67.350 | 67.350–67.350 | 33.208 | 14404.5 | 72787 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 42.143 | 42.143–42.143 | 17.421 | 9946.1 | 72787 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 65.508 | 65.508–65.508 | 32.945 | 9946.1 | 72787 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 9.452 | 9.452–9.452 | 5.139 | 12587.2 | 69602 | 1 |

## holes, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 3.118 | 3.118–3.118 | 0.062 | 18964.4 | 575 | 1 |
| essential512/enum | views-inputs | source | bounded | certified | 22.053 | 22.053–22.053 | 0.028 | 7554.5 | 51470 | 1 |
| essential512/enum | views-inputs | source | bounded | gated | 29.949 | 29.949–29.949 | 0.029 | 7554.5 | 51470 | 1 |
| essential512/slab | views-inputs | source | bounded | certified | 21.778 | 21.778–21.778 | 0.026 | 5469.2 | 51470 | 1 |
| essential512/slab | views-inputs | source | bounded | gated | 29.894 | 29.894–29.894 | 0.029 | 5469.2 | 51470 | 1 |
| essential64/enum | views-inputs | source | bounded | certified | 31.875 | 31.875–31.875 | 0.025 | 14419.6 | 72787 | 1 |
| essential64/enum | views-inputs | source | bounded | gated | 41.768 | 41.768–41.768 | 0.029 | 14419.6 | 72787 | 1 |
| essential64/slab | views-inputs | source | bounded | certified | 31.336 | 31.336–31.336 | 0.026 | 9961.2 | 72787 | 1 |
| essential64/slab | views-inputs | source | bounded | gated | 41.023 | 41.023–41.023 | 0.026 | 9961.2 | 72787 | 1 |
| packed512/enum | materialized | deferred | off | materialized | 6.857 | 6.857–6.857 | 0.705 | 12604.2 | 69602 | 1 |

240 cases; 632 matched comparisons; 73 retained processes.

- [Raw evidence](results/legal-smoke.json).
- [Raw evidence](results/legal-initial-sweep.json).
