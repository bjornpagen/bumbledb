# Scoped contraction through Free Join

Exact staged clipped maps, conjunction and elimination; 16 grouped Event outputs.
Left input rotates bits within each face, right input cycles faces, Y is hidden,
then output Y/Z are swapped. Coupled supports are not called ordinary relation
composition. Every fresh and warm output is checked pointwise outside timing.
Times are milliseconds; resident KB includes outer memos and retained maps,
excluding temporary plane caches and allocator overhead. See [the contract](SCOPED-PRODUCT.md).
Executable: `eba1e378503ae7186f7965ceca6549b615773cba65bd385d3a5b54936990314b`.

## asymmetric, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.279 | 0.279–0.279 | 0.247 | 79.9 | 135 | 1 |
| essential512/enum | materialized | deferred | off | 3.477 | 3.477–3.477 | 3.077 | 490.4 | 2982 | 1 |
| essential512/enum | views | source | off | 2.931 | 2.931–2.931 | 1.772 | 432.1 | 1889 | 1 |
| essential512/enum | views-inputs | source | off | 3.229 | 3.229–3.229 | 1.894 | 473.8 | 2287 | 1 |
| packed512/enum | materialized | deferred | off | 4.567 | 4.567–4.567 | 4.106 | 618.7 | 3882 | 1 |

## asymmetric, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.069 | 0.069–0.069 | 0.007 | 85.3 | 135 | 1 |
| essential512/enum | materialized | deferred | off | 2.661 | 2.661–2.661 | 0.010 | 495.8 | 2982 | 1 |
| essential512/enum | views | source | off | 1.386 | 1.386–1.386 | 0.007 | 437.4 | 1889 | 1 |
| essential512/enum | views-inputs | source | off | 1.675 | 1.675–1.675 | 0.007 | 479.2 | 2287 | 1 |
| packed512/enum | materialized | deferred | off | 1.075 | 1.075–1.075 | 0.014 | 624.1 | 3882 | 1 |

## asymmetric, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.273 | 0.273–0.273 | 0.247 | 79.9 | 135 | 1 |
| essential512/enum | materialized | deferred | off | 5.332 | 5.332–5.332 | 2.728 | 893.3 | 4445 | 1 |
| essential512/enum | views | source | off | 3.142 | 3.142–3.142 | 1.547 | 594.5 | 2494 | 1 |
| essential512/enum | views-inputs | source | off | 4.584 | 4.584–4.584 | 2.575 | 631.8 | 3306 | 1 |
| packed512/enum | materialized | deferred | off | 15.370 | 15.370–15.370 | 14.274 | 1787.3 | 10263 | 1 |

## asymmetric, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.069 | 0.069–0.069 | 0.007 | 85.3 | 135 | 1 |
| essential512/enum | materialized | deferred | off | 3.136 | 3.136–3.136 | 0.009 | 898.7 | 4445 | 1 |
| essential512/enum | views | source | off | 1.922 | 1.922–1.922 | 0.007 | 599.9 | 2494 | 1 |
| essential512/enum | views-inputs | source | off | 2.582 | 2.582–2.582 | 0.009 | 637.2 | 3306 | 1 |
| packed512/enum | materialized | deferred | off | 3.426 | 3.426–3.426 | 0.016 | 1792.7 | 10263 | 1 |

## asymmetric, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 14.019 | 14.019–14.019 | 13.922 | 4564.8 | 137 | 1 |
| essential512/enum | materialized | deferred | off | 39.441 | 39.441–39.441 | 17.612 | 7054.3 | 32499 | 1 |
| essential512/enum | views | source | off | 25.697 | 25.697–25.697 | 14.541 | 3721.1 | 18210 | 1 |
| essential512/enum | views-inputs | source | off | 31.762 | 31.762–31.762 | 16.967 | 4355.4 | 24133 | 1 |
| packed512/enum | materialized | deferred | off | 22.474 | 22.474–22.474 | 19.338 | 4839.6 | 35298 | 1 |

## asymmetric, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.794 | 2.794–2.794 | 0.018 | 4570.2 | 137 | 1 |
| essential512/enum | materialized | deferred | off | 23.553 | 23.553–23.553 | 0.016 | 7059.7 | 32499 | 1 |
| essential512/enum | views | source | off | 13.700 | 13.700–13.700 | 0.018 | 3726.5 | 18210 | 1 |
| essential512/enum | views-inputs | source | off | 17.651 | 17.651–17.651 | 0.015 | 4360.8 | 24133 | 1 |
| packed512/enum | materialized | deferred | off | 6.580 | 6.580–6.580 | 0.133 | 4844.9 | 35298 | 1 |

## asymmetric, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 14.179 | 14.179–14.179 | 14.426 | 4564.8 | 137 | 1 |
| essential512/enum | materialized | deferred | off | 182.493 | 182.493–182.493 | 48.386 | 26967.5 | 158235 | 1 |
| essential512/enum | views | source | off | 139.079 | 139.079–139.079 | 46.710 | 21038.1 | 94203 | 1 |
| essential512/enum | views-inputs | source | off | 140.435 | 140.435–140.435 | 40.582 | 26324.7 | 121532 | 1 |
| packed512/enum | materialized | deferred | off | 166.431 | 166.431–166.431 | 150.819 | 28269.2 | 178795 | 1 |

## asymmetric, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.769 | 2.769–2.769 | 0.018 | 4570.2 | 137 | 1 |
| essential512/enum | materialized | deferred | off | 124.044 | 124.044–124.044 | 0.018 | 26972.9 | 158235 | 1 |
| essential512/enum | views | source | off | 85.224 | 85.224–85.224 | 0.016 | 21043.5 | 94203 | 1 |
| essential512/enum | views-inputs | source | off | 115.066 | 115.066–115.066 | 0.028 | 26330.0 | 121532 | 1 |
| packed512/enum | materialized | deferred | off | 47.068 | 47.068–47.068 | 0.270 | 28274.6 | 178795 | 1 |

## copied-face, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.242 | 0.242–0.242 | 0.226 | 22.2 | 36 | 1 |
| essential512/enum | materialized | deferred | off | 2.560 | 2.560–2.560 | 0.717 | 200.8 | 956 | 1 |
| essential512/enum | views | source | off | 1.436 | 1.436–1.436 | 0.996 | 154.2 | 789 | 1 |
| essential512/enum | views-inputs | source | off | 1.485 | 1.485–1.485 | 0.992 | 154.0 | 780 | 1 |
| packed512/enum | materialized | deferred | off | 2.868 | 2.868–2.868 | 2.725 | 168.5 | 1174 | 1 |

## copied-face, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.055 | 0.055–0.055 | 0.007 | 23.4 | 36 | 1 |
| essential512/enum | materialized | deferred | off | 0.585 | 0.585–0.585 | 0.008 | 202.0 | 956 | 1 |
| essential512/enum | views | source | off | 0.565 | 0.565–0.565 | 0.007 | 155.3 | 789 | 1 |
| essential512/enum | views-inputs | source | off | 0.521 | 0.521–0.521 | 0.007 | 155.2 | 780 | 1 |
| packed512/enum | materialized | deferred | off | 0.601 | 0.601–0.601 | 0.009 | 169.7 | 1174 | 1 |

## copied-face, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.253 | 0.253–0.253 | 0.236 | 22.2 | 36 | 1 |
| essential512/enum | materialized | deferred | off | 2.228 | 2.228–2.228 | 1.272 | 276.6 | 1655 | 1 |
| essential512/enum | views | source | off | 3.151 | 3.151–3.151 | 2.418 | 275.8 | 1262 | 1 |
| essential512/enum | views-inputs | source | off | 3.264 | 3.264–3.264 | 2.673 | 247.8 | 1154 | 1 |
| packed512/enum | materialized | deferred | off | 7.804 | 7.804–7.804 | 7.543 | 421.8 | 2763 | 1 |

## copied-face, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.053 | 0.053–0.053 | 0.007 | 23.4 | 36 | 1 |
| essential512/enum | materialized | deferred | off | 1.030 | 1.030–1.030 | 0.009 | 277.7 | 1655 | 1 |
| essential512/enum | views | source | off | 1.178 | 1.178–1.178 | 0.008 | 277.0 | 1262 | 1 |
| essential512/enum | views-inputs | source | off | 1.082 | 1.082–1.082 | 0.008 | 249.0 | 1154 | 1 |
| packed512/enum | materialized | deferred | off | 1.608 | 1.608–1.608 | 0.010 | 423.0 | 2763 | 1 |

## copied-face, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 13.411 | 13.411–13.411 | 13.152 | 1248.2 | 36 | 1 |
| essential512/enum | materialized | deferred | off | 5.721 | 5.721–5.721 | 3.986 | 955.8 | 6698 | 1 |
| essential512/enum | views | source | off | 5.266 | 5.266–5.266 | 3.632 | 960.9 | 4596 | 1 |
| essential512/enum | views-inputs | source | off | 5.375 | 5.375–5.375 | 3.994 | 965.7 | 4606 | 1 |
| packed512/enum | materialized | deferred | off | 6.294 | 6.294–6.294 | 5.852 | 966.4 | 7085 | 1 |

## copied-face, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.584 | 2.584–2.584 | 0.014 | 1249.3 | 36 | 1 |
| essential512/enum | materialized | deferred | off | 2.632 | 2.632–2.632 | 0.013 | 957.0 | 6698 | 1 |
| essential512/enum | views | source | off | 2.062 | 2.062–2.062 | 0.034 | 962.1 | 4596 | 1 |
| essential512/enum | views-inputs | source | off | 2.063 | 2.063–2.063 | 0.015 | 966.9 | 4606 | 1 |
| packed512/enum | materialized | deferred | off | 1.535 | 1.535–1.535 | 0.022 | 967.6 | 7085 | 1 |

## copied-face, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 13.307 | 13.307–13.307 | 12.957 | 1248.2 | 36 | 1 |
| essential512/enum | materialized | deferred | off | 28.904 | 28.904–28.904 | 19.265 | 4449.1 | 25882 | 1 |
| essential512/enum | views | source | off | 45.098 | 45.098–45.098 | 44.158 | 5097.9 | 25516 | 1 |
| essential512/enum | views-inputs | source | off | 40.050 | 40.050–40.050 | 32.978 | 3718.2 | 19116 | 1 |
| packed512/enum | materialized | deferred | off | 77.044 | 77.044–77.044 | 73.997 | 3470.5 | 25071 | 1 |

## copied-face, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.606 | 2.606–2.606 | 0.023 | 1249.3 | 36 | 1 |
| essential512/enum | materialized | deferred | off | 13.071 | 13.071–13.071 | 0.015 | 4450.2 | 25882 | 1 |
| essential512/enum | views | source | off | 19.557 | 19.557–19.557 | 0.016 | 5099.1 | 25516 | 1 |
| essential512/enum | views-inputs | source | off | 13.494 | 13.494–13.494 | 0.014 | 3719.4 | 19116 | 1 |
| packed512/enum | materialized | deferred | off | 15.346 | 15.346–15.346 | 0.069 | 3471.7 | 25071 | 1 |

## legal-product, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.324 | 0.324–0.324 | 0.243 | 79.4 | 134 | 1 |
| essential512/enum | materialized | deferred | off | 2.600 | 2.600–2.600 | 1.298 | 407.6 | 2087 | 1 |
| essential512/enum | views | source | off | 2.294 | 2.294–2.294 | 1.440 | 296.7 | 1434 | 1 |
| essential512/enum | views-inputs | source | off | 2.971 | 2.971–2.971 | 1.765 | 412.0 | 1845 | 1 |
| packed512/enum | materialized | deferred | off | 4.258 | 4.258–4.258 | 4.033 | 509.6 | 2936 | 1 |

## legal-product, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.069 | 0.069–0.069 | 0.007 | 84.8 | 134 | 1 |
| essential512/enum | materialized | deferred | off | 1.574 | 1.574–1.574 | 0.007 | 413.0 | 2087 | 1 |
| essential512/enum | views | source | off | 1.058 | 1.058–1.058 | 0.007 | 302.0 | 1434 | 1 |
| essential512/enum | views-inputs | source | off | 1.752 | 1.752–1.752 | 0.009 | 417.3 | 1845 | 1 |
| packed512/enum | materialized | deferred | off | 0.980 | 0.980–0.980 | 0.012 | 515.0 | 2936 | 1 |

## legal-product, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.264 | 0.264–0.264 | 0.242 | 79.4 | 134 | 1 |
| essential512/enum | materialized | deferred | off | 3.196 | 3.196–3.196 | 1.573 | 492.3 | 2829 | 1 |
| essential512/enum | views | source | off | 2.293 | 2.293–2.293 | 1.459 | 247.4 | 1235 | 1 |
| essential512/enum | views-inputs | source | off | 3.475 | 3.475–3.475 | 1.846 | 485.0 | 2467 | 1 |
| packed512/enum | materialized | deferred | off | 6.208 | 6.208–6.208 | 6.080 | 622.5 | 3985 | 1 |

## legal-product, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.069 | 0.069–0.069 | 0.007 | 84.8 | 134 | 1 |
| essential512/enum | materialized | deferred | off | 1.923 | 1.923–1.923 | 0.007 | 497.6 | 2829 | 1 |
| essential512/enum | views | source | off | 1.027 | 1.027–1.027 | 0.040 | 252.7 | 1235 | 1 |
| essential512/enum | views-inputs | source | off | 1.944 | 1.944–1.944 | 0.010 | 490.4 | 2467 | 1 |
| packed512/enum | materialized | deferred | off | 1.484 | 1.484–1.484 | 0.010 | 627.9 | 3985 | 1 |

## legal-product, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 13.800 | 13.800–13.800 | 13.889 | 4564.8 | 137 | 1 |
| essential512/enum | materialized | deferred | off | 23.535 | 23.535–23.535 | 12.478 | 3798.3 | 20047 | 1 |
| essential512/enum | views | source | off | 16.984 | 16.984–16.984 | 11.007 | 1992.3 | 10459 | 1 |
| essential512/enum | views-inputs | source | off | 23.609 | 23.609–23.609 | 13.463 | 2947.9 | 16302 | 1 |
| packed512/enum | materialized | deferred | off | 15.680 | 15.680–15.680 | 13.609 | 2899.3 | 23658 | 1 |

## legal-product, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.598 | 2.598–2.598 | 0.015 | 4570.2 | 137 | 1 |
| essential512/enum | materialized | deferred | off | 13.538 | 13.538–13.538 | 0.022 | 3803.7 | 20047 | 1 |
| essential512/enum | views | source | off | 8.263 | 8.263–8.263 | 0.014 | 1997.7 | 10459 | 1 |
| essential512/enum | views-inputs | source | off | 12.535 | 12.535–12.535 | 0.015 | 2953.2 | 16302 | 1 |
| packed512/enum | materialized | deferred | off | 4.334 | 4.334–4.334 | 0.075 | 2904.7 | 23658 | 1 |

## legal-product, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 13.334 | 13.334–13.334 | 13.227 | 4564.8 | 137 | 1 |
| essential512/enum | materialized | deferred | off | 26.409 | 26.409–26.409 | 8.668 | 6633.5 | 33764 | 1 |
| essential512/enum | views | source | off | 38.169 | 38.169–38.169 | 81.419 | 2602.7 | 14258 | 1 |
| essential512/enum | views-inputs | source | off | 25.802 | 25.802–25.802 | 10.262 | 6499.2 | 29466 | 1 |
| packed512/enum | materialized | deferred | off | 25.308 | 25.308–25.308 | 23.099 | 5079.7 | 37394 | 1 |

## legal-product, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.624 | 2.624–2.624 | 0.017 | 4570.2 | 137 | 1 |
| essential512/enum | materialized | deferred | off | 17.769 | 17.769–17.769 | 0.015 | 6638.9 | 33764 | 1 |
| essential512/enum | views | source | off | 13.331 | 13.331–13.331 | 0.017 | 2608.1 | 14258 | 1 |
| essential512/enum | views-inputs | source | off | 16.897 | 16.897–16.897 | 0.013 | 6504.6 | 29466 | 1 |
| packed512/enum | materialized | deferred | off | 7.048 | 7.048–7.048 | 0.030 | 5085.1 | 37394 | 1 |

120 cases; 72 matched comparisons; 33 retained processes.

- [Raw evidence](results/scoped-smoke.json).
