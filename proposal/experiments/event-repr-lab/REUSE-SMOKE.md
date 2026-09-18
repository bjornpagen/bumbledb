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
| essential512/enum | materialized | deferred | off | 0.808 | 0.808–0.808 | 0.591 | 64.9 | 329 | 1 |
| essential512/enum | views | deferred | bounded | 1.674 | 1.674–1.674 | 1.718 | 32.5 | 216 | 1 |
| essential512/enum | views | deferred | off | 1.969 | 1.969–1.969 | 1.966 | 32.5 | 216 | 1 |
| essential512/enum | views | source | bounded | 0.907 | 0.907–0.907 | 0.764 | 44.7 | 261 | 1 |
| essential512/enum | views | source | off | 0.829 | 0.829–0.829 | 0.722 | 44.7 | 261 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 1.933 | 1.933–1.933 | 1.744 | 42.0 | 230 | 1 |
| essential512/enum | views-inputs | deferred | off | 2.022 | 2.022–2.022 | 1.886 | 42.0 | 230 | 1 |
| essential512/enum | views-inputs | source | bounded | 0.964 | 0.964–0.964 | 0.753 | 45.0 | 277 | 1 |
| essential512/enum | views-inputs | source | off | 0.939 | 0.939–0.939 | 0.683 | 45.0 | 277 | 1 |
| essential64/enum | materialized | deferred | off | 1.671 | 1.671–1.671 | 1.168 | 125.2 | 829 | 1 |
| essential64/enum | views | deferred | bounded | 3.162 | 3.162–3.162 | 2.854 | 79.6 | 486 | 1 |
| essential64/enum | views | deferred | off | 3.768 | 3.768–3.768 | 3.597 | 79.6 | 486 | 1 |
| essential64/enum | views | source | bounded | 1.609 | 1.609–1.609 | 1.452 | 86.2 | 549 | 1 |
| essential64/enum | views | source | off | 1.680 | 1.680–1.680 | 1.422 | 86.2 | 549 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 3.146 | 3.146–3.146 | 2.856 | 80.3 | 616 | 1 |
| essential64/enum | views-inputs | deferred | off | 4.098 | 4.098–4.098 | 3.633 | 80.3 | 616 | 1 |
| essential64/enum | views-inputs | source | bounded | 1.716 | 1.716–1.716 | 1.504 | 87.3 | 696 | 1 |
| essential64/enum | views-inputs | source | off | 1.776 | 1.776–1.776 | 1.461 | 87.3 | 696 | 1 |

## product_free_join, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 0.264 | 0.264–0.264 | 0.008 | 72.3 | 329 | 1 |
| essential512/enum | views | deferred | bounded | 0.386 | 0.386–0.386 | 0.008 | 38.6 | 216 | 1 |
| essential512/enum | views | deferred | off | 0.439 | 0.439–0.439 | 0.007 | 38.6 | 216 | 1 |
| essential512/enum | views | source | bounded | 0.266 | 0.266–0.266 | 0.018 | 50.7 | 261 | 1 |
| essential512/enum | views | source | off | 0.429 | 0.429–0.429 | 0.010 | 50.7 | 261 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 1.008 | 1.008–1.008 | 0.014 | 49.0 | 230 | 1 |
| essential512/enum | views-inputs | deferred | off | 0.437 | 0.437–0.437 | 0.007 | 49.0 | 230 | 1 |
| essential512/enum | views-inputs | source | bounded | 0.262 | 0.262–0.262 | 0.008 | 52.0 | 277 | 1 |
| essential512/enum | views-inputs | source | off | 0.274 | 0.274–0.274 | 0.008 | 52.0 | 277 | 1 |
| essential64/enum | materialized | deferred | off | 0.554 | 0.554–0.554 | 0.009 | 132.6 | 829 | 1 |
| essential64/enum | views | deferred | bounded | 0.731 | 0.731–0.731 | 0.009 | 85.6 | 486 | 1 |
| essential64/enum | views | deferred | off | 0.860 | 0.860–0.860 | 0.011 | 85.6 | 486 | 1 |
| essential64/enum | views | source | bounded | 0.490 | 0.490–0.490 | 0.009 | 92.3 | 549 | 1 |
| essential64/enum | views | source | off | 0.437 | 0.437–0.437 | 0.022 | 92.3 | 549 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 0.757 | 0.757–0.757 | 0.021 | 87.3 | 616 | 1 |
| essential64/enum | views-inputs | deferred | off | 0.907 | 0.907–0.907 | 0.008 | 87.3 | 616 | 1 |
| essential64/enum | views-inputs | source | bounded | 0.503 | 0.503–0.503 | 0.013 | 94.3 | 696 | 1 |
| essential64/enum | views-inputs | source | off | 0.497 | 0.497–0.497 | 0.007 | 94.3 | 696 | 1 |

## product_free_join, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 1.218 | 1.218–1.218 | 0.985 | 88.6 | 456 | 1 |
| essential512/enum | views | deferred | bounded | 1.657 | 1.657–1.657 | 1.434 | 48.8 | 291 | 1 |
| essential512/enum | views | deferred | off | 1.702 | 1.702–1.702 | 1.408 | 48.8 | 291 | 1 |
| essential512/enum | views | source | bounded | 0.970 | 0.970–0.970 | 0.737 | 67.3 | 389 | 1 |
| essential512/enum | views | source | off | 0.963 | 0.963–0.963 | 0.677 | 67.3 | 389 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 1.795 | 1.795–1.795 | 1.683 | 41.6 | 292 | 1 |
| essential512/enum | views-inputs | deferred | off | 2.084 | 2.084–2.084 | 1.896 | 41.6 | 292 | 1 |
| essential512/enum | views-inputs | source | bounded | 1.308 | 1.308–1.308 | 1.160 | 57.3 | 350 | 1 |
| essential512/enum | views-inputs | source | off | 1.368 | 1.368–1.368 | 1.120 | 57.3 | 350 | 1 |
| essential64/enum | materialized | deferred | off | 1.465 | 1.465–1.465 | 1.078 | 102.0 | 851 | 1 |
| essential64/enum | views | deferred | bounded | 6.653 | 6.653–6.653 | 6.092 | 177.0 | 1018 | 1 |
| essential64/enum | views | deferred | off | 8.196 | 8.196–8.196 | 7.600 | 177.0 | 1018 | 1 |
| essential64/enum | views | source | bounded | 1.855 | 1.855–1.855 | 1.228 | 190.0 | 1123 | 1 |
| essential64/enum | views | source | off | 1.922 | 1.922–1.922 | 1.325 | 190.0 | 1123 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 5.238 | 5.238–5.238 | 4.877 | 98.5 | 716 | 1 |
| essential64/enum | views-inputs | deferred | off | 5.460 | 5.460–5.460 | 5.451 | 98.5 | 716 | 1 |
| essential64/enum | views-inputs | source | bounded | 1.511 | 1.511–1.511 | 1.192 | 101.1 | 760 | 1 |
| essential64/enum | views-inputs | source | off | 1.456 | 1.456–1.456 | 1.388 | 101.1 | 760 | 1 |

## product_free_join, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 0.347 | 0.347–0.347 | 0.009 | 96.0 | 456 | 1 |
| essential512/enum | views | deferred | bounded | 0.430 | 0.430–0.430 | 0.007 | 54.8 | 291 | 1 |
| essential512/enum | views | deferred | off | 0.421 | 0.421–0.421 | 0.011 | 54.8 | 291 | 1 |
| essential512/enum | views | source | bounded | 0.321 | 0.321–0.321 | 0.007 | 73.4 | 389 | 1 |
| essential512/enum | views | source | off | 0.293 | 0.293–0.293 | 0.007 | 73.4 | 389 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 0.388 | 0.388–0.388 | 0.008 | 48.6 | 292 | 1 |
| essential512/enum | views-inputs | deferred | off | 0.436 | 0.436–0.436 | 0.008 | 48.6 | 292 | 1 |
| essential512/enum | views-inputs | source | bounded | 0.355 | 0.355–0.355 | 0.008 | 64.3 | 350 | 1 |
| essential512/enum | views-inputs | source | off | 0.300 | 0.300–0.300 | 0.007 | 64.3 | 350 | 1 |
| essential64/enum | materialized | deferred | off | 0.480 | 0.480–0.480 | 0.012 | 109.4 | 851 | 1 |
| essential64/enum | views | deferred | bounded | 1.645 | 1.645–1.645 | 0.007 | 183.1 | 1018 | 1 |
| essential64/enum | views | deferred | off | 2.021 | 2.021–2.021 | 0.008 | 183.1 | 1018 | 1 |
| essential64/enum | views | source | bounded | 0.725 | 0.725–0.725 | 0.007 | 196.1 | 1123 | 1 |
| essential64/enum | views | source | off | 0.749 | 0.749–0.749 | 0.008 | 196.1 | 1123 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 1.174 | 1.174–1.174 | 0.008 | 105.5 | 716 | 1 |
| essential64/enum | views-inputs | deferred | off | 1.224 | 1.224–1.224 | 0.008 | 105.5 | 716 | 1 |
| essential64/enum | views-inputs | source | bounded | 0.636 | 0.636–0.636 | 0.008 | 108.1 | 760 | 1 |
| essential64/enum | views-inputs | source | off | 0.472 | 0.472–0.472 | 0.008 | 108.1 | 760 | 1 |

## product_free_join, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 7.620 | 7.620–7.620 | 5.843 | 583.9 | 3112 | 1 |
| essential512/enum | views | deferred | bounded | 40.306 | 40.306–40.306 | 38.526 | 311.2 | 1792 | 1 |
| essential512/enum | views | deferred | off | 63.647 | 63.647–63.647 | 62.781 | 311.2 | 1792 | 1 |
| essential512/enum | views | source | bounded | 7.378 | 7.378–7.378 | 6.911 | 407.6 | 2082 | 1 |
| essential512/enum | views | source | off | 8.125 | 8.125–8.125 | 7.079 | 407.6 | 2082 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 51.000 | 51.000–51.000 | 41.818 | 400.2 | 2342 | 1 |
| essential512/enum | views-inputs | deferred | off | 65.481 | 65.481–65.481 | 65.363 | 400.2 | 2342 | 1 |
| essential512/enum | views-inputs | source | bounded | 8.027 | 8.027–8.027 | 6.794 | 424.6 | 2673 | 1 |
| essential512/enum | views-inputs | source | off | 8.311 | 8.311–8.311 | 7.243 | 424.6 | 2673 | 1 |
| essential64/enum | materialized | deferred | off | 10.650 | 10.650–10.650 | 8.774 | 826.7 | 5582 | 1 |
| essential64/enum | views | deferred | bounded | 27.053 | 27.053–27.053 | 25.406 | 408.4 | 3102 | 1 |
| essential64/enum | views | deferred | off | 35.399 | 35.399–35.399 | 34.047 | 408.4 | 3102 | 1 |
| essential64/enum | views | source | bounded | 10.244 | 10.244–10.244 | 9.063 | 432.9 | 3282 | 1 |
| essential64/enum | views | source | off | 11.300 | 11.300–11.300 | 9.904 | 432.9 | 3282 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 26.992 | 26.992–26.992 | 25.524 | 796.2 | 4643 | 1 |
| essential64/enum | views-inputs | deferred | off | 36.563 | 36.563–36.563 | 34.715 | 796.2 | 4643 | 1 |
| essential64/enum | views-inputs | source | bounded | 10.777 | 10.777–10.777 | 9.485 | 806.1 | 4784 | 1 |
| essential64/enum | views-inputs | source | off | 11.872 | 11.872–11.872 | 9.941 | 806.1 | 4784 | 1 |

## product_free_join, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 2.528 | 2.528–2.528 | 0.022 | 591.3 | 3112 | 1 |
| essential512/enum | views | deferred | bounded | 8.164 | 8.164–8.164 | 0.017 | 317.3 | 1792 | 1 |
| essential512/enum | views | deferred | off | 12.803 | 12.803–12.803 | 0.013 | 317.3 | 1792 | 1 |
| essential512/enum | views | source | bounded | 2.199 | 2.199–2.199 | 0.016 | 413.6 | 2082 | 1 |
| essential512/enum | views | source | off | 2.390 | 2.390–2.390 | 0.017 | 413.6 | 2082 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 10.091 | 10.091–10.091 | 0.043 | 407.2 | 2342 | 1 |
| essential512/enum | views-inputs | deferred | off | 13.008 | 13.008–13.008 | 0.016 | 407.2 | 2342 | 1 |
| essential512/enum | views-inputs | source | bounded | 2.312 | 2.312–2.312 | 0.018 | 431.6 | 2673 | 1 |
| essential512/enum | views-inputs | source | off | 2.660 | 2.660–2.660 | 0.019 | 431.6 | 2673 | 1 |
| essential64/enum | materialized | deferred | off | 3.288 | 3.288–3.288 | 0.015 | 834.1 | 5582 | 1 |
| essential64/enum | views | deferred | bounded | 5.898 | 5.898–5.898 | 0.017 | 414.4 | 3102 | 1 |
| essential64/enum | views | deferred | off | 7.699 | 7.699–7.699 | 0.020 | 414.4 | 3102 | 1 |
| essential64/enum | views | source | bounded | 2.847 | 2.847–2.847 | 0.019 | 439.0 | 3282 | 1 |
| essential64/enum | views | source | off | 3.244 | 3.244–3.244 | 0.017 | 439.0 | 3282 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 6.229 | 6.229–6.229 | 0.021 | 803.2 | 4643 | 1 |
| essential64/enum | views-inputs | deferred | off | 7.859 | 7.859–7.859 | 0.014 | 803.2 | 4643 | 1 |
| essential64/enum | views-inputs | source | bounded | 3.260 | 3.260–3.260 | 0.015 | 813.0 | 4784 | 1 |
| essential64/enum | views-inputs | source | off | 3.387 | 3.387–3.387 | 0.023 | 813.0 | 4784 | 1 |

## product_free_join, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 7.184 | 7.184–7.184 | 6.242 | 512.2 | 3227 | 1 |
| essential512/enum | views | deferred | bounded | 105.834 | 105.834–105.834 | 104.980 | 1011.5 | 5565 | 1 |
| essential512/enum | views | deferred | off | 138.091 | 138.091–138.091 | 133.899 | 1011.5 | 5565 | 1 |
| essential512/enum | views | source | bounded | 15.025 | 15.025–15.025 | 11.092 | 1082.5 | 6007 | 1 |
| essential512/enum | views | source | off | 19.709 | 19.709–19.709 | 15.821 | 1082.5 | 6007 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 87.148 | 87.148–87.148 | 84.248 | 461.9 | 2645 | 1 |
| essential512/enum | views-inputs | deferred | off | 100.785 | 100.785–100.785 | 99.835 | 461.9 | 2645 | 1 |
| essential512/enum | views-inputs | source | bounded | 8.640 | 8.640–8.640 | 6.832 | 501.7 | 3035 | 1 |
| essential512/enum | views-inputs | source | off | 8.742 | 8.742–8.742 | 7.140 | 501.7 | 3035 | 1 |
| essential64/enum | materialized | deferred | off | 6.625 | 6.625–6.625 | 4.173 | 1087.0 | 7382 | 1 |
| essential64/enum | views | deferred | bounded | 82.096 | 82.096–82.096 | 79.754 | 1537.1 | 9887 | 1 |
| essential64/enum | views | deferred | off | 94.552 | 94.552–94.552 | 100.976 | 1537.1 | 9887 | 1 |
| essential64/enum | views | source | bounded | 20.535 | 20.535–20.535 | 16.852 | 1601.4 | 11069 | 1 |
| essential64/enum | views | source | off | 21.626 | 21.626–21.626 | 17.333 | 1601.4 | 11069 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 17.481 | 17.481–17.481 | 15.121 | 777.2 | 6468 | 1 |
| essential64/enum | views-inputs | deferred | off | 17.641 | 17.641–17.641 | 14.868 | 777.2 | 6468 | 1 |
| essential64/enum | views-inputs | source | bounded | 8.845 | 8.845–8.845 | 6.608 | 790.4 | 6582 | 1 |
| essential64/enum | views-inputs | source | off | 8.371 | 8.371–8.371 | 7.074 | 790.4 | 6582 | 1 |

## product_free_join, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 2.239 | 2.239–2.239 | 0.018 | 519.6 | 3227 | 1 |
| essential512/enum | views | deferred | bounded | 23.215 | 23.215–23.215 | 0.013 | 1017.5 | 5565 | 1 |
| essential512/enum | views | deferred | off | 29.231 | 29.231–29.231 | 0.016 | 1017.5 | 5565 | 1 |
| essential512/enum | views | source | bounded | 5.750 | 5.750–5.750 | 0.017 | 1088.6 | 6007 | 1 |
| essential512/enum | views | source | off | 6.512 | 6.512–6.512 | 0.017 | 1088.6 | 6007 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 16.389 | 16.389–16.389 | 0.016 | 468.9 | 2645 | 1 |
| essential512/enum | views-inputs | deferred | off | 19.641 | 19.641–19.641 | 0.021 | 468.9 | 2645 | 1 |
| essential512/enum | views-inputs | source | bounded | 2.479 | 2.479–2.479 | 0.018 | 508.6 | 3035 | 1 |
| essential512/enum | views-inputs | source | off | 2.413 | 2.413–2.413 | 0.015 | 508.6 | 3035 | 1 |
| essential64/enum | materialized | deferred | off | 3.157 | 3.157–3.157 | 0.024 | 1094.4 | 7382 | 1 |
| essential64/enum | views | deferred | bounded | 17.516 | 17.516–17.516 | 0.016 | 1543.2 | 9887 | 1 |
| essential64/enum | views | deferred | off | 20.629 | 20.629–20.629 | 0.015 | 1543.2 | 9887 | 1 |
| essential64/enum | views | source | bounded | 6.763 | 6.763–6.763 | 0.019 | 1607.4 | 11069 | 1 |
| essential64/enum | views | source | off | 7.268 | 7.268–7.268 | 0.019 | 1607.4 | 11069 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 5.149 | 5.149–5.149 | 0.018 | 784.1 | 6468 | 1 |
| essential64/enum | views-inputs | deferred | off | 4.975 | 4.975–4.975 | 0.018 | 784.1 | 6468 | 1 |
| essential64/enum | views-inputs | source | bounded | 3.634 | 3.634–3.634 | 0.018 | 797.4 | 6582 | 1 |
| essential64/enum | views-inputs | source | off | 3.362 | 3.362–3.362 | 0.017 | 797.4 | 6582 | 1 |

## relations_free_join, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 0.495 | 0.495–0.495 | 0.223 | 66.9 | 345 | 1 |
| essential512/enum | views | deferred | bounded | 0.680 | 0.680–0.680 | 0.538 | 22.7 | 130 | 1 |
| essential512/enum | views | deferred | off | 0.726 | 0.726–0.726 | 0.625 | 22.7 | 130 | 1 |
| essential512/enum | views | source | bounded | 0.565 | 0.565–0.565 | 0.307 | 35.2 | 215 | 1 |
| essential512/enum | views | source | off | 0.482 | 0.482–0.482 | 0.264 | 35.2 | 215 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 0.718 | 0.718–0.718 | 0.567 | 22.2 | 121 | 1 |
| essential512/enum | views-inputs | deferred | off | 0.811 | 0.811–0.811 | 0.643 | 22.2 | 121 | 1 |
| essential512/enum | views-inputs | source | bounded | 0.532 | 0.532–0.532 | 0.328 | 34.5 | 205 | 1 |
| essential512/enum | views-inputs | source | off | 0.461 | 0.461–0.461 | 0.284 | 34.5 | 205 | 1 |
| essential64/enum | materialized | deferred | off | 1.143 | 1.143–1.143 | 0.570 | 167.9 | 1159 | 1 |
| essential64/enum | views | deferred | bounded | 0.921 | 0.921–0.921 | 0.693 | 79.5 | 495 | 1 |
| essential64/enum | views | deferred | off | 0.976 | 0.976–0.976 | 0.808 | 79.5 | 495 | 1 |
| essential64/enum | views | source | bounded | 0.965 | 0.965–0.965 | 0.653 | 89.1 | 699 | 1 |
| essential64/enum | views | source | off | 0.959 | 0.959–0.959 | 0.654 | 89.1 | 699 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 0.902 | 0.902–0.902 | 0.664 | 79.4 | 519 | 1 |
| essential64/enum | views-inputs | deferred | off | 1.151 | 1.151–1.151 | 0.757 | 79.4 | 519 | 1 |
| essential64/enum | views-inputs | source | bounded | 0.928 | 0.928–0.928 | 0.670 | 85.0 | 663 | 1 |
| essential64/enum | views-inputs | source | off | 0.879 | 0.879–0.879 | 0.600 | 85.0 | 663 | 1 |

## relations_free_join, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 0.322 | 0.322–0.322 | 0.020 | 79.9 | 345 | 1 |
| essential512/enum | views | deferred | bounded | 0.363 | 0.363–0.363 | 0.012 | 34.5 | 130 | 1 |
| essential512/enum | views | deferred | off | 0.404 | 0.404–0.404 | 0.016 | 34.5 | 130 | 1 |
| essential512/enum | views | source | bounded | 0.309 | 0.309–0.309 | 0.009 | 47.0 | 215 | 1 |
| essential512/enum | views | source | off | 0.304 | 0.304–0.304 | 0.014 | 47.0 | 215 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 0.369 | 0.369–0.369 | 0.011 | 34.4 | 121 | 1 |
| essential512/enum | views-inputs | deferred | off | 0.428 | 0.428–0.428 | 0.032 | 34.4 | 121 | 1 |
| essential512/enum | views-inputs | source | bounded | 0.284 | 0.284–0.284 | 0.009 | 46.6 | 205 | 1 |
| essential512/enum | views-inputs | source | off | 0.262 | 0.262–0.262 | 0.009 | 46.6 | 205 | 1 |
| essential64/enum | materialized | deferred | off | 0.685 | 0.685–0.685 | 0.011 | 180.9 | 1159 | 1 |
| essential64/enum | views | deferred | bounded | 0.491 | 0.491–0.491 | 0.021 | 91.2 | 495 | 1 |
| essential64/enum | views | deferred | off | 0.621 | 0.621–0.621 | 0.010 | 91.2 | 495 | 1 |
| essential64/enum | views | source | bounded | 0.565 | 0.565–0.565 | 0.009 | 100.9 | 699 | 1 |
| essential64/enum | views | source | off | 0.550 | 0.550–0.550 | 0.021 | 100.9 | 699 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 0.495 | 0.495–0.495 | 0.010 | 91.5 | 519 | 1 |
| essential64/enum | views-inputs | deferred | off | 0.551 | 0.551–0.551 | 0.009 | 91.5 | 519 | 1 |
| essential64/enum | views-inputs | source | bounded | 0.538 | 0.538–0.538 | 0.010 | 97.2 | 663 | 1 |
| essential64/enum | views-inputs | source | off | 0.507 | 0.507–0.507 | 0.009 | 97.2 | 663 | 1 |

## relations_free_join, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 0.827 | 0.827–0.827 | 0.458 | 99.1 | 510 | 1 |
| essential512/enum | views | deferred | bounded | 0.800 | 0.800–0.800 | 0.559 | 47.5 | 251 | 1 |
| essential512/enum | views | deferred | off | 0.886 | 0.886–0.886 | 0.525 | 47.5 | 251 | 1 |
| essential512/enum | views | source | bounded | 0.756 | 0.756–0.756 | 0.447 | 92.9 | 593 | 1 |
| essential512/enum | views | source | off | 0.692 | 0.692–0.692 | 0.386 | 92.9 | 593 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 0.754 | 0.754–0.754 | 0.666 | 22.8 | 156 | 1 |
| essential512/enum | views-inputs | deferred | off | 0.895 | 0.895–0.895 | 0.633 | 22.8 | 156 | 1 |
| essential512/enum | views-inputs | source | bounded | 0.755 | 0.755–0.755 | 0.570 | 58.5 | 343 | 1 |
| essential512/enum | views-inputs | source | off | 0.754 | 0.754–0.754 | 0.521 | 58.5 | 343 | 1 |
| essential64/enum | materialized | deferred | off | 1.544 | 1.544–1.544 | 0.687 | 210.8 | 1342 | 1 |
| essential64/enum | views | deferred | bounded | 3.398 | 3.398–3.398 | 2.766 | 196.9 | 1499 | 1 |
| essential64/enum | views | deferred | off | 3.862 | 3.862–3.862 | 3.075 | 196.9 | 1499 | 1 |
| essential64/enum | views | source | bounded | 1.895 | 1.895–1.895 | 0.796 | 383.1 | 2243 | 1 |
| essential64/enum | views | source | off | 1.956 | 1.956–1.956 | 0.725 | 383.1 | 2243 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 2.557 | 2.557–2.557 | 2.280 | 96.6 | 544 | 1 |
| essential64/enum | views-inputs | deferred | off | 2.820 | 2.820–2.820 | 2.540 | 96.6 | 544 | 1 |
| essential64/enum | views-inputs | source | bounded | 1.414 | 1.414–1.414 | 0.934 | 114.7 | 779 | 1 |
| essential64/enum | views-inputs | source | off | 1.533 | 1.533–1.533 | 0.993 | 114.7 | 779 | 1 |

## relations_free_join, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 0.522 | 0.522–0.522 | 0.010 | 112.1 | 510 | 1 |
| essential512/enum | views | deferred | bounded | 0.454 | 0.454–0.454 | 0.009 | 59.2 | 251 | 1 |
| essential512/enum | views | deferred | off | 0.421 | 0.421–0.421 | 0.009 | 59.2 | 251 | 1 |
| essential512/enum | views | source | bounded | 0.477 | 0.477–0.477 | 0.009 | 104.6 | 593 | 1 |
| essential512/enum | views | source | off | 0.462 | 0.462–0.462 | 0.009 | 104.6 | 593 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 0.410 | 0.410–0.410 | 0.010 | 34.9 | 156 | 1 |
| essential512/enum | views-inputs | deferred | off | 0.407 | 0.407–0.407 | 0.010 | 34.9 | 156 | 1 |
| essential512/enum | views-inputs | source | bounded | 0.401 | 0.401–0.401 | 0.010 | 70.6 | 343 | 1 |
| essential512/enum | views-inputs | source | off | 0.398 | 0.398–0.398 | 0.009 | 70.6 | 343 | 1 |
| essential64/enum | materialized | deferred | off | 1.057 | 1.057–1.057 | 0.019 | 223.8 | 1342 | 1 |
| essential64/enum | views | deferred | bounded | 1.988 | 1.988–1.988 | 0.011 | 208.7 | 1499 | 1 |
| essential64/enum | views | deferred | off | 2.150 | 2.150–2.150 | 0.012 | 208.7 | 1499 | 1 |
| essential64/enum | views | source | bounded | 1.338 | 1.338–1.338 | 0.019 | 394.8 | 2243 | 1 |
| essential64/enum | views | source | off | 1.313 | 1.313–1.313 | 0.015 | 394.8 | 2243 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 1.467 | 1.467–1.467 | 0.014 | 108.8 | 544 | 1 |
| essential64/enum | views-inputs | deferred | off | 1.630 | 1.630–1.630 | 0.012 | 108.8 | 544 | 1 |
| essential64/enum | views-inputs | source | bounded | 0.880 | 0.880–0.880 | 0.010 | 126.8 | 779 | 1 |
| essential64/enum | views-inputs | source | off | 0.965 | 0.965–0.965 | 0.015 | 126.8 | 779 | 1 |

## relations_free_join, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 3.271 | 3.271–3.271 | 1.736 | 450.5 | 2668 | 1 |
| essential512/enum | views | deferred | bounded | 10.361 | 10.361–10.361 | 9.547 | 195.5 | 1011 | 1 |
| essential512/enum | views | deferred | off | 15.269 | 15.269–15.269 | 14.784 | 195.5 | 1011 | 1 |
| essential512/enum | views | source | bounded | 3.212 | 3.212–3.212 | 1.903 | 347.3 | 1776 | 1 |
| essential512/enum | views | source | off | 3.179 | 3.179–3.179 | 1.857 | 347.3 | 1776 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 10.548 | 10.548–10.548 | 10.021 | 195.0 | 1024 | 1 |
| essential512/enum | views-inputs | deferred | off | 14.508 | 14.508–14.508 | 14.073 | 195.0 | 1024 | 1 |
| essential512/enum | views-inputs | source | bounded | 2.927 | 2.927–2.927 | 2.137 | 302.5 | 1645 | 1 |
| essential512/enum | views-inputs | source | off | 3.037 | 3.037–3.037 | 2.189 | 302.5 | 1645 | 1 |
| essential64/enum | materialized | deferred | off | 5.074 | 5.074–5.074 | 3.002 | 879.5 | 5699 | 1 |
| essential64/enum | views | deferred | bounded | 6.359 | 6.359–6.359 | 5.123 | 415.1 | 2640 | 1 |
| essential64/enum | views | deferred | off | 7.252 | 7.252–7.252 | 6.279 | 415.1 | 2640 | 1 |
| essential64/enum | views | source | bounded | 4.318 | 4.318–4.318 | 3.248 | 476.9 | 3577 | 1 |
| essential64/enum | views | source | off | 4.612 | 4.612–4.612 | 3.463 | 476.9 | 3577 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 5.971 | 5.971–5.971 | 5.360 | 414.0 | 2863 | 1 |
| essential64/enum | views-inputs | deferred | off | 6.871 | 6.871–6.871 | 5.897 | 414.0 | 2863 | 1 |
| essential64/enum | views-inputs | source | bounded | 4.141 | 4.141–4.141 | 2.814 | 443.7 | 3294 | 1 |
| essential64/enum | views-inputs | source | off | 4.071 | 4.071–4.071 | 3.314 | 443.7 | 3294 | 1 |

## relations_free_join, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 2.300 | 2.300–2.300 | 0.022 | 463.5 | 2668 | 1 |
| essential512/enum | views | deferred | bounded | 5.324 | 5.324–5.324 | 0.022 | 207.2 | 1011 | 1 |
| essential512/enum | views | deferred | off | 7.876 | 7.876–7.876 | 0.020 | 207.2 | 1011 | 1 |
| essential512/enum | views | source | bounded | 2.255 | 2.255–2.255 | 0.024 | 359.1 | 1776 | 1 |
| essential512/enum | views | source | off | 2.275 | 2.275–2.275 | 0.022 | 359.1 | 1776 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 5.571 | 5.571–5.571 | 0.022 | 207.5 | 1024 | 1 |
| essential512/enum | views-inputs | deferred | off | 7.455 | 7.455–7.455 | 0.022 | 207.5 | 1024 | 1 |
| essential512/enum | views-inputs | source | bounded | 1.983 | 1.983–1.983 | 0.026 | 315.0 | 1645 | 1 |
| essential512/enum | views-inputs | source | off | 1.990 | 1.990–1.990 | 0.029 | 315.0 | 1645 | 1 |
| essential64/enum | materialized | deferred | off | 3.376 | 3.376–3.376 | 0.022 | 892.5 | 5699 | 1 |
| essential64/enum | views | deferred | bounded | 3.579 | 3.579–3.579 | 0.021 | 426.9 | 2640 | 1 |
| essential64/enum | views | deferred | off | 4.086 | 4.086–4.086 | 0.021 | 426.9 | 2640 | 1 |
| essential64/enum | views | source | bounded | 2.893 | 2.893–2.893 | 0.022 | 488.7 | 3577 | 1 |
| essential64/enum | views | source | off | 2.815 | 2.815–2.815 | 0.022 | 488.7 | 3577 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 3.264 | 3.264–3.264 | 0.025 | 426.5 | 2863 | 1 |
| essential64/enum | views-inputs | deferred | off | 3.801 | 3.801–3.801 | 0.021 | 426.5 | 2863 | 1 |
| essential64/enum | views-inputs | source | bounded | 2.613 | 2.613–2.613 | 0.023 | 456.3 | 3294 | 1 |
| essential64/enum | views-inputs | source | off | 2.620 | 2.620–2.620 | 0.021 | 456.3 | 3294 | 1 |

## relations_free_join, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 10.354 | 10.354–10.354 | 5.878 | 1043.6 | 5273 | 1 |
| essential512/enum | views | deferred | bounded | 56.514 | 56.514–56.514 | 56.125 | 1722.5 | 8088 | 1 |
| essential512/enum | views | deferred | off | 99.178 | 99.178–99.178 | 94.407 | 1722.5 | 8088 | 1 |
| essential512/enum | views | source | bounded | 10.392 | 10.392–10.392 | 4.715 | 2098.2 | 11800 | 1 |
| essential512/enum | views | source | off | 11.189 | 11.189–11.189 | 5.230 | 2098.2 | 11800 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 35.630 | 35.630–35.630 | 34.906 | 263.4 | 1642 | 1 |
| essential512/enum | views-inputs | deferred | off | 44.007 | 44.007–44.007 | 43.809 | 263.4 | 1642 | 1 |
| essential512/enum | views-inputs | source | bounded | 8.693 | 8.693–8.693 | 6.758 | 738.7 | 3752 | 1 |
| essential512/enum | views-inputs | source | off | 10.000 | 10.000–10.000 | 7.579 | 738.7 | 3752 | 1 |
| essential64/enum | materialized | deferred | off | 27.375 | 27.375–27.375 | 17.565 | 1866.6 | 9513 | 1 |
| essential64/enum | views | deferred | bounded | 37.018 | 37.018–37.018 | 30.118 | 2402.2 | 14862 | 1 |
| essential64/enum | views | deferred | off | 35.664 | 35.664–35.664 | 29.909 | 2402.2 | 14862 | 1 |
| essential64/enum | views | source | bounded | 17.535 | 17.535–17.535 | 6.767 | 3616.1 | 23097 | 1 |
| essential64/enum | views | source | off | 17.831 | 17.831–17.831 | 7.179 | 3616.1 | 23097 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 34.729 | 34.729–34.729 | 31.426 | 911.1 | 4924 | 1 |
| essential64/enum | views-inputs | deferred | off | 40.301 | 40.301–40.301 | 37.407 | 911.1 | 4924 | 1 |
| essential64/enum | views-inputs | source | bounded | 24.586 | 24.586–24.586 | 19.100 | 978.6 | 5814 | 1 |
| essential64/enum | views-inputs | source | off | 31.383 | 31.383–31.383 | 23.091 | 978.6 | 5814 | 1 |

## relations_free_join, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | deferred | off | 7.019 | 7.019–7.019 | 0.026 | 1056.6 | 5273 | 1 |
| essential512/enum | views | deferred | bounded | 31.057 | 31.057–31.057 | 0.021 | 1734.3 | 8088 | 1 |
| essential512/enum | views | deferred | off | 49.734 | 49.734–49.734 | 0.020 | 1734.3 | 8088 | 1 |
| essential512/enum | views | source | bounded | 8.251 | 8.251–8.251 | 0.021 | 2110.0 | 11800 | 1 |
| essential512/enum | views | source | off | 9.339 | 9.339–9.339 | 0.030 | 2110.0 | 11800 | 1 |
| essential512/enum | views-inputs | deferred | bounded | 17.936 | 17.936–17.936 | 0.038 | 275.9 | 1642 | 1 |
| essential512/enum | views-inputs | deferred | off | 22.410 | 22.410–22.410 | 0.023 | 275.9 | 1642 | 1 |
| essential512/enum | views-inputs | source | bounded | 5.561 | 5.561–5.561 | 0.020 | 751.2 | 3752 | 1 |
| essential512/enum | views-inputs | source | off | 6.168 | 6.168–6.168 | 0.021 | 751.2 | 3752 | 1 |
| essential64/enum | materialized | deferred | off | 19.192 | 19.192–19.192 | 0.024 | 1879.6 | 9513 | 1 |
| essential64/enum | views | deferred | bounded | 21.277 | 21.277–21.277 | 0.021 | 2414.0 | 14862 | 1 |
| essential64/enum | views | deferred | off | 21.016 | 21.016–21.016 | 0.023 | 2414.0 | 14862 | 1 |
| essential64/enum | views | source | bounded | 14.461 | 14.461–14.461 | 0.028 | 3627.9 | 23097 | 1 |
| essential64/enum | views | source | off | 14.871 | 14.871–14.871 | 0.021 | 3627.9 | 23097 | 1 |
| essential64/enum | views-inputs | deferred | bounded | 19.050 | 19.050–19.050 | 0.018 | 923.7 | 4924 | 1 |
| essential64/enum | views-inputs | deferred | off | 21.829 | 21.829–21.829 | 0.020 | 923.7 | 4924 | 1 |
| essential64/enum | views-inputs | source | bounded | 13.948 | 13.948–13.948 | 0.025 | 991.2 | 5814 | 1 |
| essential64/enum | views-inputs | source | off | 15.292 | 15.292–15.292 | 0.022 | 991.2 | 5814 | 1 |

288 cases; 640 matched comparisons; 81 retained processes.

- [Raw evidence](results/reuse-smoke.json).
