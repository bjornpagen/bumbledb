# Source normalization and bounded plane reuse through Free Join

Same executable and output contracts. Source normalization reduces pinned operands
in the existing canonical arena before product memo lookup. Bounded reuse caches
local aligned planes within one product; recomputation remains the control.
Reuse pairs must retain exactly identical final nodes, caches and arena bytes.
Resident KB estimates omit the bounded temporary plane cache, other temporary
workspace and allocator overhead; see [the contract and bounds](REUSE.md).
Times are milliseconds. Query includes actual Free Join, canonical construction
and exact count. Input construction, cloning, setup and bitset validation are outside.
The 16-output product lane and 80-output relation program stay separate.
Executable: `82b65160194612f6cbafee6a2c7bf414ba85fa2a89fabd56c1ff16c5d140dd2c`.

## product_free_join, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.180 | 0.178–0.203 | 0.181 | 76.6 | 129 | 9 |
| essential512/enum | materialized | deferred | off | 0.781 | 0.717–0.808 | 0.598 | 64.9 | 329 | 5 |
| essential512/enum | views | deferred | bounded | 1.664 | 1.616–1.791 | 1.608 | 32.5 | 216 | 5 |
| essential512/enum | views | deferred | off | 1.994 | 1.946–2.065 | 1.934 | 32.5 | 216 | 5 |
| essential512/enum | views | source | bounded | 0.847 | 0.831–0.925 | 0.754 | 44.7 | 261 | 5 |
| essential512/enum | views | source | off | 0.760 | 0.753–0.858 | 0.683 | 44.7 | 261 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 1.826 | 1.782–2.103 | 1.696 | 42.0 | 230 | 5 |
| essential512/enum | views-inputs | deferred | off | 2.036 | 1.972–2.117 | 1.968 | 42.0 | 230 | 5 |
| essential512/enum | views-inputs | source | bounded | 0.902 | 0.849–0.986 | 0.803 | 45.0 | 277 | 5 |
| essential512/enum | views-inputs | source | off | 0.799 | 0.769–0.834 | 0.730 | 45.0 | 277 | 5 |
| essential512/slab | materialized | deferred | off | 0.754 | 0.738–0.827 | 0.616 | 41.8 | 329 | 9 |
| essential512/slab | views | deferred | bounded | 1.672 | 1.600–1.815 | 1.603 | 20.1 | 216 | 9 |
| essential512/slab | views | deferred | off | 2.026 | 1.903–2.069 | 1.958 | 20.1 | 216 | 9 |
| essential512/slab | views | source | bounded | 0.848 | 0.805–0.900 | 0.740 | 25.8 | 261 | 9 |
| essential512/slab | views | source | off | 0.779 | 0.768–0.837 | 0.677 | 25.8 | 261 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 1.758 | 1.664–1.794 | 1.634 | 23.9 | 230 | 9 |
| essential512/slab | views-inputs | deferred | off | 2.082 | 2.032–2.128 | 1.980 | 23.9 | 230 | 9 |
| essential512/slab | views-inputs | source | bounded | 0.889 | 0.863–0.924 | 0.778 | 25.8 | 277 | 9 |
| essential512/slab | views-inputs | source | off | 0.815 | 0.782–0.889 | 0.705 | 25.8 | 277 | 9 |
| essential64/enum | materialized | deferred | off | 1.488 | 1.473–1.609 | 1.140 | 125.2 | 829 | 5 |
| essential64/enum | views | deferred | bounded | 3.047 | 2.938–3.081 | 3.030 | 79.6 | 486 | 5 |
| essential64/enum | views | deferred | off | 3.844 | 3.771–4.025 | 3.747 | 79.6 | 486 | 5 |
| essential64/enum | views | source | bounded | 1.649 | 1.590–1.727 | 1.386 | 86.2 | 549 | 5 |
| essential64/enum | views | source | off | 1.601 | 1.553–1.726 | 1.470 | 86.2 | 549 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 3.060 | 3.038–3.219 | 2.855 | 80.3 | 616 | 5 |
| essential64/enum | views-inputs | deferred | off | 4.077 | 3.875–10.728 | 3.585 | 80.3 | 616 | 5 |
| essential64/enum | views-inputs | source | bounded | 1.762 | 1.681–1.850 | 1.432 | 87.3 | 696 | 5 |
| essential64/enum | views-inputs | source | off | 1.755 | 1.700–1.789 | 1.481 | 87.3 | 696 | 5 |
| essential64/slab | materialized | deferred | off | 1.471 | 1.451–1.537 | 1.140 | 84.6 | 829 | 5 |
| essential64/slab | views | deferred | bounded | 3.045 | 3.006–3.091 | 2.874 | 48.2 | 486 | 5 |
| essential64/slab | views | deferred | off | 3.726 | 3.710–3.929 | 3.599 | 48.2 | 486 | 5 |
| essential64/slab | views | source | bounded | 1.600 | 1.577–1.691 | 1.430 | 53.9 | 549 | 5 |
| essential64/slab | views | source | off | 1.678 | 1.600–1.759 | 1.527 | 53.9 | 549 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 3.226 | 3.176–3.281 | 3.034 | 48.2 | 616 | 5 |
| essential64/slab | views-inputs | deferred | off | 3.862 | 3.750–3.943 | 3.712 | 48.2 | 616 | 5 |
| essential64/slab | views-inputs | source | bounded | 1.801 | 1.722–4.855 | 2.720 | 57.0 | 696 | 5 |
| essential64/slab | views-inputs | source | off | 1.782 | 1.715–1.824 | 1.599 | 57.0 | 696 | 5 |
| packed512/enum | materialized | deferred | off | 1.503 | 1.458–1.561 | 1.427 | 63.6 | 548 | 9 |

## product_free_join, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.040 | 0.038–0.077 | 0.009 | 84.0 | 129 | 9 |
| essential512/enum | materialized | deferred | off | 0.245 | 0.243–0.276 | 0.008 | 72.3 | 329 | 5 |
| essential512/enum | views | deferred | bounded | 0.386 | 0.353–0.401 | 0.007 | 38.6 | 216 | 5 |
| essential512/enum | views | deferred | off | 0.454 | 0.433–0.474 | 0.008 | 38.6 | 216 | 5 |
| essential512/enum | views | source | bounded | 0.238 | 0.233–0.248 | 0.007 | 50.7 | 261 | 5 |
| essential512/enum | views | source | off | 0.228 | 0.226–0.253 | 0.007 | 50.7 | 261 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 0.420 | 0.395–0.468 | 0.007 | 49.0 | 230 | 5 |
| essential512/enum | views-inputs | deferred | off | 0.458 | 0.443–0.506 | 0.007 | 49.0 | 230 | 5 |
| essential512/enum | views-inputs | source | bounded | 0.258 | 0.244–0.262 | 0.008 | 52.0 | 277 | 5 |
| essential512/enum | views-inputs | source | off | 0.235 | 0.227–0.274 | 0.007 | 52.0 | 277 | 5 |
| essential512/slab | materialized | deferred | off | 0.273 | 0.248–0.284 | 0.009 | 49.2 | 329 | 9 |
| essential512/slab | views | deferred | bounded | 0.358 | 0.350–0.404 | 0.007 | 26.2 | 216 | 9 |
| essential512/slab | views | deferred | off | 0.427 | 0.408–0.484 | 0.007 | 26.2 | 216 | 9 |
| essential512/slab | views | source | bounded | 0.232 | 0.228–0.254 | 0.007 | 31.9 | 261 | 9 |
| essential512/slab | views | source | off | 0.222 | 0.220–0.433 | 0.007 | 31.9 | 261 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 0.385 | 0.369–0.415 | 0.007 | 30.9 | 230 | 9 |
| essential512/slab | views-inputs | deferred | off | 0.457 | 0.432–0.470 | 0.007 | 30.9 | 230 | 9 |
| essential512/slab | views-inputs | source | bounded | 0.240 | 0.238–0.258 | 0.007 | 32.8 | 277 | 9 |
| essential512/slab | views-inputs | source | off | 0.233 | 0.227–0.259 | 0.007 | 32.8 | 277 | 9 |
| essential64/enum | materialized | deferred | off | 0.518 | 0.515–0.562 | 0.008 | 132.6 | 829 | 5 |
| essential64/enum | views | deferred | bounded | 0.702 | 0.677–0.751 | 0.007 | 85.6 | 486 | 5 |
| essential64/enum | views | deferred | off | 0.804 | 0.798–0.851 | 0.007 | 85.6 | 486 | 5 |
| essential64/enum | views | source | bounded | 0.433 | 0.416–0.450 | 0.007 | 92.3 | 549 | 5 |
| essential64/enum | views | source | off | 0.441 | 0.423–0.469 | 0.007 | 92.3 | 549 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 0.740 | 0.724–0.797 | 0.007 | 87.3 | 616 | 5 |
| essential64/enum | views-inputs | deferred | off | 0.843 | 0.830–0.879 | 0.007 | 87.3 | 616 | 5 |
| essential64/enum | views-inputs | source | bounded | 0.524 | 0.489–0.568 | 0.007 | 94.3 | 696 | 5 |
| essential64/enum | views-inputs | source | off | 0.479 | 0.476–0.525 | 0.007 | 94.3 | 696 | 5 |
| essential64/slab | materialized | deferred | off | 0.549 | 0.510–0.597 | 0.008 | 92.0 | 829 | 5 |
| essential64/slab | views | deferred | bounded | 0.658 | 0.656–0.678 | 0.007 | 54.2 | 486 | 5 |
| essential64/slab | views | deferred | off | 0.805 | 0.792–0.862 | 0.009 | 54.2 | 486 | 5 |
| essential64/slab | views | source | bounded | 0.447 | 0.436–0.504 | 0.007 | 59.9 | 549 | 5 |
| essential64/slab | views | source | off | 0.449 | 0.429–0.483 | 0.007 | 59.9 | 549 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 0.762 | 0.739–0.823 | 0.007 | 55.1 | 616 | 5 |
| essential64/slab | views-inputs | deferred | off | 0.884 | 0.844–0.887 | 0.008 | 55.1 | 616 | 5 |
| essential64/slab | views-inputs | source | bounded | 0.528 | 0.499–2.918 | 0.008 | 64.0 | 696 | 5 |
| essential64/slab | views-inputs | source | off | 0.495 | 0.479–0.506 | 0.007 | 64.0 | 696 | 5 |
| packed512/enum | materialized | deferred | off | 0.222 | 0.212–0.272 | 0.011 | 70.9 | 548 | 9 |

## product_free_join, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.179 | 0.178–0.200 | 0.181 | 76.6 | 129 | 9 |
| essential512/enum | materialized | deferred | off | 1.197 | 1.149–1.292 | 1.004 | 88.6 | 456 | 5 |
| essential512/enum | views | deferred | bounded | 1.670 | 1.600–1.785 | 1.459 | 48.8 | 291 | 5 |
| essential512/enum | views | deferred | off | 1.640 | 1.559–1.700 | 1.421 | 48.8 | 291 | 5 |
| essential512/enum | views | source | bounded | 0.891 | 0.879–0.955 | 0.720 | 67.3 | 389 | 5 |
| essential512/enum | views | source | off | 0.858 | 0.843–0.907 | 0.688 | 67.3 | 389 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 1.783 | 1.726–1.830 | 1.726 | 41.6 | 292 | 5 |
| essential512/enum | views-inputs | deferred | off | 1.993 | 1.922–2.018 | 1.894 | 41.6 | 292 | 5 |
| essential512/enum | views-inputs | source | bounded | 1.215 | 1.190–1.248 | 1.172 | 57.3 | 350 | 5 |
| essential512/enum | views-inputs | source | off | 1.254 | 1.231–1.293 | 1.140 | 57.3 | 350 | 5 |
| essential512/slab | materialized | deferred | off | 1.222 | 1.144–1.359 | 1.005 | 49.4 | 456 | 9 |
| essential512/slab | views | deferred | bounded | 1.621 | 1.601–1.790 | 1.467 | 31.0 | 291 | 9 |
| essential512/slab | views | deferred | off | 1.595 | 1.578–1.723 | 1.493 | 31.0 | 291 | 9 |
| essential512/slab | views | source | bounded | 0.918 | 0.887–0.978 | 0.741 | 43.7 | 389 | 9 |
| essential512/slab | views | source | off | 0.877 | 0.839–0.948 | 0.691 | 43.7 | 389 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 1.806 | 1.739–1.878 | 1.741 | 23.0 | 292 | 9 |
| essential512/slab | views-inputs | deferred | off | 1.931 | 1.886–1.981 | 1.924 | 23.0 | 292 | 9 |
| essential512/slab | views-inputs | source | bounded | 1.262 | 1.202–1.313 | 1.149 | 33.8 | 350 | 9 |
| essential512/slab | views-inputs | source | off | 1.246 | 1.191–1.313 | 1.143 | 33.8 | 350 | 9 |
| essential64/enum | materialized | deferred | off | 1.308 | 1.285–1.391 | 1.027 | 102.0 | 851 | 5 |
| essential64/enum | views | deferred | bounded | 6.398 | 6.334–6.501 | 5.994 | 177.0 | 1018 | 5 |
| essential64/enum | views | deferred | off | 8.303 | 8.201–8.459 | 7.680 | 177.0 | 1018 | 5 |
| essential64/enum | views | source | bounded | 1.790 | 1.739–1.954 | 1.266 | 190.0 | 1123 | 5 |
| essential64/enum | views | source | off | 1.887 | 1.802–1.956 | 1.368 | 190.0 | 1123 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 4.908 | 4.755–4.969 | 4.672 | 98.5 | 716 | 5 |
| essential64/enum | views-inputs | deferred | off | 5.565 | 5.357–5.619 | 5.190 | 98.5 | 716 | 5 |
| essential64/enum | views-inputs | source | bounded | 1.443 | 1.402–1.557 | 1.182 | 101.1 | 760 | 5 |
| essential64/enum | views-inputs | source | off | 1.474 | 1.373–1.530 | 1.139 | 101.1 | 760 | 5 |
| essential64/slab | materialized | deferred | off | 1.336 | 1.313–1.454 | 1.056 | 65.7 | 851 | 5 |
| essential64/slab | views | deferred | bounded | 6.458 | 6.372–6.705 | 5.925 | 107.2 | 1018 | 5 |
| essential64/slab | views | deferred | off | 8.102 | 7.966–8.200 | 7.678 | 107.2 | 1018 | 5 |
| essential64/slab | views | source | bounded | 1.804 | 1.734–1.943 | 1.302 | 123.5 | 1123 | 5 |
| essential64/slab | views | source | off | 1.895 | 1.853–1.992 | 1.414 | 123.5 | 1123 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 4.924 | 4.880–4.939 | 4.686 | 63.8 | 716 | 5 |
| essential64/slab | views-inputs | deferred | off | 5.513 | 5.379–5.579 | 5.307 | 63.8 | 716 | 5 |
| essential64/slab | views-inputs | source | bounded | 1.501 | 1.454–1.710 | 1.183 | 65.7 | 760 | 5 |
| essential64/slab | views-inputs | source | off | 1.464 | 1.433–1.538 | 1.174 | 65.7 | 760 | 5 |
| packed512/enum | materialized | deferred | off | 4.663 | 4.510–4.774 | 4.466 | 190.8 | 990 | 9 |

## product_free_join, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.038 | 0.038–0.052 | 0.009 | 84.0 | 129 | 9 |
| essential512/enum | materialized | deferred | off | 0.361 | 0.335–0.383 | 0.008 | 96.0 | 456 | 5 |
| essential512/enum | views | deferred | bounded | 0.432 | 0.421–0.453 | 0.007 | 54.8 | 291 | 5 |
| essential512/enum | views | deferred | off | 0.411 | 0.405–0.448 | 0.007 | 54.8 | 291 | 5 |
| essential512/enum | views | source | bounded | 0.299 | 0.289–0.334 | 0.007 | 73.4 | 389 | 5 |
| essential512/enum | views | source | off | 0.282 | 0.281–0.300 | 0.007 | 73.4 | 389 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 0.378 | 0.371–0.401 | 0.007 | 48.6 | 292 | 5 |
| essential512/enum | views-inputs | deferred | off | 0.413 | 0.396–0.427 | 0.008 | 48.6 | 292 | 5 |
| essential512/enum | views-inputs | source | bounded | 0.276 | 0.275–0.284 | 0.007 | 64.3 | 350 | 5 |
| essential512/enum | views-inputs | source | off | 0.283 | 0.281–0.313 | 0.007 | 64.3 | 350 | 5 |
| essential512/slab | materialized | deferred | off | 0.340 | 0.336–0.386 | 0.009 | 56.8 | 456 | 9 |
| essential512/slab | views | deferred | bounded | 0.421 | 0.416–0.519 | 0.007 | 37.0 | 291 | 9 |
| essential512/slab | views | deferred | off | 0.414 | 0.411–0.453 | 0.007 | 37.0 | 291 | 9 |
| essential512/slab | views | source | bounded | 0.295 | 0.291–0.338 | 0.007 | 49.8 | 389 | 9 |
| essential512/slab | views | source | off | 0.288 | 0.284–0.340 | 0.008 | 49.8 | 389 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 0.382 | 0.370–0.411 | 0.007 | 30.0 | 292 | 9 |
| essential512/slab | views-inputs | deferred | off | 0.397 | 0.394–0.438 | 0.007 | 30.0 | 292 | 9 |
| essential512/slab | views-inputs | source | bounded | 0.286 | 0.280–0.332 | 0.008 | 40.8 | 350 | 9 |
| essential512/slab | views-inputs | source | off | 0.292 | 0.276–0.357 | 0.008 | 40.8 | 350 | 9 |
| essential64/enum | materialized | deferred | off | 0.460 | 0.446–0.464 | 0.009 | 109.4 | 851 | 5 |
| essential64/enum | views | deferred | bounded | 1.625 | 1.554–1.664 | 0.007 | 183.1 | 1018 | 5 |
| essential64/enum | views | deferred | off | 1.894 | 1.876–1.956 | 0.007 | 183.1 | 1018 | 5 |
| essential64/enum | views | source | bounded | 0.750 | 0.705–0.800 | 0.007 | 196.1 | 1123 | 5 |
| essential64/enum | views | source | off | 0.749 | 0.712–0.773 | 0.007 | 196.1 | 1123 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 1.140 | 1.059–1.157 | 0.007 | 105.5 | 716 | 5 |
| essential64/enum | views-inputs | deferred | off | 1.221 | 1.207–1.330 | 0.007 | 105.5 | 716 | 5 |
| essential64/enum | views-inputs | source | bounded | 0.478 | 0.448–0.498 | 0.007 | 108.1 | 760 | 5 |
| essential64/enum | views-inputs | source | off | 0.449 | 0.442–0.466 | 0.008 | 108.1 | 760 | 5 |
| essential64/slab | materialized | deferred | off | 0.479 | 0.454–0.553 | 0.009 | 73.1 | 851 | 5 |
| essential64/slab | views | deferred | bounded | 1.551 | 1.534–1.670 | 0.007 | 113.3 | 1018 | 5 |
| essential64/slab | views | deferred | off | 1.852 | 1.839–1.965 | 0.007 | 113.3 | 1018 | 5 |
| essential64/slab | views | source | bounded | 0.747 | 0.701–0.818 | 0.007 | 129.6 | 1123 | 5 |
| essential64/slab | views | source | off | 0.746 | 0.731–0.770 | 0.007 | 129.6 | 1123 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 1.121 | 1.056–1.173 | 0.007 | 70.8 | 716 | 5 |
| essential64/slab | views-inputs | deferred | off | 1.231 | 1.169–1.278 | 0.007 | 70.8 | 716 | 5 |
| essential64/slab | views-inputs | source | bounded | 0.467 | 0.453–0.501 | 0.007 | 72.7 | 760 | 5 |
| essential64/slab | views-inputs | source | off | 0.470 | 0.465–0.566 | 0.007 | 72.7 | 760 | 5 |
| packed512/enum | materialized | deferred | off | 0.787 | 0.766–0.989 | 0.009 | 198.2 | 990 | 9 |

## product_free_join, 12 coordinates, pair-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.179 | 0.178–0.218 | 0.175 | 76.6 | 129 | 5 |
| essential512/enum | materialized | deferred | off | 0.990 | 0.929–1.146 | 0.761 | 68.8 | 427 | 5 |
| essential512/enum | views | deferred | bounded | 1.834 | 1.785–2.109 | 1.901 | 46.4 | 270 | 5 |
| essential512/enum | views | deferred | off | 1.958 | 1.882–1.980 | 1.815 | 46.4 | 270 | 5 |
| essential512/enum | views | source | bounded | 0.985 | 0.958–1.034 | 0.858 | 62.6 | 338 | 5 |
| essential512/enum | views | source | off | 0.963 | 0.912–1.066 | 0.848 | 62.6 | 338 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 1.746 | 1.681–1.754 | 1.636 | 44.4 | 286 | 5 |
| essential512/enum | views-inputs | deferred | off | 1.831 | 1.789–1.962 | 1.778 | 44.4 | 286 | 5 |
| essential512/enum | views-inputs | source | bounded | 1.072 | 1.065–1.262 | 0.953 | 58.4 | 342 | 5 |
| essential512/enum | views-inputs | source | off | 1.094 | 1.053–1.294 | 0.936 | 58.4 | 342 | 5 |
| essential512/slab | materialized | deferred | off | 1.014 | 0.990–1.054 | 0.814 | 41.8 | 427 | 5 |
| essential512/slab | views | deferred | bounded | 1.889 | 1.754–2.002 | 1.739 | 31.0 | 270 | 5 |
| essential512/slab | views | deferred | off | 1.943 | 1.842–1.997 | 1.780 | 31.0 | 270 | 5 |
| essential512/slab | views | source | bounded | 0.976 | 0.949–1.062 | 0.839 | 41.8 | 338 | 5 |
| essential512/slab | views | source | off | 0.956 | 0.894–1.003 | 0.825 | 41.8 | 338 | 5 |
| essential512/slab | views-inputs | deferred | bounded | 1.678 | 1.658–1.702 | 1.608 | 25.8 | 286 | 5 |
| essential512/slab | views-inputs | deferred | off | 1.936 | 1.855–1.988 | 1.786 | 25.8 | 286 | 5 |
| essential512/slab | views-inputs | source | bounded | 1.125 | 1.105–1.771 | 0.991 | 39.9 | 342 | 5 |
| essential512/slab | views-inputs | source | off | 1.079 | 1.050–1.140 | 0.958 | 39.9 | 342 | 5 |
| essential64/enum | materialized | deferred | off | 1.766 | 1.677–1.832 | 1.319 | 183.0 | 1034 | 5 |
| essential64/enum | views | deferred | bounded | 6.220 | 6.091–6.410 | 5.819 | 99.6 | 757 | 5 |
| essential64/enum | views | deferred | off | 8.592 | 8.245–8.801 | 8.257 | 99.6 | 757 | 5 |
| essential64/enum | views | source | bounded | 1.935 | 1.876–2.050 | 1.604 | 106.5 | 837 | 5 |
| essential64/enum | views | source | off | 2.012 | 1.914–2.133 | 1.704 | 106.5 | 837 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 6.327 | 6.241–6.487 | 5.822 | 100.1 | 816 | 5 |
| essential64/enum | views-inputs | deferred | off | 8.385 | 8.256–8.475 | 8.111 | 100.1 | 816 | 5 |
| essential64/enum | views-inputs | source | bounded | 1.953 | 1.904–2.004 | 1.618 | 107.0 | 889 | 5 |
| essential64/enum | views-inputs | source | off | 2.013 | 1.934–2.122 | 1.647 | 107.0 | 889 | 5 |
| essential64/slab | materialized | deferred | off | 1.724 | 1.682–1.948 | 1.313 | 115.9 | 1034 | 5 |
| essential64/slab | views | deferred | bounded | 6.372 | 6.151–6.443 | 6.030 | 63.8 | 757 | 5 |
| essential64/slab | views | deferred | off | 8.356 | 8.218–8.594 | 8.254 | 63.8 | 757 | 5 |
| essential64/slab | views | source | bounded | 1.945 | 1.862–1.988 | 1.597 | 69.6 | 837 | 5 |
| essential64/slab | views | source | off | 1.976 | 1.908–2.047 | 1.624 | 69.6 | 837 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 6.333 | 6.172–6.379 | 5.994 | 63.8 | 816 | 5 |
| essential64/slab | views-inputs | deferred | off | 8.527 | 8.395–8.681 | 8.176 | 63.8 | 816 | 5 |
| essential64/slab | views-inputs | source | bounded | 1.905 | 1.852–1.970 | 1.554 | 69.6 | 889 | 5 |
| essential64/slab | views-inputs | source | off | 2.017 | 1.960–2.088 | 1.651 | 69.6 | 889 | 5 |
| packed512/enum | materialized | deferred | off | 3.637 | 3.533–3.799 | 3.568 | 190.0 | 1086 | 5 |

## product_free_join, 12 coordinates, pair-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.045 | 0.041–0.047 | 0.009 | 84.0 | 129 | 5 |
| essential512/enum | materialized | deferred | off | 0.339 | 0.311–0.369 | 0.008 | 76.2 | 427 | 5 |
| essential512/enum | views | deferred | bounded | 0.412 | 0.410–0.471 | 0.007 | 52.5 | 270 | 5 |
| essential512/enum | views | deferred | off | 0.436 | 0.432–0.450 | 0.008 | 52.5 | 270 | 5 |
| essential512/enum | views | source | bounded | 0.289 | 0.275–0.317 | 0.007 | 68.6 | 338 | 5 |
| essential512/enum | views | source | off | 0.272 | 0.268–0.307 | 0.007 | 68.6 | 338 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 0.399 | 0.379–0.419 | 0.007 | 51.3 | 286 | 5 |
| essential512/enum | views-inputs | deferred | off | 0.420 | 0.416–0.508 | 0.007 | 51.3 | 286 | 5 |
| essential512/enum | views-inputs | source | bounded | 0.297 | 0.286–0.338 | 0.007 | 65.4 | 342 | 5 |
| essential512/enum | views-inputs | source | off | 0.285 | 0.278–0.304 | 0.007 | 65.4 | 342 | 5 |
| essential512/slab | materialized | deferred | off | 0.355 | 0.352–0.514 | 0.009 | 49.2 | 427 | 5 |
| essential512/slab | views | deferred | bounded | 0.438 | 0.418–0.464 | 0.007 | 37.0 | 270 | 5 |
| essential512/slab | views | deferred | off | 0.436 | 0.431–0.510 | 0.007 | 37.0 | 270 | 5 |
| essential512/slab | views | source | bounded | 0.293 | 0.278–0.363 | 0.007 | 47.9 | 338 | 5 |
| essential512/slab | views | source | off | 0.265 | 0.264–0.294 | 0.007 | 47.9 | 338 | 5 |
| essential512/slab | views-inputs | deferred | bounded | 0.391 | 0.379–0.397 | 0.007 | 32.8 | 286 | 5 |
| essential512/slab | views-inputs | deferred | off | 0.426 | 0.417–0.467 | 0.007 | 32.8 | 286 | 5 |
| essential512/slab | views-inputs | source | bounded | 0.304 | 0.300–0.335 | 0.007 | 46.9 | 342 | 5 |
| essential512/slab | views-inputs | source | off | 0.312 | 0.279–0.323 | 0.007 | 46.9 | 342 | 5 |
| essential64/enum | materialized | deferred | off | 0.672 | 0.639–0.698 | 0.008 | 190.4 | 1034 | 5 |
| essential64/enum | views | deferred | bounded | 1.372 | 1.344–1.450 | 0.007 | 105.7 | 757 | 5 |
| essential64/enum | views | deferred | off | 1.757 | 1.719–1.880 | 0.007 | 105.7 | 757 | 5 |
| essential64/enum | views | source | bounded | 0.633 | 0.611–0.649 | 0.007 | 112.6 | 837 | 5 |
| essential64/enum | views | source | off | 0.623 | 0.604–0.643 | 0.007 | 112.6 | 837 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 1.375 | 1.355–1.414 | 0.007 | 107.1 | 816 | 5 |
| essential64/enum | views-inputs | deferred | off | 1.735 | 1.706–1.784 | 0.007 | 107.1 | 816 | 5 |
| essential64/enum | views-inputs | source | bounded | 0.641 | 0.594–0.653 | 0.007 | 113.9 | 889 | 5 |
| essential64/enum | views-inputs | source | off | 0.648 | 0.618–0.694 | 0.007 | 113.9 | 889 | 5 |
| essential64/slab | materialized | deferred | off | 0.661 | 0.651–0.732 | 0.008 | 123.3 | 1034 | 5 |
| essential64/slab | views | deferred | bounded | 1.356 | 1.318–1.434 | 0.007 | 69.9 | 757 | 5 |
| essential64/slab | views | deferred | off | 1.738 | 1.726–1.842 | 0.007 | 69.9 | 757 | 5 |
| essential64/slab | views | source | bounded | 0.623 | 0.616–0.670 | 0.007 | 75.6 | 837 | 5 |
| essential64/slab | views | source | off | 0.661 | 0.618–0.692 | 0.007 | 75.6 | 837 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 1.379 | 1.329–1.418 | 0.007 | 70.8 | 816 | 5 |
| essential64/slab | views-inputs | deferred | off | 1.763 | 1.728–1.842 | 0.007 | 70.8 | 816 | 5 |
| essential64/slab | views-inputs | source | bounded | 0.597 | 0.586–0.617 | 0.007 | 76.5 | 889 | 5 |
| essential64/slab | views-inputs | source | off | 0.609 | 0.594–0.669 | 0.007 | 76.5 | 889 | 5 |
| packed512/enum | materialized | deferred | off | 0.581 | 0.564–0.645 | 0.009 | 197.4 | 1086 | 5 |

## product_free_join, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 9.082 | 8.801–9.145 | 8.981 | 4335.1 | 130 | 9 |
| essential512/enum | materialized | deferred | off | 7.387 | 7.281–7.484 | 5.939 | 583.9 | 3112 | 5 |
| essential512/enum | views | deferred | bounded | 39.224 | 38.912–39.633 | 38.097 | 311.2 | 1792 | 5 |
| essential512/enum | views | deferred | off | 63.913 | 62.608–75.330 | 63.970 | 311.2 | 1792 | 5 |
| essential512/enum | views | source | bounded | 7.643 | 7.603–7.962 | 6.428 | 407.6 | 2082 | 5 |
| essential512/enum | views | source | off | 8.001 | 7.929–8.292 | 6.875 | 407.6 | 2082 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 44.620 | 44.354–45.378 | 43.689 | 400.2 | 2342 | 5 |
| essential512/enum | views-inputs | deferred | off | 66.269 | 65.964–66.420 | 64.833 | 400.2 | 2342 | 5 |
| essential512/enum | views-inputs | source | bounded | 7.874 | 7.626–7.990 | 6.635 | 424.6 | 2673 | 5 |
| essential512/enum | views-inputs | source | off | 8.353 | 7.879–8.459 | 6.819 | 424.6 | 2673 | 5 |
| essential512/slab | materialized | deferred | off | 7.306 | 7.038–7.553 | 5.770 | 356.7 | 3112 | 9 |
| essential512/slab | views | deferred | bounded | 40.148 | 39.806–40.521 | 38.652 | 231.5 | 1792 | 9 |
| essential512/slab | views | deferred | off | 62.616 | 61.330–63.158 | 61.797 | 231.5 | 1792 | 9 |
| essential512/slab | views | source | bounded | 7.501 | 7.379–7.621 | 6.443 | 273.4 | 2082 | 9 |
| essential512/slab | views | source | off | 7.944 | 7.666–8.372 | 6.945 | 273.4 | 2082 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 42.151 | 41.330–42.479 | 41.254 | 260.1 | 2342 | 9 |
| essential512/slab | views-inputs | deferred | off | 65.487 | 65.012–66.702 | 64.822 | 260.1 | 2342 | 9 |
| essential512/slab | views-inputs | source | bounded | 7.884 | 7.816–8.075 | 6.669 | 273.4 | 2673 | 9 |
| essential512/slab | views-inputs | source | off | 8.139 | 8.071–8.312 | 7.069 | 273.4 | 2673 | 9 |
| essential64/enum | materialized | deferred | off | 10.138 | 10.054–10.520 | 8.346 | 826.7 | 5582 | 5 |
| essential64/enum | views | deferred | bounded | 26.425 | 26.357–28.340 | 44.446 | 408.4 | 3102 | 5 |
| essential64/enum | views | deferred | off | 35.779 | 35.104–36.417 | 34.117 | 408.4 | 3102 | 5 |
| essential64/enum | views | source | bounded | 10.543 | 10.356–10.732 | 9.170 | 432.9 | 3282 | 5 |
| essential64/enum | views | source | off | 10.898 | 10.643–11.010 | 9.399 | 432.9 | 3282 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 26.141 | 25.723–27.036 | 24.241 | 796.2 | 4643 | 5 |
| essential64/enum | views-inputs | deferred | off | 35.261 | 34.929–42.105 | 34.092 | 796.2 | 4643 | 5 |
| essential64/enum | views-inputs | source | bounded | 10.671 | 10.599–10.707 | 9.016 | 806.1 | 4784 | 5 |
| essential64/enum | views-inputs | source | off | 11.355 | 11.252–12.204 | 9.476 | 806.1 | 4784 | 5 |
| essential64/slab | materialized | deferred | off | 10.115 | 10.004–10.314 | 8.228 | 543.6 | 5582 | 5 |
| essential64/slab | views | deferred | bounded | 26.068 | 25.749–26.252 | 25.120 | 264.4 | 3102 | 5 |
| essential64/slab | views | deferred | off | 35.708 | 35.341–35.961 | 34.233 | 264.4 | 3102 | 5 |
| essential64/slab | views | source | bounded | 10.637 | 10.526–10.821 | 9.195 | 287.3 | 3282 | 5 |
| essential64/slab | views | source | off | 11.190 | 11.129–11.461 | 9.814 | 287.3 | 3282 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 26.858 | 26.726–27.451 | 25.391 | 509.9 | 4643 | 5 |
| essential64/slab | views-inputs | deferred | off | 35.720 | 35.684–36.374 | 34.806 | 509.9 | 4643 | 5 |
| essential64/slab | views-inputs | source | bounded | 33.880 | 21.704–93.200 | 12.014 | 517.6 | 4784 | 5 |
| essential64/slab | views-inputs | source | off | 11.367 | 11.000–11.459 | 9.696 | 517.6 | 4784 | 5 |
| packed512/enum | materialized | deferred | off | 3.387 | 3.220–3.482 | 2.830 | 676.8 | 5073 | 9 |

## product_free_join, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 1.389 | 1.317–1.409 | 0.017 | 4342.5 | 130 | 9 |
| essential512/enum | materialized | deferred | off | 2.537 | 2.408–2.586 | 0.009 | 591.3 | 3112 | 5 |
| essential512/enum | views | deferred | bounded | 8.150 | 7.864–8.288 | 0.007 | 317.3 | 1792 | 5 |
| essential512/enum | views | deferred | off | 12.204 | 12.098–12.553 | 0.007 | 317.3 | 1792 | 5 |
| essential512/enum | views | source | bounded | 2.294 | 2.173–2.316 | 0.007 | 413.6 | 2082 | 5 |
| essential512/enum | views | source | off | 2.343 | 2.273–2.553 | 0.008 | 413.6 | 2082 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 9.204 | 9.123–9.312 | 0.007 | 407.2 | 2342 | 5 |
| essential512/enum | views-inputs | deferred | off | 13.020 | 12.753–13.353 | 0.008 | 407.2 | 2342 | 5 |
| essential512/enum | views-inputs | source | bounded | 2.404 | 2.353–2.467 | 0.007 | 431.6 | 2673 | 5 |
| essential512/enum | views-inputs | source | off | 2.489 | 2.401–2.734 | 0.007 | 431.6 | 2673 | 5 |
| essential512/slab | materialized | deferred | off | 2.431 | 2.358–2.577 | 0.009 | 364.1 | 3112 | 9 |
| essential512/slab | views | deferred | bounded | 8.210 | 7.936–8.528 | 0.008 | 237.6 | 1792 | 9 |
| essential512/slab | views | deferred | off | 12.488 | 12.258–12.831 | 0.007 | 237.6 | 1792 | 9 |
| essential512/slab | views | source | bounded | 2.244 | 2.124–2.317 | 0.007 | 279.5 | 2082 | 9 |
| essential512/slab | views | source | off | 2.272 | 2.243–2.596 | 0.007 | 279.5 | 2082 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 8.729 | 8.468–8.981 | 0.007 | 267.0 | 2342 | 9 |
| essential512/slab | views-inputs | deferred | off | 13.013 | 12.754–13.230 | 0.007 | 267.0 | 2342 | 9 |
| essential512/slab | views-inputs | source | bounded | 2.365 | 2.268–2.680 | 0.007 | 280.4 | 2673 | 9 |
| essential512/slab | views-inputs | source | off | 2.406 | 2.328–2.474 | 0.007 | 280.4 | 2673 | 9 |
| essential64/enum | materialized | deferred | off | 3.505 | 3.353–3.561 | 0.009 | 834.1 | 5582 | 5 |
| essential64/enum | views | deferred | bounded | 5.946 | 5.821–5.963 | 0.008 | 414.4 | 3102 | 5 |
| essential64/enum | views | deferred | off | 7.481 | 7.357–7.536 | 0.007 | 414.4 | 3102 | 5 |
| essential64/enum | views | source | bounded | 2.925 | 2.910–3.015 | 0.007 | 439.0 | 3282 | 5 |
| essential64/enum | views | source | off | 3.066 | 2.998–3.129 | 0.007 | 439.0 | 3282 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 6.159 | 6.032–6.253 | 0.007 | 803.2 | 4643 | 5 |
| essential64/enum | views-inputs | deferred | off | 8.039 | 7.781–8.145 | 0.007 | 803.2 | 4643 | 5 |
| essential64/enum | views-inputs | source | bounded | 3.429 | 3.258–3.636 | 0.007 | 813.0 | 4784 | 5 |
| essential64/enum | views-inputs | source | off | 3.409 | 3.361–3.601 | 0.007 | 813.0 | 4784 | 5 |
| essential64/slab | materialized | deferred | off | 3.434 | 3.298–3.556 | 0.009 | 551.0 | 5582 | 5 |
| essential64/slab | views | deferred | bounded | 5.898 | 5.832–6.119 | 0.007 | 270.5 | 3102 | 5 |
| essential64/slab | views | deferred | off | 7.520 | 7.341–7.672 | 0.007 | 270.5 | 3102 | 5 |
| essential64/slab | views | source | bounded | 3.050 | 2.916–3.082 | 0.007 | 293.4 | 3282 | 5 |
| essential64/slab | views | source | off | 3.191 | 3.026–3.281 | 0.007 | 293.4 | 3282 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 6.132 | 6.091–6.259 | 0.007 | 516.9 | 4643 | 5 |
| essential64/slab | views-inputs | deferred | off | 7.937 | 7.720–8.215 | 0.007 | 516.9 | 4643 | 5 |
| essential64/slab | views-inputs | source | bounded | 4.972 | 4.241–5.302 | 0.008 | 524.5 | 4784 | 5 |
| essential64/slab | views-inputs | source | off | 3.370 | 3.336–3.629 | 0.007 | 524.5 | 4784 | 5 |
| packed512/enum | materialized | deferred | off | 0.753 | 0.704–0.834 | 0.025 | 684.2 | 5073 | 9 |

## product_free_join, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 8.783 | 8.670–8.983 | 8.867 | 4335.1 | 130 | 9 |
| essential512/enum | materialized | deferred | off | 7.341 | 7.283–7.572 | 5.941 | 512.2 | 3227 | 5 |
| essential512/enum | views | deferred | bounded | 104.594 | 103.512–107.232 | 100.780 | 1011.5 | 5565 | 5 |
| essential512/enum | views | deferred | off | 135.834 | 135.360–136.325 | 131.823 | 1011.5 | 5565 | 5 |
| essential512/enum | views | source | bounded | 13.869 | 13.662–14.275 | 10.657 | 1082.5 | 6007 | 5 |
| essential512/enum | views | source | off | 18.844 | 18.672–19.201 | 15.405 | 1082.5 | 6007 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 84.504 | 83.646–86.097 | 83.523 | 461.9 | 2645 | 5 |
| essential512/enum | views-inputs | deferred | off | 100.451 | 98.429–101.551 | 99.254 | 461.9 | 2645 | 5 |
| essential512/enum | views-inputs | source | bounded | 8.015 | 7.896–8.331 | 6.860 | 501.7 | 3035 | 5 |
| essential512/enum | views-inputs | source | off | 7.966 | 7.812–8.432 | 6.835 | 501.7 | 3035 | 5 |
| essential512/slab | materialized | deferred | off | 7.208 | 7.024–7.303 | 5.831 | 360.3 | 3227 | 9 |
| essential512/slab | views | deferred | bounded | 104.483 | 102.949–105.741 | 101.888 | 735.4 | 5565 | 9 |
| essential512/slab | views | deferred | off | 138.799 | 136.806–139.106 | 135.256 | 735.4 | 5565 | 9 |
| essential512/slab | views | source | bounded | 13.931 | 13.809–14.140 | 10.540 | 781.0 | 6007 | 9 |
| essential512/slab | views | source | off | 19.192 | 18.789–19.610 | 15.508 | 781.0 | 6007 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 85.717 | 84.735–86.953 | 85.234 | 268.8 | 2645 | 9 |
| essential512/slab | views-inputs | deferred | off | 97.453 | 96.729–98.460 | 97.078 | 268.8 | 2645 | 9 |
| essential512/slab | views-inputs | source | bounded | 8.080 | 7.923–8.376 | 6.837 | 360.3 | 3035 | 9 |
| essential512/slab | views-inputs | source | off | 8.091 | 7.704–8.381 | 6.821 | 360.3 | 3035 | 9 |
| essential64/enum | materialized | deferred | off | 6.109 | 6.085–6.255 | 3.654 | 1087.0 | 7382 | 5 |
| essential64/enum | views | deferred | bounded | 83.564 | 82.092–122.947 | 85.138 | 1537.1 | 9887 | 5 |
| essential64/enum | views | deferred | off | 96.091 | 95.912–97.663 | 91.657 | 1537.1 | 9887 | 5 |
| essential64/enum | views | source | bounded | 20.441 | 20.081–21.024 | 16.316 | 1601.4 | 11069 | 5 |
| essential64/enum | views | source | off | 20.730 | 20.443–20.871 | 16.876 | 1601.4 | 11069 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 16.826 | 16.388–17.061 | 14.811 | 777.2 | 6468 | 5 |
| essential64/enum | views-inputs | deferred | off | 17.306 | 17.015–27.831 | 14.954 | 777.2 | 6468 | 5 |
| essential64/enum | views-inputs | source | bounded | 8.716 | 8.568–8.818 | 6.530 | 790.4 | 6582 | 5 |
| essential64/enum | views-inputs | source | off | 8.459 | 8.312–8.478 | 6.110 | 790.4 | 6582 | 5 |
| essential64/slab | materialized | deferred | off | 6.358 | 6.152–6.481 | 3.882 | 639.0 | 7382 | 5 |
| essential64/slab | views | deferred | bounded | 81.898 | 81.428–82.264 | 79.172 | 979.9 | 9887 | 5 |
| essential64/slab | views | deferred | off | 96.074 | 94.941–97.938 | 92.120 | 979.9 | 9887 | 5 |
| essential64/slab | views | source | bounded | 20.139 | 20.047–20.643 | 16.489 | 1043.6 | 11069 | 5 |
| essential64/slab | views | source | off | 21.411 | 20.904–21.745 | 16.918 | 1043.6 | 11069 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 16.980 | 16.687–17.642 | 14.772 | 495.4 | 6468 | 5 |
| essential64/slab | views-inputs | deferred | off | 16.612 | 16.520–16.673 | 14.673 | 495.4 | 6468 | 5 |
| essential64/slab | views-inputs | source | bounded | 8.557 | 8.425–8.838 | 6.447 | 506.8 | 6582 | 5 |
| essential64/slab | views-inputs | source | off | 8.387 | 8.353–8.603 | 6.331 | 506.8 | 6582 | 5 |
| packed512/enum | materialized | deferred | off | 12.438 | 12.234–12.831 | 11.714 | 994.3 | 6218 | 9 |

## product_free_join, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 1.351 | 1.309–1.540 | 0.017 | 4342.5 | 130 | 9 |
| essential512/enum | materialized | deferred | off | 2.264 | 2.227–2.415 | 0.009 | 519.6 | 3227 | 5 |
| essential512/enum | views | deferred | bounded | 22.512 | 21.578–23.179 | 0.007 | 1017.5 | 5565 | 5 |
| essential512/enum | views | deferred | off | 28.027 | 27.202–29.030 | 0.007 | 1017.5 | 5565 | 5 |
| essential512/enum | views | source | bounded | 5.433 | 5.271–5.475 | 0.007 | 1088.6 | 6007 | 5 |
| essential512/enum | views | source | off | 6.402 | 6.227–6.600 | 0.007 | 1088.6 | 6007 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 16.795 | 16.683–16.939 | 0.007 | 468.9 | 2645 | 5 |
| essential512/enum | views-inputs | deferred | off | 19.793 | 19.612–19.890 | 0.007 | 468.9 | 2645 | 5 |
| essential512/enum | views-inputs | source | bounded | 2.438 | 2.357–2.523 | 0.009 | 508.6 | 3035 | 5 |
| essential512/enum | views-inputs | source | off | 2.404 | 2.317–2.477 | 0.008 | 508.6 | 3035 | 5 |
| essential512/slab | materialized | deferred | off | 2.240 | 2.179–2.367 | 0.009 | 367.7 | 3227 | 9 |
| essential512/slab | views | deferred | bounded | 22.178 | 21.742–22.532 | 0.007 | 741.4 | 5565 | 9 |
| essential512/slab | views | deferred | off | 28.542 | 27.722–29.148 | 0.007 | 741.4 | 5565 | 9 |
| essential512/slab | views | source | bounded | 5.320 | 5.158–5.855 | 0.008 | 787.1 | 6007 | 9 |
| essential512/slab | views | source | off | 6.259 | 6.203–6.803 | 0.007 | 787.1 | 6007 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 16.847 | 16.589–17.623 | 0.007 | 275.7 | 2645 | 9 |
| essential512/slab | views-inputs | deferred | off | 18.977 | 18.877–19.398 | 0.007 | 275.7 | 2645 | 9 |
| essential512/slab | views-inputs | source | bounded | 2.363 | 2.325–2.619 | 0.008 | 367.3 | 3035 | 9 |
| essential512/slab | views-inputs | source | off | 2.374 | 2.262–2.502 | 0.007 | 367.3 | 3035 | 9 |
| essential64/enum | materialized | deferred | off | 2.882 | 2.866–3.018 | 0.011 | 1094.4 | 7382 | 5 |
| essential64/enum | views | deferred | bounded | 18.187 | 18.146–19.594 | 0.008 | 1543.2 | 9887 | 5 |
| essential64/enum | views | deferred | off | 20.415 | 20.136–21.055 | 0.007 | 1543.2 | 9887 | 5 |
| essential64/enum | views | source | bounded | 6.937 | 6.641–7.299 | 0.008 | 1607.4 | 11069 | 5 |
| essential64/enum | views | source | off | 6.997 | 6.885–7.740 | 0.007 | 1607.4 | 11069 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 4.846 | 4.685–4.976 | 0.007 | 784.1 | 6468 | 5 |
| essential64/enum | views-inputs | deferred | off | 4.747 | 4.720–4.898 | 0.008 | 784.1 | 6468 | 5 |
| essential64/enum | views-inputs | source | bounded | 3.342 | 3.299–4.831 | 0.008 | 797.4 | 6582 | 5 |
| essential64/enum | views-inputs | source | off | 3.306 | 3.186–3.514 | 0.008 | 797.4 | 6582 | 5 |
| essential64/slab | materialized | deferred | off | 2.995 | 2.905–3.047 | 0.009 | 646.4 | 7382 | 5 |
| essential64/slab | views | deferred | bounded | 17.991 | 17.333–18.267 | 0.007 | 986.0 | 9887 | 5 |
| essential64/slab | views | deferred | off | 20.027 | 19.897–20.857 | 0.012 | 986.0 | 9887 | 5 |
| essential64/slab | views | source | bounded | 6.656 | 6.564–6.935 | 0.007 | 1049.7 | 11069 | 5 |
| essential64/slab | views | source | off | 6.936 | 6.781–7.148 | 0.007 | 1049.7 | 11069 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 4.715 | 4.705–4.959 | 0.007 | 502.4 | 6468 | 5 |
| essential64/slab | views-inputs | deferred | off | 4.886 | 4.747–5.135 | 0.008 | 502.4 | 6468 | 5 |
| essential64/slab | views-inputs | source | bounded | 3.281 | 3.224–3.444 | 0.007 | 513.8 | 6582 | 5 |
| essential64/slab | views-inputs | source | off | 3.393 | 3.217–3.454 | 0.008 | 513.8 | 6582 | 5 |
| packed512/enum | materialized | deferred | off | 2.333 | 2.082–2.522 | 0.015 | 1001.7 | 6218 | 9 |

## product_free_join, 18 coordinates, pair-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 8.856 | 8.667–9.024 | 8.857 | 4335.1 | 130 | 5 |
| essential512/enum | materialized | deferred | off | 7.656 | 7.570–7.889 | 6.006 | 639.7 | 3479 | 5 |
| essential512/enum | views | deferred | bounded | 43.159 | 42.925–43.281 | 42.453 | 418.4 | 2378 | 5 |
| essential512/enum | views | deferred | off | 63.776 | 63.255–65.084 | 61.960 | 418.4 | 2378 | 5 |
| essential512/enum | views | source | bounded | 8.775 | 8.554–8.960 | 7.337 | 459.0 | 2710 | 5 |
| essential512/enum | views | source | off | 9.819 | 9.732–10.081 | 8.431 | 459.0 | 2710 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 44.624 | 44.437–44.663 | 43.679 | 416.1 | 2643 | 5 |
| essential512/enum | views-inputs | deferred | off | 63.645 | 63.429–64.178 | 62.650 | 416.1 | 2643 | 5 |
| essential512/enum | views-inputs | source | bounded | 8.421 | 8.322–8.567 | 7.345 | 439.7 | 3038 | 5 |
| essential512/enum | views-inputs | source | off | 8.779 | 8.442–8.887 | 7.136 | 439.7 | 3038 | 5 |
| essential512/slab | materialized | deferred | off | 7.887 | 7.871–8.364 | 6.342 | 405.1 | 3479 | 5 |
| essential512/slab | views | deferred | bounded | 43.391 | 41.943–43.749 | 42.107 | 270.9 | 2378 | 5 |
| essential512/slab | views | deferred | off | 63.092 | 61.834–64.035 | 62.357 | 270.9 | 2378 | 5 |
| essential512/slab | views | source | bounded | 8.390 | 8.321–8.427 | 7.083 | 297.6 | 2710 | 5 |
| essential512/slab | views | source | off | 9.498 | 9.362–9.804 | 8.175 | 297.6 | 2710 | 5 |
| essential512/slab | views-inputs | deferred | bounded | 44.955 | 44.480–48.954 | 44.126 | 270.9 | 2643 | 5 |
| essential512/slab | views-inputs | deferred | off | 62.752 | 62.705–63.756 | 61.956 | 270.9 | 2643 | 5 |
| essential512/slab | views-inputs | source | bounded | 46.998 | 8.499–60.289 | 7.198 | 282.4 | 3038 | 5 |
| essential512/slab | views-inputs | source | off | 8.689 | 8.630–8.758 | 7.378 | 282.4 | 3038 | 5 |
| essential64/enum | materialized | deferred | off | 10.570 | 10.463–10.787 | 8.123 | 872.8 | 6490 | 5 |
| essential64/enum | views | deferred | bounded | 41.793 | 41.120–42.563 | 40.002 | 581.2 | 4275 | 5 |
| essential64/enum | views | deferred | off | 60.668 | 59.976–60.896 | 58.605 | 581.2 | 4275 | 5 |
| essential64/enum | views | source | bounded | 11.451 | 11.269–11.610 | 9.439 | 607.3 | 4602 | 5 |
| essential64/enum | views | source | off | 12.025 | 11.690–12.199 | 10.071 | 607.3 | 4602 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 42.154 | 41.896–42.262 | 40.095 | 842.1 | 5496 | 5 |
| essential64/enum | views-inputs | deferred | off | 60.673 | 59.817–61.326 | 58.537 | 842.1 | 5496 | 5 |
| essential64/enum | views-inputs | source | bounded | 11.394 | 11.160–11.503 | 9.277 | 867.4 | 5644 | 5 |
| essential64/enum | views-inputs | source | off | 11.886 | 11.711–12.063 | 9.894 | 867.4 | 5644 | 5 |
| essential64/slab | materialized | deferred | off | 10.507 | 10.253–10.670 | 8.266 | 574.6 | 6490 | 5 |
| essential64/slab | views | deferred | bounded | 41.054 | 40.730–42.775 | 39.655 | 340.9 | 4275 | 5 |
| essential64/slab | views | deferred | off | 60.488 | 60.344–61.058 | 59.404 | 340.9 | 4275 | 5 |
| essential64/slab | views | source | bounded | 11.402 | 11.258–11.562 | 9.598 | 363.7 | 4602 | 5 |
| essential64/slab | views | source | off | 11.977 | 11.861–12.666 | 10.212 | 363.7 | 4602 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 42.074 | 41.977–43.084 | 41.052 | 551.8 | 5496 | 5 |
| essential64/slab | views-inputs | deferred | off | 60.486 | 59.003–60.867 | 58.584 | 551.8 | 5496 | 5 |
| essential64/slab | views-inputs | source | bounded | 11.234 | 10.923–11.511 | 9.311 | 574.6 | 5644 | 5 |
| essential64/slab | views-inputs | source | off | 11.954 | 11.617–12.282 | 9.866 | 574.6 | 5644 | 5 |
| packed512/enum | materialized | deferred | off | 15.754 | 15.391–15.889 | 15.087 | 965.8 | 6500 | 5 |

## product_free_join, 18 coordinates, pair-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 1.298 | 1.292–1.424 | 0.018 | 4342.5 | 130 | 5 |
| essential512/enum | materialized | deferred | off | 2.653 | 2.647–2.717 | 0.008 | 647.0 | 3479 | 5 |
| essential512/enum | views | deferred | bounded | 9.170 | 8.955–9.421 | 0.007 | 424.5 | 2378 | 5 |
| essential512/enum | views | deferred | off | 12.684 | 12.643–13.473 | 0.007 | 424.5 | 2378 | 5 |
| essential512/enum | views | source | bounded | 2.606 | 2.567–2.611 | 0.007 | 465.0 | 2710 | 5 |
| essential512/enum | views | source | off | 3.028 | 2.776–3.194 | 0.008 | 465.0 | 2710 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 9.314 | 9.211–9.669 | 0.007 | 423.1 | 2643 | 5 |
| essential512/enum | views-inputs | deferred | off | 12.753 | 12.487–13.048 | 0.007 | 423.1 | 2643 | 5 |
| essential512/enum | views-inputs | source | bounded | 2.592 | 2.526–2.635 | 0.007 | 446.7 | 3038 | 5 |
| essential512/enum | views-inputs | source | off | 2.645 | 2.497–2.831 | 0.007 | 446.7 | 3038 | 5 |
| essential512/slab | materialized | deferred | off | 2.680 | 2.660–4.850 | 0.009 | 412.5 | 3479 | 5 |
| essential512/slab | views | deferred | bounded | 8.813 | 8.625–8.948 | 0.007 | 277.0 | 2378 | 5 |
| essential512/slab | views | deferred | off | 12.699 | 12.566–12.844 | 0.007 | 277.0 | 2378 | 5 |
| essential512/slab | views | source | bounded | 2.615 | 2.576–2.663 | 0.007 | 303.7 | 2710 | 5 |
| essential512/slab | views | source | off | 2.785 | 2.738–2.884 | 0.009 | 303.7 | 2710 | 5 |
| essential512/slab | views-inputs | deferred | bounded | 9.179 | 9.105–9.373 | 0.008 | 277.9 | 2643 | 5 |
| essential512/slab | views-inputs | deferred | off | 12.561 | 12.493–13.067 | 0.007 | 277.9 | 2643 | 5 |
| essential512/slab | views-inputs | source | bounded | 2.585 | 2.552–42.001 | 0.008 | 289.3 | 3038 | 5 |
| essential512/slab | views-inputs | source | off | 2.581 | 2.517–2.716 | 0.007 | 289.3 | 3038 | 5 |
| essential64/enum | materialized | deferred | off | 3.856 | 3.617–3.943 | 0.008 | 880.2 | 6490 | 5 |
| essential64/enum | views | deferred | bounded | 9.001 | 8.730–9.208 | 0.007 | 587.2 | 4275 | 5 |
| essential64/enum | views | deferred | off | 12.592 | 12.405–13.245 | 0.007 | 587.2 | 4275 | 5 |
| essential64/enum | views | source | bounded | 3.559 | 3.443–3.693 | 0.007 | 613.4 | 4602 | 5 |
| essential64/enum | views | source | off | 3.719 | 3.660–3.754 | 0.007 | 613.4 | 4602 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 9.385 | 9.128–9.515 | 0.007 | 849.1 | 5496 | 5 |
| essential64/enum | views-inputs | deferred | off | 12.774 | 12.532–12.952 | 0.007 | 849.1 | 5496 | 5 |
| essential64/enum | views-inputs | source | bounded | 3.639 | 3.536–3.797 | 0.007 | 874.3 | 5644 | 5 |
| essential64/enum | views-inputs | source | off | 3.821 | 3.720–3.892 | 0.007 | 874.3 | 5644 | 5 |
| essential64/slab | materialized | deferred | off | 3.743 | 3.607–3.889 | 0.008 | 582.0 | 6490 | 5 |
| essential64/slab | views | deferred | bounded | 9.003 | 8.738–9.249 | 0.007 | 347.0 | 4275 | 5 |
| essential64/slab | views | deferred | off | 12.578 | 12.506–12.730 | 0.007 | 347.0 | 4275 | 5 |
| essential64/slab | views | source | bounded | 3.545 | 3.395–3.574 | 0.007 | 369.8 | 4602 | 5 |
| essential64/slab | views | source | off | 3.579 | 3.486–3.705 | 0.007 | 369.8 | 4602 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 9.375 | 9.258–9.518 | 0.008 | 558.8 | 5496 | 5 |
| essential64/slab | views-inputs | deferred | off | 12.783 | 12.435–14.066 | 0.007 | 558.8 | 5496 | 5 |
| essential64/slab | views-inputs | source | bounded | 3.595 | 3.501–3.706 | 0.007 | 581.6 | 5644 | 5 |
| essential64/slab | views-inputs | source | off | 3.723 | 3.570–3.836 | 0.007 | 581.6 | 5644 | 5 |
| packed512/enum | materialized | deferred | off | 2.565 | 2.505–2.603 | 0.021 | 973.2 | 6500 | 5 |

## relations_free_join, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.149 | 0.142–0.166 | 0.142 | 82.5 | 140 | 9 |
| essential512/enum | materialized | deferred | off | 0.753 | 0.463–0.893 | 0.375 | 66.9 | 345 | 9 |
| essential512/enum | views | deferred | bounded | 0.699 | 0.656–1.272 | 0.566 | 22.7 | 130 | 5 |
| essential512/enum | views | deferred | off | 0.709 | 0.675–0.780 | 0.614 | 22.7 | 130 | 5 |
| essential512/enum | views | source | bounded | 0.500 | 0.470–1.306 | 0.303 | 35.2 | 215 | 5 |
| essential512/enum | views | source | off | 0.418 | 0.398–0.476 | 0.263 | 35.2 | 215 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 0.663 | 0.631–0.691 | 0.569 | 22.2 | 121 | 5 |
| essential512/enum | views-inputs | deferred | off | 0.701 | 0.692–0.764 | 0.623 | 22.2 | 121 | 5 |
| essential512/enum | views-inputs | source | bounded | 0.471 | 0.455–0.549 | 0.341 | 34.5 | 205 | 9 |
| essential512/enum | views-inputs | source | off | 0.425 | 0.391–0.483 | 0.282 | 34.5 | 205 | 9 |
| essential512/slab | materialized | deferred | off | 0.497 | 0.438–0.819 | 0.398 | 43.7 | 345 | 9 |
| essential512/slab | views | deferred | bounded | 0.680 | 0.618–0.752 | 0.541 | 13.1 | 130 | 9 |
| essential512/slab | views | deferred | off | 0.732 | 0.703–0.793 | 0.620 | 13.1 | 130 | 9 |
| essential512/slab | views | source | bounded | 0.461 | 0.452–0.548 | 0.319 | 23.9 | 215 | 9 |
| essential512/slab | views | source | off | 0.451 | 0.409–0.507 | 0.264 | 23.9 | 215 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 0.650 | 0.640–0.714 | 0.593 | 13.1 | 121 | 9 |
| essential512/slab | views-inputs | deferred | off | 0.702 | 0.696–0.752 | 0.629 | 13.1 | 121 | 9 |
| essential512/slab | views-inputs | source | bounded | 0.453 | 0.441–0.515 | 0.339 | 23.9 | 205 | 9 |
| essential512/slab | views-inputs | source | off | 0.432 | 0.397–0.473 | 0.291 | 23.9 | 205 | 9 |
| essential64/enum | materialized | deferred | off | 0.992 | 0.952–1.085 | 0.554 | 167.9 | 1159 | 5 |
| essential64/enum | views | deferred | bounded | 0.844 | 0.830–0.904 | 0.715 | 79.5 | 495 | 5 |
| essential64/enum | views | deferred | off | 0.895 | 0.865–0.987 | 0.778 | 79.5 | 495 | 5 |
| essential64/enum | views | source | bounded | 0.816 | 0.791–0.945 | 0.579 | 89.1 | 699 | 5 |
| essential64/enum | views | source | off | 0.893 | 0.840–0.941 | 0.623 | 89.1 | 699 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 1.198 | 0.885–1.593 | 0.846 | 79.4 | 519 | 5 |
| essential64/enum | views-inputs | deferred | off | 0.910 | 0.854–0.930 | 0.761 | 79.4 | 519 | 5 |
| essential64/enum | views-inputs | source | bounded | 0.869 | 0.785–0.947 | 0.617 | 85.0 | 663 | 5 |
| essential64/enum | views-inputs | source | off | 0.838 | 0.796–0.894 | 0.584 | 85.0 | 663 | 5 |
| essential64/slab | materialized | deferred | off | 0.951 | 0.945–1.071 | 0.569 | 106.1 | 1159 | 5 |
| essential64/slab | views | deferred | bounded | 0.834 | 0.819–0.918 | 0.702 | 46.7 | 495 | 5 |
| essential64/slab | views | deferred | off | 0.927 | 0.868–0.978 | 0.772 | 46.7 | 495 | 5 |
| essential64/slab | views | source | bounded | 0.825 | 0.803–0.943 | 0.603 | 57.0 | 699 | 5 |
| essential64/slab | views | source | off | 0.847 | 0.813–0.892 | 0.624 | 57.0 | 699 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 0.917 | 0.840–0.998 | 0.708 | 46.7 | 519 | 5 |
| essential64/slab | views-inputs | deferred | off | 0.899 | 0.875–0.965 | 0.732 | 46.7 | 519 | 5 |
| essential64/slab | views-inputs | source | bounded | 0.834 | 0.805–0.953 | 0.597 | 53.2 | 663 | 5 |
| essential64/slab | views-inputs | source | off | 0.797 | 0.791–0.862 | 0.598 | 53.2 | 663 | 5 |
| packed512/enum | materialized | deferred | off | 0.826 | 0.765–0.932 | 0.745 | 74.3 | 482 | 9 |

## relations_free_join, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.054 | 0.052–0.066 | 0.010 | 95.5 | 140 | 9 |
| essential512/enum | materialized | deferred | off | 0.352 | 0.319–0.924 | 0.015 | 79.9 | 345 | 9 |
| essential512/enum | views | deferred | bounded | 0.381 | 0.361–0.394 | 0.009 | 34.5 | 130 | 5 |
| essential512/enum | views | deferred | off | 0.385 | 0.377–0.396 | 0.008 | 34.5 | 130 | 5 |
| essential512/enum | views | source | bounded | 0.341 | 0.311–0.383 | 0.008 | 47.0 | 215 | 5 |
| essential512/enum | views | source | off | 0.301 | 0.282–0.311 | 0.008 | 47.0 | 215 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 0.352 | 0.343–0.546 | 0.008 | 34.4 | 121 | 5 |
| essential512/enum | views-inputs | deferred | off | 0.388 | 0.374–0.396 | 0.008 | 34.4 | 121 | 5 |
| essential512/enum | views-inputs | source | bounded | 0.289 | 0.279–0.344 | 0.009 | 46.6 | 205 | 9 |
| essential512/enum | views-inputs | source | off | 0.265 | 0.245–0.309 | 0.010 | 46.6 | 205 | 9 |
| essential512/slab | materialized | deferred | off | 0.359 | 0.318–0.812 | 0.010 | 56.7 | 345 | 9 |
| essential512/slab | views | deferred | bounded | 0.356 | 0.340–0.390 | 0.008 | 24.9 | 130 | 9 |
| essential512/slab | views | deferred | off | 0.399 | 0.383–0.441 | 0.009 | 24.9 | 130 | 9 |
| essential512/slab | views | source | bounded | 0.320 | 0.295–0.355 | 0.009 | 35.7 | 215 | 9 |
| essential512/slab | views | source | off | 0.305 | 0.271–0.336 | 0.009 | 35.7 | 215 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 0.375 | 0.344–0.459 | 0.008 | 25.2 | 121 | 9 |
| essential512/slab | views-inputs | deferred | off | 0.382 | 0.376–0.409 | 0.008 | 25.2 | 121 | 9 |
| essential512/slab | views-inputs | source | bounded | 0.297 | 0.270–0.330 | 0.009 | 36.1 | 205 | 9 |
| essential512/slab | views-inputs | source | off | 0.257 | 0.247–0.297 | 0.008 | 36.1 | 205 | 9 |
| essential64/enum | materialized | deferred | off | 0.706 | 0.680–0.776 | 0.009 | 180.9 | 1159 | 5 |
| essential64/enum | views | deferred | bounded | 0.491 | 0.474–0.508 | 0.008 | 91.2 | 495 | 5 |
| essential64/enum | views | deferred | off | 0.534 | 0.497–0.586 | 0.008 | 91.2 | 495 | 5 |
| essential64/enum | views | source | bounded | 0.504 | 0.491–0.573 | 0.008 | 100.9 | 699 | 5 |
| essential64/enum | views | source | off | 0.530 | 0.519–0.547 | 0.009 | 100.9 | 699 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 0.626 | 0.526–0.742 | 0.009 | 91.5 | 519 | 5 |
| essential64/enum | views-inputs | deferred | off | 0.511 | 0.505–0.575 | 0.008 | 91.5 | 519 | 5 |
| essential64/enum | views-inputs | source | bounded | 0.519 | 0.481–0.549 | 0.008 | 97.2 | 663 | 5 |
| essential64/enum | views-inputs | source | off | 0.512 | 0.497–0.610 | 0.009 | 97.2 | 663 | 5 |
| essential64/slab | materialized | deferred | off | 0.677 | 0.637–0.712 | 0.009 | 119.1 | 1159 | 5 |
| essential64/slab | views | deferred | bounded | 0.489 | 0.469–0.620 | 0.008 | 58.5 | 495 | 5 |
| essential64/slab | views | deferred | off | 0.516 | 0.504–0.520 | 0.008 | 58.5 | 495 | 5 |
| essential64/slab | views | source | bounded | 0.524 | 0.503–0.547 | 0.008 | 68.8 | 699 | 5 |
| essential64/slab | views | source | off | 0.515 | 0.492–0.558 | 0.008 | 68.8 | 699 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 0.489 | 0.473–0.559 | 0.008 | 58.9 | 519 | 5 |
| essential64/slab | views-inputs | deferred | off | 0.516 | 0.496–0.529 | 0.008 | 58.9 | 519 | 5 |
| essential64/slab | views-inputs | source | bounded | 0.499 | 0.480–0.541 | 0.008 | 65.3 | 663 | 5 |
| essential64/slab | views-inputs | source | off | 0.487 | 0.480–0.495 | 0.010 | 65.3 | 663 | 5 |
| packed512/enum | materialized | deferred | off | 0.242 | 0.233–0.286 | 0.017 | 87.3 | 482 | 9 |

## relations_free_join, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.144 | 0.140–0.179 | 0.139 | 82.5 | 140 | 9 |
| essential512/enum | materialized | deferred | off | 0.749 | 0.696–0.808 | 0.439 | 99.1 | 510 | 9 |
| essential512/enum | views | deferred | bounded | 0.758 | 0.720–0.858 | 0.573 | 47.5 | 251 | 5 |
| essential512/enum | views | deferred | off | 0.687 | 0.657–0.768 | 0.544 | 47.5 | 251 | 5 |
| essential512/enum | views | source | bounded | 0.692 | 0.678–1.986 | 0.423 | 92.9 | 593 | 5 |
| essential512/enum | views | source | off | 0.620 | 0.608–0.696 | 0.357 | 92.9 | 593 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 0.718 | 0.679–0.744 | 0.606 | 22.8 | 156 | 5 |
| essential512/enum | views-inputs | deferred | off | 0.768 | 0.696–0.778 | 0.617 | 22.8 | 156 | 5 |
| essential512/enum | views-inputs | source | bounded | 0.645 | 0.631–0.767 | 0.562 | 58.5 | 343 | 9 |
| essential512/enum | views-inputs | source | off | 0.648 | 0.625–0.710 | 0.532 | 58.5 | 343 | 9 |
| essential512/slab | materialized | deferred | off | 0.742 | 0.691–0.809 | 0.466 | 65.4 | 510 | 9 |
| essential512/slab | views | deferred | bounded | 0.726 | 0.714–0.805 | 0.558 | 31.0 | 251 | 9 |
| essential512/slab | views | deferred | off | 0.677 | 0.671–0.826 | 0.517 | 31.0 | 251 | 9 |
| essential512/slab | views | source | bounded | 0.690 | 0.651–0.774 | 0.413 | 55.2 | 593 | 9 |
| essential512/slab | views | source | off | 0.654 | 0.622–0.718 | 0.396 | 55.2 | 593 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 0.707 | 0.678–0.885 | 0.624 | 13.1 | 156 | 9 |
| essential512/slab | views-inputs | deferred | off | 0.724 | 0.700–0.761 | 0.630 | 13.1 | 156 | 9 |
| essential512/slab | views-inputs | source | bounded | 0.679 | 0.633–0.752 | 0.535 | 36.7 | 343 | 9 |
| essential512/slab | views-inputs | source | off | 0.685 | 0.645–0.725 | 0.527 | 36.7 | 343 | 9 |
| essential64/enum | materialized | deferred | off | 1.524 | 1.475–1.585 | 0.733 | 210.8 | 1342 | 5 |
| essential64/enum | views | deferred | bounded | 3.322 | 3.166–3.425 | 2.589 | 196.9 | 1499 | 5 |
| essential64/enum | views | deferred | off | 3.565 | 3.511–3.661 | 2.968 | 196.9 | 1499 | 5 |
| essential64/enum | views | source | bounded | 1.722 | 1.636–1.751 | 0.703 | 383.1 | 2243 | 5 |
| essential64/enum | views | source | off | 1.749 | 1.673–1.837 | 0.715 | 383.1 | 2243 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 2.431 | 2.368–2.461 | 2.173 | 96.6 | 544 | 5 |
| essential64/enum | views-inputs | deferred | off | 2.788 | 2.707–2.856 | 2.485 | 96.6 | 544 | 5 |
| essential64/enum | views-inputs | source | bounded | 1.352 | 1.322–1.383 | 0.890 | 114.7 | 779 | 5 |
| essential64/enum | views-inputs | source | off | 1.435 | 1.391–1.624 | 0.967 | 114.7 | 779 | 5 |
| essential64/slab | materialized | deferred | off | 1.438 | 1.386–1.747 | 0.708 | 138.8 | 1342 | 5 |
| essential64/slab | views | deferred | bounded | 3.415 | 3.324–7.161 | 2.700 | 127.3 | 1499 | 5 |
| essential64/slab | views | deferred | off | 3.605 | 3.546–3.664 | 2.955 | 127.3 | 1499 | 5 |
| essential64/slab | views | source | bounded | 1.652 | 1.647–1.770 | 0.747 | 246.7 | 2243 | 5 |
| essential64/slab | views | source | off | 1.707 | 1.659–1.897 | 0.748 | 246.7 | 2243 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 2.427 | 2.352–2.522 | 2.142 | 62.9 | 544 | 5 |
| essential64/slab | views-inputs | deferred | off | 2.866 | 2.745–2.927 | 2.525 | 62.9 | 544 | 5 |
| essential64/slab | views-inputs | source | bounded | 1.468 | 1.326–1.560 | 0.921 | 77.2 | 779 | 5 |
| essential64/slab | views-inputs | source | off | 1.426 | 1.394–1.498 | 0.980 | 77.2 | 779 | 5 |
| packed512/enum | materialized | deferred | off | 1.402 | 1.354–1.441 | 1.285 | 291.1 | 1326 | 9 |

## relations_free_join, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.054 | 0.052–0.063 | 0.010 | 95.5 | 140 | 9 |
| essential512/enum | materialized | deferred | off | 0.489 | 0.469–0.579 | 0.009 | 112.1 | 510 | 9 |
| essential512/enum | views | deferred | bounded | 0.434 | 0.417–0.514 | 0.008 | 59.2 | 251 | 5 |
| essential512/enum | views | deferred | off | 0.407 | 0.398–0.429 | 0.008 | 59.2 | 251 | 5 |
| essential512/enum | views | source | bounded | 0.467 | 0.458–0.495 | 0.009 | 104.6 | 593 | 5 |
| essential512/enum | views | source | off | 0.424 | 0.418–0.435 | 0.008 | 104.6 | 593 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 0.378 | 0.370–0.413 | 0.009 | 34.9 | 156 | 5 |
| essential512/enum | views-inputs | deferred | off | 0.389 | 0.380–0.468 | 0.009 | 34.9 | 156 | 5 |
| essential512/enum | views-inputs | source | bounded | 0.379 | 0.369–0.421 | 0.010 | 70.6 | 343 | 9 |
| essential512/enum | views-inputs | source | off | 0.375 | 0.364–0.416 | 0.008 | 70.6 | 343 | 9 |
| essential512/slab | materialized | deferred | off | 0.504 | 0.476–0.567 | 0.009 | 78.4 | 510 | 9 |
| essential512/slab | views | deferred | bounded | 0.423 | 0.420–0.452 | 0.008 | 42.7 | 251 | 9 |
| essential512/slab | views | deferred | off | 0.413 | 0.407–0.464 | 0.008 | 42.7 | 251 | 9 |
| essential512/slab | views | source | bounded | 0.469 | 0.436–0.551 | 0.008 | 66.9 | 593 | 9 |
| essential512/slab | views | source | off | 0.449 | 0.423–0.517 | 0.008 | 66.9 | 593 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 0.398 | 0.375–0.490 | 0.008 | 25.2 | 156 | 9 |
| essential512/slab | views-inputs | deferred | off | 0.384 | 0.374–0.448 | 0.009 | 25.2 | 156 | 9 |
| essential512/slab | views-inputs | source | bounded | 0.383 | 0.371–0.428 | 0.008 | 48.8 | 343 | 9 |
| essential512/slab | views-inputs | source | off | 0.398 | 0.381–0.488 | 0.008 | 48.8 | 343 | 9 |
| essential64/enum | materialized | deferred | off | 1.100 | 1.057–1.139 | 0.009 | 223.8 | 1342 | 5 |
| essential64/enum | views | deferred | bounded | 1.978 | 1.880–1.991 | 0.008 | 208.7 | 1499 | 5 |
| essential64/enum | views | deferred | off | 2.043 | 1.994–2.120 | 0.008 | 208.7 | 1499 | 5 |
| essential64/enum | views | source | bounded | 1.344 | 1.309–1.393 | 0.008 | 394.8 | 2243 | 5 |
| essential64/enum | views | source | off | 1.375 | 1.269–1.405 | 0.008 | 394.8 | 2243 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 1.363 | 1.305–1.439 | 0.008 | 108.8 | 544 | 5 |
| essential64/enum | views-inputs | deferred | off | 1.527 | 1.514–1.570 | 0.008 | 108.8 | 544 | 5 |
| essential64/enum | views-inputs | source | bounded | 0.895 | 0.863–0.992 | 0.008 | 126.8 | 779 | 5 |
| essential64/enum | views-inputs | source | off | 0.934 | 0.917–1.037 | 0.008 | 126.8 | 779 | 5 |
| essential64/slab | materialized | deferred | off | 1.088 | 1.020–1.122 | 0.009 | 151.8 | 1342 | 5 |
| essential64/slab | views | deferred | bounded | 1.957 | 1.936–1.970 | 0.009 | 139.1 | 1499 | 5 |
| essential64/slab | views | deferred | off | 2.066 | 1.964–2.078 | 0.008 | 139.1 | 1499 | 5 |
| essential64/slab | views | source | bounded | 1.319 | 1.248–1.452 | 0.008 | 258.5 | 2243 | 5 |
| essential64/slab | views | source | off | 1.298 | 1.292–1.461 | 0.008 | 258.5 | 2243 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 1.326 | 1.295–1.398 | 0.008 | 75.0 | 544 | 5 |
| essential64/slab | views-inputs | deferred | off | 1.528 | 1.492–1.573 | 0.008 | 75.0 | 544 | 5 |
| essential64/slab | views-inputs | source | bounded | 0.919 | 0.863–0.986 | 0.008 | 89.3 | 779 | 5 |
| essential64/slab | views-inputs | source | off | 0.927 | 0.893–0.987 | 0.009 | 89.3 | 779 | 5 |
| packed512/enum | materialized | deferred | off | 0.505 | 0.481–0.540 | 0.012 | 304.1 | 1326 | 9 |

## relations_free_join, 12 coordinates, pair-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.140 | 0.139–0.207 | 0.136 | 82.5 | 140 | 5 |
| essential512/enum | materialized | deferred | off | 0.571 | 0.523–0.594 | 0.298 | 75.4 | 430 | 5 |
| essential512/enum | views | deferred | bounded | 0.722 | 0.703–0.756 | 0.600 | 34.3 | 189 | 5 |
| essential512/enum | views | deferred | off | 0.761 | 0.712–0.796 | 0.605 | 34.3 | 189 | 5 |
| essential512/enum | views | source | bounded | 0.638 | 0.570–0.665 | 0.377 | 68.9 | 430 | 5 |
| essential512/enum | views | source | off | 0.544 | 0.534–0.623 | 0.337 | 68.9 | 430 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 0.722 | 0.677–0.751 | 0.588 | 22.4 | 128 | 5 |
| essential512/enum | views-inputs | deferred | off | 0.749 | 0.694–0.799 | 0.641 | 22.4 | 128 | 5 |
| essential512/enum | views-inputs | source | bounded | 0.608 | 0.557–0.632 | 0.429 | 44.9 | 285 | 5 |
| essential512/enum | views-inputs | source | off | 0.553 | 0.513–0.588 | 0.395 | 44.9 | 285 | 5 |
| essential512/slab | materialized | deferred | off | 0.552 | 0.526–0.660 | 0.302 | 47.5 | 430 | 5 |
| essential512/slab | views | deferred | bounded | 0.744 | 0.716–0.768 | 0.598 | 22.0 | 189 | 5 |
| essential512/slab | views | deferred | off | 0.724 | 0.706–0.770 | 0.611 | 22.0 | 189 | 5 |
| essential512/slab | views | source | bounded | 0.602 | 0.564–0.674 | 0.408 | 47.5 | 430 | 5 |
| essential512/slab | views | source | off | 0.555 | 0.542–0.619 | 0.329 | 47.5 | 430 | 5 |
| essential512/slab | views-inputs | deferred | bounded | 0.695 | 0.679–0.743 | 0.624 | 13.1 | 128 | 5 |
| essential512/slab | views-inputs | deferred | off | 0.753 | 0.696–0.828 | 0.638 | 13.1 | 128 | 5 |
| essential512/slab | views-inputs | source | bounded | 0.555 | 0.541–0.623 | 0.414 | 27.7 | 285 | 5 |
| essential512/slab | views-inputs | source | off | 0.536 | 0.524–0.575 | 0.405 | 27.7 | 285 | 5 |
| essential64/enum | materialized | deferred | off | 1.404 | 1.337–1.428 | 0.688 | 211.6 | 1392 | 5 |
| essential64/enum | views | deferred | bounded | 2.222 | 2.130–2.259 | 1.856 | 98.3 | 714 | 5 |
| essential64/enum | views | deferred | off | 2.809 | 2.693–2.895 | 2.519 | 98.3 | 714 | 5 |
| essential64/enum | views | source | bounded | 1.387 | 1.354–1.553 | 0.773 | 208.3 | 1131 | 5 |
| essential64/enum | views | source | off | 1.434 | 1.308–1.454 | 0.767 | 208.3 | 1131 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 2.147 | 2.098–2.195 | 1.854 | 98.7 | 672 | 5 |
| essential64/enum | views-inputs | deferred | off | 2.673 | 2.597–2.786 | 2.414 | 98.7 | 672 | 5 |
| essential64/enum | views-inputs | source | bounded | 1.220 | 1.168–1.368 | 0.804 | 108.0 | 834 | 5 |
| essential64/enum | views-inputs | source | off | 1.270 | 1.264–1.335 | 0.825 | 108.0 | 834 | 5 |
| essential64/slab | materialized | deferred | off | 1.349 | 1.285–1.381 | 0.714 | 138.8 | 1392 | 5 |
| essential64/slab | views | deferred | bounded | 2.196 | 2.158–2.310 | 1.989 | 62.4 | 714 | 5 |
| essential64/slab | views | deferred | off | 2.832 | 2.817–3.065 | 2.477 | 62.4 | 714 | 5 |
| essential64/slab | views | source | bounded | 1.427 | 1.407–1.589 | 0.764 | 138.8 | 1131 | 5 |
| essential64/slab | views | source | off | 1.405 | 1.328–1.421 | 0.791 | 138.8 | 1131 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 2.259 | 2.132–2.492 | 1.886 | 62.9 | 672 | 5 |
| essential64/slab | views-inputs | deferred | off | 2.714 | 2.657–2.780 | 2.377 | 62.9 | 672 | 5 |
| essential64/slab | views-inputs | source | bounded | 1.218 | 1.151–1.364 | 0.814 | 74.5 | 834 | 5 |
| essential64/slab | views-inputs | source | off | 1.236 | 1.182–1.342 | 0.852 | 74.5 | 834 | 5 |
| packed512/enum | materialized | deferred | off | 1.336 | 1.295–1.409 | 1.269 | 190.0 | 1107 | 5 |

## relations_free_join, 12 coordinates, pair-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.059 | 0.057–0.068 | 0.010 | 95.5 | 140 | 5 |
| essential512/enum | materialized | deferred | off | 0.379 | 0.367–0.402 | 0.009 | 88.4 | 430 | 5 |
| essential512/enum | views | deferred | bounded | 0.404 | 0.398–0.437 | 0.008 | 46.0 | 189 | 5 |
| essential512/enum | views | deferred | off | 0.413 | 0.399–0.470 | 0.008 | 46.0 | 189 | 5 |
| essential512/enum | views | source | bounded | 0.377 | 0.371–0.410 | 0.009 | 80.7 | 430 | 5 |
| essential512/enum | views | source | off | 0.395 | 0.361–0.430 | 0.008 | 80.7 | 430 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 0.375 | 0.363–0.425 | 0.009 | 34.5 | 128 | 5 |
| essential512/enum | views-inputs | deferred | off | 0.413 | 0.379–0.445 | 0.008 | 34.5 | 128 | 5 |
| essential512/enum | views-inputs | source | bounded | 0.351 | 0.331–0.358 | 0.008 | 57.0 | 285 | 5 |
| essential512/enum | views-inputs | source | off | 0.323 | 0.316–0.346 | 0.010 | 57.0 | 285 | 5 |
| essential512/slab | materialized | deferred | off | 0.405 | 0.380–0.425 | 0.009 | 60.5 | 430 | 5 |
| essential512/slab | views | deferred | bounded | 0.417 | 0.405–0.469 | 0.009 | 33.8 | 189 | 5 |
| essential512/slab | views | deferred | off | 0.414 | 0.405–0.438 | 0.008 | 33.8 | 189 | 5 |
| essential512/slab | views | source | bounded | 0.384 | 0.374–0.399 | 0.008 | 59.3 | 430 | 5 |
| essential512/slab | views | source | off | 0.386 | 0.376–0.452 | 0.008 | 59.3 | 430 | 5 |
| essential512/slab | views-inputs | deferred | bounded | 0.400 | 0.376–0.413 | 0.009 | 25.2 | 128 | 5 |
| essential512/slab | views-inputs | deferred | off | 0.394 | 0.376–0.416 | 0.008 | 25.2 | 128 | 5 |
| essential512/slab | views-inputs | source | bounded | 0.352 | 0.340–0.422 | 0.009 | 39.9 | 285 | 5 |
| essential512/slab | views-inputs | source | off | 0.323 | 0.316–0.357 | 0.008 | 39.9 | 285 | 5 |
| essential64/enum | materialized | deferred | off | 1.037 | 0.973–1.045 | 0.010 | 224.6 | 1392 | 5 |
| essential64/enum | views | deferred | bounded | 1.210 | 1.194–1.229 | 0.009 | 110.1 | 714 | 5 |
| essential64/enum | views | deferred | off | 1.564 | 1.462–1.582 | 0.008 | 110.1 | 714 | 5 |
| essential64/enum | views | source | bounded | 0.997 | 0.970–1.069 | 0.008 | 220.1 | 1131 | 5 |
| essential64/enum | views | source | off | 0.981 | 0.953–1.002 | 0.008 | 220.1 | 1131 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 1.198 | 1.175–1.225 | 0.008 | 110.8 | 672 | 5 |
| essential64/enum | views-inputs | deferred | off | 1.484 | 1.435–1.527 | 0.009 | 110.8 | 672 | 5 |
| essential64/enum | views-inputs | source | bounded | 0.761 | 0.735–0.827 | 0.008 | 120.1 | 834 | 5 |
| essential64/enum | views-inputs | source | off | 0.788 | 0.774–0.861 | 0.008 | 120.1 | 834 | 5 |
| essential64/slab | materialized | deferred | off | 0.957 | 0.923–0.999 | 0.010 | 151.8 | 1392 | 5 |
| essential64/slab | views | deferred | bounded | 1.296 | 1.245–1.534 | 0.008 | 74.2 | 714 | 5 |
| essential64/slab | views | deferred | off | 1.617 | 1.566–1.671 | 0.008 | 74.2 | 714 | 5 |
| essential64/slab | views | source | bounded | 0.973 | 0.918–1.005 | 0.008 | 150.5 | 1131 | 5 |
| essential64/slab | views | source | off | 0.956 | 0.944–1.069 | 0.008 | 150.5 | 1131 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 1.196 | 1.192–1.226 | 0.008 | 75.0 | 672 | 5 |
| essential64/slab | views-inputs | deferred | off | 1.472 | 1.435–1.538 | 0.009 | 75.0 | 672 | 5 |
| essential64/slab | views-inputs | source | bounded | 0.734 | 0.732–0.807 | 0.009 | 86.6 | 834 | 5 |
| essential64/slab | views-inputs | source | off | 0.789 | 0.761–0.827 | 0.008 | 86.6 | 834 | 5 |
| packed512/enum | materialized | deferred | off | 0.457 | 0.449–0.472 | 0.013 | 203.0 | 1107 | 5 |

## relations_free_join, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 6.927 | 6.869–7.182 | 6.959 | 5810.7 | 175 | 9 |
| essential512/enum | materialized | deferred | off | 3.392 | 3.216–6.443 | 1.718 | 450.5 | 2668 | 9 |
| essential512/enum | views | deferred | bounded | 12.187 | 10.180–48.584 | 9.596 | 195.5 | 1011 | 5 |
| essential512/enum | views | deferred | off | 14.962 | 14.860–15.179 | 14.244 | 195.5 | 1011 | 5 |
| essential512/enum | views | source | bounded | 3.301 | 3.228–3.371 | 1.829 | 347.3 | 1776 | 5 |
| essential512/enum | views | source | off | 3.220 | 3.205–3.460 | 1.813 | 347.3 | 1776 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 10.411 | 10.255–10.450 | 9.879 | 195.0 | 1024 | 5 |
| essential512/enum | views-inputs | deferred | off | 14.514 | 14.430–15.228 | 14.152 | 195.0 | 1024 | 5 |
| essential512/enum | views-inputs | source | bounded | 3.027 | 2.931–3.201 | 1.928 | 302.5 | 1645 | 9 |
| essential512/enum | views-inputs | source | off | 3.030 | 2.923–11.692 | 2.098 | 302.5 | 1645 | 9 |
| essential512/slab | materialized | deferred | off | 3.440 | 3.141–6.012 | 1.604 | 288.6 | 2668 | 9 |
| essential512/slab | views | deferred | bounded | 10.075 | 9.839–10.344 | 9.513 | 131.2 | 1011 | 9 |
| essential512/slab | views | deferred | off | 15.298 | 14.938–15.463 | 14.364 | 131.2 | 1011 | 9 |
| essential512/slab | views | source | bounded | 3.350 | 3.168–3.466 | 1.901 | 231.9 | 1776 | 9 |
| essential512/slab | views | source | off | 3.319 | 3.200–3.416 | 1.866 | 231.9 | 1776 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 10.477 | 10.313–10.674 | 9.893 | 131.2 | 1024 | 9 |
| essential512/slab | views-inputs | deferred | off | 14.759 | 14.183–15.093 | 14.121 | 131.2 | 1024 | 9 |
| essential512/slab | views-inputs | source | bounded | 3.041 | 2.897–3.201 | 1.960 | 201.5 | 1645 | 9 |
| essential512/slab | views-inputs | source | off | 2.940 | 2.870–3.087 | 1.891 | 201.5 | 1645 | 9 |
| essential64/enum | materialized | deferred | off | 6.431 | 4.889–48.832 | 2.787 | 879.5 | 5699 | 5 |
| essential64/enum | views | deferred | bounded | 5.887 | 5.824–6.011 | 4.996 | 415.1 | 2640 | 5 |
| essential64/enum | views | deferred | off | 7.214 | 7.131–7.570 | 6.129 | 415.1 | 2640 | 5 |
| essential64/enum | views | source | bounded | 4.444 | 4.228–4.502 | 3.166 | 476.9 | 3577 | 5 |
| essential64/enum | views | source | off | 4.529 | 4.500–4.682 | 3.142 | 476.9 | 3577 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 7.341 | 6.246–10.160 | 5.519 | 414.0 | 2863 | 5 |
| essential64/enum | views-inputs | deferred | off | 6.663 | 6.477–6.719 | 5.559 | 414.0 | 2863 | 5 |
| essential64/enum | views-inputs | source | bounded | 4.111 | 4.023–4.179 | 2.703 | 443.7 | 3294 | 5 |
| essential64/enum | views-inputs | source | off | 4.179 | 4.115–4.369 | 2.835 | 443.7 | 3294 | 5 |
| essential64/slab | materialized | deferred | off | 4.674 | 4.551–4.847 | 2.614 | 595.7 | 5699 | 5 |
| essential64/slab | views | deferred | bounded | 5.913 | 5.809–6.062 | 4.886 | 275.3 | 2640 | 5 |
| essential64/slab | views | deferred | off | 7.104 | 6.937–7.212 | 6.177 | 275.3 | 2640 | 5 |
| essential64/slab | views | source | bounded | 4.379 | 4.211–4.428 | 2.946 | 328.6 | 3577 | 5 |
| essential64/slab | views | source | off | 4.616 | 4.450–5.089 | 3.039 | 328.6 | 3577 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 5.557 | 5.435–5.687 | 4.583 | 264.4 | 2863 | 5 |
| essential64/slab | views-inputs | deferred | off | 6.627 | 6.539–6.744 | 5.633 | 264.4 | 2863 | 5 |
| essential64/slab | views-inputs | source | bounded | 4.042 | 3.919–4.155 | 2.758 | 298.1 | 3294 | 5 |
| essential64/slab | views-inputs | source | off | 4.169 | 4.053–4.384 | 3.075 | 298.1 | 3294 | 5 |
| packed512/enum | materialized | deferred | off | 1.527 | 1.459–1.594 | 1.177 | 499.4 | 3815 | 9 |

## relations_free_join, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.365 | 2.319–2.447 | 0.048 | 5823.7 | 175 | 9 |
| essential512/enum | materialized | deferred | off | 2.531 | 2.435–2.730 | 0.010 | 463.5 | 2668 | 9 |
| essential512/enum | views | deferred | bounded | 5.396 | 5.324–5.486 | 0.009 | 207.2 | 1011 | 5 |
| essential512/enum | views | deferred | off | 7.840 | 7.811–8.017 | 0.009 | 207.2 | 1011 | 5 |
| essential512/enum | views | source | bounded | 2.404 | 2.286–2.456 | 0.008 | 359.1 | 1776 | 5 |
| essential512/enum | views | source | off | 2.327 | 2.258–2.368 | 0.008 | 359.1 | 1776 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 5.565 | 5.355–5.773 | 0.009 | 207.5 | 1024 | 5 |
| essential512/enum | views-inputs | deferred | off | 7.590 | 7.531–7.854 | 0.009 | 207.5 | 1024 | 5 |
| essential512/enum | views-inputs | source | bounded | 2.050 | 1.924–2.141 | 0.009 | 315.0 | 1645 | 9 |
| essential512/enum | views-inputs | source | off | 2.041 | 1.958–21.969 | 0.009 | 315.0 | 1645 | 9 |
| essential512/slab | materialized | deferred | off | 2.342 | 2.315–2.566 | 0.010 | 301.6 | 2668 | 9 |
| essential512/slab | views | deferred | bounded | 5.337 | 5.172–5.549 | 0.008 | 143.0 | 1011 | 9 |
| essential512/slab | views | deferred | off | 7.897 | 7.795–8.021 | 0.009 | 143.0 | 1011 | 9 |
| essential512/slab | views | source | bounded | 2.290 | 2.277–2.418 | 0.009 | 243.7 | 1776 | 9 |
| essential512/slab | views | source | off | 2.367 | 2.285–2.572 | 0.009 | 243.7 | 1776 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 5.565 | 5.333–5.635 | 0.009 | 143.8 | 1024 | 9 |
| essential512/slab | views-inputs | deferred | off | 7.574 | 7.283–8.007 | 0.009 | 143.8 | 1024 | 9 |
| essential512/slab | views-inputs | source | bounded | 2.032 | 1.916–2.116 | 0.009 | 214.0 | 1645 | 9 |
| essential512/slab | views-inputs | source | off | 2.001 | 1.867–2.129 | 0.009 | 214.0 | 1645 | 9 |
| essential64/enum | materialized | deferred | off | 3.482 | 3.473–3.490 | 0.010 | 892.5 | 5699 | 5 |
| essential64/enum | views | deferred | bounded | 3.339 | 3.295–3.366 | 0.009 | 426.9 | 2640 | 5 |
| essential64/enum | views | deferred | off | 4.092 | 3.992–5.033 | 0.009 | 426.9 | 2640 | 5 |
| essential64/enum | views | source | bounded | 2.904 | 2.870–3.015 | 0.009 | 488.7 | 3577 | 5 |
| essential64/enum | views | source | off | 2.877 | 2.873–2.918 | 0.008 | 488.7 | 3577 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 3.546 | 3.362–4.435 | 0.009 | 426.5 | 2863 | 5 |
| essential64/enum | views-inputs | deferred | off | 3.782 | 3.637–3.864 | 0.009 | 426.5 | 2863 | 5 |
| essential64/enum | views-inputs | source | bounded | 2.578 | 2.517–2.603 | 0.008 | 456.3 | 3294 | 5 |
| essential64/enum | views-inputs | source | off | 2.668 | 2.610–3.047 | 0.009 | 456.3 | 3294 | 5 |
| essential64/slab | materialized | deferred | off | 3.386 | 3.345–3.494 | 0.010 | 608.7 | 5699 | 5 |
| essential64/slab | views | deferred | bounded | 3.328 | 3.224–3.373 | 0.009 | 287.0 | 2640 | 5 |
| essential64/slab | views | deferred | off | 3.932 | 3.840–4.034 | 0.010 | 287.0 | 2640 | 5 |
| essential64/slab | views | source | bounded | 2.757 | 2.730–2.813 | 0.008 | 340.3 | 3577 | 5 |
| essential64/slab | views | source | off | 2.920 | 2.830–2.984 | 0.009 | 340.3 | 3577 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 3.159 | 3.065–3.368 | 0.009 | 277.0 | 2863 | 5 |
| essential64/slab | views-inputs | deferred | off | 3.743 | 3.634–3.765 | 0.009 | 277.0 | 2863 | 5 |
| essential64/slab | views-inputs | source | bounded | 2.576 | 2.527–2.728 | 0.008 | 310.7 | 3294 | 5 |
| essential64/slab | views-inputs | source | off | 2.684 | 2.645–2.773 | 0.008 | 310.7 | 3294 | 5 |
| packed512/enum | materialized | deferred | off | 0.738 | 0.715–0.896 | 0.040 | 512.4 | 3815 | 9 |

## relations_free_join, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 7.104 | 6.785–7.438 | 6.940 | 5810.7 | 175 | 9 |
| essential512/enum | materialized | deferred | off | 10.268 | 9.736–10.927 | 5.814 | 1043.6 | 5273 | 9 |
| essential512/enum | views | deferred | bounded | 56.836 | 56.515–57.255 | 52.597 | 1722.5 | 8088 | 5 |
| essential512/enum | views | deferred | off | 95.888 | 94.473–99.479 | 91.861 | 1722.5 | 8088 | 5 |
| essential512/enum | views | source | bounded | 12.932 | 10.323–82.196 | 3.933 | 2098.2 | 11800 | 5 |
| essential512/enum | views | source | off | 10.901 | 10.814–11.321 | 4.555 | 2098.2 | 11800 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 35.744 | 35.042–35.885 | 34.352 | 263.4 | 1642 | 5 |
| essential512/enum | views-inputs | deferred | off | 43.916 | 43.471–45.171 | 42.693 | 263.4 | 1642 | 5 |
| essential512/enum | views-inputs | source | bounded | 8.667 | 8.446–9.194 | 6.264 | 738.7 | 3752 | 9 |
| essential512/enum | views-inputs | source | off | 10.059 | 9.647–10.565 | 8.026 | 738.7 | 3752 | 9 |
| essential512/slab | materialized | deferred | off | 10.011 | 9.726–10.498 | 5.889 | 655.1 | 5273 | 9 |
| essential512/slab | views | deferred | bounded | 54.586 | 53.637–55.922 | 50.674 | 1096.4 | 8088 | 9 |
| essential512/slab | views | deferred | off | 96.078 | 95.086–97.974 | 92.479 | 1096.4 | 8088 | 9 |
| essential512/slab | views | source | bounded | 10.217 | 10.045–10.712 | 4.032 | 1561.6 | 11800 | 9 |
| essential512/slab | views | source | off | 10.880 | 10.516–11.569 | 4.597 | 1561.6 | 11800 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 35.587 | 34.970–36.112 | 34.561 | 182.3 | 1642 | 9 |
| essential512/slab | views-inputs | deferred | off | 44.417 | 44.012–46.278 | 43.173 | 182.3 | 1642 | 9 |
| essential512/slab | views-inputs | source | bounded | 8.543 | 8.383–9.025 | 6.193 | 512.6 | 3752 | 9 |
| essential512/slab | views-inputs | source | off | 9.862 | 9.744–10.203 | 7.707 | 512.6 | 3752 | 9 |
| essential64/enum | materialized | deferred | off | 26.354 | 25.318–26.974 | 16.762 | 1866.6 | 9513 | 5 |
| essential64/enum | views | deferred | bounded | 34.272 | 33.769–34.916 | 28.527 | 2402.2 | 14862 | 5 |
| essential64/enum | views | deferred | off | 35.371 | 34.568–35.648 | 29.420 | 2402.2 | 14862 | 5 |
| essential64/enum | views | source | bounded | 17.261 | 17.056–18.973 | 5.959 | 3616.1 | 23097 | 5 |
| essential64/enum | views | source | off | 16.892 | 16.745–18.322 | 5.779 | 3616.1 | 23097 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 34.130 | 33.842–34.693 | 30.266 | 911.1 | 4924 | 5 |
| essential64/enum | views-inputs | deferred | off | 39.756 | 39.501–40.660 | 35.975 | 911.1 | 4924 | 5 |
| essential64/enum | views-inputs | source | bounded | 23.272 | 22.810–23.374 | 18.711 | 978.6 | 5814 | 5 |
| essential64/enum | views-inputs | source | off | 27.241 | 26.648–27.536 | 22.274 | 978.6 | 5814 | 5 |
| essential64/slab | materialized | deferred | off | 25.585 | 25.437–27.430 | 16.516 | 1308.0 | 9513 | 5 |
| essential64/slab | views | deferred | bounded | 38.408 | 35.265–57.790 | 28.803 | 1500.5 | 14862 | 5 |
| essential64/slab | views | deferred | off | 35.757 | 35.515–37.796 | 30.500 | 1500.5 | 14862 | 5 |
| essential64/slab | views | source | bounded | 16.740 | 16.607–16.919 | 5.876 | 2493.7 | 23097 | 5 |
| essential64/slab | views | source | off | 16.743 | 16.597–18.399 | 5.877 | 2493.7 | 23097 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 34.309 | 33.962–34.927 | 30.331 | 631.4 | 4924 | 5 |
| essential64/slab | views-inputs | deferred | off | 39.842 | 38.996–40.019 | 36.289 | 631.4 | 4924 | 5 |
| essential64/slab | views-inputs | source | bounded | 23.405 | 22.469–23.537 | 18.814 | 684.7 | 5814 | 5 |
| essential64/slab | views-inputs | source | off | 27.050 | 26.331–27.892 | 22.611 | 684.7 | 5814 | 5 |
| packed512/enum | materialized | deferred | off | 5.346 | 5.201–5.516 | 4.808 | 1200.6 | 7882 | 9 |

## relations_free_join, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.441 | 2.255–2.552 | 0.048 | 5823.7 | 175 | 9 |
| essential512/enum | materialized | deferred | off | 7.015 | 6.839–7.143 | 0.010 | 1056.6 | 5273 | 9 |
| essential512/enum | views | deferred | bounded | 29.235 | 29.073–31.272 | 0.009 | 1734.3 | 8088 | 5 |
| essential512/enum | views | deferred | off | 49.524 | 49.225–50.834 | 0.009 | 1734.3 | 8088 | 5 |
| essential512/enum | views | source | bounded | 8.521 | 8.289–36.272 | 0.009 | 2110.0 | 11800 | 5 |
| essential512/enum | views | source | off | 8.472 | 8.260–8.760 | 0.009 | 2110.0 | 11800 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 18.090 | 17.875–18.286 | 0.009 | 275.9 | 1642 | 5 |
| essential512/enum | views-inputs | deferred | off | 22.387 | 22.076–22.673 | 0.009 | 275.9 | 1642 | 5 |
| essential512/enum | views-inputs | source | bounded | 5.379 | 5.228–5.656 | 0.009 | 751.2 | 3752 | 9 |
| essential512/enum | views-inputs | source | off | 6.230 | 6.068–11.647 | 0.024 | 751.2 | 3752 | 9 |
| essential512/slab | materialized | deferred | off | 6.931 | 6.861–7.306 | 0.010 | 668.1 | 5273 | 9 |
| essential512/slab | views | deferred | bounded | 29.197 | 28.487–29.548 | 0.008 | 1108.2 | 8088 | 9 |
| essential512/slab | views | deferred | off | 50.445 | 48.884–51.218 | 0.008 | 1108.2 | 8088 | 9 |
| essential512/slab | views | source | bounded | 8.194 | 7.890–8.522 | 0.009 | 1573.4 | 11800 | 9 |
| essential512/slab | views | source | off | 8.467 | 8.238–8.765 | 0.009 | 1573.4 | 11800 | 9 |
| essential512/slab | views-inputs | deferred | bounded | 18.254 | 17.667–18.874 | 0.008 | 194.9 | 1642 | 9 |
| essential512/slab | views-inputs | deferred | off | 22.443 | 22.093–22.932 | 0.009 | 194.9 | 1642 | 9 |
| essential512/slab | views-inputs | source | bounded | 5.272 | 5.115–5.466 | 0.009 | 525.2 | 3752 | 9 |
| essential512/slab | views-inputs | source | off | 5.934 | 5.855–6.306 | 0.009 | 525.2 | 3752 | 9 |
| essential64/enum | materialized | deferred | off | 17.765 | 17.566–18.058 | 0.010 | 1879.6 | 9513 | 5 |
| essential64/enum | views | deferred | bounded | 20.181 | 19.720–21.011 | 0.009 | 2414.0 | 14862 | 5 |
| essential64/enum | views | deferred | off | 20.565 | 19.800–20.780 | 0.009 | 2414.0 | 14862 | 5 |
| essential64/enum | views | source | bounded | 13.939 | 13.551–14.124 | 0.009 | 3627.9 | 23097 | 5 |
| essential64/enum | views | source | off | 13.639 | 13.338–14.283 | 0.009 | 3627.9 | 23097 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 18.439 | 18.120–18.614 | 0.008 | 923.7 | 4924 | 5 |
| essential64/enum | views-inputs | deferred | off | 21.344 | 20.885–21.537 | 0.009 | 923.7 | 4924 | 5 |
| essential64/enum | views-inputs | source | bounded | 13.466 | 13.367–13.648 | 0.009 | 991.2 | 5814 | 5 |
| essential64/enum | views-inputs | source | off | 15.753 | 15.321–16.020 | 0.009 | 991.2 | 5814 | 5 |
| essential64/slab | materialized | deferred | off | 17.266 | 17.237–17.535 | 0.011 | 1321.0 | 9513 | 5 |
| essential64/slab | views | deferred | bounded | 20.362 | 20.239–20.538 | 0.009 | 1512.3 | 14862 | 5 |
| essential64/slab | views | deferred | off | 20.747 | 20.471–20.769 | 0.009 | 1512.3 | 14862 | 5 |
| essential64/slab | views | source | bounded | 13.442 | 13.253–13.776 | 0.009 | 2505.5 | 23097 | 5 |
| essential64/slab | views | source | off | 13.815 | 13.495–13.915 | 0.009 | 2505.5 | 23097 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 18.677 | 18.188–18.712 | 0.009 | 644.0 | 4924 | 5 |
| essential64/slab | views-inputs | deferred | off | 21.202 | 20.854–21.730 | 0.009 | 644.0 | 4924 | 5 |
| essential64/slab | views-inputs | source | bounded | 13.549 | 13.486–13.633 | 0.009 | 697.3 | 5814 | 5 |
| essential64/slab | views-inputs | source | off | 15.401 | 15.348–16.006 | 0.009 | 697.3 | 5814 | 5 |
| packed512/enum | materialized | deferred | off | 2.310 | 2.255–2.407 | 0.032 | 1213.6 | 7882 | 9 |

## relations_free_join, 18 coordinates, pair-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 7.124 | 6.768–7.337 | 7.074 | 5810.7 | 175 | 5 |
| essential512/enum | materialized | deferred | off | 4.061 | 3.958–4.224 | 2.097 | 543.8 | 3223 | 5 |
| essential512/enum | views | deferred | bounded | 11.717 | 11.577–11.876 | 10.737 | 235.1 | 1496 | 5 |
| essential512/enum | views | deferred | off | 15.915 | 15.613–16.245 | 15.162 | 235.1 | 1496 | 5 |
| essential512/enum | views | source | bounded | 4.288 | 4.185–4.423 | 2.439 | 480.2 | 3107 | 5 |
| essential512/enum | views | source | off | 4.347 | 4.326–4.418 | 2.470 | 480.2 | 3107 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 10.845 | 10.584–10.988 | 10.357 | 217.2 | 1192 | 5 |
| essential512/enum | views-inputs | deferred | off | 14.871 | 14.591–15.359 | 14.347 | 217.2 | 1192 | 5 |
| essential512/enum | views-inputs | source | bounded | 3.682 | 3.555–3.790 | 2.545 | 409.1 | 2096 | 5 |
| essential512/enum | views-inputs | source | off | 3.936 | 3.742–5.095 | 2.668 | 409.1 | 2096 | 5 |
| essential512/slab | materialized | deferred | off | 3.964 | 3.929–4.123 | 2.113 | 358.5 | 3223 | 5 |
| essential512/slab | views | deferred | bounded | 11.638 | 11.407–11.860 | 10.668 | 151.0 | 1496 | 5 |
| essential512/slab | views | deferred | off | 15.947 | 15.679–16.040 | 15.075 | 151.0 | 1496 | 5 |
| essential512/slab | views | source | bounded | 4.200 | 4.056–4.326 | 2.418 | 328.1 | 3107 | 5 |
| essential512/slab | views | source | off | 4.281 | 4.063–4.427 | 2.596 | 328.1 | 3107 | 5 |
| essential512/slab | views-inputs | deferred | bounded | 10.981 | 10.900–11.111 | 10.279 | 151.0 | 1192 | 5 |
| essential512/slab | views-inputs | deferred | off | 14.885 | 14.625–15.496 | 14.407 | 151.0 | 1192 | 5 |
| essential512/slab | views-inputs | source | bounded | 3.694 | 3.607–3.976 | 2.541 | 256.1 | 2096 | 5 |
| essential512/slab | views-inputs | source | off | 3.688 | 3.676–3.849 | 2.685 | 256.1 | 2096 | 5 |
| essential64/enum | materialized | deferred | off | 6.625 | 6.473–7.077 | 3.444 | 927.0 | 6713 | 5 |
| essential64/enum | views | deferred | bounded | 13.269 | 13.121–13.606 | 11.803 | 588.5 | 3722 | 5 |
| essential64/enum | views | deferred | off | 17.785 | 17.304–18.263 | 16.052 | 588.5 | 3722 | 5 |
| essential64/enum | views | source | bounded | 6.908 | 6.864–7.439 | 3.934 | 919.5 | 5643 | 5 |
| essential64/enum | views | source | off | 7.007 | 6.912–7.429 | 4.105 | 919.5 | 5643 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 12.512 | 12.485–12.663 | 11.134 | 440.3 | 3522 | 5 |
| essential64/enum | views-inputs | deferred | off | 16.057 | 15.523–16.240 | 14.430 | 440.3 | 3522 | 5 |
| essential64/enum | views-inputs | source | bounded | 5.900 | 5.781–6.320 | 4.097 | 617.3 | 3974 | 5 |
| essential64/enum | views-inputs | source | off | 6.273 | 5.984–6.327 | 4.430 | 617.3 | 3974 | 5 |
| essential64/slab | materialized | deferred | off | 6.475 | 6.354–6.871 | 3.513 | 631.4 | 6713 | 5 |
| essential64/slab | views | deferred | bounded | 13.382 | 13.302–14.273 | 11.778 | 354.0 | 3722 | 5 |
| essential64/slab | views | deferred | off | 18.159 | 17.941–18.611 | 16.185 | 354.0 | 3722 | 5 |
| essential64/slab | views | source | bounded | 6.875 | 6.689–6.952 | 3.918 | 605.1 | 5643 | 5 |
| essential64/slab | views | source | off | 7.056 | 6.885–7.278 | 4.240 | 605.1 | 5643 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 12.451 | 12.256–12.612 | 10.913 | 293.1 | 3522 | 5 |
| essential64/slab | views-inputs | deferred | off | 16.171 | 16.052–16.400 | 14.488 | 293.1 | 3522 | 5 |
| essential64/slab | views-inputs | source | bounded | 5.885 | 5.780–5.950 | 3.921 | 376.9 | 3974 | 5 |
| essential64/slab | views-inputs | source | off | 6.139 | 6.067–6.417 | 4.322 | 376.9 | 3974 | 5 |
| packed512/enum | materialized | deferred | off | 3.933 | 3.868–4.010 | 3.450 | 851.2 | 5840 | 5 |

## relations_free_join, 18 coordinates, pair-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.343 | 2.280–2.466 | 0.049 | 5823.7 | 175 | 5 |
| essential512/enum | materialized | deferred | off | 3.072 | 2.914–3.164 | 0.009 | 556.8 | 3223 | 5 |
| essential512/enum | views | deferred | bounded | 6.250 | 6.020–6.291 | 0.008 | 246.9 | 1496 | 5 |
| essential512/enum | views | deferred | off | 8.418 | 8.386–8.464 | 0.008 | 246.9 | 1496 | 5 |
| essential512/enum | views | source | bounded | 2.911 | 2.872–2.963 | 0.009 | 492.0 | 3107 | 5 |
| essential512/enum | views | source | off | 3.075 | 2.970–3.217 | 0.009 | 492.0 | 3107 | 5 |
| essential512/enum | views-inputs | deferred | bounded | 5.764 | 5.581–6.043 | 0.009 | 229.8 | 1192 | 5 |
| essential512/enum | views-inputs | deferred | off | 7.814 | 7.717–8.044 | 0.009 | 229.8 | 1192 | 5 |
| essential512/enum | views-inputs | source | bounded | 2.474 | 2.405–2.527 | 0.008 | 421.7 | 2096 | 5 |
| essential512/enum | views-inputs | source | off | 2.426 | 2.385–2.508 | 0.008 | 421.7 | 2096 | 5 |
| essential512/slab | materialized | deferred | off | 2.877 | 2.846–3.102 | 0.010 | 371.5 | 3223 | 5 |
| essential512/slab | views | deferred | bounded | 6.230 | 6.106–6.880 | 0.008 | 162.7 | 1496 | 5 |
| essential512/slab | views | deferred | off | 8.383 | 8.306–8.479 | 0.009 | 162.7 | 1496 | 5 |
| essential512/slab | views | source | bounded | 2.987 | 2.933–3.049 | 0.009 | 339.8 | 3107 | 5 |
| essential512/slab | views | source | off | 2.936 | 2.856–3.074 | 0.008 | 339.8 | 3107 | 5 |
| essential512/slab | views-inputs | deferred | bounded | 5.832 | 5.523–5.942 | 0.009 | 163.5 | 1192 | 5 |
| essential512/slab | views-inputs | deferred | off | 7.704 | 7.609–7.904 | 0.009 | 163.5 | 1192 | 5 |
| essential512/slab | views-inputs | source | bounded | 2.341 | 2.302–2.379 | 0.009 | 268.7 | 2096 | 5 |
| essential512/slab | views-inputs | source | off | 2.497 | 2.355–2.808 | 0.009 | 268.7 | 2096 | 5 |
| essential64/enum | materialized | deferred | off | 4.620 | 4.537–4.786 | 0.013 | 940.0 | 6713 | 5 |
| essential64/enum | views | deferred | bounded | 7.467 | 7.375–7.779 | 0.009 | 600.3 | 3722 | 5 |
| essential64/enum | views | deferred | off | 9.530 | 9.465–9.697 | 0.009 | 600.3 | 3722 | 5 |
| essential64/enum | views | source | bounded | 4.900 | 4.807–4.939 | 0.009 | 931.3 | 5643 | 5 |
| essential64/enum | views | source | off | 4.927 | 4.828–5.000 | 0.009 | 931.3 | 5643 | 5 |
| essential64/enum | views-inputs | deferred | bounded | 6.767 | 6.704–7.203 | 0.009 | 452.9 | 3522 | 5 |
| essential64/enum | views-inputs | deferred | off | 8.682 | 8.492–8.763 | 0.009 | 452.9 | 3522 | 5 |
| essential64/enum | views-inputs | source | bounded | 3.871 | 3.822–3.906 | 0.008 | 629.9 | 3974 | 5 |
| essential64/enum | views-inputs | source | off | 3.933 | 3.869–4.134 | 0.009 | 629.9 | 3974 | 5 |
| essential64/slab | materialized | deferred | off | 4.577 | 4.491–4.746 | 0.010 | 644.3 | 6713 | 5 |
| essential64/slab | views | deferred | bounded | 7.358 | 7.027–9.027 | 0.009 | 365.8 | 3722 | 5 |
| essential64/slab | views | deferred | off | 9.728 | 9.540–9.968 | 0.009 | 365.8 | 3722 | 5 |
| essential64/slab | views | source | bounded | 4.841 | 4.714–4.941 | 0.009 | 616.9 | 5643 | 5 |
| essential64/slab | views | source | off | 4.895 | 4.807–4.968 | 0.009 | 616.9 | 5643 | 5 |
| essential64/slab | views-inputs | deferred | bounded | 6.968 | 6.620–7.403 | 0.009 | 305.7 | 3522 | 5 |
| essential64/slab | views-inputs | deferred | off | 8.553 | 8.505–8.729 | 0.009 | 305.7 | 3522 | 5 |
| essential64/slab | views-inputs | source | bounded | 3.710 | 3.596–3.866 | 0.009 | 389.4 | 3974 | 5 |
| essential64/slab | views-inputs | source | off | 3.840 | 3.812–3.937 | 0.009 | 389.4 | 3974 | 5 |
| packed512/enum | materialized | deferred | off | 2.010 | 1.871–2.109 | 0.033 | 864.1 | 5840 | 5 |

912 cases; 1920 matched comparisons; 445 retained processes.

- [Raw evidence](results/reuse-smoke.json).
- [Raw evidence](results/reuse-initial-sweep.json).
- [Raw evidence](results/reuse-focused-repeat.json).
- [Raw evidence](results/reuse-enum-repeat.json).
- [Raw evidence](results/reuse-acceptance.json).
