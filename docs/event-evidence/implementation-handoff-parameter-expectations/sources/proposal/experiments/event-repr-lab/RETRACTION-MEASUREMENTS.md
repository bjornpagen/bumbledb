# Fixed-decoder completion: native Free Join comparison

Construction is measured separately. Fresh and warm queries include exact legal
population checksums over all 80 outputs. Every output is also checked pointwise
outside timing. Retained KB includes construction residue and owner caches; temporary
observation memo and allocator overhead are excluded. No raw alias counts are used.
Executable: `c27b4aca8568e345e5499cc9a594a7703d397411b9a585243f83be185fc1b8d2`.

## below, 12 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.077 | 0.153 | 0.153 | 0.001 | 0.151–0.172 | 0.140 | 101.1 | 173 | 5 |
| essential512/slab | materialized | materialized | words | 0.479 | 2.033 | 2.032 | 0.000 | 2.033–2.033 | 0.642 | 330.9 | 2130 | 1 |
| essential512/slab | views | certified | words | 0.377 | 1.473 | 1.472 | 0.000 | 1.441–1.612 | 0.632 | 173.9 | 1408 | 5 |
| essential512/slab | views-inputs | certified | words | 0.515 | 1.793 | 1.793 | 0.000 | 1.793–1.793 | 0.813 | 204.2 | 1521 | 1 |
| packed512/enum | materialized | materialized | words | 0.583 | 1.119 | 1.096 | 0.023 | 1.085–1.171 | 0.953 | 330.6 | 2427 | 5 |
| retraction512/slab | materialized | materialized | scalar | 0.566 | 3.014 | 0.926 | 2.087 | 3.014–3.014 | 2.743 | 152.4 | 1229 | 1 |
| retraction512/slab | materialized | materialized | words | 0.556 | 1.020 | 0.918 | 0.102 | 1.020–1.020 | 0.654 | 152.4 | 1229 | 1 |
| retraction512/slab | views | certified | scalar | 0.489 | 3.097 | 0.915 | 2.148 | 2.999–3.147 | 2.842 | 126.7 | 959 | 5 |
| retraction512/slab | views | certified | words | 0.489 | 1.011 | 0.910 | 0.100 | 0.972–1.039 | 0.816 | 126.7 | 959 | 5 |
| retraction512/slab | views-inputs | certified | scalar | 0.572 | 3.027 | 0.899 | 2.128 | 3.027–3.027 | 3.212 | 126.7 | 960 | 1 |
| retraction512/slab | views-inputs | certified | words | 0.582 | 0.987 | 0.890 | 0.098 | 0.987–0.987 | 0.793 | 126.7 | 960 | 1 |

## below, 12 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.077 | 0.071 | 0.071 | 0.001 | 0.070–0.078 | 0.013 | 114.1 | 173 | 5 |
| essential512/slab | materialized | materialized | words | 0.479 | 1.657 | 1.657 | 0.000 | 1.657–1.657 | 0.014 | 343.9 | 2130 | 1 |
| essential512/slab | views | certified | words | 0.377 | 1.115 | 1.114 | 0.000 | 1.089–1.141 | 0.012 | 185.8 | 1408 | 5 |
| essential512/slab | views-inputs | certified | words | 0.515 | 1.369 | 1.368 | 0.000 | 1.369–1.369 | 0.014 | 216.8 | 1521 | 1 |
| packed512/enum | materialized | materialized | words | 0.583 | 0.429 | 0.408 | 0.021 | 0.414–0.508 | 0.033 | 343.6 | 2427 | 5 |
| retraction512/slab | materialized | materialized | scalar | 0.566 | 2.716 | 0.590 | 2.126 | 2.716–2.716 | 2.147 | 165.4 | 1229 | 1 |
| retraction512/slab | materialized | materialized | words | 0.556 | 0.678 | 0.583 | 0.095 | 0.678–0.678 | 0.106 | 165.4 | 1229 | 1 |
| retraction512/slab | views | certified | scalar | 0.489 | 2.596 | 0.478 | 2.115 | 2.591–2.646 | 2.127 | 138.6 | 959 | 5 |
| retraction512/slab | views | certified | words | 0.489 | 0.586 | 0.492 | 0.095 | 0.574–0.655 | 0.108 | 138.6 | 959 | 5 |
| retraction512/slab | views-inputs | certified | scalar | 0.572 | 2.665 | 0.526 | 2.139 | 2.665–2.665 | 2.097 | 139.3 | 960 | 1 |
| retraction512/slab | views-inputs | certified | words | 0.582 | 0.562 | 0.468 | 0.094 | 0.562–0.562 | 0.106 | 139.3 | 960 | 1 |

## below, 12 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.076 | 0.161 | 0.160 | 0.001 | 0.154–0.180 | 0.142 | 101.1 | 173 | 5 |
| essential512/slab | views | certified | words | 0.262 | 2.198 | 2.198 | 0.000 | 2.167–2.275 | 0.748 | 306.8 | 2528 | 5 |
| essential64/slab | views | certified | words | 0.441 | 3.562 | 3.562 | 0.000 | 3.562–3.562 | 1.124 | 624.5 | 5633 | 1 |
| packed512/enum | materialized | materialized | words | 0.089 | 2.171 | 2.160 | 0.010 | 2.116–2.269 | 1.852 | 802.8 | 4803 | 5 |
| retraction512/slab | views | certified | words | 0.515 | 0.950 | 0.914 | 0.031 | 0.912–1.001 | 0.593 | 180.5 | 1359 | 5 |
| retraction64/slab | views | certified | words | 0.846 | 2.133 | 2.085 | 0.048 | 2.133–2.133 | 0.992 | 359.7 | 3584 | 1 |

## below, 12 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.076 | 0.074 | 0.073 | 0.001 | 0.070–0.083 | 0.013 | 114.1 | 173 | 5 |
| essential512/slab | views | certified | words | 0.262 | 1.803 | 1.802 | 0.000 | 1.777–1.850 | 0.013 | 318.7 | 2528 | 5 |
| essential64/slab | views | certified | words | 0.441 | 2.907 | 2.907 | 0.000 | 2.907–2.907 | 0.015 | 636.4 | 5633 | 1 |
| packed512/enum | materialized | materialized | words | 0.089 | 0.866 | 0.856 | 0.010 | 0.853–0.927 | 0.022 | 815.8 | 4803 | 5 |
| retraction512/slab | views | certified | words | 0.515 | 0.589 | 0.559 | 0.029 | 0.582–0.647 | 0.042 | 192.4 | 1359 | 5 |
| retraction64/slab | views | certified | words | 0.846 | 1.577 | 1.533 | 0.043 | 1.577–1.577 | 0.063 | 371.6 | 3584 | 1 |

## below, 18 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 6.372 | 7.066 | 7.023 | 0.043 | 6.993–7.529 | 6.838 | 7418.7 | 224 | 5 |
| essential512/slab | materialized | materialized | words | 2.605 | 12.849 | 12.848 | 0.001 | 12.849–12.849 | 3.792 | 2226.7 | 16146 | 1 |
| essential512/slab | views | certified | words | 2.634 | 8.349 | 8.348 | 0.000 | 8.247–8.512 | 3.712 | 1175.3 | 8419 | 5 |
| essential512/slab | views-inputs | certified | words | 2.617 | 9.577 | 9.577 | 0.000 | 9.577–9.577 | 3.883 | 1175.0 | 10032 | 1 |
| packed512/enum | materialized | materialized | words | 27.450 | 3.219 | 3.042 | 0.172 | 3.115–3.297 | 2.086 | 2465.2 | 17618 | 5 |
| retraction512/slab | materialized | materialized | scalar | 7.682 | 39.259 | 5.861 | 33.397 | 39.259–39.259 | 37.831 | 1948.6 | 14617 | 1 |
| retraction512/slab | materialized | materialized | words | 7.677 | 7.628 | 5.830 | 1.799 | 7.628–7.628 | 5.390 | 1948.6 | 14617 | 1 |
| retraction512/slab | views | certified | scalar | 7.685 | 39.416 | 5.692 | 33.724 | 39.395–39.780 | 37.860 | 1704.9 | 13037 | 5 |
| retraction512/slab | views | certified | words | 7.936 | 7.890 | 6.021 | 1.884 | 7.726–8.317 | 5.676 | 1704.9 | 13037 | 5 |
| retraction512/slab | views-inputs | certified | scalar | 7.739 | 38.688 | 5.536 | 33.152 | 38.688–38.688 | 38.936 | 1704.9 | 13101 | 1 |
| retraction512/slab | views-inputs | certified | words | 7.784 | 7.583 | 5.699 | 1.883 | 7.583–7.583 | 6.215 | 1704.9 | 13101 | 1 |

## below, 18 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 6.372 | 2.804 | 2.763 | 0.041 | 2.789–2.821 | 0.053 | 7431.7 | 224 | 5 |
| essential512/slab | materialized | materialized | words | 2.605 | 10.855 | 10.855 | 0.000 | 10.855–10.855 | 0.033 | 2239.7 | 16146 | 1 |
| essential512/slab | views | certified | words | 2.634 | 6.616 | 6.615 | 0.000 | 6.508–6.712 | 0.037 | 1188.8 | 8419 | 5 |
| essential512/slab | views-inputs | certified | words | 2.617 | 7.735 | 7.734 | 0.000 | 7.735–7.735 | 0.061 | 1189.2 | 10032 | 1 |
| packed512/enum | materialized | materialized | words | 27.450 | 2.280 | 2.106 | 0.171 | 2.136–2.343 | 0.203 | 2478.1 | 17618 | 5 |
| retraction512/slab | materialized | materialized | scalar | 7.682 | 37.826 | 4.345 | 33.480 | 37.826–37.826 | 33.895 | 1961.6 | 14617 | 1 |
| retraction512/slab | materialized | materialized | words | 7.677 | 6.147 | 4.306 | 1.841 | 6.147–6.147 | 1.861 | 1961.6 | 14617 | 1 |
| retraction512/slab | views | certified | scalar | 7.685 | 37.602 | 3.777 | 33.899 | 37.529–38.637 | 33.860 | 1718.4 | 13037 | 5 |
| retraction512/slab | views | certified | words | 7.936 | 5.654 | 3.768 | 1.823 | 5.444–5.735 | 1.870 | 1718.4 | 13037 | 5 |
| retraction512/slab | views-inputs | certified | scalar | 7.739 | 37.518 | 3.572 | 33.945 | 37.518–37.518 | 33.798 | 1719.1 | 13101 | 1 |
| retraction512/slab | views-inputs | certified | words | 7.784 | 5.396 | 3.565 | 1.831 | 5.396–5.396 | 1.847 | 1719.1 | 13101 | 1 |

## below, 18 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 6.390 | 7.079 | 7.037 | 0.042 | 7.020–7.386 | 6.805 | 7418.7 | 224 | 5 |
| essential512/slab | views | certified | words | 1.264 | 25.865 | 25.864 | 0.001 | 25.594–26.482 | 7.874 | 4275.4 | 33993 | 5 |
| essential64/slab | views | certified | words | 1.758 | 27.259 | 27.258 | 0.000 | 27.259–27.259 | 9.068 | 5196.3 | 48880 | 1 |
| packed512/enum | materialized | materialized | words | 0.485 | 10.950 | 10.905 | 0.045 | 10.579–11.116 | 7.311 | 6857.2 | 60584 | 5 |
| retraction512/slab | views | certified | words | 6.843 | 12.075 | 11.971 | 0.103 | 11.496–12.233 | 5.618 | 3090.0 | 22406 | 5 |
| retraction64/slab | views | certified | words | 4.312 | 17.358 | 17.172 | 0.185 | 17.358–17.358 | 7.528 | 3285.8 | 34518 | 1 |

## below, 18 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 6.390 | 2.815 | 2.775 | 0.040 | 2.757–2.887 | 0.054 | 7431.7 | 224 | 5 |
| essential512/slab | views | certified | words | 1.264 | 21.941 | 21.941 | 0.001 | 21.809–21.976 | 0.029 | 4288.8 | 33993 | 5 |
| essential64/slab | views | certified | words | 1.758 | 23.045 | 23.044 | 0.000 | 23.045–23.045 | 0.045 | 5209.8 | 48880 | 1 |
| packed512/enum | materialized | materialized | words | 0.485 | 6.917 | 6.872 | 0.046 | 6.767–6.934 | 0.073 | 6870.2 | 60584 | 5 |
| retraction512/slab | views | certified | words | 6.843 | 9.508 | 9.409 | 0.103 | 9.259–12.170 | 0.126 | 3103.5 | 22406 | 5 |
| retraction64/slab | views | certified | words | 4.312 | 14.810 | 14.612 | 0.197 | 14.810–14.810 | 0.241 | 3299.3 | 34518 | 1 |

## fibred, 13 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 1.269 | 0.257 | 0.255 | 0.002 | 0.254–0.295 | 0.231 | 317.0 | 287 | 5 |
| essential512/slab | materialized | materialized | words | 2.124 | 4.906 | 4.906 | 0.000 | 4.906–4.906 | 1.698 | 815.7 | 6212 | 1 |
| essential512/slab | views | certified | words | 1.941 | 3.542 | 3.542 | 0.000 | 3.506–3.644 | 1.463 | 484.9 | 4420 | 5 |
| essential512/slab | views-inputs | certified | words | 2.126 | 4.423 | 4.419 | 0.000 | 4.423–4.423 | 1.744 | 611.8 | 4766 | 1 |
| packed512/enum | materialized | materialized | words | 1.327 | 1.628 | 1.585 | 0.047 | 1.533–1.697 | 1.135 | 1034.7 | 6535 | 5 |
| retraction512/slab | materialized | materialized | scalar | 1.336 | 9.051 | 1.862 | 7.189 | 9.051–9.051 | 8.337 | 312.9 | 2634 | 1 |
| retraction512/slab | materialized | materialized | words | 1.340 | 2.414 | 2.099 | 0.315 | 2.414–2.414 | 1.521 | 312.9 | 2634 | 1 |
| retraction512/slab | views | certified | scalar | 1.201 | 8.745 | 1.719 | 7.019 | 8.698–9.094 | 8.439 | 312.9 | 2070 | 5 |
| retraction512/slab | views | certified | words | 1.176 | 2.035 | 1.729 | 0.300 | 1.990–2.078 | 1.718 | 312.9 | 2070 | 5 |
| retraction512/slab | views-inputs | certified | scalar | 1.351 | 8.812 | 1.752 | 7.060 | 8.812–8.812 | 8.468 | 312.9 | 2087 | 1 |
| retraction512/slab | views-inputs | certified | words | 1.326 | 2.099 | 1.764 | 0.334 | 2.099–2.099 | 1.877 | 312.9 | 2087 | 1 |

## fibred, 13 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 1.269 | 0.116 | 0.115 | 0.001 | 0.115–0.130 | 0.014 | 330.0 | 287 | 5 |
| essential512/slab | materialized | materialized | words | 2.124 | 3.985 | 3.985 | 0.000 | 3.985–3.985 | 0.015 | 828.7 | 6212 | 1 |
| essential512/slab | views | certified | words | 1.941 | 2.818 | 2.818 | 0.000 | 2.738–2.920 | 0.013 | 498.4 | 4420 | 5 |
| essential512/slab | views-inputs | certified | words | 2.126 | 3.251 | 3.250 | 0.000 | 3.251–3.251 | 0.015 | 626.0 | 4766 | 1 |
| packed512/enum | materialized | materialized | words | 1.327 | 0.842 | 0.800 | 0.042 | 0.826–0.885 | 0.054 | 1047.7 | 6535 | 5 |
| retraction512/slab | materialized | materialized | scalar | 1.336 | 8.387 | 1.271 | 7.116 | 8.387–8.387 | 7.168 | 325.9 | 2634 | 1 |
| retraction512/slab | materialized | materialized | words | 1.340 | 1.513 | 1.221 | 0.292 | 1.513–1.513 | 0.327 | 325.9 | 2634 | 1 |
| retraction512/slab | views | certified | scalar | 1.201 | 8.002 | 1.001 | 7.004 | 7.993–8.153 | 7.000 | 326.4 | 2070 | 5 |
| retraction512/slab | views | certified | words | 1.176 | 1.302 | 1.006 | 0.296 | 1.284–1.328 | 0.311 | 326.4 | 2070 | 5 |
| retraction512/slab | views-inputs | certified | scalar | 1.351 | 8.014 | 0.950 | 7.065 | 8.014–8.014 | 6.997 | 327.1 | 2087 | 1 |
| retraction512/slab | views-inputs | certified | words | 1.326 | 1.313 | 1.015 | 0.298 | 1.313–1.313 | 0.325 | 327.1 | 2087 | 1 |

## fibred, 13 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 1.266 | 0.240 | 0.239 | 0.001 | 0.237–0.264 | 0.226 | 317.0 | 287 | 5 |
| essential512/slab | views | certified | words | 1.364 | 5.950 | 5.950 | 0.000 | 5.835–6.069 | 1.845 | 1088.7 | 7971 | 5 |
| essential64/slab | views | certified | words | 1.428 | 9.107 | 9.107 | 0.000 | 9.107–9.107 | 2.587 | 1974.9 | 15541 | 1 |
| packed512/enum | materialized | materialized | words | 0.381 | 5.357 | 5.317 | 0.036 | 5.181–5.581 | 4.097 | 2327.3 | 16261 | 5 |
| retraction512/slab | views | certified | words | 1.272 | 1.938 | 1.813 | 0.122 | 1.896–1.985 | 1.327 | 433.2 | 3010 | 5 |
| retraction64/slab | views | certified | words | 2.849 | 5.107 | 4.961 | 0.146 | 5.107–5.107 | 2.067 | 996.9 | 9986 | 1 |

## fibred, 13 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 1.266 | 0.113 | 0.112 | 0.001 | 0.112–0.122 | 0.014 | 330.0 | 287 | 5 |
| essential512/slab | views | certified | words | 1.364 | 5.154 | 5.153 | 0.000 | 5.015–5.788 | 0.021 | 1102.2 | 7971 | 5 |
| essential64/slab | views | certified | words | 1.428 | 7.515 | 7.515 | 0.000 | 7.515–7.515 | 0.016 | 1988.4 | 15541 | 1 |
| packed512/enum | materialized | materialized | words | 0.381 | 2.610 | 2.575 | 0.034 | 2.511–2.706 | 0.048 | 2340.3 | 16261 | 5 |
| retraction512/slab | views | certified | words | 1.272 | 1.316 | 1.197 | 0.119 | 1.283–1.439 | 0.146 | 446.7 | 3010 | 5 |
| retraction64/slab | views | certified | words | 2.849 | 3.602 | 3.463 | 0.136 | 3.602–3.602 | 0.165 | 1010.4 | 9986 | 1 |

## fibred, 19 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 461.279 | 14.501 | 14.415 | 0.086 | 14.484–15.240 | 14.836 | 45001.8 | 684 | 5 |
| essential512/slab | materialized | materialized | words | 29.918 | 52.012 | 52.011 | 0.001 | 52.012–52.012 | 14.527 | 11510.3 | 96826 | 1 |
| essential512/slab | views | certified | words | 29.246 | 30.874 | 30.873 | 0.001 | 30.364–33.440 | 10.835 | 6853.3 | 59905 | 5 |
| essential512/slab | views-inputs | certified | words | 29.910 | 38.973 | 38.972 | 0.001 | 38.973–38.973 | 14.539 | 9389.3 | 71409 | 1 |
| packed512/enum | materialized | materialized | words | 60.312 | 10.841 | 9.998 | 0.854 | 10.707–11.370 | 6.729 | 13305.1 | 98153 | 5 |
| retraction512/slab | materialized | materialized | scalar | 35.177 | 218.017 | 36.823 | 181.193 | 218.017–218.017 | 207.842 | 9943.3 | 64607 | 1 |
| retraction512/slab | materialized | materialized | words | 35.803 | 49.436 | 40.194 | 9.242 | 49.436–49.436 | 36.225 | 9943.3 | 64607 | 1 |
| retraction512/slab | views | certified | scalar | 34.385 | 218.228 | 35.403 | 182.808 | 217.080–218.710 | 210.296 | 9943.3 | 59803 | 5 |
| retraction512/slab | views | certified | words | 35.242 | 43.721 | 34.956 | 8.781 | 43.288–44.899 | 38.784 | 9943.3 | 59803 | 5 |
| retraction512/slab | views-inputs | certified | scalar | 33.990 | 210.946 | 33.410 | 177.535 | 210.946–210.946 | 209.694 | 9943.3 | 60022 | 1 |
| retraction512/slab | views-inputs | certified | words | 35.534 | 42.165 | 33.367 | 8.798 | 42.165–42.165 | 38.924 | 9943.3 | 60022 | 1 |

## fibred, 19 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 461.279 | 6.476 | 6.390 | 0.087 | 6.398–6.534 | 0.101 | 45018.8 | 684 | 5 |
| essential512/slab | materialized | materialized | words | 29.918 | 45.191 | 45.190 | 0.000 | 45.191–45.191 | 0.030 | 11527.3 | 96826 | 1 |
| essential512/slab | views | certified | words | 29.246 | 25.666 | 25.666 | 0.001 | 24.900–27.970 | 0.026 | 6867.1 | 59905 | 5 |
| essential512/slab | views-inputs | certified | words | 29.910 | 31.480 | 31.480 | 0.001 | 31.480–31.480 | 0.048 | 9404.4 | 71409 | 1 |
| packed512/enum | materialized | materialized | words | 60.312 | 8.414 | 7.468 | 0.881 | 8.263–8.905 | 0.888 | 13322.2 | 98153 | 5 |
| retraction512/slab | materialized | materialized | scalar | 35.177 | 205.403 | 22.164 | 183.239 | 205.403–205.403 | 181.584 | 9960.3 | 64607 | 1 |
| retraction512/slab | materialized | materialized | words | 35.803 | 30.665 | 21.787 | 8.878 | 30.665–30.665 | 9.386 | 9960.3 | 64607 | 1 |
| retraction512/slab | views | certified | scalar | 34.385 | 200.371 | 20.358 | 180.018 | 200.122–202.308 | 180.397 | 9957.0 | 59803 | 5 |
| retraction512/slab | views | certified | words | 35.242 | 28.727 | 19.850 | 8.854 | 28.553–28.958 | 9.055 | 9957.0 | 59803 | 5 |
| retraction512/slab | views-inputs | certified | scalar | 33.990 | 200.416 | 18.966 | 181.449 | 200.416–200.416 | 182.511 | 9958.4 | 60022 | 1 |
| retraction512/slab | views-inputs | certified | words | 35.534 | 27.830 | 19.053 | 8.777 | 27.830–27.830 | 8.863 | 9958.4 | 60022 | 1 |

## fibred, 19 coordinates, face-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 463.933 | 15.210 | 15.106 | 0.102 | 14.906–15.788 | 14.845 | 45001.8 | 684 | 5 |
| essential512/slab | views | certified | words | 6.989 | 70.332 | 70.331 | 0.001 | 70.056–77.737 | 16.990 | 12039.0 | 109616 | 5 |
| essential64/slab | views | certified | words | 7.852 | 72.621 | 72.620 | 0.001 | 72.621–72.621 | 20.993 | 14446.6 | 143819 | 1 |
| packed512/enum | materialized | materialized | words | 2.401 | 40.315 | 40.200 | 0.118 | 38.926–41.776 | 25.544 | 25957.2 | 218715 | 5 |
| retraction512/slab | views | certified | words | 29.645 | 28.057 | 27.779 | 0.281 | 25.830–33.243 | 11.584 | 9112.8 | 80302 | 5 |
| retraction64/slab | views | certified | words | 19.423 | 44.181 | 43.597 | 0.583 | 44.181–44.181 | 17.921 | 12351.3 | 118134 | 1 |

## fibred, 19 coordinates, face-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 463.933 | 6.681 | 6.584 | 0.105 | 6.623–7.336 | 0.112 | 45018.8 | 684 | 5 |
| essential512/slab | views | certified | words | 6.989 | 63.080 | 63.078 | 0.001 | 61.676–69.945 | 0.030 | 12052.8 | 109616 | 5 |
| essential64/slab | views | certified | words | 7.852 | 60.945 | 60.944 | 0.001 | 60.945–60.945 | 0.032 | 14460.3 | 143819 | 1 |
| packed512/enum | materialized | materialized | words | 2.401 | 26.224 | 26.106 | 0.118 | 24.420–27.209 | 0.155 | 25974.3 | 218715 | 5 |
| retraction512/slab | views | certified | words | 29.645 | 22.549 | 22.276 | 0.274 | 21.796–23.302 | 0.320 | 9126.5 | 80302 | 5 |
| retraction64/slab | views | certified | words | 19.423 | 37.872 | 37.292 | 0.579 | 37.872–37.872 | 0.687 | 12365.1 | 118134 | 1 |

136 configurations; 216 matched completion/control comparisons; 58 retained processes.

- [Raw evidence](results/retraction-words-smoke.json).
- [Raw evidence](results/retraction-words-repeat.json).
- [Raw evidence](results/retraction-layout-screen.json).
- [Raw evidence](results/retraction-layout-repeat.json).
