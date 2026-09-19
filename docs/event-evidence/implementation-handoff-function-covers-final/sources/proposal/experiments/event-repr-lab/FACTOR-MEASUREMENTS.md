# Product-fibre counting: native Free Join comparison

Construction is measured separately. Fresh and warm queries include exact legal
population checksums over all 80 outputs. Every output is also checked pointwise
outside timing. Retained KB includes construction residue and owner caches; temporary
observation memo and allocator overhead are excluded. No raw alias counts are used.
Executable: `95c0134c4eafbd775d0c8de0df93967cab03660d1135e2981aeda95f9fcf7305`.

## below, 12 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.077 | 0.156 | 0.155 | 0.001 | 0.149–0.193 | 0.156 | 101.1 | 173 | 5 |
| essential512/slab | views | certified | words | joint | 0.510 | 1.607 | 1.606 | 0.000 | 1.607–1.607 | 0.636 | 173.9 | 1408 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.551 | 1.020 | 0.999 | 0.021 | 1.012–1.073 | 0.885 | 330.6 | 2427 | 5 |
| prefix512/slab | views | certified | words | faces | 0.505 | 0.505 | 0.492 | 0.013 | 0.473–0.551 | 0.338 | 109.9 | 827 | 5 |
| prefix512/slab | views | certified | words | joint | 0.506 | 0.580 | 0.512 | 0.068 | 0.532–0.590 | 0.408 | 109.9 | 827 | 5 |
| prefix64/slab | views | certified | words | faces | 1.054 | 0.935 | 0.905 | 0.030 | 0.935–0.935 | 0.583 | 260.9 | 2071 | 1 |
| retraction512/slab | views | certified | words | faces | 0.484 | 0.861 | 0.855 | 0.006 | 0.849–0.940 | 0.717 | 127.1 | 962 | 5 |
| retraction512/slab | views | certified | words | joint | 0.495 | 0.963 | 0.866 | 0.097 | 0.935–1.012 | 0.809 | 127.1 | 962 | 5 |
| retraction64/slab | views | certified | words | faces | 1.195 | 2.277 | 2.252 | 0.025 | 2.277–2.277 | 1.758 | 278.5 | 2710 | 1 |

## below, 12 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.077 | 0.071 | 0.070 | 0.001 | 0.070–0.083 | 0.013 | 114.1 | 173 | 5 |
| essential512/slab | views | certified | words | joint | 0.510 | 1.162 | 1.162 | 0.000 | 1.162–1.162 | 0.049 | 185.8 | 1408 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.551 | 0.420 | 0.399 | 0.021 | 0.407–0.449 | 0.033 | 343.6 | 2427 | 5 |
| prefix512/slab | views | certified | words | faces | 0.505 | 0.320 | 0.307 | 0.013 | 0.318–0.360 | 0.029 | 121.8 | 827 | 5 |
| prefix512/slab | views | certified | words | joint | 0.506 | 0.401 | 0.334 | 0.066 | 0.391–0.427 | 0.077 | 121.8 | 827 | 5 |
| prefix64/slab | views | certified | words | faces | 1.054 | 0.554 | 0.530 | 0.024 | 0.554–0.554 | 0.037 | 272.8 | 2071 | 1 |
| retraction512/slab | views | certified | words | faces | 0.484 | 0.487 | 0.481 | 0.006 | 0.481–0.500 | 0.018 | 139.0 | 962 | 5 |
| retraction512/slab | views | certified | words | joint | 0.495 | 0.580 | 0.484 | 0.096 | 0.575–0.623 | 0.109 | 139.0 | 962 | 5 |
| retraction64/slab | views | certified | words | faces | 1.195 | 1.317 | 1.298 | 0.019 | 1.317–1.317 | 0.031 | 290.4 | 2710 | 1 |

## below, 12 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.078 | 0.154 | 0.153 | 0.001 | 0.152–0.174 | 0.143 | 101.1 | 173 | 5 |
| essential512/slab | views | certified | words | joint | 0.364 | 2.359 | 2.359 | 0.000 | 2.359–2.359 | 0.817 | 306.8 | 2528 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.093 | 2.224 | 2.213 | 0.010 | 2.146–2.283 | 1.893 | 802.8 | 4803 | 5 |
| prefix512/slab | views | certified | words | faces | 0.534 | 0.644 | 0.631 | 0.013 | 0.621–0.681 | 0.410 | 126.1 | 1146 | 5 |
| prefix512/slab | views | certified | words | joint | 0.497 | 0.658 | 0.617 | 0.042 | 0.634–0.690 | 0.424 | 126.1 | 1146 | 5 |
| prefix64/slab | views | certified | words | faces | 0.718 | 1.443 | 1.401 | 0.042 | 1.443–1.443 | 0.671 | 332.9 | 2713 | 1 |
| retraction512/slab | views | certified | words | faces | 0.503 | 0.842 | 0.836 | 0.006 | 0.827–0.899 | 0.553 | 181.3 | 1364 | 5 |
| retraction512/slab | views | certified | words | joint | 0.509 | 0.875 | 0.845 | 0.030 | 0.853–0.917 | 0.571 | 181.3 | 1364 | 5 |
| retraction64/slab | views | certified | words | faces | 0.831 | 2.111 | 2.080 | 0.030 | 2.111–2.111 | 1.056 | 421.6 | 3591 | 1 |

## below, 12 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.078 | 0.072 | 0.071 | 0.001 | 0.069–0.083 | 0.014 | 114.1 | 173 | 5 |
| essential512/slab | views | certified | words | joint | 0.364 | 1.886 | 1.885 | 0.000 | 1.886–1.886 | 0.015 | 318.7 | 2528 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.093 | 0.903 | 0.893 | 0.010 | 0.890–0.976 | 0.023 | 815.8 | 4803 | 5 |
| prefix512/slab | views | certified | words | faces | 0.534 | 0.429 | 0.416 | 0.013 | 0.419–0.487 | 0.024 | 138.0 | 1146 | 5 |
| prefix512/slab | views | certified | words | joint | 0.497 | 0.453 | 0.412 | 0.041 | 0.446–0.483 | 0.053 | 138.0 | 1146 | 5 |
| prefix64/slab | views | certified | words | faces | 0.718 | 1.120 | 1.082 | 0.037 | 1.120–1.120 | 0.049 | 344.8 | 2713 | 1 |
| retraction512/slab | views | certified | words | faces | 0.503 | 0.550 | 0.544 | 0.006 | 0.547–0.576 | 0.017 | 193.2 | 1364 | 5 |
| retraction512/slab | views | certified | words | joint | 0.509 | 0.567 | 0.538 | 0.029 | 0.566–0.588 | 0.041 | 193.2 | 1364 | 5 |
| retraction64/slab | views | certified | words | faces | 0.831 | 1.690 | 1.668 | 0.022 | 1.690–1.690 | 0.037 | 433.5 | 3591 | 1 |

## below, 18 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 6.192 | 7.067 | 7.026 | 0.041 | 6.950–7.308 | 7.117 | 7418.7 | 224 | 5 |
| essential512/slab | views | certified | words | joint | 2.677 | 8.728 | 8.727 | 0.001 | 8.728–8.728 | 3.431 | 1175.3 | 8419 | 1 |
| packed512/enum | materialized | materialized | words | joint | 27.139 | 3.229 | 3.058 | 0.170 | 3.115–3.373 | 1.999 | 2465.2 | 17618 | 5 |
| prefix512/slab | views | certified | words | faces | 6.064 | 3.469 | 3.380 | 0.089 | 3.398–3.559 | 2.189 | 1324.4 | 8465 | 5 |
| prefix512/slab | views | certified | words | joint | 6.080 | 5.030 | 3.416 | 1.618 | 4.851–5.310 | 3.610 | 1324.4 | 8465 | 5 |
| prefix64/slab | views | certified | words | faces | 5.296 | 4.879 | 4.610 | 0.269 | 4.879–4.879 | 3.534 | 1496.3 | 13557 | 1 |
| retraction512/slab | views | certified | words | faces | 7.783 | 5.746 | 5.687 | 0.059 | 5.732–5.949 | 4.001 | 1705.2 | 13041 | 5 |
| retraction512/slab | views | certified | words | joint | 7.768 | 7.671 | 5.769 | 1.859 | 7.529–7.847 | 5.698 | 1705.2 | 13041 | 5 |
| retraction64/slab | views | certified | words | faces | 7.635 | 7.588 | 7.339 | 0.250 | 7.588–7.588 | 6.204 | 2098.7 | 20734 | 1 |

## below, 18 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 6.192 | 2.912 | 2.869 | 0.040 | 2.817–2.982 | 0.052 | 7431.7 | 224 | 5 |
| essential512/slab | views | certified | words | joint | 2.677 | 6.480 | 6.479 | 0.000 | 6.480–6.480 | 0.033 | 1188.8 | 8419 | 1 |
| packed512/enum | materialized | materialized | words | joint | 27.139 | 2.126 | 1.956 | 0.169 | 2.028–2.216 | 0.194 | 2478.1 | 17618 | 5 |
| prefix512/slab | views | certified | words | faces | 6.064 | 2.476 | 2.386 | 0.089 | 2.374–2.515 | 0.120 | 1337.9 | 8465 | 5 |
| prefix512/slab | views | certified | words | joint | 6.080 | 4.076 | 2.462 | 1.602 | 4.055–4.203 | 1.704 | 1337.9 | 8465 | 5 |
| prefix64/slab | views | certified | words | faces | 5.296 | 3.254 | 2.982 | 0.272 | 3.254–3.254 | 0.302 | 1509.8 | 13557 | 1 |
| retraction512/slab | views | certified | words | faces | 7.783 | 3.949 | 3.887 | 0.062 | 3.806–4.413 | 0.093 | 1718.7 | 13041 | 5 |
| retraction512/slab | views | certified | words | joint | 7.768 | 5.670 | 3.804 | 1.862 | 5.608–5.945 | 1.879 | 1718.7 | 13041 | 5 |
| retraction64/slab | views | certified | words | faces | 7.635 | 5.318 | 5.067 | 0.251 | 5.318–5.318 | 0.300 | 2112.2 | 20734 | 1 |

## below, 18 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 6.474 | 7.078 | 7.034 | 0.044 | 7.026–7.510 | 7.037 | 7418.7 | 224 | 5 |
| essential512/slab | views | certified | words | joint | 1.430 | 26.172 | 26.171 | 0.001 | 26.172–26.172 | 8.035 | 4275.4 | 33993 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.505 | 10.788 | 10.742 | 0.046 | 10.588–11.174 | 7.139 | 6857.2 | 60584 | 5 |
| prefix512/slab | views | certified | words | faces | 5.926 | 10.030 | 9.940 | 0.090 | 9.748–10.357 | 4.301 | 2525.7 | 19056 | 5 |
| prefix512/slab | views | certified | words | joint | 5.957 | 10.054 | 9.922 | 0.131 | 9.793–10.478 | 4.420 | 2525.7 | 19056 | 5 |
| prefix64/slab | views | certified | words | faces | 4.014 | 16.405 | 16.076 | 0.328 | 16.405–16.405 | 7.269 | 3069.4 | 29836 | 1 |
| retraction512/slab | views | certified | words | faces | 6.641 | 10.964 | 10.907 | 0.057 | 10.929–11.036 | 4.863 | 3091.9 | 22416 | 5 |
| retraction512/slab | views | certified | words | joint | 6.533 | 11.213 | 11.106 | 0.100 | 10.835–11.237 | 4.663 | 3091.9 | 22416 | 5 |
| retraction64/slab | views | certified | words | faces | 4.467 | 19.360 | 19.191 | 0.169 | 19.360–19.360 | 7.664 | 3287.2 | 34530 | 1 |

## below, 18 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 6.474 | 2.914 | 2.870 | 0.042 | 2.828–2.922 | 0.054 | 7431.7 | 224 | 5 |
| essential512/slab | views | certified | words | joint | 1.430 | 22.538 | 22.537 | 0.001 | 22.538–22.538 | 0.031 | 4288.8 | 33993 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.505 | 6.758 | 6.710 | 0.046 | 6.563–6.927 | 0.073 | 6870.2 | 60584 | 5 |
| prefix512/slab | views | certified | words | faces | 5.926 | 8.071 | 7.981 | 0.089 | 7.767–8.360 | 0.121 | 2539.2 | 19056 | 5 |
| prefix512/slab | views | certified | words | joint | 5.957 | 8.045 | 7.914 | 0.136 | 7.838–8.109 | 0.176 | 2539.2 | 19056 | 5 |
| prefix64/slab | views | certified | words | faces | 4.014 | 13.017 | 12.700 | 0.316 | 13.017–13.017 | 0.375 | 3082.9 | 29836 | 1 |
| retraction512/slab | views | certified | words | faces | 6.641 | 8.657 | 8.600 | 0.056 | 8.649–9.518 | 0.086 | 3105.4 | 22416 | 5 |
| retraction512/slab | views | certified | words | joint | 6.533 | 8.927 | 8.822 | 0.102 | 8.741–9.546 | 0.134 | 3105.4 | 22416 | 5 |
| retraction64/slab | views | certified | words | faces | 4.467 | 14.599 | 14.440 | 0.159 | 14.599–14.599 | 0.175 | 3300.7 | 34530 | 1 |

## fibred, 13 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 1.273 | 0.244 | 0.243 | 0.001 | 0.238–0.264 | 0.226 | 317.0 | 287 | 5 |
| essential512/slab | views | certified | words | joint | 2.057 | 3.495 | 3.495 | 0.000 | 3.495–3.495 | 1.415 | 484.9 | 4420 | 1 |
| packed512/enum | materialized | materialized | words | joint | 1.243 | 1.510 | 1.464 | 0.042 | 1.443–1.555 | 1.151 | 1034.7 | 6535 | 5 |
| prefix512/slab | views | certified | words | faces | 1.176 | 0.972 | 0.952 | 0.020 | 0.925–1.035 | 0.673 | 308.9 | 1850 | 5 |
| prefix512/slab | views | certified | words | joint | 1.320 | 1.207 | 0.939 | 0.266 | 1.171–1.250 | 0.867 | 308.9 | 1850 | 5 |
| prefix64/slab | views | certified | words | faces | 3.787 | 1.866 | 1.806 | 0.060 | 1.827–1.946 | 1.359 | 981.0 | 7659 | 5 |
| retraction512/slab | views | certified | words | faces | 1.253 | 1.827 | 1.814 | 0.014 | 1.811–1.901 | 1.459 | 315.2 | 2088 | 5 |
| retraction512/slab | views | certified | words | joint | 1.179 | 2.010 | 1.714 | 0.295 | 1.961–2.054 | 1.680 | 315.2 | 2088 | 5 |
| retraction64/slab | views | certified | words | faces | 4.039 | 5.146 | 5.073 | 0.073 | 5.146–5.146 | 4.351 | 1047.8 | 9466 | 1 |

## fibred, 13 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 1.273 | 0.116 | 0.114 | 0.001 | 0.113–0.125 | 0.014 | 330.0 | 287 | 5 |
| essential512/slab | views | certified | words | joint | 2.057 | 2.705 | 2.705 | 0.000 | 2.705–2.705 | 0.014 | 498.4 | 4420 | 1 |
| packed512/enum | materialized | materialized | words | joint | 1.243 | 0.811 | 0.771 | 0.041 | 0.784–0.887 | 0.052 | 1047.7 | 6535 | 5 |
| prefix512/slab | views | certified | words | faces | 1.176 | 0.615 | 0.596 | 0.020 | 0.612–0.621 | 0.032 | 322.4 | 1850 | 5 |
| prefix512/slab | views | certified | words | joint | 1.320 | 0.899 | 0.639 | 0.261 | 0.866–0.910 | 0.271 | 322.4 | 1850 | 5 |
| prefix64/slab | views | certified | words | faces | 3.787 | 1.295 | 1.233 | 0.062 | 1.246–1.312 | 0.077 | 994.5 | 7659 | 5 |
| retraction512/slab | views | certified | words | faces | 1.253 | 1.005 | 0.993 | 0.013 | 0.990–1.041 | 0.028 | 328.7 | 2088 | 5 |
| retraction512/slab | views | certified | words | joint | 1.179 | 1.285 | 0.989 | 0.295 | 1.268–1.292 | 0.309 | 328.7 | 2088 | 5 |
| retraction64/slab | views | certified | words | faces | 4.039 | 2.992 | 2.924 | 0.067 | 2.992–2.992 | 0.081 | 1061.3 | 9466 | 1 |

## fibred, 13 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 1.265 | 0.251 | 0.250 | 0.001 | 0.233–0.259 | 0.231 | 317.0 | 287 | 5 |
| essential512/slab | views | certified | words | joint | 1.511 | 6.298 | 6.297 | 0.000 | 6.298–6.298 | 1.898 | 1088.7 | 7971 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.384 | 5.126 | 5.094 | 0.032 | 5.101–5.178 | 4.067 | 2327.3 | 16261 | 5 |
| prefix512/slab | views | certified | words | faces | 1.260 | 1.156 | 1.135 | 0.021 | 1.146–1.228 | 0.763 | 309.3 | 2474 | 5 |
| prefix512/slab | views | certified | words | joint | 1.293 | 1.369 | 1.228 | 0.138 | 1.316–1.508 | 0.886 | 309.3 | 2474 | 5 |
| prefix64/slab | views | certified | words | faces | 2.358 | 2.760 | 2.665 | 0.095 | 2.714–2.816 | 1.293 | 781.5 | 7358 | 5 |
| retraction512/slab | views | certified | words | faces | 1.288 | 1.723 | 1.710 | 0.013 | 1.672–1.804 | 1.107 | 436.8 | 3031 | 5 |
| retraction512/slab | views | certified | words | joint | 1.329 | 1.818 | 1.697 | 0.123 | 1.774–1.867 | 1.181 | 436.8 | 3031 | 5 |
| retraction64/slab | views | certified | words | faces | 2.789 | 4.466 | 4.399 | 0.067 | 4.466–4.466 | 2.015 | 999.1 | 10017 | 1 |

## fibred, 13 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 1.265 | 0.113 | 0.111 | 0.001 | 0.110–0.120 | 0.014 | 330.0 | 287 | 5 |
| essential512/slab | views | certified | words | joint | 1.511 | 5.160 | 5.159 | 0.000 | 5.160–5.160 | 0.014 | 1102.2 | 7971 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.384 | 2.498 | 2.467 | 0.032 | 2.412–2.586 | 0.044 | 2340.3 | 16261 | 5 |
| prefix512/slab | views | certified | words | faces | 1.260 | 0.825 | 0.804 | 0.021 | 0.792–0.907 | 0.037 | 322.8 | 2474 | 5 |
| prefix512/slab | views | certified | words | joint | 1.293 | 0.950 | 0.813 | 0.137 | 0.942–0.992 | 0.158 | 322.8 | 2474 | 5 |
| prefix64/slab | views | certified | words | faces | 2.358 | 2.180 | 2.086 | 0.095 | 2.140–2.182 | 0.111 | 795.0 | 7358 | 5 |
| retraction512/slab | views | certified | words | faces | 1.288 | 1.201 | 1.187 | 0.013 | 1.153–1.281 | 0.031 | 450.3 | 3031 | 5 |
| retraction512/slab | views | certified | words | joint | 1.329 | 1.262 | 1.149 | 0.113 | 1.203–1.332 | 0.128 | 450.3 | 3031 | 5 |
| retraction64/slab | views | certified | words | faces | 2.789 | 3.331 | 3.270 | 0.060 | 3.331–3.331 | 0.073 | 1012.6 | 10017 | 1 |

## fibred, 19 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 462.959 | 14.561 | 14.469 | 0.086 | 14.439–15.322 | 14.382 | 45001.8 | 684 | 5 |
| essential512/slab | views | certified | words | joint | 29.138 | 29.744 | 29.743 | 0.001 | 29.744–29.744 | 10.531 | 6853.3 | 59905 | 1 |
| packed512/enum | materialized | materialized | words | joint | 59.507 | 11.457 | 10.592 | 0.855 | 10.529–12.122 | 6.870 | 13305.1 | 98153 | 5 |
| prefix512/slab | views | certified | words | faces | 25.825 | 7.171 | 6.997 | 0.169 | 7.030–7.346 | 4.686 | 6471.9 | 43265 | 5 |
| prefix512/slab | views | certified | words | joint | 26.274 | 14.029 | 7.195 | 6.685 | 13.618–14.189 | 11.367 | 6471.9 | 43265 | 5 |
| prefix64/slab | views | certified | words | faces | 43.085 | 11.161 | 10.557 | 0.646 | 10.703–12.455 | 8.213 | 9214.1 | 92041 | 5 |
| retraction512/slab | views | certified | words | faces | 35.270 | 35.348 | 35.216 | 0.132 | 34.864–37.232 | 30.343 | 9952.9 | 59894 | 5 |
| retraction512/slab | views | certified | words | joint | 35.605 | 44.286 | 35.535 | 8.779 | 44.023–46.074 | 38.920 | 9952.9 | 59894 | 5 |
| retraction64/slab | views | certified | words | faces | 52.211 | 55.685 | 54.996 | 0.688 | 55.685–55.685 | 51.445 | 15141.3 | 131287 | 1 |

## fibred, 19 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 462.959 | 6.406 | 6.321 | 0.085 | 6.360–6.509 | 0.093 | 45018.8 | 684 | 5 |
| essential512/slab | views | certified | words | joint | 29.138 | 26.026 | 26.025 | 0.001 | 26.026–26.026 | 0.031 | 6867.1 | 59905 | 1 |
| packed512/enum | materialized | materialized | words | joint | 59.507 | 8.402 | 7.510 | 0.883 | 8.341–9.176 | 0.927 | 13322.2 | 98153 | 5 |
| prefix512/slab | views | certified | words | faces | 25.825 | 5.208 | 5.039 | 0.168 | 4.933–5.385 | 0.207 | 6485.7 | 43265 | 5 |
| prefix512/slab | views | certified | words | joint | 26.274 | 11.719 | 5.006 | 6.696 | 11.573–11.846 | 6.829 | 6485.7 | 43265 | 5 |
| prefix64/slab | views | certified | words | faces | 43.085 | 7.418 | 6.811 | 0.610 | 7.050–7.790 | 0.644 | 9227.8 | 92041 | 5 |
| retraction512/slab | views | certified | words | faces | 35.270 | 20.602 | 20.473 | 0.128 | 20.107–20.717 | 0.180 | 9966.6 | 59894 | 5 |
| retraction512/slab | views | certified | words | joint | 35.605 | 29.110 | 20.230 | 8.889 | 28.960–29.963 | 8.817 | 9966.6 | 59894 | 5 |
| retraction64/slab | views | certified | words | faces | 52.211 | 31.415 | 30.742 | 0.673 | 31.415–31.415 | 0.703 | 15155.0 | 131287 | 1 |

## fibred, 19 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 451.550 | 14.596 | 14.515 | 0.084 | 14.336–14.980 | 14.472 | 45001.8 | 684 | 5 |
| essential512/slab | views | certified | words | joint | 7.178 | 68.599 | 68.599 | 0.001 | 68.599–68.599 | 17.046 | 12039.0 | 109616 | 1 |
| packed512/enum | materialized | materialized | words | joint | 2.423 | 39.459 | 39.348 | 0.113 | 38.564–39.990 | 24.770 | 25957.2 | 218715 | 5 |
| prefix512/slab | views | certified | words | faces | 25.346 | 20.672 | 20.468 | 0.204 | 19.574–21.227 | 8.638 | 7077.6 | 57705 | 5 |
| prefix512/slab | views | certified | words | joint | 22.993 | 19.495 | 19.127 | 0.367 | 19.422–21.173 | 8.709 | 7077.6 | 57705 | 5 |
| prefix64/slab | views | certified | words | faces | 15.836 | 30.478 | 29.816 | 0.678 | 29.755–68.789 | 13.340 | 7690.0 | 83049 | 5 |
| retraction512/slab | views | certified | words | faces | 28.711 | 26.575 | 26.447 | 0.131 | 26.305–29.668 | 11.488 | 9117.2 | 80348 | 5 |
| retraction512/slab | views | certified | words | joint | 28.151 | 27.769 | 27.482 | 0.280 | 27.048–28.108 | 11.493 | 9117.2 | 80348 | 5 |
| retraction64/slab | views | certified | words | faces | 19.291 | 44.679 | 44.240 | 0.438 | 44.679–44.679 | 16.247 | 12354.5 | 118188 | 1 |

## fibred, 19 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 451.550 | 6.446 | 6.361 | 0.086 | 6.322–6.500 | 0.093 | 45018.8 | 684 | 5 |
| essential512/slab | views | certified | words | joint | 7.178 | 68.226 | 68.225 | 0.001 | 68.226–68.226 | 0.038 | 12052.8 | 109616 | 1 |
| packed512/enum | materialized | materialized | words | joint | 2.423 | 25.746 | 25.615 | 0.115 | 25.013–26.566 | 0.168 | 25974.3 | 218715 | 5 |
| prefix512/slab | views | certified | words | faces | 25.346 | 15.557 | 15.360 | 0.198 | 15.308–16.198 | 0.236 | 7091.4 | 57705 | 5 |
| prefix512/slab | views | certified | words | joint | 22.993 | 16.774 | 16.402 | 0.371 | 15.526–17.119 | 0.425 | 7091.4 | 57705 | 5 |
| prefix64/slab | views | certified | words | faces | 15.836 | 24.941 | 24.278 | 0.683 | 24.841–33.251 | 0.742 | 7703.7 | 83049 | 5 |
| retraction512/slab | views | certified | words | faces | 28.711 | 22.155 | 22.030 | 0.126 | 22.105–22.742 | 0.178 | 9130.9 | 80348 | 5 |
| retraction512/slab | views | certified | words | joint | 28.151 | 23.055 | 22.779 | 0.278 | 21.938–25.543 | 0.335 | 9130.9 | 80348 | 5 |
| retraction64/slab | views | certified | words | faces | 19.291 | 36.489 | 36.071 | 0.418 | 36.489–36.489 | 0.467 | 12368.2 | 118188 | 1 |

## holes, 12 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.598 | 0.165 | 0.164 | 0.001 | 0.159–0.180 | 0.147 | 148.7 | 250 | 5 |
| essential512/slab | views | certified | words | joint | 1.258 | 2.095 | 2.094 | 0.000 | 2.095–2.095 | 0.817 | 385.4 | 2524 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.674 | 1.165 | 1.142 | 0.022 | 1.135–1.288 | 0.970 | 587.9 | 3459 | 5 |
| prefix512/slab | views | certified | words | faces | 0.570 | 0.557 | 0.546 | 0.010 | 0.532–0.622 | 0.371 | 139.5 | 1017 | 5 |
| prefix512/slab | views | certified | words | joint | 0.578 | 0.683 | 0.549 | 0.134 | 0.659–0.729 | 0.497 | 139.5 | 1017 | 5 |
| prefix64/slab | views | certified | words | faces | 1.330 | 0.995 | 0.964 | 0.031 | 0.995–0.995 | 0.688 | 298.1 | 2697 | 1 |
| retraction512/slab | views | certified | words | faces | 0.545 | 0.988 | 0.982 | 0.006 | 0.965–1.052 | 0.816 | 138.5 | 1135 | 5 |
| retraction512/slab | views | certified | words | joint | 0.548 | 1.130 | 0.970 | 0.158 | 1.106–1.210 | 0.966 | 138.5 | 1135 | 5 |
| retraction64/slab | views | certified | words | faces | 1.400 | 3.491 | 3.449 | 0.042 | 3.491–3.491 | 3.090 | 312.6 | 3507 | 1 |

## holes, 12 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.598 | 0.085 | 0.084 | 0.001 | 0.082–0.093 | 0.015 | 163.9 | 250 | 5 |
| essential512/slab | views | certified | words | joint | 1.258 | 1.705 | 1.705 | 0.000 | 1.705–1.705 | 0.017 | 398.9 | 2524 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.674 | 0.571 | 0.549 | 0.023 | 0.568–0.604 | 0.036 | 603.1 | 3459 | 5 |
| prefix512/slab | views | certified | words | faces | 0.570 | 0.364 | 0.354 | 0.010 | 0.356–0.387 | 0.022 | 153.0 | 1017 | 5 |
| prefix512/slab | views | certified | words | joint | 0.578 | 0.494 | 0.359 | 0.131 | 0.481–0.520 | 0.142 | 153.0 | 1017 | 5 |
| prefix64/slab | views | certified | words | faces | 1.330 | 0.731 | 0.704 | 0.027 | 0.731–0.731 | 0.043 | 311.6 | 2697 | 1 |
| retraction512/slab | views | certified | words | faces | 0.545 | 0.606 | 0.599 | 0.006 | 0.601–0.652 | 0.018 | 152.0 | 1135 | 5 |
| retraction512/slab | views | certified | words | joint | 0.548 | 0.729 | 0.566 | 0.160 | 0.716–0.773 | 0.170 | 152.0 | 1135 | 5 |
| retraction64/slab | views | certified | words | faces | 1.400 | 1.969 | 1.931 | 0.037 | 1.969–1.969 | 0.052 | 326.1 | 3507 | 1 |

## holes, 12 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.574 | 0.159 | 0.158 | 0.001 | 0.157–0.180 | 0.150 | 148.7 | 250 | 5 |
| essential512/slab | views | certified | words | joint | 0.997 | 4.271 | 4.271 | 0.000 | 4.271–4.271 | 1.105 | 894.9 | 5252 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.301 | 3.722 | 3.704 | 0.018 | 3.662–3.777 | 2.985 | 1767.5 | 11385 | 5 |
| prefix512/slab | views | certified | words | faces | 0.591 | 0.693 | 0.682 | 0.011 | 0.677–0.751 | 0.447 | 146.9 | 1232 | 5 |
| prefix512/slab | views | certified | words | joint | 0.581 | 0.724 | 0.649 | 0.075 | 0.711–0.798 | 0.476 | 146.9 | 1232 | 5 |
| prefix64/slab | views | certified | words | faces | 0.939 | 1.540 | 1.486 | 0.054 | 1.540–1.540 | 0.630 | 284.5 | 2800 | 1 |
| retraction512/slab | views | certified | words | faces | 0.530 | 1.020 | 1.013 | 0.007 | 0.980–1.037 | 0.646 | 219.0 | 1636 | 5 |
| retraction512/slab | views | certified | words | joint | 0.544 | 1.047 | 0.986 | 0.060 | 1.018–1.091 | 0.688 | 219.0 | 1636 | 5 |
| retraction64/slab | views | certified | words | faces | 1.086 | 2.487 | 2.452 | 0.035 | 2.487–2.487 | 1.061 | 536.8 | 4335 | 1 |

## holes, 12 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 0.574 | 0.081 | 0.080 | 0.001 | 0.079–0.090 | 0.013 | 163.9 | 250 | 5 |
| essential512/slab | views | certified | words | joint | 0.997 | 3.687 | 3.687 | 0.000 | 3.687–3.687 | 0.019 | 908.4 | 5252 | 1 |
| packed512/enum | materialized | materialized | words | joint | 0.301 | 1.738 | 1.720 | 0.018 | 1.716–1.773 | 0.033 | 1782.8 | 11385 | 5 |
| prefix512/slab | views | certified | words | faces | 0.591 | 0.504 | 0.494 | 0.010 | 0.492–0.510 | 0.023 | 160.4 | 1232 | 5 |
| prefix512/slab | views | certified | words | joint | 0.581 | 0.545 | 0.471 | 0.075 | 0.521–0.583 | 0.087 | 160.4 | 1232 | 5 |
| prefix64/slab | views | certified | words | faces | 0.939 | 1.203 | 1.155 | 0.048 | 1.203–1.203 | 0.060 | 298.0 | 2800 | 1 |
| retraction512/slab | views | certified | words | faces | 0.530 | 0.652 | 0.646 | 0.006 | 0.644–0.720 | 0.018 | 232.5 | 1636 | 5 |
| retraction512/slab | views | certified | words | joint | 0.544 | 0.703 | 0.643 | 0.059 | 0.699–0.732 | 0.070 | 232.5 | 1636 | 5 |
| retraction64/slab | views | certified | words | faces | 1.086 | 2.035 | 2.006 | 0.029 | 2.035–2.035 | 0.041 | 550.3 | 4335 | 1 |

## holes, 18 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 225.332 | 7.110 | 7.068 | 0.043 | 7.079–7.510 | 7.013 | 18947.4 | 575 | 5 |
| essential512/slab | views | certified | words | joint | 21.920 | 20.200 | 20.199 | 0.000 | 20.200–20.200 | 7.147 | 5057.9 | 44045 | 1 |
| packed512/enum | materialized | materialized | words | joint | 32.149 | 8.926 | 8.291 | 0.648 | 8.287–9.442 | 5.075 | 12587.2 | 69602 | 5 |
| prefix512/slab | views | certified | words | faces | 11.783 | 3.786 | 3.690 | 0.095 | 3.695–3.887 | 2.417 | 2991.5 | 18837 | 5 |
| prefix512/slab | views | certified | words | joint | 11.764 | 8.013 | 3.669 | 4.378 | 7.910–8.281 | 6.716 | 2991.5 | 18837 | 5 |
| prefix64/slab | views | certified | words | faces | 17.054 | 5.371 | 5.025 | 0.345 | 5.371–5.371 | 4.185 | 4530.2 | 44346 | 1 |
| retraction512/slab | views | certified | words | faces | 14.717 | 26.902 | 26.839 | 0.063 | 26.857–27.029 | 25.080 | 3649.2 | 26179 | 5 |
| retraction512/slab | views | certified | words | joint | 14.747 | 33.398 | 27.173 | 6.294 | 33.287–35.018 | 31.297 | 3649.2 | 26179 | 5 |
| retraction64/slab | views | certified | words | faces | 28.501 | 50.251 | 49.860 | 0.391 | 50.251–50.251 | 48.472 | 7610.2 | 65343 | 1 |

## holes, 18 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 225.332 | 3.095 | 3.053 | 0.042 | 3.066–3.190 | 0.054 | 18964.4 | 575 | 5 |
| essential512/slab | views | certified | words | joint | 21.920 | 17.225 | 17.224 | 0.001 | 17.225–17.225 | 0.027 | 5071.4 | 44045 | 1 |
| packed512/enum | materialized | materialized | words | joint | 32.149 | 7.039 | 6.383 | 0.670 | 6.771–7.113 | 0.657 | 12604.2 | 69602 | 5 |
| prefix512/slab | views | certified | words | faces | 11.783 | 2.642 | 2.549 | 0.091 | 2.626–2.750 | 0.136 | 3005.0 | 18837 | 5 |
| prefix512/slab | views | certified | words | joint | 11.764 | 7.058 | 2.561 | 4.349 | 6.865–7.107 | 4.404 | 3005.0 | 18837 | 5 |
| prefix64/slab | views | certified | words | faces | 17.054 | 3.728 | 3.410 | 0.317 | 3.728–3.728 | 0.378 | 4543.7 | 44346 | 1 |
| retraction512/slab | views | certified | words | faces | 14.717 | 14.575 | 14.513 | 0.062 | 14.367–14.653 | 0.100 | 3662.7 | 26179 | 5 |
| retraction512/slab | views | certified | words | joint | 14.747 | 20.724 | 14.567 | 6.166 | 20.616–21.777 | 6.447 | 3662.7 | 26179 | 5 |
| retraction64/slab | views | certified | words | faces | 28.501 | 26.912 | 26.522 | 0.389 | 26.912–26.912 | 0.479 | 7623.7 | 65343 | 1 |

## holes, 18 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 224.928 | 7.035 | 6.992 | 0.043 | 7.010–7.423 | 6.917 | 18947.4 | 575 | 5 |
| essential512/slab | views | certified | words | joint | 4.851 | 44.655 | 44.654 | 0.001 | 44.655–44.655 | 8.625 | 7905.0 | 73266 | 1 |
| packed512/enum | materialized | materialized | words | joint | 1.611 | 24.710 | 24.638 | 0.073 | 24.271–26.473 | 15.095 | 17678.7 | 151260 | 5 |
| prefix512/slab | views | certified | words | faces | 9.036 | 9.058 | 8.936 | 0.115 | 8.949–9.243 | 3.648 | 2551.3 | 21442 | 5 |
| prefix512/slab | views | certified | words | joint | 8.732 | 9.230 | 8.971 | 0.229 | 8.932–9.342 | 4.097 | 2551.3 | 21442 | 5 |
| prefix64/slab | views | certified | words | faces | 7.319 | 13.296 | 12.933 | 0.363 | 13.296–13.296 | 11.563 | 3576.2 | 34179 | 1 |
| retraction512/slab | views | certified | words | faces | 11.435 | 15.684 | 15.619 | 0.070 | 15.377–16.072 | 6.248 | 4811.6 | 36885 | 5 |
| retraction512/slab | views | certified | words | joint | 11.326 | 15.729 | 15.536 | 0.175 | 15.291–16.063 | 6.083 | 4811.6 | 36885 | 5 |
| retraction64/slab | views | certified | words | faces | 7.538 | 24.593 | 24.344 | 0.248 | 24.593–24.593 | 8.772 | 6130.6 | 59122 | 1 |

## holes, 18 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Factor | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | joint | 224.928 | 3.142 | 3.100 | 0.042 | 3.080–3.155 | 0.054 | 18964.4 | 575 | 5 |
| essential512/slab | views | certified | words | joint | 4.851 | 43.511 | 43.510 | 0.001 | 43.511–43.511 | 0.030 | 7918.5 | 73266 | 1 |
| packed512/enum | materialized | materialized | words | joint | 1.611 | 16.034 | 15.957 | 0.075 | 15.487–16.466 | 0.111 | 17695.7 | 151260 | 5 |
| prefix512/slab | views | certified | words | faces | 9.036 | 7.405 | 7.275 | 0.113 | 7.177–7.636 | 0.148 | 2564.8 | 21442 | 5 |
| prefix512/slab | views | certified | words | joint | 8.732 | 7.538 | 7.301 | 0.235 | 7.311–7.590 | 0.274 | 2564.8 | 21442 | 5 |
| prefix64/slab | views | certified | words | faces | 7.319 | 11.378 | 11.015 | 0.363 | 11.378–11.378 | 0.477 | 3589.7 | 34179 | 1 |
| retraction512/slab | views | certified | words | faces | 11.435 | 12.783 | 12.717 | 0.065 | 12.641–13.140 | 0.109 | 4825.1 | 36885 | 5 |
| retraction512/slab | views | certified | words | joint | 11.326 | 13.144 | 12.964 | 0.177 | 12.940–13.972 | 0.211 | 4825.1 | 36885 | 5 |
| retraction64/slab | views | certified | words | faces | 7.538 | 21.046 | 20.793 | 0.253 | 21.046–21.046 | 0.300 | 6144.1 | 59122 | 1 |

216 configurations; 552 matched completion/control comparisons; 102 retained processes.

- [Raw evidence](results/factor-smoke.json).
- [Raw evidence](results/factor-repeat.json).
- [Raw evidence](results/factor-cutoff-smoke.json).
- [Raw evidence](results/factor-cutoff-repeat.json).
