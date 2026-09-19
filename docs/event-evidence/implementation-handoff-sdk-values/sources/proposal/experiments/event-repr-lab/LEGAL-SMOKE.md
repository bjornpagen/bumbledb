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
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.182 | 0.176–0.187 | 0.141 | 101.1 | 173 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 2.085 | 2.083–2.086 | 0.640 | 330.9 | 2130 | 2 |
| essential512/slab | views | source | bounded | certified | 1.549 | 1.540–1.558 | 0.691 | 173.9 | 1408 | 2 |
| essential512/slab | views | source | bounded | gated | 2.022 | 1.994–2.050 | 0.981 | 204.2 | 1408 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 1.742 | 1.721–1.763 | 0.776 | 204.2 | 1521 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 2.256 | 2.156–2.356 | 1.051 | 204.2 | 1521 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 1.135 | 1.126–1.145 | 0.933 | 330.6 | 2427 | 2 |

## below, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.084 | 0.083–0.085 | 0.016 | 114.1 | 173 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 1.725 | 1.711–1.740 | 0.025 | 343.9 | 2130 | 2 |
| essential512/slab | views | source | bounded | certified | 1.211 | 1.176–1.245 | 0.020 | 185.8 | 1408 | 2 |
| essential512/slab | views | source | bounded | gated | 1.439 | 1.428–1.449 | 0.013 | 216.1 | 1408 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 1.372 | 1.348–1.396 | 0.016 | 216.8 | 1521 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 1.636 | 1.552–1.720 | 0.013 | 216.8 | 1521 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 0.454 | 0.442–0.465 | 0.038 | 343.6 | 2427 | 2 |

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
| dense-dispatched/enum | materialized | deferred | off | materialized | 7.718 | 7.717–7.719 | 6.925 | 7418.7 | 224 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 14.064 | 13.592–14.537 | 3.799 | 2226.7 | 16146 | 2 |
| essential512/slab | views | source | bounded | certified | 8.923 | 8.585–9.262 | 3.432 | 1175.3 | 8419 | 2 |
| essential512/slab | views | source | bounded | gated | 10.198 | 10.190–10.207 | 4.774 | 1175.0 | 8419 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 10.271 | 9.627–10.915 | 3.885 | 1175.0 | 10032 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 11.640 | 11.603–11.678 | 5.077 | 1418.8 | 10032 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 3.256 | 3.256–3.257 | 2.204 | 2465.2 | 17618 | 2 |

## below, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 2.857 | 2.833–2.881 | 0.057 | 7431.7 | 224 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 11.255 | 11.025–11.484 | 0.032 | 2239.7 | 16146 | 2 |
| essential512/slab | views | source | bounded | certified | 7.251 | 6.748–7.754 | 0.028 | 1188.8 | 8419 | 2 |
| essential512/slab | views | source | bounded | gated | 8.150 | 7.645–8.656 | 0.025 | 1188.5 | 8419 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 7.916 | 7.899–7.933 | 0.028 | 1189.2 | 10032 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 9.223 | 8.811–9.635 | 0.025 | 1432.9 | 10032 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 2.247 | 2.178–2.316 | 0.209 | 2478.1 | 17618 | 2 |

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
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.293 | 0.250–0.336 | 0.244 | 317.0 | 287 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 4.788 | 4.768–4.808 | 1.615 | 815.7 | 6212 | 2 |
| essential512/slab | views | source | bounded | certified | 3.759 | 3.651–3.867 | 1.521 | 484.9 | 4420 | 2 |
| essential512/slab | views | source | bounded | gated | 5.044 | 4.990–5.097 | 2.734 | 484.7 | 4420 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 4.354 | 4.172–4.535 | 1.677 | 611.8 | 4766 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 10.283 | 8.875–11.690 | 3.611 | 733.7 | 4766 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 1.699 | 1.609–1.788 | 1.210 | 1034.7 | 6535 | 2 |

## fibred, 13 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 0.139 | 0.138–0.140 | 0.016 | 330.0 | 287 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 4.540 | 4.525–4.556 | 0.027 | 828.7 | 6212 | 2 |
| essential512/slab | views | source | bounded | certified | 2.921 | 2.843–2.999 | 0.022 | 498.4 | 4420 | 2 |
| essential512/slab | views | source | bounded | gated | 3.580 | 3.563–3.597 | 0.018 | 498.2 | 4420 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 3.252 | 3.155–3.348 | 0.020 | 626.0 | 4766 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 7.290 | 7.093–7.487 | 0.024 | 747.9 | 4766 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 0.885 | 0.855–0.915 | 0.062 | 1047.7 | 6535 | 2 |

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
| dense-dispatched/enum | materialized | deferred | off | materialized | 15.614 | 15.473–15.754 | 14.623 | 45001.8 | 684 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 60.226 | 58.890–61.563 | 14.970 | 11510.3 | 96826 | 2 |
| essential512/slab | views | source | bounded | certified | 32.393 | 31.561–33.224 | 11.463 | 6853.3 | 59905 | 2 |
| essential512/slab | views | source | bounded | gated | 50.231 | 47.216–53.247 | 24.940 | 8802.8 | 59905 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 40.168 | 40.001–40.336 | 14.352 | 9389.3 | 71409 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 70.505 | 58.884–82.125 | 29.882 | 9389.3 | 71409 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 12.166 | 12.080–12.251 | 7.231 | 13305.1 | 98153 | 2 |

## fibred, 19 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Gates | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | materialized | 6.743 | 6.704–6.781 | 0.111 | 45018.8 | 684 | 2 |
| essential512/slab | materialized | deferred | off | materialized | 49.833 | 48.054–51.611 | 0.031 | 11527.3 | 96826 | 2 |
| essential512/slab | views | source | bounded | certified | 28.623 | 25.732–31.513 | 0.029 | 6867.1 | 59905 | 2 |
| essential512/slab | views | source | bounded | gated | 35.574 | 35.231–35.917 | 0.026 | 8816.5 | 59905 | 2 |
| essential512/slab | views-inputs | source | bounded | certified | 35.388 | 33.912–36.865 | 0.029 | 9404.4 | 71409 | 2 |
| essential512/slab | views-inputs | source | bounded | gated | 129.945 | 45.461–214.429 | 0.025 | 9404.4 | 71409 | 2 |
| packed512/enum | materialized | deferred | off | materialized | 9.042 | 8.694–9.391 | 0.909 | 13322.2 | 98153 | 2 |

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

112 cases; 288 matched comparisons; 31 retained processes.

- [Raw evidence](results/legal-smoke.json).
