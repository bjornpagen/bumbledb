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
| dense-dispatched | materialized | 0.005 | 0.183 | 0.177–0.197 | 0.175 | 16.0 | 76.6 | 129 | 5 |
| essential512/enum | materialized | 0.005 | 0.751 | 0.730–0.834 | 0.609 | 5.6 | 64.9 | 329 | 5 |
| essential512/enum | views | 0.005 | 1.974 | 1.917–2.127 | 1.915 | 5.6 | 32.5 | 216 | 5 |
| essential512/slab | materialized | 0.005 | 0.772 | 0.735–0.874 | 0.598 | 3.2 | 41.8 | 329 | 5 |
| essential512/slab | views | 0.005 | 2.013 | 1.914–2.086 | 1.961 | 3.2 | 20.1 | 216 | 5 |
| essential64/enum | materialized | 0.005 | 1.518 | 1.494–1.615 | 1.167 | 9.6 | 125.2 | 829 | 5 |
| essential64/enum | views | 0.005 | 3.738 | 3.674–3.803 | 3.533 | 9.6 | 79.6 | 486 | 5 |
| essential64/slab | materialized | 0.005 | 1.584 | 1.521–1.624 | 1.241 | 5.5 | 84.6 | 829 | 5 |
| essential64/slab | views | 0.005 | 3.787 | 3.607–3.900 | 3.611 | 5.5 | 48.2 | 486 | 5 |
| packed512 | materialized | 0.005 | 1.496 | 1.483–1.544 | 1.443 | 12.9 | 63.6 | 548 | 5 |

## product_free_join, 12 coordinates, bit-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 0.040 | 0.040–0.053 | 0.008 | 16.0 | 84.0 | 129 | 5 |
| essential512/enum | materialized | 0.005 | 0.255 | 0.248–0.264 | 0.008 | 5.6 | 72.3 | 329 | 5 |
| essential512/enum | views | 0.005 | 0.422 | 0.412–0.460 | 0.007 | 5.6 | 38.6 | 216 | 5 |
| essential512/slab | materialized | 0.005 | 0.263 | 0.256–0.386 | 0.008 | 3.2 | 49.2 | 329 | 5 |
| essential512/slab | views | 0.005 | 0.432 | 0.408–0.465 | 0.007 | 3.2 | 26.2 | 216 | 5 |
| essential64/enum | materialized | 0.005 | 0.538 | 0.527–0.582 | 0.008 | 9.6 | 132.6 | 829 | 5 |
| essential64/enum | views | 0.005 | 0.796 | 0.775–0.887 | 0.007 | 9.6 | 85.6 | 486 | 5 |
| essential64/slab | materialized | 0.005 | 0.569 | 0.539–0.715 | 0.009 | 5.5 | 92.0 | 829 | 5 |
| essential64/slab | views | 0.005 | 0.799 | 0.775–0.867 | 0.007 | 5.5 | 54.2 | 486 | 5 |
| packed512 | materialized | 0.005 | 0.229 | 0.213–0.236 | 0.010 | 12.9 | 70.9 | 548 | 5 |

## product_free_join, 12 coordinates, face-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 0.176 | 0.175–0.197 | 0.173 | 16.0 | 76.6 | 129 | 5 |
| essential512/enum | materialized | 0.005 | 1.193 | 1.161–1.277 | 1.013 | 5.6 | 88.6 | 456 | 5 |
| essential512/enum | views | 0.005 | 1.616 | 1.576–1.713 | 1.465 | 5.6 | 48.8 | 291 | 5 |
| essential512/slab | materialized | 0.005 | 1.185 | 1.158–1.227 | 1.019 | 3.2 | 49.4 | 456 | 5 |
| essential512/slab | views | 0.008 | 1.602 | 1.558–1.624 | 1.412 | 3.2 | 31.0 | 291 | 5 |
| essential64/enum | materialized | 0.005 | 1.355 | 1.309–1.604 | 1.057 | 15.8 | 102.0 | 851 | 5 |
| essential64/enum | views | 0.005 | 8.006 | 7.821–8.128 | 7.498 | 15.8 | 177.0 | 1018 | 5 |
| essential64/slab | materialized | 0.005 | 1.388 | 1.318–1.501 | 1.073 | 8.4 | 65.7 | 851 | 5 |
| essential64/slab | views | 0.005 | 8.734 | 8.200–8.803 | 7.532 | 8.4 | 107.2 | 1018 | 5 |
| packed512 | materialized | 0.005 | 4.562 | 4.462–4.718 | 4.569 | 7.7 | 190.8 | 990 | 5 |

## product_free_join, 12 coordinates, face-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 0.041 | 0.039–0.044 | 0.008 | 16.0 | 84.0 | 129 | 5 |
| essential512/enum | materialized | 0.005 | 0.340 | 0.331–0.439 | 0.008 | 5.6 | 96.0 | 456 | 5 |
| essential512/enum | views | 0.005 | 0.439 | 0.429–0.448 | 0.007 | 5.6 | 54.8 | 291 | 5 |
| essential512/slab | materialized | 0.005 | 0.347 | 0.331–0.360 | 0.010 | 3.2 | 56.8 | 456 | 5 |
| essential512/slab | views | 0.008 | 0.457 | 0.420–0.534 | 0.007 | 3.2 | 37.0 | 291 | 5 |
| essential64/enum | materialized | 0.005 | 0.481 | 0.455–0.502 | 0.009 | 15.8 | 109.4 | 851 | 5 |
| essential64/enum | views | 0.005 | 1.799 | 1.780–1.885 | 0.009 | 15.8 | 183.1 | 1018 | 5 |
| essential64/slab | materialized | 0.005 | 0.454 | 0.449–0.485 | 0.008 | 8.4 | 73.1 | 851 | 5 |
| essential64/slab | views | 0.005 | 1.856 | 1.799–1.936 | 0.007 | 8.4 | 113.3 | 1018 | 5 |
| packed512 | materialized | 0.005 | 0.822 | 0.796–0.827 | 0.009 | 7.7 | 198.2 | 990 | 5 |

## product_free_join, 12 coordinates, pair-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 0.184 | 0.178–0.203 | 0.175 | 16.0 | 76.6 | 129 | 5 |
| essential512/enum | materialized | 0.005 | 0.961 | 0.931–1.171 | 0.766 | 5.6 | 68.8 | 427 | 5 |
| essential512/enum | views | 0.005 | 1.951 | 1.912–2.003 | 1.832 | 5.6 | 46.4 | 270 | 5 |
| essential512/slab | materialized | 0.005 | 0.992 | 0.976–1.047 | 0.809 | 3.2 | 41.8 | 427 | 5 |
| essential512/slab | views | 0.005 | 1.891 | 1.861–2.114 | 1.772 | 3.2 | 31.0 | 270 | 5 |
| essential64/enum | materialized | 0.005 | 1.784 | 1.706–1.887 | 1.300 | 15.8 | 183.0 | 1034 | 5 |
| essential64/enum | views | 0.005 | 8.238 | 8.099–8.366 | 8.044 | 15.8 | 99.6 | 757 | 5 |
| essential64/slab | materialized | 0.005 | 1.813 | 1.768–1.847 | 1.320 | 8.4 | 115.9 | 1034 | 5 |
| essential64/slab | views | 0.005 | 8.448 | 8.303–8.612 | 8.228 | 8.4 | 63.8 | 757 | 5 |
| packed512 | materialized | 0.005 | 3.735 | 3.650–3.841 | 3.583 | 10.7 | 190.0 | 1086 | 5 |

## product_free_join, 12 coordinates, pair-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 0.042 | 0.039–0.050 | 0.009 | 16.0 | 84.0 | 129 | 5 |
| essential512/enum | materialized | 0.005 | 0.325 | 0.317–0.358 | 0.009 | 5.6 | 76.2 | 427 | 5 |
| essential512/enum | views | 0.005 | 0.444 | 0.437–0.451 | 0.009 | 5.6 | 52.5 | 270 | 5 |
| essential512/slab | materialized | 0.005 | 0.338 | 0.323–0.386 | 0.008 | 3.2 | 49.2 | 427 | 5 |
| essential512/slab | views | 0.005 | 0.431 | 0.428–0.512 | 0.008 | 3.2 | 37.0 | 270 | 5 |
| essential64/enum | materialized | 0.005 | 0.672 | 0.639–0.702 | 0.008 | 15.8 | 190.4 | 1034 | 5 |
| essential64/enum | views | 0.005 | 1.779 | 1.702–1.807 | 0.007 | 15.8 | 105.7 | 757 | 5 |
| essential64/slab | materialized | 0.005 | 0.657 | 0.650–1.105 | 0.008 | 8.4 | 123.3 | 1034 | 5 |
| essential64/slab | views | 0.005 | 1.819 | 1.772–2.002 | 0.007 | 8.4 | 69.9 | 757 | 5 |
| packed512 | materialized | 0.005 | 0.588 | 0.579–0.658 | 0.009 | 10.7 | 197.4 | 1086 | 5 |

## product_free_join, 18 coordinates, bit-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 8.964 | 8.861–9.260 | 8.994 | 919.3 | 4335.1 | 130 | 5 |
| essential512/enum | materialized | 0.005 | 7.504 | 7.370–7.683 | 5.790 | 28.1 | 583.9 | 3112 | 5 |
| essential512/enum | views | 0.005 | 63.034 | 62.584–67.606 | 61.564 | 28.1 | 311.2 | 1792 | 5 |
| essential512/slab | materialized | 0.005 | 7.443 | 7.172–7.481 | 5.915 | 15.3 | 356.7 | 3112 | 5 |
| essential512/slab | views | 0.005 | 62.155 | 61.471–62.816 | 61.037 | 15.3 | 231.5 | 1792 | 5 |
| essential64/enum | materialized | 0.006 | 10.370 | 10.276–10.636 | 8.299 | 62.7 | 826.7 | 5582 | 5 |
| essential64/enum | views | 0.005 | 34.870 | 34.700–35.538 | 33.239 | 62.7 | 408.4 | 3102 | 5 |
| essential64/slab | materialized | 0.005 | 10.324 | 10.237–10.500 | 8.268 | 33.3 | 543.6 | 5582 | 5 |
| essential64/slab | views | 0.005 | 34.777 | 34.719–35.100 | 33.708 | 33.3 | 264.4 | 3102 | 5 |
| packed512 | materialized | 0.005 | 3.350 | 3.220–3.523 | 2.979 | 65.3 | 676.8 | 5073 | 5 |

## product_free_join, 18 coordinates, bit-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 1.337 | 1.317–1.538 | 0.017 | 919.3 | 4342.5 | 130 | 5 |
| essential512/enum | materialized | 0.005 | 2.527 | 2.435–2.551 | 0.009 | 28.1 | 591.3 | 3112 | 5 |
| essential512/enum | views | 0.005 | 12.531 | 12.230–12.984 | 0.007 | 28.1 | 317.3 | 1792 | 5 |
| essential512/slab | materialized | 0.005 | 2.403 | 2.370–2.467 | 0.008 | 15.3 | 364.1 | 3112 | 5 |
| essential512/slab | views | 0.005 | 12.542 | 12.321–16.544 | 0.008 | 15.3 | 237.6 | 1792 | 5 |
| essential64/enum | materialized | 0.006 | 3.446 | 3.409–3.846 | 0.008 | 62.7 | 834.1 | 5582 | 5 |
| essential64/enum | views | 0.005 | 7.455 | 7.436–7.589 | 0.007 | 62.7 | 414.4 | 3102 | 5 |
| essential64/slab | materialized | 0.005 | 3.333 | 3.273–3.774 | 0.008 | 33.3 | 551.0 | 5582 | 5 |
| essential64/slab | views | 0.005 | 7.641 | 7.420–7.742 | 0.007 | 33.3 | 270.5 | 3102 | 5 |
| packed512 | materialized | 0.005 | 0.738 | 0.726–0.832 | 0.024 | 65.3 | 684.2 | 5073 | 5 |

## product_free_join, 18 coordinates, face-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 9.054 | 8.873–9.300 | 8.993 | 919.3 | 4335.1 | 130 | 5 |
| essential512/enum | materialized | 0.005 | 7.453 | 7.210–11.658 | 6.511 | 45.3 | 512.2 | 3227 | 5 |
| essential512/enum | views | 0.005 | 138.934 | 138.033–140.503 | 152.461 | 45.3 | 1011.5 | 5565 | 5 |
| essential512/slab | materialized | 0.005 | 7.099 | 7.024–7.624 | 5.920 | 23.7 | 360.3 | 3227 | 5 |
| essential512/slab | views | 0.005 | 138.478 | 137.401–139.307 | 134.251 | 23.7 | 735.4 | 5565 | 5 |
| essential64/enum | materialized | 0.005 | 6.299 | 6.105–6.721 | 3.772 | 126.2 | 1087.0 | 7382 | 5 |
| essential64/enum | views | 0.005 | 108.687 | 107.955–109.897 | 105.917 | 126.2 | 1537.1 | 9887 | 5 |
| essential64/slab | materialized | 0.005 | 6.299 | 6.170–6.498 | 3.825 | 67.9 | 639.0 | 7382 | 5 |
| essential64/slab | views | 0.005 | 113.240 | 112.020–129.328 | 109.435 | 67.9 | 979.9 | 9887 | 5 |
| packed512 | materialized | 0.005 | 12.969 | 12.609–23.764 | 12.317 | 40.0 | 994.3 | 6218 | 5 |

## product_free_join, 18 coordinates, face-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 1.331 | 1.313–1.466 | 0.016 | 919.3 | 4342.5 | 130 | 5 |
| essential512/enum | materialized | 0.005 | 2.290 | 2.196–2.384 | 0.009 | 45.3 | 519.6 | 3227 | 5 |
| essential512/enum | views | 0.005 | 29.447 | 28.674–29.745 | 0.019 | 45.3 | 1017.5 | 5565 | 5 |
| essential512/slab | materialized | 0.005 | 2.227 | 2.165–2.269 | 0.008 | 23.7 | 367.7 | 3227 | 5 |
| essential512/slab | views | 0.005 | 28.120 | 28.036–28.906 | 0.007 | 23.7 | 741.4 | 5565 | 5 |
| essential64/enum | materialized | 0.005 | 3.034 | 2.913–3.084 | 0.009 | 126.2 | 1094.4 | 7382 | 5 |
| essential64/enum | views | 0.005 | 23.974 | 22.734–24.337 | 0.007 | 126.2 | 1543.2 | 9887 | 5 |
| essential64/slab | materialized | 0.005 | 3.014 | 2.891–10.848 | 0.009 | 67.9 | 646.4 | 7382 | 5 |
| essential64/slab | views | 0.005 | 23.489 | 23.316–23.790 | 0.008 | 67.9 | 986.0 | 9887 | 5 |
| packed512 | materialized | 0.005 | 2.192 | 2.132–2.203 | 0.016 | 40.0 | 1001.7 | 6218 | 5 |

## product_free_join, 18 coordinates, pair-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 8.788 | 8.601–8.985 | 8.835 | 919.3 | 4335.1 | 130 | 5 |
| essential512/enum | materialized | 0.005 | 7.670 | 7.545–8.183 | 6.143 | 28.9 | 639.7 | 3479 | 5 |
| essential512/enum | views | 0.005 | 63.308 | 62.938–64.103 | 62.599 | 28.9 | 418.4 | 2378 | 5 |
| essential512/slab | materialized | 0.005 | 7.759 | 7.523–7.968 | 6.282 | 15.9 | 405.1 | 3479 | 5 |
| essential512/slab | views | 0.005 | 64.385 | 63.800–65.222 | 62.604 | 15.9 | 270.9 | 2378 | 5 |
| essential64/enum | materialized | 0.005 | 10.565 | 10.468–11.089 | 8.217 | 65.7 | 872.8 | 6490 | 5 |
| essential64/enum | views | 0.005 | 59.652 | 58.716–61.784 | 57.518 | 65.7 | 581.2 | 4275 | 5 |
| essential64/slab | materialized | 0.005 | 10.797 | 10.546–10.930 | 8.216 | 35.2 | 574.6 | 6490 | 5 |
| essential64/slab | views | 0.005 | 61.339 | 61.220–75.498 | 58.590 | 35.2 | 340.9 | 4275 | 5 |
| packed512 | materialized | 0.005 | 15.955 | 15.774–16.366 | 15.245 | 50.7 | 965.8 | 6500 | 5 |

## product_free_join, 18 coordinates, pair-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 1.314 | 1.304–1.361 | 0.017 | 919.3 | 4342.5 | 130 | 5 |
| essential512/enum | materialized | 0.005 | 2.615 | 2.592–2.792 | 0.009 | 28.9 | 647.0 | 3479 | 5 |
| essential512/enum | views | 0.005 | 12.679 | 12.385–15.689 | 0.007 | 28.9 | 424.5 | 2378 | 5 |
| essential512/slab | materialized | 0.005 | 2.806 | 2.587–2.870 | 0.008 | 15.9 | 412.5 | 3479 | 5 |
| essential512/slab | views | 0.005 | 12.881 | 12.761–13.591 | 0.007 | 15.9 | 277.0 | 2378 | 5 |
| essential64/enum | materialized | 0.005 | 3.781 | 3.616–4.012 | 0.008 | 65.7 | 880.2 | 6490 | 5 |
| essential64/enum | views | 0.005 | 12.290 | 12.130–12.639 | 0.007 | 65.7 | 587.2 | 4275 | 5 |
| essential64/slab | materialized | 0.005 | 3.832 | 3.711–4.057 | 0.008 | 35.2 | 582.0 | 6490 | 5 |
| essential64/slab | views | 0.005 | 12.729 | 12.615–12.958 | 0.007 | 35.2 | 347.0 | 4275 | 5 |
| packed512 | materialized | 0.005 | 2.647 | 2.559–2.693 | 0.022 | 50.7 | 973.2 | 6500 | 5 |

## relations_free_join, 12 coordinates, bit-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 0.150 | 0.146–0.169 | 0.140 | 16.0 | 82.5 | 140 | 5 |
| essential512/enum | materialized | 0.005 | 0.472 | 0.456–0.484 | 0.227 | 5.6 | 66.9 | 345 | 5 |
| essential512/enum | views | 0.005 | 0.709 | 0.692–0.723 | 0.603 | 5.6 | 22.7 | 130 | 5 |
| essential512/slab | materialized | 0.005 | 0.453 | 0.427–0.487 | 0.229 | 3.2 | 43.7 | 345 | 5 |
| essential512/slab | views | 0.005 | 0.716 | 0.690–0.749 | 0.640 | 3.2 | 13.1 | 130 | 5 |
| essential64/enum | materialized | 0.005 | 1.085 | 0.992–1.130 | 0.597 | 9.6 | 167.9 | 1159 | 5 |
| essential64/enum | views | 0.005 | 0.886 | 0.855–0.926 | 0.730 | 9.6 | 79.5 | 495 | 5 |
| essential64/slab | materialized | 0.005 | 1.030 | 0.963–1.242 | 0.594 | 5.5 | 106.1 | 1159 | 5 |
| essential64/slab | views | 0.005 | 0.929 | 0.871–0.989 | 0.729 | 5.5 | 46.7 | 495 | 5 |
| packed512 | materialized | 0.005 | 0.815 | 0.796–0.840 | 0.766 | 12.9 | 74.3 | 482 | 5 |

## relations_free_join, 12 coordinates, bit-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 0.061 | 0.057–0.066 | 0.010 | 16.0 | 95.5 | 140 | 5 |
| essential512/enum | materialized | 0.005 | 0.332 | 0.322–0.362 | 0.010 | 5.6 | 79.9 | 345 | 5 |
| essential512/enum | views | 0.005 | 0.384 | 0.369–0.403 | 0.008 | 5.6 | 34.5 | 130 | 5 |
| essential512/slab | materialized | 0.005 | 0.349 | 0.308–0.363 | 0.009 | 3.2 | 56.7 | 345 | 5 |
| essential512/slab | views | 0.005 | 0.419 | 0.376–0.426 | 0.008 | 3.2 | 24.9 | 130 | 5 |
| essential64/enum | materialized | 0.005 | 0.654 | 0.644–0.700 | 0.011 | 9.6 | 180.9 | 1159 | 5 |
| essential64/enum | views | 0.005 | 0.530 | 0.503–0.643 | 0.010 | 9.6 | 91.2 | 495 | 5 |
| essential64/slab | materialized | 0.005 | 0.735 | 0.682–0.745 | 0.009 | 5.5 | 119.1 | 1159 | 5 |
| essential64/slab | views | 0.005 | 0.508 | 0.489–0.513 | 0.008 | 5.5 | 58.5 | 495 | 5 |
| packed512 | materialized | 0.005 | 0.274 | 0.251–0.376 | 0.017 | 12.9 | 87.3 | 482 | 5 |

## relations_free_join, 12 coordinates, face-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 0.143 | 0.141–0.159 | 0.139 | 16.0 | 82.5 | 140 | 5 |
| essential512/enum | materialized | 0.005 | 0.763 | 0.728–0.924 | 0.440 | 5.6 | 99.1 | 510 | 5 |
| essential512/enum | views | 0.005 | 0.717 | 0.674–0.815 | 0.550 | 5.6 | 47.5 | 251 | 5 |
| essential512/slab | materialized | 0.005 | 0.735 | 0.703–0.894 | 0.455 | 3.2 | 65.4 | 510 | 5 |
| essential512/slab | views | 0.006 | 0.674 | 0.662–0.784 | 0.533 | 3.2 | 31.0 | 251 | 5 |
| essential64/enum | materialized | 0.005 | 1.460 | 1.450–1.722 | 0.679 | 15.8 | 210.8 | 1342 | 5 |
| essential64/enum | views | 0.005 | 3.400 | 3.364–3.618 | 2.896 | 15.8 | 196.9 | 1499 | 5 |
| essential64/slab | materialized | 0.013 | 1.499 | 1.461–4.013 | 0.716 | 8.4 | 138.8 | 1342 | 5 |
| essential64/slab | views | 0.005 | 3.549 | 3.443–3.638 | 2.902 | 8.4 | 127.3 | 1499 | 5 |
| packed512 | materialized | 0.005 | 1.412 | 1.362–1.449 | 1.285 | 7.7 | 291.1 | 1326 | 5 |

## relations_free_join, 12 coordinates, face-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 0.056 | 0.054–0.060 | 0.010 | 16.0 | 95.5 | 140 | 5 |
| essential512/enum | materialized | 0.005 | 0.500 | 0.469–0.511 | 0.010 | 5.6 | 112.1 | 510 | 5 |
| essential512/enum | views | 0.005 | 0.433 | 0.417–0.660 | 0.008 | 5.6 | 59.2 | 251 | 5 |
| essential512/slab | materialized | 0.005 | 0.513 | 0.473–0.535 | 0.010 | 3.2 | 78.4 | 510 | 5 |
| essential512/slab | views | 0.006 | 0.443 | 0.422–0.459 | 0.009 | 3.2 | 42.7 | 251 | 5 |
| essential64/enum | materialized | 0.005 | 1.102 | 1.061–1.157 | 0.009 | 15.8 | 223.8 | 1342 | 5 |
| essential64/enum | views | 0.005 | 2.007 | 1.927–2.113 | 0.010 | 15.8 | 208.7 | 1499 | 5 |
| essential64/slab | materialized | 0.013 | 1.124 | 1.103–1.346 | 0.010 | 8.4 | 151.8 | 1342 | 5 |
| essential64/slab | views | 0.005 | 2.030 | 1.970–2.083 | 0.008 | 8.4 | 139.1 | 1499 | 5 |
| packed512 | materialized | 0.005 | 0.521 | 0.501–0.546 | 0.012 | 7.7 | 304.1 | 1326 | 5 |

## relations_free_join, 12 coordinates, pair-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 0.147 | 0.147–0.171 | 0.139 | 16.0 | 82.5 | 140 | 5 |
| essential512/enum | materialized | 0.005 | 0.616 | 0.570–0.767 | 0.329 | 5.6 | 75.4 | 430 | 5 |
| essential512/enum | views | 0.005 | 0.753 | 0.720–0.765 | 0.620 | 5.6 | 34.3 | 189 | 5 |
| essential512/slab | materialized | 0.005 | 0.556 | 0.534–0.590 | 0.308 | 3.2 | 47.5 | 430 | 5 |
| essential512/slab | views | 0.005 | 0.777 | 0.710–0.811 | 0.634 | 3.2 | 22.0 | 189 | 5 |
| essential64/enum | materialized | 0.005 | 1.430 | 1.282–1.494 | 0.692 | 15.8 | 211.6 | 1392 | 5 |
| essential64/enum | views | 0.005 | 2.813 | 2.717–2.885 | 2.515 | 15.8 | 98.3 | 714 | 5 |
| essential64/slab | materialized | 0.006 | 1.350 | 1.311–1.439 | 0.668 | 8.4 | 138.8 | 1392 | 5 |
| essential64/slab | views | 0.005 | 2.915 | 2.790–2.958 | 2.645 | 8.4 | 62.4 | 714 | 5 |
| packed512 | materialized | 0.005 | 1.379 | 1.330–1.619 | 1.209 | 10.7 | 190.0 | 1107 | 5 |

## relations_free_join, 12 coordinates, pair-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 0.056 | 0.053–0.064 | 0.010 | 16.0 | 95.5 | 140 | 5 |
| essential512/enum | materialized | 0.005 | 0.425 | 0.417–0.448 | 0.011 | 5.6 | 88.4 | 430 | 5 |
| essential512/enum | views | 0.005 | 0.414 | 0.406–0.480 | 0.008 | 5.6 | 46.0 | 189 | 5 |
| essential512/slab | materialized | 0.005 | 0.391 | 0.368–0.408 | 0.009 | 3.2 | 60.5 | 430 | 5 |
| essential512/slab | views | 0.005 | 0.414 | 0.405–0.429 | 0.009 | 3.2 | 33.8 | 189 | 5 |
| essential64/enum | materialized | 0.005 | 1.016 | 0.953–1.038 | 0.009 | 15.8 | 224.6 | 1392 | 5 |
| essential64/enum | views | 0.005 | 1.577 | 1.493–1.632 | 0.008 | 15.8 | 110.1 | 714 | 5 |
| essential64/slab | materialized | 0.006 | 0.982 | 0.937–1.033 | 0.009 | 8.4 | 151.8 | 1392 | 5 |
| essential64/slab | views | 0.005 | 1.606 | 1.531–10.703 | 0.009 | 8.4 | 74.2 | 714 | 5 |
| packed512 | materialized | 0.005 | 0.482 | 0.466–0.498 | 0.014 | 10.7 | 203.0 | 1107 | 5 |

## relations_free_join, 18 coordinates, bit-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 7.014 | 6.871–7.098 | 6.958 | 919.3 | 5810.7 | 175 | 5 |
| essential512/enum | materialized | 0.005 | 3.384 | 3.132–3.493 | 1.612 | 28.1 | 450.5 | 2668 | 5 |
| essential512/enum | views | 0.006 | 15.125 | 14.697–15.483 | 14.510 | 28.1 | 195.5 | 1011 | 5 |
| essential512/slab | materialized | 0.005 | 3.263 | 3.207–3.419 | 1.672 | 15.3 | 288.6 | 2668 | 5 |
| essential512/slab | views | 0.005 | 15.233 | 14.819–18.915 | 14.293 | 15.3 | 131.2 | 1011 | 5 |
| essential64/enum | materialized | 0.005 | 4.978 | 4.647–5.111 | 2.635 | 62.7 | 879.5 | 5699 | 5 |
| essential64/enum | views | 0.005 | 7.219 | 6.997–7.417 | 6.100 | 62.7 | 415.1 | 2640 | 5 |
| essential64/slab | materialized | 0.006 | 4.924 | 4.729–4.953 | 2.743 | 33.3 | 595.7 | 5699 | 5 |
| essential64/slab | views | 0.005 | 7.044 | 6.838–7.175 | 6.023 | 33.3 | 275.3 | 2640 | 5 |
| packed512 | materialized | 0.005 | 1.543 | 1.465–1.623 | 1.154 | 65.3 | 499.4 | 3815 | 5 |

## relations_free_join, 18 coordinates, bit-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 2.446 | 2.374–2.677 | 0.049 | 919.3 | 5823.7 | 175 | 5 |
| essential512/enum | materialized | 0.005 | 2.356 | 2.330–2.395 | 0.010 | 28.1 | 463.5 | 2668 | 5 |
| essential512/enum | views | 0.006 | 7.826 | 7.620–8.040 | 0.008 | 28.1 | 207.2 | 1011 | 5 |
| essential512/slab | materialized | 0.005 | 2.438 | 2.401–3.194 | 0.010 | 15.3 | 301.6 | 2668 | 5 |
| essential512/slab | views | 0.005 | 7.794 | 7.743–7.938 | 0.009 | 15.3 | 143.0 | 1011 | 5 |
| essential64/enum | materialized | 0.005 | 3.372 | 3.285–3.892 | 0.010 | 62.7 | 892.5 | 5699 | 5 |
| essential64/enum | views | 0.005 | 3.997 | 3.765–4.032 | 0.008 | 62.7 | 426.9 | 2640 | 5 |
| essential64/slab | materialized | 0.006 | 3.439 | 3.375–3.739 | 0.010 | 33.3 | 608.7 | 5699 | 5 |
| essential64/slab | views | 0.005 | 3.882 | 3.773–3.983 | 0.009 | 33.3 | 287.0 | 2640 | 5 |
| packed512 | materialized | 0.005 | 0.746 | 0.711–0.805 | 0.039 | 65.3 | 512.4 | 3815 | 5 |

## relations_free_join, 18 coordinates, face-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 7.079 | 6.785–7.341 | 6.953 | 919.3 | 5810.7 | 175 | 5 |
| essential512/enum | materialized | 0.005 | 10.209 | 9.859–10.739 | 5.931 | 45.3 | 1043.6 | 5273 | 5 |
| essential512/enum | views | 0.005 | 159.384 | 159.227–160.959 | 154.174 | 45.3 | 1722.5 | 8088 | 5 |
| essential512/slab | materialized | 0.005 | 10.174 | 9.833–10.440 | 5.989 | 23.7 | 655.1 | 5273 | 5 |
| essential512/slab | views | 0.005 | 171.856 | 169.609–173.718 | 169.002 | 23.7 | 1096.4 | 8088 | 5 |
| essential64/enum | materialized | 0.005 | 26.476 | 26.018–27.372 | 16.892 | 126.2 | 1866.6 | 9513 | 5 |
| essential64/enum | views | 0.006 | 42.183 | 41.690–42.598 | 36.768 | 126.2 | 2402.2 | 14862 | 5 |
| essential64/slab | materialized | 0.005 | 26.566 | 25.669–48.048 | 16.445 | 67.9 | 1308.0 | 9513 | 5 |
| essential64/slab | views | 0.005 | 44.810 | 44.250–72.638 | 38.077 | 67.9 | 1500.5 | 14862 | 5 |
| packed512 | materialized | 0.005 | 5.368 | 5.334–5.615 | 5.047 | 40.0 | 1200.6 | 7882 | 5 |

## relations_free_join, 18 coordinates, face-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 2.511 | 2.458–2.556 | 0.048 | 919.3 | 5823.7 | 175 | 5 |
| essential512/enum | materialized | 0.005 | 7.124 | 6.926–7.323 | 0.010 | 45.3 | 1056.6 | 5273 | 5 |
| essential512/enum | views | 0.005 | 80.624 | 79.972–81.939 | 0.008 | 45.3 | 1734.3 | 8088 | 5 |
| essential512/slab | materialized | 0.005 | 7.115 | 7.015–7.480 | 0.009 | 23.7 | 668.1 | 5273 | 5 |
| essential512/slab | views | 0.005 | 90.077 | 89.769–91.016 | 0.010 | 23.7 | 1108.2 | 8088 | 5 |
| essential64/enum | materialized | 0.005 | 17.544 | 17.436–19.136 | 0.010 | 126.2 | 1879.6 | 9513 | 5 |
| essential64/enum | views | 0.006 | 24.403 | 23.735–25.168 | 0.008 | 126.2 | 2414.0 | 14862 | 5 |
| essential64/slab | materialized | 0.005 | 17.999 | 17.744–19.437 | 0.010 | 67.9 | 1321.0 | 9513 | 5 |
| essential64/slab | views | 0.005 | 25.149 | 24.817–25.895 | 0.009 | 67.9 | 1512.3 | 14862 | 5 |
| packed512 | materialized | 0.005 | 2.399 | 2.291–2.438 | 0.031 | 40.0 | 1213.6 | 7882 | 5 |

## relations_free_join, 18 coordinates, pair-major, memo off

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 7.134 | 6.982–7.540 | 7.270 | 919.3 | 5810.7 | 175 | 5 |
| essential512/enum | materialized | 0.005 | 4.323 | 4.163–6.344 | 2.203 | 28.9 | 543.8 | 3223 | 5 |
| essential512/enum | views | 0.005 | 16.226 | 15.951–16.593 | 15.279 | 28.9 | 235.1 | 1496 | 5 |
| essential512/slab | materialized | 0.005 | 4.066 | 3.909–8.817 | 2.144 | 15.9 | 358.5 | 3223 | 5 |
| essential512/slab | views | 0.005 | 16.051 | 15.928–16.440 | 15.153 | 15.9 | 151.0 | 1496 | 5 |
| essential64/enum | materialized | 0.005 | 6.609 | 6.486–6.976 | 3.388 | 65.7 | 927.0 | 6713 | 5 |
| essential64/enum | views | 0.005 | 18.079 | 17.670–18.388 | 15.946 | 65.7 | 588.5 | 3722 | 5 |
| essential64/slab | materialized | 0.005 | 6.459 | 6.392–6.750 | 3.501 | 35.2 | 631.4 | 6713 | 5 |
| essential64/slab | views | 0.005 | 18.274 | 18.186–18.748 | 16.514 | 35.2 | 354.0 | 3722 | 5 |
| packed512 | materialized | 0.005 | 4.021 | 3.899–4.087 | 3.473 | 50.7 | 851.2 | 5840 | 5 |

## relations_free_join, 18 coordinates, pair-major, memo on

| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| dense-dispatched | materialized | 0.005 | 2.428 | 2.334–2.477 | 0.051 | 919.3 | 5823.7 | 175 | 5 |
| essential512/enum | materialized | 0.005 | 3.021 | 2.907–3.088 | 0.010 | 28.9 | 556.8 | 3223 | 5 |
| essential512/enum | views | 0.005 | 8.464 | 8.209–8.627 | 0.009 | 28.9 | 246.9 | 1496 | 5 |
| essential512/slab | materialized | 0.005 | 2.906 | 2.834–3.069 | 0.010 | 15.9 | 371.5 | 3223 | 5 |
| essential512/slab | views | 0.005 | 8.384 | 8.272–8.460 | 0.009 | 15.9 | 162.7 | 1496 | 5 |
| essential64/enum | materialized | 0.005 | 4.738 | 4.528–5.039 | 0.010 | 65.7 | 940.0 | 6713 | 5 |
| essential64/enum | views | 0.005 | 9.599 | 9.391–9.927 | 0.009 | 65.7 | 600.3 | 3722 | 5 |
| essential64/slab | materialized | 0.005 | 4.677 | 4.564–4.822 | 0.009 | 35.2 | 644.3 | 6713 | 5 |
| essential64/slab | views | 0.005 | 9.827 | 9.609–10.027 | 0.008 | 35.2 | 365.8 | 3722 | 5 |
| packed512 | materialized | 0.005 | 1.989 | 1.955–2.116 | 0.037 | 50.7 | 864.1 | 5840 | 5 |

## Evidence

240 distinct cases, 96 matched strategy pairs, 78 retained processes.

- [Raw evidence](results/view-native-smoke.json).
- [Raw evidence](results/view-initial-sweep.json).
