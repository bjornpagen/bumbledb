# Mapped products through Free Join

One executable; latest supplied process per exact case. Times are
milliseconds; KB means 1,000 bytes. Full relation programs and per-binding
products have different output contracts and remain separate. Query time
includes native join, canonical result construction and exact count readout.
Construction, cloning, native setup and full bitset verification are outside.
Resident byte estimates include arenas and outer operation caches but omit
temporary traversal storage and allocator overhead. Failed/resource-limited
processes do not supply timings. See [the kernel contract](VIEW-PRODUCT.md).

Executable SHA-256: `7e17c705f3310183de4a3d87443990e34ba4523dc8da59d76daabd47a53ae068`.

## product_free_join, 12 coordinates, bit-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.066 | 0.245 | 0.245–0.245 | 0.176 | 16.0 | 76.6 | 129 | 1 |
| essential512/enum | materialized | 0.029 | 0.787 | 0.787–0.787 | 0.609 | 5.6 | 64.9 | 329 | 1 |
| essential512/enum | views | 0.058 | 2.000 | 2.000–2.000 | 1.943 | 5.6 | 32.5 | 216 | 1 |
| essential64/enum | materialized | 0.051 | 1.660 | 1.660–1.660 | 1.261 | 9.6 | 125.2 | 829 | 1 |
| essential64/enum | views | 0.030 | 3.637 | 3.637–3.637 | 3.495 | 9.6 | 79.6 | 486 | 1 |
| packed512 | materialized | 0.094 | 1.572 | 1.572–1.572 | 1.399 | 12.9 | 63.6 | 548 | 1 |

## product_free_join, 12 coordinates, bit-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.066 | 0.051 | 0.051–0.051 | 0.009 | 16.0 | 84.0 | 129 | 1 |
| essential512/enum | materialized | 0.029 | 0.268 | 0.268–0.268 | 0.009 | 5.6 | 72.3 | 329 | 1 |
| essential512/enum | views | 0.058 | 0.479 | 0.479–0.479 | 0.010 | 5.6 | 38.6 | 216 | 1 |
| essential64/enum | materialized | 0.051 | 0.568 | 0.568–0.568 | 0.009 | 9.6 | 132.6 | 829 | 1 |
| essential64/enum | views | 0.030 | 0.790 | 0.790–0.790 | 0.007 | 9.6 | 85.6 | 486 | 1 |
| packed512 | materialized | 0.094 | 0.226 | 0.226–0.226 | 0.010 | 12.9 | 70.9 | 548 | 1 |

## product_free_join, 18 coordinates, bit-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.021 | 9.180 | 9.180–9.180 | 8.850 | 919.3 | 4335.1 | 130 | 1 |
| essential512/enum | materialized | 0.015 | 7.254 | 7.254–7.254 | 5.774 | 28.1 | 583.9 | 3112 | 1 |
| essential512/enum | views | 0.017 | 62.570 | 62.570–62.570 | 61.097 | 28.1 | 311.2 | 1792 | 1 |
| essential64/enum | materialized | 0.022 | 10.547 | 10.547–10.547 | 8.318 | 62.7 | 826.7 | 5582 | 1 |
| essential64/enum | views | 0.016 | 34.492 | 34.492–34.492 | 33.492 | 62.7 | 408.4 | 3102 | 1 |
| packed512 | materialized | 0.016 | 3.251 | 3.251–3.251 | 2.811 | 65.3 | 676.8 | 5073 | 1 |

## product_free_join, 18 coordinates, bit-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.021 | 1.334 | 1.334–1.334 | 0.017 | 919.3 | 4342.5 | 130 | 1 |
| essential512/enum | materialized | 0.015 | 2.642 | 2.642–2.642 | 0.015 | 28.1 | 591.3 | 3112 | 1 |
| essential512/enum | views | 0.017 | 12.274 | 12.274–12.274 | 0.013 | 28.1 | 317.3 | 1792 | 1 |
| essential64/enum | materialized | 0.022 | 3.324 | 3.324–3.324 | 0.020 | 62.7 | 834.1 | 5582 | 1 |
| essential64/enum | views | 0.016 | 7.473 | 7.473–7.473 | 0.014 | 62.7 | 414.4 | 3102 | 1 |
| packed512 | materialized | 0.016 | 0.695 | 0.695–0.695 | 0.057 | 65.3 | 684.2 | 5073 | 1 |

## relations_free_join, 12 coordinates, bit-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.060 | 0.221 | 0.221–0.221 | 0.146 | 16.0 | 82.5 | 140 | 1 |
| essential512/enum | materialized | 0.060 | 0.580 | 0.580–0.580 | 0.229 | 5.6 | 66.9 | 345 | 1 |
| essential512/enum | views | 0.028 | 0.720 | 0.720–0.720 | 0.645 | 5.6 | 22.7 | 130 | 1 |
| essential64/enum | materialized | 0.052 | 1.099 | 1.099–1.099 | 0.639 | 9.6 | 167.9 | 1159 | 1 |
| essential64/enum | views | 0.027 | 0.948 | 0.948–0.948 | 0.867 | 9.6 | 79.5 | 495 | 1 |
| packed512 | materialized | 0.064 | 0.915 | 0.915–0.915 | 0.805 | 12.9 | 74.3 | 482 | 1 |

## relations_free_join, 12 coordinates, bit-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.060 | 0.064 | 0.064–0.064 | 0.010 | 16.0 | 95.5 | 140 | 1 |
| essential512/enum | materialized | 0.060 | 0.326 | 0.326–0.326 | 0.011 | 5.6 | 79.9 | 345 | 1 |
| essential512/enum | views | 0.028 | 0.389 | 0.389–0.389 | 0.009 | 5.6 | 34.5 | 130 | 1 |
| essential64/enum | materialized | 0.052 | 0.725 | 0.725–0.725 | 0.010 | 9.6 | 180.9 | 1159 | 1 |
| essential64/enum | views | 0.027 | 0.544 | 0.544–0.544 | 0.009 | 9.6 | 91.2 | 495 | 1 |
| packed512 | materialized | 0.064 | 0.269 | 0.269–0.269 | 0.018 | 12.9 | 87.3 | 482 | 1 |

## relations_free_join, 18 coordinates, bit-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.022 | 7.301 | 7.301–7.301 | 7.145 | 919.3 | 5810.7 | 175 | 1 |
| essential512/enum | materialized | 0.014 | 3.258 | 3.258–3.258 | 1.646 | 28.1 | 450.5 | 2668 | 1 |
| essential512/enum | views | 0.015 | 15.366 | 15.366–15.366 | 14.454 | 28.1 | 195.5 | 1011 | 1 |
| essential64/enum | materialized | 0.016 | 4.732 | 4.732–4.732 | 2.844 | 62.7 | 879.5 | 5699 | 1 |
| essential64/enum | views | 0.019 | 6.857 | 6.857–6.857 | 6.159 | 62.7 | 415.1 | 2640 | 1 |
| packed512 | materialized | 0.021 | 1.510 | 1.510–1.510 | 1.287 | 65.3 | 499.4 | 3815 | 1 |

## relations_free_join, 18 coordinates, bit-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.022 | 2.497 | 2.497–2.497 | 0.057 | 919.3 | 5823.7 | 175 | 1 |
| essential512/enum | materialized | 0.014 | 2.374 | 2.374–2.374 | 0.026 | 28.1 | 463.5 | 2668 | 1 |
| essential512/enum | views | 0.015 | 7.920 | 7.920–7.920 | 0.019 | 28.1 | 207.2 | 1011 | 1 |
| essential64/enum | materialized | 0.016 | 3.314 | 3.314–3.314 | 0.030 | 62.7 | 892.5 | 5699 | 1 |
| essential64/enum | views | 0.019 | 4.208 | 4.208–4.208 | 0.026 | 62.7 | 426.9 | 2640 | 1 |
| packed512 | materialized | 0.021 | 0.703 | 0.703–0.703 | 0.059 | 65.3 | 512.4 | 3815 | 1 |

## Evidence

48 distinct cases, 16 matched strategy pairs, 14 retained processes.

- [Raw evidence](results/view-native-smoke.json).
