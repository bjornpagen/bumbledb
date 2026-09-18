# Fixed-decoder completion: native Free Join comparison

Construction is measured separately. Fresh and warm queries include exact legal
population checksums over all 80 outputs. Every output is also checked pointwise
outside timing. Retained KB includes construction residue and owner caches; temporary
observation memo and allocator overhead are excluded. No raw alias counts are used.
Executable: `012956ac63684e8e7d9cade58e605bd49f822e14c9a484b6c51dcda16e4303b4`.

## below, 12 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Build ms | Fresh ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | 0.095 | 0.215 | 0.215–0.215 | 0.142 | 101.1 | 173 | 1 |
| essential512/slab | materialized | materialized | 0.459 | 1.986 | 1.986–1.986 | 0.600 | 330.9 | 2130 | 1 |
| essential512/slab | views | certified | 0.453 | 1.468 | 1.468–1.468 | 1.120 | 173.9 | 1408 | 1 |
| essential512/slab | views-inputs | certified | 0.453 | 1.771 | 1.771–1.771 | 0.716 | 204.2 | 1521 | 1 |
| packed512/enum | materialized | materialized | 0.605 | 1.196 | 1.196–1.196 | 0.951 | 330.6 | 2427 | 1 |
| retraction512/slab | materialized | materialized | 0.552 | 3.095 | 3.095–3.095 | 2.798 | 152.4 | 1229 | 1 |
| retraction512/slab | views | certified | 0.568 | 3.186 | 3.186–3.186 | 2.850 | 126.7 | 959 | 1 |
| retraction512/slab | views-inputs | certified | 0.583 | 3.044 | 3.044–3.044 | 2.857 | 126.7 | 960 | 1 |

## below, 12 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Build ms | Fresh ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | 0.095 | 0.084 | 0.084–0.084 | 0.014 | 114.1 | 173 | 1 |
| essential512/slab | materialized | materialized | 0.459 | 1.611 | 1.611–1.611 | 0.015 | 343.9 | 2130 | 1 |
| essential512/slab | views | certified | 0.453 | 1.619 | 1.619–1.619 | 0.052 | 185.8 | 1408 | 1 |
| essential512/slab | views-inputs | certified | 0.453 | 1.321 | 1.321–1.321 | 0.019 | 216.8 | 1521 | 1 |
| packed512/enum | materialized | materialized | 0.605 | 0.475 | 0.475–0.475 | 0.038 | 343.6 | 2427 | 1 |
| retraction512/slab | materialized | materialized | 0.552 | 2.856 | 2.856–2.856 | 2.219 | 165.4 | 1229 | 1 |
| retraction512/slab | views | certified | 0.568 | 2.840 | 2.840–2.840 | 2.231 | 138.6 | 959 | 1 |
| retraction512/slab | views-inputs | certified | 0.583 | 2.626 | 2.626–2.626 | 2.136 | 139.3 | 960 | 1 |

## below, 18 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Build ms | Fresh ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | 6.521 | 7.393 | 7.393–7.393 | 7.118 | 7418.7 | 224 | 1 |
| essential512/slab | materialized | materialized | 2.510 | 12.596 | 12.596–12.596 | 3.704 | 2226.7 | 16146 | 1 |
| essential512/slab | views | certified | 2.510 | 8.231 | 8.231–8.231 | 3.525 | 1175.3 | 8419 | 1 |
| essential512/slab | views-inputs | certified | 2.480 | 9.601 | 9.601–9.601 | 4.098 | 1175.0 | 10032 | 1 |
| packed512/enum | materialized | materialized | 27.630 | 3.269 | 3.269–3.269 | 2.237 | 2465.2 | 17618 | 1 |
| retraction512/slab | materialized | materialized | 7.495 | 39.174 | 39.174–39.174 | 37.027 | 1948.6 | 14617 | 1 |
| retraction512/slab | views | certified | 7.828 | 40.102 | 40.102–40.102 | 37.661 | 1704.9 | 13037 | 1 |
| retraction512/slab | views-inputs | certified | 7.754 | 39.594 | 39.594–39.594 | 38.778 | 1704.9 | 13101 | 1 |

## below, 18 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Build ms | Fresh ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | 6.521 | 2.892 | 2.892–2.892 | 0.055 | 7431.7 | 224 | 1 |
| essential512/slab | materialized | materialized | 2.510 | 11.072 | 11.072–11.072 | 0.029 | 2239.7 | 16146 | 1 |
| essential512/slab | views | certified | 2.510 | 6.470 | 6.470–6.470 | 0.027 | 1188.8 | 8419 | 1 |
| essential512/slab | views-inputs | certified | 2.480 | 7.711 | 7.711–7.711 | 0.027 | 1189.2 | 10032 | 1 |
| packed512/enum | materialized | materialized | 27.630 | 2.102 | 2.102–2.102 | 0.207 | 2478.1 | 17618 | 1 |
| retraction512/slab | materialized | materialized | 7.495 | 37.477 | 37.477–37.477 | 34.064 | 1961.6 | 14617 | 1 |
| retraction512/slab | views | certified | 7.828 | 39.228 | 39.228–39.228 | 33.232 | 1718.4 | 13037 | 1 |
| retraction512/slab | views-inputs | certified | 7.754 | 37.531 | 37.531–37.531 | 33.786 | 1719.1 | 13101 | 1 |

## fibred, 13 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Build ms | Fresh ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | 1.294 | 0.268 | 0.268–0.268 | 0.227 | 317.0 | 287 | 1 |
| essential512/slab | materialized | materialized | 2.068 | 4.863 | 4.863–4.863 | 1.613 | 815.7 | 6212 | 1 |
| essential512/slab | views | certified | 2.030 | 3.554 | 3.554–3.554 | 1.447 | 484.9 | 4420 | 1 |
| essential512/slab | views-inputs | certified | 2.058 | 4.076 | 4.076–4.076 | 1.707 | 611.8 | 4766 | 1 |
| packed512/enum | materialized | materialized | 1.347 | 1.663 | 1.663–1.663 | 1.191 | 1034.7 | 6535 | 1 |
| retraction512/slab | materialized | materialized | 1.315 | 8.963 | 8.963–8.963 | 8.237 | 312.9 | 2634 | 1 |
| retraction512/slab | views | certified | 1.257 | 8.986 | 8.986–8.986 | 8.692 | 312.9 | 2070 | 1 |
| retraction512/slab | views-inputs | certified | 1.318 | 9.059 | 9.059–9.059 | 8.664 | 312.9 | 2087 | 1 |

## fibred, 13 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Build ms | Fresh ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | 1.294 | 0.127 | 0.127–0.127 | 0.014 | 330.0 | 287 | 1 |
| essential512/slab | materialized | materialized | 2.068 | 3.873 | 3.873–3.873 | 0.015 | 828.7 | 6212 | 1 |
| essential512/slab | views | certified | 2.030 | 2.693 | 2.693–2.693 | 0.020 | 498.4 | 4420 | 1 |
| essential512/slab | views-inputs | certified | 2.058 | 3.082 | 3.082–3.082 | 0.018 | 626.0 | 4766 | 1 |
| packed512/enum | materialized | materialized | 1.347 | 0.879 | 0.879–0.879 | 0.058 | 1047.7 | 6535 | 1 |
| retraction512/slab | materialized | materialized | 1.315 | 8.574 | 8.574–8.574 | 7.170 | 325.9 | 2634 | 1 |
| retraction512/slab | views | certified | 1.257 | 7.822 | 7.822–7.822 | 7.144 | 326.4 | 2070 | 1 |
| retraction512/slab | views-inputs | certified | 1.318 | 8.281 | 8.281–8.281 | 7.228 | 327.1 | 2087 | 1 |

## fibred, 19 coordinates, bit-major, memo False

| Carrier/store | Product | Gates | Build ms | Fresh ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | 462.778 | 15.537 | 15.537–15.537 | 14.339 | 45001.8 | 684 | 1 |
| essential512/slab | materialized | materialized | 28.677 | 51.435 | 51.435–51.435 | 14.169 | 11510.3 | 96826 | 1 |
| essential512/slab | views | certified | 28.796 | 29.668 | 29.668–29.668 | 10.917 | 6853.3 | 59905 | 1 |
| essential512/slab | views-inputs | certified | 29.029 | 38.630 | 38.630–38.630 | 14.343 | 9389.3 | 71409 | 1 |
| packed512/enum | materialized | materialized | 59.852 | 12.031 | 12.031–12.031 | 7.597 | 13305.1 | 98153 | 1 |
| retraction512/slab | materialized | materialized | 33.769 | 215.690 | 215.690–215.690 | 208.872 | 9943.3 | 64607 | 1 |
| retraction512/slab | views | certified | 35.375 | 219.975 | 219.975–219.975 | 211.543 | 9943.3 | 59803 | 1 |
| retraction512/slab | views-inputs | certified | 35.851 | 244.976 | 244.976–244.976 | 206.380 | 9943.3 | 60022 | 1 |

## fibred, 19 coordinates, bit-major, memo True

| Carrier/store | Product | Gates | Build ms | Fresh ms | Min–max | Warm ms | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | materialized | 462.778 | 7.013 | 7.013–7.013 | 0.101 | 45018.8 | 684 | 1 |
| essential512/slab | materialized | materialized | 28.677 | 43.914 | 43.914–43.914 | 0.040 | 11527.3 | 96826 | 1 |
| essential512/slab | views | certified | 28.796 | 24.686 | 24.686–24.686 | 0.030 | 6867.1 | 59905 | 1 |
| essential512/slab | views-inputs | certified | 29.029 | 32.417 | 32.417–32.417 | 0.035 | 9404.4 | 71409 | 1 |
| packed512/enum | materialized | materialized | 59.852 | 8.623 | 8.623–8.623 | 0.989 | 13322.2 | 98153 | 1 |
| retraction512/slab | materialized | materialized | 33.769 | 202.385 | 202.385–202.385 | 180.744 | 9960.3 | 64607 | 1 |
| retraction512/slab | views | certified | 35.375 | 285.617 | 285.617–285.617 | 181.959 | 9957.0 | 59803 | 1 |
| retraction512/slab | views-inputs | certified | 35.851 | 196.039 | 196.039–196.039 | 176.902 | 9958.4 | 60022 | 1 |

64 configurations; 72 matched completion/control comparisons; 19 retained processes.

- [Raw evidence](results/retraction-smoke.json).
