# Fixed-decoder completion: native Free Join comparison

Construction is measured separately. Fresh and warm queries include exact legal
population checksums over all 80 outputs. Every output is also checked pointwise
outside timing. Retained KB includes construction residue and owner caches; temporary
observation memo and allocator overhead are excluded. No raw alias counts are used.
Executable: `c27b4aca8568e345e5499cc9a594a7703d397411b9a585243f83be185fc1b8d2`.

## below, 12 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.099 | 0.205 | 0.204 | 0.001 | 0.205–0.205 | 0.150 | 101.1 | 173 | 1 |
| essential512/slab | materialized | materialized | words | 0.479 | 2.033 | 2.032 | 0.000 | 2.033–2.033 | 0.642 | 330.9 | 2130 | 1 |
| essential512/slab | views | certified | words | 0.494 | 1.565 | 1.564 | 0.000 | 1.565–1.565 | 0.641 | 173.9 | 1408 | 1 |
| essential512/slab | views-inputs | certified | words | 0.515 | 1.793 | 1.793 | 0.000 | 1.793–1.793 | 0.813 | 204.2 | 1521 | 1 |
| packed512/enum | materialized | materialized | words | 0.614 | 1.221 | 1.198 | 0.023 | 1.221–1.221 | 0.885 | 330.6 | 2427 | 1 |
| retraction512/slab | materialized | materialized | scalar | 0.566 | 3.014 | 0.926 | 2.087 | 3.014–3.014 | 2.743 | 152.4 | 1229 | 1 |
| retraction512/slab | materialized | materialized | words | 0.556 | 1.020 | 0.918 | 0.102 | 1.020–1.020 | 0.654 | 152.4 | 1229 | 1 |
| retraction512/slab | views | certified | scalar | 0.568 | 3.104 | 0.911 | 2.193 | 3.104–3.104 | 2.820 | 126.7 | 959 | 1 |
| retraction512/slab | views | certified | words | 0.657 | 1.062 | 0.957 | 0.105 | 1.062–1.062 | 0.878 | 126.7 | 959 | 1 |
| retraction512/slab | views-inputs | certified | scalar | 0.572 | 3.027 | 0.899 | 2.128 | 3.027–3.027 | 3.212 | 126.7 | 960 | 1 |
| retraction512/slab | views-inputs | certified | words | 0.582 | 0.987 | 0.890 | 0.098 | 0.987–0.987 | 0.793 | 126.7 | 960 | 1 |

## below, 12 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 0.099 | 0.086 | 0.085 | 0.001 | 0.086–0.086 | 0.014 | 114.1 | 173 | 1 |
| essential512/slab | materialized | materialized | words | 0.479 | 1.657 | 1.657 | 0.000 | 1.657–1.657 | 0.014 | 343.9 | 2130 | 1 |
| essential512/slab | views | certified | words | 0.494 | 1.144 | 1.143 | 0.000 | 1.144–1.144 | 0.014 | 185.8 | 1408 | 1 |
| essential512/slab | views-inputs | certified | words | 0.515 | 1.369 | 1.368 | 0.000 | 1.369–1.369 | 0.014 | 216.8 | 1521 | 1 |
| packed512/enum | materialized | materialized | words | 0.614 | 0.451 | 0.430 | 0.021 | 0.451–0.451 | 0.033 | 343.6 | 2427 | 1 |
| retraction512/slab | materialized | materialized | scalar | 0.566 | 2.716 | 0.590 | 2.126 | 2.716–2.716 | 2.147 | 165.4 | 1229 | 1 |
| retraction512/slab | materialized | materialized | words | 0.556 | 0.678 | 0.583 | 0.095 | 0.678–0.678 | 0.106 | 165.4 | 1229 | 1 |
| retraction512/slab | views | certified | scalar | 0.568 | 2.641 | 0.518 | 2.122 | 2.641–2.641 | 2.193 | 138.6 | 959 | 1 |
| retraction512/slab | views | certified | words | 0.657 | 0.634 | 0.535 | 0.099 | 0.634–0.634 | 0.132 | 138.6 | 959 | 1 |
| retraction512/slab | views-inputs | certified | scalar | 0.572 | 2.665 | 0.526 | 2.139 | 2.665–2.665 | 2.097 | 139.3 | 960 | 1 |
| retraction512/slab | views-inputs | certified | words | 0.582 | 0.562 | 0.468 | 0.094 | 0.562–0.562 | 0.106 | 139.3 | 960 | 1 |

## below, 18 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 6.477 | 7.335 | 7.292 | 0.042 | 7.335–7.335 | 6.886 | 7418.7 | 224 | 1 |
| essential512/slab | materialized | materialized | words | 2.605 | 12.849 | 12.848 | 0.001 | 12.849–12.849 | 3.792 | 2226.7 | 16146 | 1 |
| essential512/slab | views | certified | words | 2.587 | 8.961 | 8.960 | 0.000 | 8.961–8.961 | 3.809 | 1175.3 | 8419 | 1 |
| essential512/slab | views-inputs | certified | words | 2.617 | 9.577 | 9.577 | 0.000 | 9.577–9.577 | 3.883 | 1175.0 | 10032 | 1 |
| packed512/enum | materialized | materialized | words | 27.635 | 3.269 | 3.098 | 0.171 | 3.269–3.269 | 2.225 | 2465.2 | 17618 | 1 |
| retraction512/slab | materialized | materialized | scalar | 7.682 | 39.259 | 5.861 | 33.397 | 39.259–39.259 | 37.831 | 1948.6 | 14617 | 1 |
| retraction512/slab | materialized | materialized | words | 7.677 | 7.628 | 5.830 | 1.799 | 7.628–7.628 | 5.390 | 1948.6 | 14617 | 1 |
| retraction512/slab | views | certified | scalar | 7.835 | 39.533 | 5.638 | 33.895 | 39.533–39.533 | 37.730 | 1704.9 | 13037 | 1 |
| retraction512/slab | views | certified | words | 7.890 | 8.651 | 6.656 | 1.995 | 8.651–8.651 | 5.819 | 1704.9 | 13037 | 1 |
| retraction512/slab | views-inputs | certified | scalar | 7.739 | 38.688 | 5.536 | 33.152 | 38.688–38.688 | 38.936 | 1704.9 | 13101 | 1 |
| retraction512/slab | views-inputs | certified | words | 7.784 | 7.583 | 5.699 | 1.883 | 7.583–7.583 | 6.215 | 1704.9 | 13101 | 1 |

## below, 18 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 6.477 | 2.813 | 2.772 | 0.041 | 2.813–2.813 | 0.055 | 7431.7 | 224 | 1 |
| essential512/slab | materialized | materialized | words | 2.605 | 10.855 | 10.855 | 0.000 | 10.855–10.855 | 0.033 | 2239.7 | 16146 | 1 |
| essential512/slab | views | certified | words | 2.587 | 6.870 | 6.867 | 0.001 | 6.870–6.870 | 0.025 | 1188.8 | 8419 | 1 |
| essential512/slab | views-inputs | certified | words | 2.617 | 7.735 | 7.734 | 0.000 | 7.735–7.735 | 0.061 | 1189.2 | 10032 | 1 |
| packed512/enum | materialized | materialized | words | 27.635 | 2.182 | 2.013 | 0.168 | 2.182–2.182 | 0.200 | 2478.1 | 17618 | 1 |
| retraction512/slab | materialized | materialized | scalar | 7.682 | 37.826 | 4.345 | 33.480 | 37.826–37.826 | 33.895 | 1961.6 | 14617 | 1 |
| retraction512/slab | materialized | materialized | words | 7.677 | 6.147 | 4.306 | 1.841 | 6.147–6.147 | 1.861 | 1961.6 | 14617 | 1 |
| retraction512/slab | views | certified | scalar | 7.835 | 37.443 | 3.935 | 33.508 | 37.443–37.443 | 34.049 | 1718.4 | 13037 | 1 |
| retraction512/slab | views | certified | words | 7.890 | 5.667 | 3.827 | 1.840 | 5.667–5.667 | 1.957 | 1718.4 | 13037 | 1 |
| retraction512/slab | views-inputs | certified | scalar | 7.739 | 37.518 | 3.572 | 33.945 | 37.518–37.518 | 33.798 | 1719.1 | 13101 | 1 |
| retraction512/slab | views-inputs | certified | words | 7.784 | 5.396 | 3.565 | 1.831 | 5.396–5.396 | 1.847 | 1719.1 | 13101 | 1 |

## fibred, 13 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 1.365 | 0.284 | 0.282 | 0.001 | 0.284–0.284 | 0.230 | 317.0 | 287 | 1 |
| essential512/slab | materialized | materialized | words | 2.124 | 4.906 | 4.906 | 0.000 | 4.906–4.906 | 1.698 | 815.7 | 6212 | 1 |
| essential512/slab | views | certified | words | 2.245 | 3.739 | 3.738 | 0.001 | 3.739–3.739 | 1.554 | 484.9 | 4420 | 1 |
| essential512/slab | views-inputs | certified | words | 2.126 | 4.423 | 4.419 | 0.000 | 4.423–4.423 | 1.744 | 611.8 | 4766 | 1 |
| packed512/enum | materialized | materialized | words | 1.368 | 1.662 | 1.614 | 0.048 | 1.662–1.662 | 1.204 | 1034.7 | 6535 | 1 |
| retraction512/slab | materialized | materialized | scalar | 1.336 | 9.051 | 1.862 | 7.189 | 9.051–9.051 | 8.337 | 312.9 | 2634 | 1 |
| retraction512/slab | materialized | materialized | words | 1.340 | 2.414 | 2.099 | 0.315 | 2.414–2.414 | 1.521 | 312.9 | 2634 | 1 |
| retraction512/slab | views | certified | scalar | 1.278 | 8.845 | 1.735 | 7.110 | 8.845–8.845 | 8.440 | 312.9 | 2070 | 1 |
| retraction512/slab | views | certified | words | 1.306 | 2.036 | 1.738 | 0.298 | 2.036–2.036 | 1.677 | 312.9 | 2070 | 1 |
| retraction512/slab | views-inputs | certified | scalar | 1.351 | 8.812 | 1.752 | 7.060 | 8.812–8.812 | 8.468 | 312.9 | 2087 | 1 |
| retraction512/slab | views-inputs | certified | words | 1.326 | 2.099 | 1.764 | 0.334 | 2.099–2.099 | 1.877 | 312.9 | 2087 | 1 |

## fibred, 13 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 1.365 | 0.138 | 0.137 | 0.001 | 0.138–0.138 | 0.015 | 330.0 | 287 | 1 |
| essential512/slab | materialized | materialized | words | 2.124 | 3.985 | 3.985 | 0.000 | 3.985–3.985 | 0.015 | 828.7 | 6212 | 1 |
| essential512/slab | views | certified | words | 2.245 | 2.926 | 2.925 | 0.000 | 2.926–2.926 | 0.020 | 498.4 | 4420 | 1 |
| essential512/slab | views-inputs | certified | words | 2.126 | 3.251 | 3.250 | 0.000 | 3.251–3.251 | 0.015 | 626.0 | 4766 | 1 |
| packed512/enum | materialized | materialized | words | 1.368 | 0.898 | 0.855 | 0.043 | 0.898–0.898 | 0.055 | 1047.7 | 6535 | 1 |
| retraction512/slab | materialized | materialized | scalar | 1.336 | 8.387 | 1.271 | 7.116 | 8.387–8.387 | 7.168 | 325.9 | 2634 | 1 |
| retraction512/slab | materialized | materialized | words | 1.340 | 1.513 | 1.221 | 0.292 | 1.513–1.513 | 0.327 | 325.9 | 2634 | 1 |
| retraction512/slab | views | certified | scalar | 1.278 | 7.967 | 1.021 | 6.946 | 7.967–7.967 | 6.967 | 326.4 | 2070 | 1 |
| retraction512/slab | views | certified | words | 1.306 | 1.276 | 0.982 | 0.294 | 1.276–1.276 | 0.325 | 326.4 | 2070 | 1 |
| retraction512/slab | views-inputs | certified | scalar | 1.351 | 8.014 | 0.950 | 7.065 | 8.014–8.014 | 6.997 | 327.1 | 2087 | 1 |
| retraction512/slab | views-inputs | certified | words | 1.326 | 1.313 | 1.015 | 0.298 | 1.313–1.313 | 0.325 | 327.1 | 2087 | 1 |

## fibred, 19 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 461.661 | 15.368 | 15.252 | 0.115 | 15.368–15.368 | 14.456 | 45001.8 | 684 | 1 |
| essential512/slab | materialized | materialized | words | 29.918 | 52.012 | 52.011 | 0.001 | 52.012–52.012 | 14.527 | 11510.3 | 96826 | 1 |
| essential512/slab | views | certified | words | 29.658 | 29.732 | 29.731 | 0.000 | 29.732–29.732 | 10.577 | 6853.3 | 59905 | 1 |
| essential512/slab | views-inputs | certified | words | 29.910 | 38.973 | 38.972 | 0.001 | 38.973–38.973 | 14.539 | 9389.3 | 71409 | 1 |
| packed512/enum | materialized | materialized | words | 61.336 | 11.423 | 10.542 | 0.881 | 11.423–11.423 | 7.143 | 13305.1 | 98153 | 1 |
| retraction512/slab | materialized | materialized | scalar | 35.177 | 218.017 | 36.823 | 181.193 | 218.017–218.017 | 207.842 | 9943.3 | 64607 | 1 |
| retraction512/slab | materialized | materialized | words | 35.803 | 49.436 | 40.194 | 9.242 | 49.436–49.436 | 36.225 | 9943.3 | 64607 | 1 |
| retraction512/slab | views | certified | scalar | 34.427 | 212.974 | 35.027 | 177.946 | 212.974–212.974 | 208.884 | 9943.3 | 59803 | 1 |
| retraction512/slab | views | certified | words | 36.142 | 44.161 | 35.402 | 8.759 | 44.161–44.161 | 39.403 | 9943.3 | 59803 | 1 |
| retraction512/slab | views-inputs | certified | scalar | 33.990 | 210.946 | 33.410 | 177.535 | 210.946–210.946 | 209.694 | 9943.3 | 60022 | 1 |
| retraction512/slab | views-inputs | certified | words | 35.534 | 42.165 | 33.367 | 8.798 | 42.165–42.165 | 38.924 | 9943.3 | 60022 | 1 |

## fibred, 19 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | words | 461.661 | 6.658 | 6.570 | 0.088 | 6.658–6.658 | 0.120 | 45018.8 | 684 | 1 |
| essential512/slab | materialized | materialized | words | 29.918 | 45.191 | 45.190 | 0.000 | 45.191–45.191 | 0.030 | 11527.3 | 96826 | 1 |
| essential512/slab | views | certified | words | 29.658 | 24.752 | 24.751 | 0.000 | 24.752–24.752 | 0.026 | 6867.1 | 59905 | 1 |
| essential512/slab | views-inputs | certified | words | 29.910 | 31.480 | 31.480 | 0.001 | 31.480–31.480 | 0.048 | 9404.4 | 71409 | 1 |
| packed512/enum | materialized | materialized | words | 61.336 | 8.839 | 7.984 | 0.854 | 8.839–8.839 | 0.895 | 13322.2 | 98153 | 1 |
| retraction512/slab | materialized | materialized | scalar | 35.177 | 205.403 | 22.164 | 183.239 | 205.403–205.403 | 181.584 | 9960.3 | 64607 | 1 |
| retraction512/slab | materialized | materialized | words | 35.803 | 30.665 | 21.787 | 8.878 | 30.665–30.665 | 9.386 | 9960.3 | 64607 | 1 |
| retraction512/slab | views | certified | scalar | 34.427 | 197.093 | 19.828 | 177.262 | 197.093–197.093 | 181.398 | 9957.0 | 59803 | 1 |
| retraction512/slab | views | certified | words | 36.142 | 28.474 | 19.838 | 8.635 | 28.474–28.474 | 9.084 | 9957.0 | 59803 | 1 |
| retraction512/slab | views-inputs | certified | scalar | 33.990 | 200.416 | 18.966 | 181.449 | 200.416–200.416 | 182.511 | 9958.4 | 60022 | 1 |
| retraction512/slab | views-inputs | certified | words | 35.534 | 27.830 | 19.053 | 8.777 | 27.830–27.830 | 8.863 | 9958.4 | 60022 | 1 |

88 configurations; 168 matched completion/control comparisons; 25 retained processes.

- [Raw evidence](results/retraction-words-smoke.json).
