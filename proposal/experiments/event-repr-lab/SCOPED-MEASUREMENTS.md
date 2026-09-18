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
| dense-dispatched/enum | materialized | deferred | off | 0.251 | 0.250–0.278 | 0.248 | 79.9 | 135 | 9 |
| essential512/enum | materialized | deferred | off | 3.279 | 3.234–3.496 | 1.540 | 490.4 | 2982 | 3 |
| essential512/enum | views | deferred | off | 4.833 | 4.710–4.926 | 4.117 | 234.0 | 1394 | 3 |
| essential512/enum | views | source | off | 2.831 | 2.734–2.854 | 1.737 | 432.1 | 1889 | 3 |
| essential512/enum | views-inputs | deferred | off | 5.161 | 5.107–5.174 | 4.100 | 429.0 | 1812 | 3 |
| essential512/enum | views-inputs | source | off | 3.394 | 3.374–3.568 | 1.945 | 473.8 | 2287 | 3 |
| essential512/slab | materialized | deferred | off | 3.283 | 3.215–3.340 | 1.603 | 356.7 | 2982 | 9 |
| essential512/slab | views | deferred | off | 4.697 | 4.661–4.905 | 4.037 | 148.6 | 1394 | 3 |
| essential512/slab | views | source | off | 2.912 | 2.801–3.071 | 1.819 | 281.4 | 1889 | 9 |
| essential512/slab | views-inputs | deferred | off | 5.243 | 5.159–5.368 | 4.169 | 296.6 | 1812 | 3 |
| essential512/slab | views-inputs | source | off | 3.255 | 3.193–3.779 | 2.011 | 311.9 | 2287 | 9 |
| essential64/enum | materialized | deferred | off | 7.337 | 7.224–7.395 | 3.641 | 1267.2 | 7686 | 3 |
| essential64/enum | views | deferred | off | 20.347 | 20.286–20.368 | 18.543 | 471.6 | 3449 | 3 |
| essential64/enum | views | source | off | 6.567 | 6.320–6.623 | 4.123 | 837.4 | 4626 | 3 |
| essential64/enum | views-inputs | deferred | off | 212.922 | 22.276–277.465 | 27.084 | 806.6 | 4779 | 3 |
| essential64/enum | views-inputs | source | off | 8.059 | 7.789–8.223 | 4.810 | 845.4 | 5939 | 3 |
| essential64/slab | materialized | deferred | off | 7.322 | 7.229–7.332 | 3.734 | 805.1 | 7686 | 3 |
| essential64/slab | views | deferred | off | 20.512 | 20.433–20.703 | 18.662 | 326.6 | 3449 | 3 |
| essential64/slab | views | source | off | 6.789 | 6.655–6.821 | 4.410 | 561.4 | 4626 | 3 |
| essential64/slab | views-inputs | deferred | off | 21.958 | 21.785–21.992 | 19.626 | 530.9 | 4779 | 3 |
| essential64/slab | views-inputs | source | off | 8.408 | 7.795–8.436 | 5.151 | 561.4 | 5939 | 3 |
| packed512/enum | materialized | deferred | off | 4.589 | 4.488–4.810 | 4.270 | 618.7 | 3882 | 9 |

## asymmetric, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.058 | 0.057–0.068 | 0.007 | 85.3 | 135 | 9 |
| essential512/enum | materialized | deferred | off | 1.937 | 1.932–1.994 | 0.007 | 495.8 | 2982 | 3 |
| essential512/enum | views | deferred | off | 1.526 | 1.487–1.582 | 0.007 | 239.4 | 1394 | 3 |
| essential512/enum | views | source | off | 1.362 | 1.349–1.440 | 0.007 | 437.4 | 1889 | 3 |
| essential512/enum | views-inputs | deferred | off | 1.780 | 1.776–1.798 | 0.007 | 434.3 | 1812 | 3 |
| essential512/enum | views-inputs | source | off | 1.685 | 1.679–1.870 | 0.007 | 479.2 | 2287 | 3 |
| essential512/slab | materialized | deferred | off | 1.938 | 1.918–1.989 | 0.007 | 362.1 | 2982 | 9 |
| essential512/slab | views | deferred | off | 1.529 | 1.513–1.706 | 0.011 | 153.9 | 1394 | 3 |
| essential512/slab | views | source | off | 1.397 | 1.338–1.600 | 0.008 | 286.8 | 1889 | 9 |
| essential512/slab | views-inputs | deferred | off | 1.883 | 1.771–1.911 | 0.011 | 302.0 | 1812 | 3 |
| essential512/slab | views-inputs | source | off | 1.627 | 1.609–1.693 | 0.007 | 317.3 | 2287 | 9 |
| essential64/enum | materialized | deferred | off | 4.185 | 4.183–4.221 | 0.007 | 1272.6 | 7686 | 3 |
| essential64/enum | views | deferred | off | 5.565 | 5.125–5.849 | 0.007 | 477.0 | 3449 | 3 |
| essential64/enum | views | source | off | 2.985 | 2.983–3.077 | 0.007 | 842.8 | 4626 | 3 |
| essential64/enum | views-inputs | deferred | off | 6.164 | 6.161–6.231 | 0.009 | 812.0 | 4779 | 3 |
| essential64/enum | views-inputs | source | off | 4.059 | 3.970–4.114 | 0.008 | 850.8 | 5939 | 3 |
| essential64/slab | materialized | deferred | off | 4.144 | 4.143–4.151 | 0.007 | 810.4 | 7686 | 3 |
| essential64/slab | views | deferred | off | 5.188 | 5.172–5.204 | 0.007 | 332.0 | 3449 | 3 |
| essential64/slab | views | source | off | 3.242 | 3.049–3.243 | 0.009 | 566.7 | 4626 | 3 |
| essential64/slab | views-inputs | deferred | off | 6.111 | 5.975–6.364 | 0.010 | 536.3 | 4779 | 3 |
| essential64/slab | views-inputs | source | off | 4.204 | 3.967–4.261 | 0.009 | 566.7 | 5939 | 3 |
| packed512/enum | materialized | deferred | off | 1.075 | 1.049–1.135 | 0.014 | 624.1 | 3882 | 9 |

## asymmetric, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.252 | 0.247–0.287 | 0.243 | 79.9 | 135 | 9 |
| essential512/enum | materialized | deferred | off | 5.454 | 5.368–5.455 | 2.616 | 893.3 | 4445 | 3 |
| essential512/enum | views | deferred | off | 10.433 | 10.361–10.498 | 9.376 | 423.6 | 1822 | 3 |
| essential512/enum | views | source | off | 3.096 | 3.080–3.226 | 1.600 | 594.5 | 2494 | 3 |
| essential512/enum | views-inputs | deferred | off | 8.313 | 8.204–8.318 | 6.593 | 577.3 | 2815 | 3 |
| essential512/enum | views-inputs | source | off | 4.675 | 4.553–4.802 | 2.634 | 631.8 | 3306 | 3 |
| essential512/slab | materialized | deferred | off | 5.210 | 4.901–5.307 | 2.642 | 561.3 | 4445 | 9 |
| essential512/slab | views | deferred | off | 10.484 | 10.371–10.671 | 9.395 | 275.2 | 1822 | 3 |
| essential512/slab | views | source | off | 3.191 | 3.088–3.377 | 1.615 | 372.8 | 2494 | 9 |
| essential512/slab | views-inputs | deferred | off | 8.249 | 8.233–8.433 | 6.675 | 431.8 | 2815 | 3 |
| essential512/slab | views-inputs | source | off | 4.446 | 4.324–4.532 | 2.548 | 454.7 | 3306 | 9 |
| essential64/enum | materialized | deferred | off | 13.850 | 13.438–14.017 | 5.912 | 2550.3 | 15136 | 3 |
| essential64/enum | views | deferred | off | 58.825 | 58.694–58.924 | 55.147 | 956.0 | 6965 | 3 |
| essential64/enum | views | source | off | 10.536 | 9.881–10.609 | 5.278 | 1378.8 | 8970 | 3 |
| essential64/enum | views-inputs | deferred | off | 53.938 | 53.906–55.135 | 48.427 | 1908.7 | 10358 | 3 |
| essential64/enum | views-inputs | source | off | 12.331 | 11.938–12.356 | 5.740 | 2009.3 | 11546 | 3 |
| essential64/slab | materialized | deferred | off | 13.922 | 13.727–14.180 | 6.371 | 1605.4 | 15136 | 3 |
| essential64/slab | views | deferred | off | 60.389 | 60.233–60.399 | 57.081 | 658.2 | 6965 | 3 |
| essential64/slab | views | source | off | 9.905 | 9.833–10.101 | 5.189 | 919.4 | 8970 | 3 |
| essential64/slab | views-inputs | deferred | off | 55.438 | 54.166–56.325 | 48.680 | 1331.2 | 10358 | 3 |
| essential64/slab | views-inputs | source | off | 11.939 | 11.865–12.070 | 5.978 | 1422.6 | 11546 | 3 |
| packed512/enum | materialized | deferred | off | 14.906 | 14.845–15.069 | 14.269 | 1787.3 | 10263 | 9 |

## asymmetric, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.057 | 0.056–0.062 | 0.007 | 85.3 | 135 | 9 |
| essential512/enum | materialized | deferred | off | 3.182 | 2.990–3.309 | 0.007 | 898.7 | 4445 | 3 |
| essential512/enum | views | deferred | off | 2.900 | 2.778–2.905 | 0.007 | 429.0 | 1822 | 3 |
| essential512/enum | views | source | off | 1.805 | 1.780–1.860 | 0.007 | 599.9 | 2494 | 3 |
| essential512/enum | views-inputs | deferred | off | 2.811 | 2.788–2.863 | 0.007 | 582.7 | 2815 | 3 |
| essential512/enum | views-inputs | source | off | 2.551 | 2.501–2.556 | 0.007 | 637.2 | 3306 | 3 |
| essential512/slab | materialized | deferred | off | 2.984 | 2.869–3.154 | 0.007 | 566.7 | 4445 | 9 |
| essential512/slab | views | deferred | off | 2.797 | 2.772–2.805 | 0.007 | 280.6 | 1822 | 3 |
| essential512/slab | views | source | off | 1.832 | 1.782–1.918 | 0.010 | 378.2 | 2494 | 9 |
| essential512/slab | views-inputs | deferred | off | 2.845 | 2.759–2.971 | 0.009 | 437.2 | 2815 | 3 |
| essential512/slab | views-inputs | source | off | 2.346 | 2.281–2.449 | 0.007 | 460.1 | 3306 | 9 |
| essential64/enum | materialized | deferred | off | 8.302 | 8.291–8.724 | 0.007 | 2555.7 | 15136 | 3 |
| essential64/enum | views | deferred | off | 13.837 | 13.786–14.828 | 0.011 | 961.4 | 6965 | 3 |
| essential64/enum | views | source | off | 5.877 | 5.776–6.476 | 0.010 | 1384.2 | 8970 | 3 |
| essential64/enum | views-inputs | deferred | off | 14.262 | 14.259–14.335 | 0.007 | 1914.1 | 10358 | 3 |
| essential64/enum | views-inputs | source | off | 7.067 | 7.064–7.094 | 0.007 | 2014.7 | 11546 | 3 |
| essential64/slab | materialized | deferred | off | 8.592 | 8.577–8.621 | 0.012 | 1610.7 | 15136 | 3 |
| essential64/slab | views | deferred | off | 14.011 | 13.983–14.176 | 0.008 | 663.6 | 6965 | 3 |
| essential64/slab | views | source | off | 5.617 | 5.570–5.640 | 0.008 | 924.7 | 8970 | 3 |
| essential64/slab | views-inputs | deferred | off | 14.320 | 14.203–14.363 | 0.008 | 1336.6 | 10358 | 3 |
| essential64/slab | views-inputs | source | off | 7.112 | 7.041–7.130 | 0.007 | 1427.9 | 11546 | 3 |
| packed512/enum | materialized | deferred | off | 3.308 | 3.247–3.423 | 0.013 | 1792.7 | 10263 | 9 |

## asymmetric, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 13.547 | 13.340–13.885 | 13.540 | 4564.8 | 137 | 9 |
| essential512/enum | materialized | deferred | off | 35.351 | 34.903–36.105 | 15.842 | 7054.3 | 32499 | 3 |
| essential512/enum | views | deferred | off | 166.729 | 165.545–167.615 | 156.047 | 2792.5 | 14790 | 3 |
| essential512/enum | views | source | off | 25.692 | 25.571–25.824 | 14.435 | 3721.1 | 18210 | 3 |
| essential512/enum | views-inputs | deferred | off | 170.491 | 170.454–173.623 | 161.758 | 4118.3 | 20994 | 3 |
| essential512/enum | views-inputs | source | off | 35.175 | 32.860–35.889 | 17.530 | 4355.4 | 24133 | 3 |
| essential512/slab | materialized | deferred | off | 36.976 | 35.041–39.011 | 16.808 | 4794.6 | 32499 | 9 |
| essential512/slab | views | deferred | off | 163.417 | 162.169–163.652 | 153.102 | 1895.9 | 14790 | 3 |
| essential512/slab | views | source | off | 25.843 | 25.691–27.148 | 15.230 | 2519.5 | 18210 | 9 |
| essential512/slab | views-inputs | deferred | off | 177.279 | 176.670–177.791 | 165.901 | 2885.0 | 20994 | 3 |
| essential512/slab | views-inputs | source | off | 31.406 | 31.358–36.677 | 17.619 | 3006.9 | 24133 | 9 |
| essential64/enum | materialized | deferred | off | 45.627 | 45.208–46.126 | 24.450 | 8357.7 | 51951 | 3 |
| essential64/enum | views | deferred | off | 381.751 | 381.639–383.003 | 369.039 | 4173.3 | 23037 | 3 |
| essential64/enum | views | source | off | 34.508 | 34.431–35.912 | 21.685 | 4308.6 | 27655 | 3 |
| essential64/enum | views-inputs | deferred | off | 1529.766 | 1274.844–1750.272 | 803.076 | 5391.1 | 35111 | 3 |
| essential64/enum | views-inputs | source | off | 45.461 | 44.846–46.869 | 26.911 | 5527.6 | 39449 | 3 |
| essential64/slab | materialized | deferred | off | 45.392 | 45.350–45.439 | 24.818 | 5853.8 | 51951 | 3 |
| essential64/slab | views | deferred | off | 387.221 | 386.298–395.560 | 371.355 | 2927.2 | 23037 | 3 |
| essential64/slab | views | source | off | 37.452 | 36.719–37.809 | 23.377 | 3049.1 | 27655 | 3 |
| essential64/slab | views-inputs | deferred | off | 406.475 | 404.257–406.769 | 382.848 | 3414.6 | 35111 | 3 |
| essential64/slab | views-inputs | source | off | 48.321 | 47.824–48.913 | 32.011 | 3596.3 | 39449 | 3 |
| packed512/enum | materialized | deferred | off | 22.218 | 22.023–23.599 | 20.168 | 4839.6 | 35298 | 9 |

## asymmetric, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.583 | 2.570–2.677 | 0.015 | 4570.2 | 137 | 9 |
| essential512/enum | materialized | deferred | off | 22.049 | 21.612–22.508 | 0.017 | 7059.7 | 32499 | 3 |
| essential512/enum | views | deferred | off | 41.214 | 39.035–41.645 | 0.020 | 2797.9 | 14790 | 3 |
| essential512/enum | views | source | off | 14.160 | 13.812–15.846 | 0.016 | 3726.5 | 18210 | 3 |
| essential512/enum | views-inputs | deferred | off | 42.913 | 42.830–45.505 | 0.020 | 4123.7 | 20994 | 3 |
| essential512/enum | views-inputs | source | off | 18.263 | 18.038–18.426 | 0.017 | 4360.8 | 24133 | 3 |
| essential512/slab | materialized | deferred | off | 21.976 | 21.724–24.560 | 0.018 | 4799.9 | 32499 | 9 |
| essential512/slab | views | deferred | off | 38.455 | 38.075–39.219 | 0.017 | 1901.2 | 14790 | 3 |
| essential512/slab | views | source | off | 14.422 | 13.993–15.149 | 0.017 | 2524.9 | 18210 | 9 |
| essential512/slab | views-inputs | deferred | off | 45.439 | 45.312–45.444 | 0.017 | 2890.4 | 20994 | 3 |
| essential512/slab | views-inputs | source | off | 17.634 | 17.496–18.191 | 0.017 | 3012.3 | 24133 | 9 |
| essential64/enum | materialized | deferred | off | 26.032 | 26.027–29.413 | 0.016 | 8363.1 | 51951 | 3 |
| essential64/enum | views | deferred | off | 80.932 | 80.781–81.898 | 0.015 | 4178.6 | 23037 | 3 |
| essential64/enum | views | source | off | 17.057 | 16.984–17.096 | 0.015 | 4314.0 | 27655 | 3 |
| essential64/enum | views-inputs | deferred | off | 88.480 | 87.853–89.501 | 0.017 | 5396.5 | 35111 | 3 |
| essential64/enum | views-inputs | source | off | 23.857 | 23.728–25.185 | 0.016 | 5533.0 | 39449 | 3 |
| essential64/slab | materialized | deferred | off | 26.411 | 25.997–27.636 | 0.015 | 5859.1 | 51951 | 3 |
| essential64/slab | views | deferred | off | 80.750 | 80.599–84.413 | 0.015 | 2932.6 | 23037 | 3 |
| essential64/slab | views | source | off | 18.410 | 18.253–20.468 | 0.016 | 3054.5 | 27655 | 3 |
| essential64/slab | views-inputs | deferred | off | 91.472 | 88.474–93.256 | 0.016 | 3420.0 | 35111 | 3 |
| essential64/slab | views-inputs | source | off | 26.249 | 26.075–26.487 | 0.017 | 3601.7 | 39449 | 3 |
| packed512/enum | materialized | deferred | off | 6.522 | 6.254–7.023 | 0.136 | 4844.9 | 35298 | 9 |

## asymmetric, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 13.865 | 13.648–14.058 | 13.660 | 4564.8 | 137 | 9 |
| essential512/enum | materialized | deferred | off | 176.004 | 158.761–192.347 | 47.641 | 26967.5 | 158235 | 3 |
| essential512/enum | views | deferred | off | 3227.121 | 3224.909–3229.930 | 3133.975 | 15847.7 | 74759 | 3 |
| essential512/enum | views | source | off | 125.032 | 124.406–126.935 | 46.405 | 21038.1 | 94203 | 3 |
| essential512/enum | views-inputs | deferred | off | 2654.952 | 2654.246–2669.817 | 2514.865 | 20236.0 | 105459 | 3 |
| essential512/enum | views-inputs | source | off | 154.304 | 152.787–154.845 | 44.603 | 26324.7 | 121532 | 3 |
| essential512/slab | materialized | deferred | off | 166.057 | 159.501–184.168 | 50.622 | 17058.6 | 158235 | 9 |
| essential512/slab | views | deferred | off | 3155.670 | 3134.956–3176.368 | 3061.669 | 10296.6 | 74759 | 3 |
| essential512/slab | views | source | off | 134.703 | 128.617–136.464 | 49.358 | 15596.3 | 94203 | 9 |
| essential512/slab | views-inputs | deferred | off | 2656.518 | 2651.424–3145.121 | 2565.896 | 14865.2 | 105459 | 3 |
| essential512/slab | views-inputs | source | off | 131.738 | 128.983–142.391 | 41.528 | 17546.0 | 121532 | 9 |
| essential64/enum | materialized | deferred | off | 324.863 | 319.149–342.343 | 61.120 | 52825.5 | 376016 | 3 |
| essential64/enum | views | deferred | off | 3070.650 | 3070.482–3071.693 | 2829.489 | 52638.8 | 266936 | 3 |
| essential64/enum | views | source | off | 304.263 | 296.068–306.861 | 46.306 | 54667.8 | 295599 | 3 |
| essential64/enum | views-inputs | deferred | off | 3031.133 | 3008.265–3098.679 | 5733.887 | 52637.0 | 300225 | 3 |
| essential64/enum | views-inputs | source | off | 276.593 | 274.927–282.865 | 41.349 | 53622.3 | 315600 | 3 |
| essential64/slab | materialized | deferred | off | 356.225 | 311.136–368.948 | 58.750 | 36525.8 | 376016 | 3 |
| essential64/slab | views | deferred | off | 4279.557 | 3170.066–8207.896 | 2933.111 | 36419.2 | 266936 | 3 |
| essential64/slab | views | source | off | 297.966 | 290.777–299.409 | 47.566 | 38353.7 | 295599 | 3 |
| essential64/slab | views-inputs | deferred | off | 3194.735 | 3104.591–3898.752 | 2905.438 | 36419.2 | 300225 | 3 |
| essential64/slab | views-inputs | source | off | 275.768 | 275.670–337.736 | 43.319 | 37378.8 | 315600 | 3 |
| packed512/enum | materialized | deferred | off | 175.009 | 170.768–182.711 | 158.081 | 28269.2 | 178795 | 9 |

## asymmetric, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.730 | 2.595–2.920 | 0.015 | 4570.2 | 137 | 9 |
| essential512/enum | materialized | deferred | off | 127.406 | 122.391–132.540 | 0.018 | 26972.9 | 158235 | 3 |
| essential512/enum | views | deferred | off | 659.255 | 658.708–661.779 | 0.017 | 15853.1 | 74759 | 3 |
| essential512/enum | views | source | off | 94.797 | 86.101–96.420 | 0.016 | 21043.5 | 94203 | 3 |
| essential512/enum | views-inputs | deferred | off | 563.929 | 555.841–573.542 | 0.021 | 20241.4 | 105459 | 3 |
| essential512/enum | views-inputs | source | off | 105.695 | 105.484–107.677 | 0.018 | 26330.0 | 121532 | 3 |
| essential512/slab | materialized | deferred | off | 132.065 | 128.165–141.856 | 0.018 | 17064.0 | 158235 | 9 |
| essential512/slab | views | deferred | off | 638.184 | 637.238–646.287 | 0.016 | 10301.9 | 74759 | 3 |
| essential512/slab | views | source | off | 94.348 | 88.403–102.327 | 0.017 | 15601.7 | 94203 | 9 |
| essential512/slab | views-inputs | deferred | off | 579.357 | 574.870–689.427 | 0.018 | 14870.6 | 105459 | 3 |
| essential512/slab | views-inputs | source | off | 106.706 | 97.084–117.543 | 0.017 | 17551.4 | 121532 | 9 |
| essential64/enum | materialized | deferred | off | 282.378 | 276.088–290.666 | 0.017 | 52830.9 | 376016 | 3 |
| essential64/enum | views | deferred | off | 772.554 | 754.448–800.923 | 0.016 | 52644.2 | 266936 | 3 |
| essential64/enum | views | source | off | 260.574 | 247.538–264.159 | 0.017 | 54673.2 | 295599 | 3 |
| essential64/enum | views-inputs | deferred | off | 791.808 | 783.636–793.325 | 0.019 | 52642.4 | 300225 | 3 |
| essential64/enum | views-inputs | source | off | 238.806 | 237.993–248.692 | 0.016 | 53627.7 | 315600 | 3 |
| essential64/slab | materialized | deferred | off | 290.620 | 275.090–293.979 | 0.016 | 36531.2 | 376016 | 3 |
| essential64/slab | views | deferred | off | 765.771 | 756.194–770.891 | 0.017 | 36424.6 | 266936 | 3 |
| essential64/slab | views | source | off | 277.480 | 268.981–279.038 | 0.017 | 38359.1 | 295599 | 3 |
| essential64/slab | views-inputs | deferred | off | 780.724 | 761.339–814.188 | 0.017 | 36424.6 | 300225 | 3 |
| essential64/slab | views-inputs | source | off | 239.804 | 239.600–240.310 | 0.018 | 37384.2 | 315600 | 3 |
| packed512/enum | materialized | deferred | off | 43.751 | 42.725–46.969 | 0.298 | 28274.6 | 178795 | 9 |

## copied-face, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.243 | 0.236–0.261 | 0.243 | 22.2 | 36 | 9 |
| essential512/enum | materialized | deferred | off | 0.985 | 0.953–1.032 | 0.556 | 200.8 | 956 | 3 |
| essential512/enum | views | deferred | off | 2.276 | 2.233–2.341 | 2.146 | 97.9 | 467 | 3 |
| essential512/enum | views | source | off | 1.293 | 1.248–1.389 | 0.929 | 154.2 | 789 | 3 |
| essential512/enum | views-inputs | deferred | off | 2.452 | 2.352–2.481 | 2.198 | 98.0 | 471 | 3 |
| essential512/enum | views-inputs | source | off | 1.360 | 1.349–1.434 | 1.027 | 154.0 | 780 | 3 |
| essential512/slab | materialized | deferred | off | 0.983 | 0.968–1.123 | 0.583 | 124.0 | 956 | 9 |
| essential512/slab | views | deferred | off | 2.361 | 2.263–2.378 | 2.210 | 62.2 | 467 | 3 |
| essential512/slab | views | source | off | 1.321 | 1.265–1.354 | 0.977 | 101.2 | 789 | 9 |
| essential512/slab | views-inputs | deferred | off | 2.464 | 2.343–2.687 | 2.250 | 62.2 | 471 | 3 |
| essential512/slab | views-inputs | source | off | 1.478 | 1.427–1.748 | 1.086 | 101.2 | 780 | 9 |
| essential64/enum | materialized | deferred | off | 1.722 | 1.711–1.764 | 1.194 | 328.7 | 1972 | 3 |
| essential64/enum | views | deferred | off | 4.688 | 4.597–4.729 | 4.468 | 169.4 | 952 | 3 |
| essential64/enum | views | source | off | 1.768 | 1.679–1.818 | 1.297 | 246.8 | 1494 | 3 |
| essential64/enum | views-inputs | deferred | off | 4.704 | 4.669–4.774 | 4.562 | 169.5 | 970 | 3 |
| essential64/enum | views-inputs | source | off | 1.894 | 1.855–1.922 | 1.443 | 247.2 | 1495 | 3 |
| essential64/slab | materialized | deferred | off | 1.639 | 1.614–1.724 | 1.164 | 210.7 | 1972 | 3 |
| essential64/slab | views | deferred | off | 4.729 | 4.703–4.839 | 4.436 | 109.4 | 952 | 3 |
| essential64/slab | views | source | off | 1.768 | 1.746–1.796 | 1.389 | 169.0 | 1494 | 3 |
| essential64/slab | views-inputs | deferred | off | 4.837 | 4.792–4.856 | 4.508 | 109.4 | 970 | 3 |
| essential64/slab | views-inputs | source | off | 1.834 | 1.787–1.878 | 1.434 | 172.6 | 1495 | 3 |
| packed512/enum | materialized | deferred | off | 2.696 | 2.682–2.779 | 2.654 | 168.5 | 1174 | 9 |

## copied-face, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.053 | 0.052–0.057 | 0.007 | 23.4 | 36 | 9 |
| essential512/enum | materialized | deferred | off | 0.542 | 0.522–0.543 | 0.007 | 202.0 | 956 | 3 |
| essential512/enum | views | deferred | off | 0.535 | 0.528–0.558 | 0.007 | 99.1 | 467 | 3 |
| essential512/enum | views | source | off | 0.502 | 0.493–0.512 | 0.007 | 155.3 | 789 | 3 |
| essential512/enum | views-inputs | deferred | off | 0.549 | 0.540–0.555 | 0.007 | 99.1 | 471 | 3 |
| essential512/enum | views-inputs | source | off | 0.514 | 0.509–0.628 | 0.007 | 155.2 | 780 | 3 |
| essential512/slab | materialized | deferred | off | 0.516 | 0.495–0.623 | 0.007 | 125.2 | 956 | 9 |
| essential512/slab | views | deferred | off | 0.523 | 0.523–0.578 | 0.007 | 63.4 | 467 | 3 |
| essential512/slab | views | source | off | 0.514 | 0.495–0.559 | 0.007 | 102.4 | 789 | 9 |
| essential512/slab | views-inputs | deferred | off | 0.948 | 0.570–1.456 | 0.011 | 63.4 | 471 | 3 |
| essential512/slab | views-inputs | source | off | 0.575 | 0.533–0.619 | 0.007 | 102.4 | 780 | 9 |
| essential64/enum | materialized | deferred | off | 0.753 | 0.710–0.808 | 0.007 | 329.8 | 1972 | 3 |
| essential64/enum | views | deferred | off | 1.018 | 1.009–1.065 | 0.006 | 170.6 | 952 | 3 |
| essential64/enum | views | source | off | 0.608 | 0.603–0.609 | 0.007 | 248.0 | 1494 | 3 |
| essential64/enum | views-inputs | deferred | off | 1.065 | 1.027–1.225 | 0.007 | 170.7 | 970 | 3 |
| essential64/enum | views-inputs | source | off | 0.675 | 0.665–0.720 | 0.007 | 248.4 | 1495 | 3 |
| essential64/slab | materialized | deferred | off | 0.678 | 0.663–0.767 | 0.007 | 211.9 | 1972 | 3 |
| essential64/slab | views | deferred | off | 1.038 | 1.023–1.047 | 0.007 | 110.6 | 952 | 3 |
| essential64/slab | views | source | off | 0.658 | 0.630–0.676 | 0.010 | 170.2 | 1494 | 3 |
| essential64/slab | views-inputs | deferred | off | 1.064 | 1.057–1.104 | 0.008 | 110.6 | 970 | 3 |
| essential64/slab | views-inputs | source | off | 0.652 | 0.643–0.718 | 0.007 | 173.8 | 1495 | 3 |
| packed512/enum | materialized | deferred | off | 0.558 | 0.549–0.608 | 0.009 | 169.7 | 1174 | 9 |

## copied-face, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.234 | 0.233–0.249 | 0.233 | 22.2 | 36 | 9 |
| essential512/enum | materialized | deferred | off | 2.149 | 2.078–2.157 | 1.266 | 276.6 | 1655 | 3 |
| essential512/enum | views | deferred | off | 8.926 | 8.822–8.995 | 8.562 | 135.3 | 742 | 3 |
| essential512/enum | views | source | off | 3.257 | 3.006–4.396 | 2.389 | 275.8 | 1262 | 3 |
| essential512/enum | views-inputs | deferred | off | 5.740 | 5.737–5.998 | 5.582 | 134.6 | 754 | 3 |
| essential512/enum | views-inputs | source | off | 3.175 | 3.158–3.253 | 2.629 | 247.8 | 1154 | 3 |
| essential512/slab | materialized | deferred | off | 2.169 | 2.082–2.219 | 1.344 | 180.9 | 1655 | 9 |
| essential512/slab | views | deferred | off | 8.922 | 8.761–8.950 | 8.703 | 88.8 | 742 | 3 |
| essential512/slab | views | source | off | 3.080 | 3.052–3.159 | 2.431 | 188.6 | 1262 | 9 |
| essential512/slab | views-inputs | deferred | off | 5.823 | 5.800–5.825 | 5.654 | 88.8 | 754 | 3 |
| essential512/slab | views-inputs | source | off | 3.208 | 3.143–3.286 | 2.706 | 173.3 | 1154 | 9 |
| essential64/enum | materialized | deferred | off | 5.500 | 4.216–11.419 | 2.904 | 680.1 | 3745 | 3 |
| essential64/enum | views | deferred | off | 41.070 | 40.714–41.370 | 40.418 | 403.7 | 2173 | 3 |
| essential64/enum | views | source | off | 5.944 | 5.917–6.014 | 4.588 | 583.3 | 3575 | 3 |
| essential64/enum | views-inputs | deferred | off | 31.998 | 31.657–32.001 | 31.111 | 403.2 | 2099 | 3 |
| essential64/enum | views-inputs | source | off | 5.936 | 5.919–5.978 | 4.970 | 428.6 | 2668 | 3 |
| essential64/slab | materialized | deferred | off | 3.925 | 3.896–4.061 | 2.838 | 436.3 | 3745 | 3 |
| essential64/slab | views | deferred | off | 41.352 | 41.138–41.429 | 41.135 | 279.3 | 2173 | 3 |
| essential64/slab | views | source | off | 5.997 | 5.967–6.064 | 4.808 | 421.1 | 3575 | 3 |
| essential64/slab | views-inputs | deferred | off | 32.728 | 32.545–33.005 | 32.145 | 279.3 | 2099 | 3 |
| essential64/slab | views-inputs | source | off | 6.145 | 6.141–6.186 | 5.245 | 302.2 | 2668 | 3 |
| packed512/enum | materialized | deferred | off | 8.008 | 7.780–8.068 | 7.642 | 421.8 | 2763 | 9 |

## copied-face, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.051 | 0.050–0.053 | 0.007 | 23.4 | 36 | 9 |
| essential512/enum | materialized | deferred | off | 1.034 | 1.010–1.039 | 0.007 | 277.7 | 1655 | 3 |
| essential512/enum | views | deferred | off | 1.839 | 1.811–1.872 | 0.008 | 136.5 | 742 | 3 |
| essential512/enum | views | source | off | 1.091 | 1.071–1.201 | 0.007 | 277.0 | 1262 | 3 |
| essential512/enum | views-inputs | deferred | off | 1.272 | 1.263–1.423 | 0.007 | 135.7 | 754 | 3 |
| essential512/enum | views-inputs | source | off | 0.970 | 0.965–0.985 | 0.007 | 249.0 | 1154 | 3 |
| essential512/slab | materialized | deferred | off | 1.048 | 1.011–1.172 | 0.007 | 182.1 | 1655 | 9 |
| essential512/slab | views | deferred | off | 1.816 | 1.798–1.878 | 0.007 | 90.0 | 742 | 3 |
| essential512/slab | views | source | off | 1.083 | 1.074–1.220 | 0.007 | 189.7 | 1262 | 9 |
| essential512/slab | views-inputs | deferred | off | 1.273 | 1.259–1.277 | 0.007 | 90.0 | 754 | 3 |
| essential512/slab | views-inputs | source | off | 0.952 | 0.947–0.966 | 0.007 | 174.5 | 1154 | 9 |
| essential64/enum | materialized | deferred | off | 3.263 | 1.740–5.129 | 0.007 | 681.2 | 3745 | 3 |
| essential64/enum | views | deferred | off | 8.189 | 7.903–8.265 | 0.007 | 404.9 | 2173 | 3 |
| essential64/enum | views | source | off | 2.161 | 2.102–2.182 | 0.007 | 584.5 | 3575 | 3 |
| essential64/enum | views-inputs | deferred | off | 6.472 | 6.366–6.548 | 0.007 | 404.4 | 2099 | 3 |
| essential64/enum | views-inputs | source | off | 1.844 | 1.799–1.892 | 0.007 | 429.8 | 2668 | 3 |
| essential64/slab | materialized | deferred | off | 1.609 | 1.590–1.617 | 0.007 | 437.5 | 3745 | 3 |
| essential64/slab | views | deferred | off | 8.225 | 8.193–8.271 | 0.009 | 280.5 | 2173 | 3 |
| essential64/slab | views | source | off | 2.176 | 2.155–2.196 | 0.007 | 422.3 | 3575 | 3 |
| essential64/slab | views-inputs | deferred | off | 6.585 | 6.580–6.603 | 0.010 | 280.5 | 2099 | 3 |
| essential64/slab | views-inputs | source | off | 1.832 | 1.816–1.915 | 0.007 | 303.3 | 2668 | 3 |
| packed512/enum | materialized | deferred | off | 1.593 | 1.564–1.650 | 0.010 | 423.0 | 2763 | 9 |

## copied-face, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 13.552 | 13.290–13.810 | 13.438 | 1248.2 | 36 | 9 |
| essential512/enum | materialized | deferred | off | 5.582 | 5.485–5.800 | 3.665 | 955.8 | 6698 | 3 |
| essential512/enum | views | deferred | off | 15.538 | 15.435–15.884 | 14.996 | 491.1 | 2823 | 3 |
| essential512/enum | views | source | off | 4.891 | 4.891–4.976 | 3.632 | 960.9 | 4596 | 3 |
| essential512/enum | views-inputs | deferred | off | 15.865 | 15.855–15.868 | 15.271 | 493.0 | 2921 | 3 |
| essential512/enum | views-inputs | source | off | 5.749 | 5.581–5.784 | 4.217 | 965.7 | 4606 | 3 |
| essential512/slab | materialized | deferred | off | 5.689 | 5.640–5.832 | 3.836 | 645.6 | 6698 | 9 |
| essential512/slab | views | deferred | off | 15.413 | 15.397–15.426 | 14.959 | 338.4 | 2823 | 3 |
| essential512/slab | views | source | off | 5.153 | 4.985–5.365 | 3.786 | 676.0 | 4596 | 9 |
| essential512/slab | views-inputs | deferred | off | 16.590 | 16.558–16.881 | 15.713 | 338.4 | 2921 | 3 |
| essential512/slab | views-inputs | source | off | 5.715 | 5.542–5.796 | 4.204 | 676.0 | 4606 | 9 |
| essential64/enum | materialized | deferred | off | 6.623 | 6.608–6.745 | 4.618 | 1362.4 | 9044 | 3 |
| essential64/enum | views | deferred | off | 30.658 | 30.448–30.872 | 29.939 | 684.0 | 3781 | 3 |
| essential64/enum | views | source | off | 4.634 | 4.633–4.673 | 3.532 | 793.4 | 5900 | 3 |
| essential64/enum | views-inputs | deferred | off | 30.431 | 30.417–31.109 | 30.691 | 684.5 | 3964 | 3 |
| essential64/enum | views-inputs | source | off | 5.136 | 5.103–5.505 | 3.890 | 794.5 | 5971 | 3 |
| essential64/slab | materialized | deferred | off | 6.505 | 6.487–6.651 | 4.830 | 866.0 | 9044 | 3 |
| essential64/slab | views | deferred | off | 30.805 | 30.413–31.216 | 30.230 | 437.5 | 3781 | 3 |
| essential64/slab | views | source | off | 5.004 | 4.798–5.283 | 3.763 | 544.1 | 5900 | 3 |
| essential64/slab | views-inputs | deferred | off | 30.588 | 30.562–30.793 | 30.164 | 437.5 | 3964 | 3 |
| essential64/slab | views-inputs | source | off | 5.143 | 5.095–5.308 | 3.893 | 544.1 | 5971 | 3 |
| packed512/enum | materialized | deferred | off | 6.267 | 6.156–6.363 | 5.876 | 966.4 | 7085 | 9 |

## copied-face, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.510 | 2.494–2.557 | 0.014 | 1249.3 | 36 | 9 |
| essential512/enum | materialized | deferred | off | 2.575 | 2.566–2.683 | 0.015 | 957.0 | 6698 | 3 |
| essential512/enum | views | deferred | off | 3.396 | 3.391–3.401 | 0.015 | 492.2 | 2823 | 3 |
| essential512/enum | views | source | off | 2.012 | 1.992–2.077 | 0.011 | 962.1 | 4596 | 3 |
| essential512/enum | views-inputs | deferred | off | 3.502 | 3.440–3.601 | 0.013 | 494.2 | 2921 | 3 |
| essential512/enum | views-inputs | source | off | 2.193 | 2.138–2.265 | 0.016 | 966.9 | 4606 | 3 |
| essential512/slab | materialized | deferred | off | 2.686 | 2.596–2.921 | 0.014 | 646.8 | 6698 | 9 |
| essential512/slab | views | deferred | off | 3.332 | 3.294–3.392 | 0.015 | 339.5 | 2823 | 3 |
| essential512/slab | views | source | off | 2.024 | 1.973–2.221 | 0.015 | 677.2 | 4596 | 9 |
| essential512/slab | views-inputs | deferred | off | 3.543 | 3.488–3.655 | 0.013 | 339.5 | 2921 | 3 |
| essential512/slab | views-inputs | source | off | 2.222 | 2.136–2.476 | 0.016 | 677.2 | 4606 | 9 |
| essential64/enum | materialized | deferred | off | 2.730 | 2.729–2.795 | 0.012 | 1363.6 | 9044 | 3 |
| essential64/enum | views | deferred | off | 6.223 | 6.216–6.445 | 0.014 | 685.2 | 3781 | 3 |
| essential64/enum | views | source | off | 1.828 | 1.771–1.899 | 0.012 | 794.6 | 5900 | 3 |
| essential64/enum | views-inputs | deferred | off | 6.502 | 6.444–6.545 | 0.018 | 685.7 | 3964 | 3 |
| essential64/enum | views-inputs | source | off | 1.945 | 1.941–1.990 | 0.014 | 795.7 | 5971 | 3 |
| essential64/slab | materialized | deferred | off | 2.806 | 2.781–3.154 | 0.016 | 867.2 | 9044 | 3 |
| essential64/slab | views | deferred | off | 6.255 | 6.228–6.309 | 0.014 | 438.6 | 3781 | 3 |
| essential64/slab | views | source | off | 1.890 | 1.881–1.924 | 0.016 | 545.3 | 5900 | 3 |
| essential64/slab | views-inputs | deferred | off | 6.289 | 6.272–6.307 | 0.013 | 438.6 | 3964 | 3 |
| essential64/slab | views-inputs | source | off | 1.940 | 1.938–1.943 | 0.014 | 545.3 | 5971 | 3 |
| packed512/enum | materialized | deferred | off | 1.577 | 1.536–1.633 | 0.020 | 967.6 | 7085 | 9 |

## copied-face, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 13.385 | 13.079–13.674 | 13.781 | 1248.2 | 36 | 9 |
| essential512/enum | materialized | deferred | off | 30.056 | 29.588–32.032 | 20.319 | 4449.1 | 25882 | 3 |
| essential512/enum | views | deferred | off | 1034.104 | 1027.803–1044.678 | 1023.144 | 3397.8 | 14346 | 3 |
| essential512/enum | views | source | off | 43.610 | 42.643–43.944 | 31.563 | 5097.9 | 25516 | 3 |
| essential512/enum | views-inputs | deferred | off | 2133.547 | 2129.395–2149.936 | 2129.680 | 2801.9 | 14300 | 3 |
| essential512/enum | views-inputs | source | off | 39.958 | 39.497–42.198 | 33.027 | 3718.2 | 19116 | 3 |
| essential512/slab | materialized | deferred | off | 30.382 | 29.831–32.614 | 20.770 | 2977.8 | 25882 | 9 |
| essential512/slab | views | deferred | off | 1016.268 | 1010.951–1020.634 | 1015.898 | 2367.3 | 14346 | 3 |
| essential512/slab | views | source | off | 43.983 | 43.062–45.641 | 33.012 | 3698.1 | 25516 | 9 |
| essential512/slab | views-inputs | deferred | off | 2112.272 | 2110.718–2114.935 | 2111.341 | 2123.6 | 14300 | 3 |
| essential512/slab | views-inputs | source | off | 39.255 | 39.101–41.103 | 32.258 | 2580.5 | 19116 | 9 |
| essential64/enum | materialized | deferred | off | 107.044 | 94.901–128.821 | 45.145 | 6249.3 | 28773 | 3 |
| essential64/enum | views | deferred | off | 159.475 | 158.026–160.354 | 156.545 | 4099.9 | 16232 | 3 |
| essential64/enum | views | source | off | 38.327 | 38.099–38.328 | 31.209 | 6741.5 | 30035 | 3 |
| essential64/enum | views-inputs | deferred | off | 130.003 | 128.748–131.192 | 128.459 | 4099.9 | 16046 | 3 |
| essential64/enum | views-inputs | source | off | 34.634 | 34.613–34.701 | 31.340 | 4340.5 | 21142 | 3 |
| essential64/slab | materialized | deferred | off | 24.160 | 24.087–24.654 | 19.138 | 4296.3 | 28773 | 3 |
| essential64/slab | views | deferred | off | 159.490 | 157.680–159.992 | 157.277 | 3123.4 | 16232 | 3 |
| essential64/slab | views | source | off | 39.447 | 38.719–39.929 | 32.332 | 4779.1 | 30035 | 3 |
| essential64/slab | views-inputs | deferred | off | 130.306 | 130.250–131.032 | 128.258 | 3123.4 | 16046 | 3 |
| essential64/slab | views-inputs | source | off | 35.770 | 35.356–36.075 | 32.723 | 3359.5 | 21142 | 3 |
| packed512/enum | materialized | deferred | off | 78.614 | 77.575–80.848 | 77.261 | 3470.5 | 25071 | 9 |

## copied-face, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.516 | 2.481–2.637 | 0.014 | 1249.3 | 36 | 9 |
| essential512/enum | materialized | deferred | off | 13.734 | 13.675–13.856 | 0.016 | 4450.2 | 25882 | 3 |
| essential512/enum | views | deferred | off | 197.369 | 197.247–199.463 | 0.016 | 3398.9 | 14346 | 3 |
| essential512/enum | views | source | off | 16.820 | 16.784–16.855 | 0.015 | 5099.1 | 25516 | 3 |
| essential512/enum | views-inputs | deferred | off | 403.219 | 402.630–406.331 | 0.015 | 2803.1 | 14300 | 3 |
| essential512/enum | views-inputs | source | off | 13.482 | 13.351–13.655 | 0.015 | 3719.4 | 19116 | 3 |
| essential512/slab | materialized | deferred | off | 13.785 | 13.380–15.284 | 0.016 | 2979.0 | 25882 | 9 |
| essential512/slab | views | deferred | off | 194.832 | 194.374–195.576 | 0.014 | 2368.5 | 14346 | 3 |
| essential512/slab | views | source | off | 17.337 | 16.902–18.615 | 0.015 | 3699.3 | 25516 | 9 |
| essential512/slab | views-inputs | deferred | off | 399.727 | 399.353–401.474 | 0.014 | 2124.7 | 14300 | 3 |
| essential512/slab | views-inputs | source | off | 13.265 | 13.118–14.005 | 0.015 | 2581.7 | 19116 | 9 |
| essential64/enum | materialized | deferred | off | 22.768 | 11.713–53.641 | 0.019 | 6250.4 | 28773 | 3 |
| essential64/enum | views | deferred | off | 31.953 | 31.351–32.015 | 0.015 | 4101.1 | 16232 | 3 |
| essential64/enum | views | source | off | 13.060 | 12.796–13.123 | 0.013 | 6742.7 | 30035 | 3 |
| essential64/enum | views-inputs | deferred | off | 26.433 | 26.215–26.442 | 0.015 | 4101.1 | 16046 | 3 |
| essential64/enum | views-inputs | source | off | 9.764 | 9.593–10.147 | 0.016 | 4341.7 | 21142 | 3 |
| essential64/slab | materialized | deferred | off | 8.892 | 8.793–8.910 | 0.013 | 4297.5 | 28773 | 3 |
| essential64/slab | views | deferred | off | 31.876 | 31.759–31.913 | 0.016 | 3124.5 | 16232 | 3 |
| essential64/slab | views | source | off | 12.788 | 12.569–13.750 | 0.014 | 4780.2 | 30035 | 3 |
| essential64/slab | views-inputs | deferred | off | 26.311 | 26.136–26.392 | 0.015 | 3124.5 | 16046 | 3 |
| essential64/slab | views-inputs | source | off | 10.414 | 9.608–10.721 | 0.016 | 3360.6 | 21142 | 3 |
| packed512/enum | materialized | deferred | off | 16.114 | 15.628–16.888 | 0.068 | 3471.7 | 25071 | 9 |

## full, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.270 | 0.258–0.328 | 0.267 | 78.3 | 132 | 9 |
| essential512/enum | materialized | deferred | off | 0.943 | 0.920–0.969 | 0.718 | 57.2 | 350 | 3 |
| essential512/enum | views | deferred | off | 2.069 | 2.061–2.097 | 1.945 | 35.5 | 218 | 3 |
| essential512/enum | views | source | off | 1.065 | 1.060–1.107 | 0.936 | 47.6 | 260 | 3 |
| essential512/enum | views-inputs | deferred | off | 2.125 | 2.105–2.196 | 1.973 | 45.0 | 230 | 3 |
| essential512/enum | views-inputs | source | off | 1.142 | 1.118–1.162 | 0.967 | 48.0 | 276 | 3 |
| essential512/slab | materialized | deferred | off | 0.906 | 0.903–0.996 | 0.738 | 40.9 | 350 | 9 |
| essential512/slab | views | deferred | off | 2.094 | 2.044–2.152 | 2.004 | 25.3 | 218 | 3 |
| essential512/slab | views | source | off | 1.104 | 1.048–1.200 | 0.986 | 31.0 | 260 | 9 |
| essential512/slab | views-inputs | deferred | off | 2.094 | 2.093–2.222 | 2.003 | 29.1 | 230 | 3 |
| essential512/slab | views-inputs | source | off | 1.113 | 1.081–1.254 | 0.997 | 31.0 | 276 | 9 |
| essential64/enum | materialized | deferred | off | 2.200 | 2.170–2.319 | 1.641 | 155.3 | 1110 | 3 |
| essential64/enum | views | deferred | off | 9.941 | 9.867–10.061 | 9.724 | 75.4 | 517 | 3 |
| essential64/enum | views | source | off | 2.190 | 2.133–2.267 | 1.872 | 107.1 | 698 | 3 |
| essential64/enum | views-inputs | deferred | off | 9.950 | 9.864–10.029 | 9.585 | 101.7 | 660 | 3 |
| essential64/enum | views-inputs | source | off | 2.517 | 2.419–2.561 | 2.088 | 108.4 | 856 | 3 |
| essential64/slab | materialized | deferred | off | 2.101 | 2.099–2.169 | 1.619 | 95.7 | 1110 | 3 |
| essential64/slab | views | deferred | off | 10.025 | 9.914–10.080 | 9.770 | 46.2 | 517 | 3 |
| essential64/slab | views | source | off | 2.169 | 2.157–2.272 | 1.916 | 68.2 | 698 | 3 |
| essential64/slab | views-inputs | deferred | off | 9.980 | 9.969–10.089 | 9.766 | 64.4 | 660 | 3 |
| essential64/slab | views-inputs | source | off | 2.453 | 2.368–2.477 | 2.068 | 72.8 | 856 | 3 |
| packed512/enum | materialized | deferred | off | 3.235 | 3.181–3.340 | 3.216 | 137.7 | 991 | 9 |

## full, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.059 | 0.057–0.071 | 0.007 | 83.7 | 132 | 9 |
| essential512/enum | materialized | deferred | off | 0.323 | 0.320–0.331 | 0.008 | 62.6 | 350 | 3 |
| essential512/enum | views | deferred | off | 0.431 | 0.428–0.443 | 0.008 | 40.9 | 218 | 3 |
| essential512/enum | views | source | off | 0.278 | 0.271–0.299 | 0.007 | 53.0 | 260 | 3 |
| essential512/enum | views-inputs | deferred | off | 0.456 | 0.444–0.462 | 0.007 | 50.4 | 230 | 3 |
| essential512/enum | views-inputs | source | off | 0.302 | 0.275–0.329 | 0.007 | 53.4 | 276 | 3 |
| essential512/slab | materialized | deferred | off | 0.320 | 0.318–0.359 | 0.007 | 46.2 | 350 | 9 |
| essential512/slab | views | deferred | off | 0.444 | 0.439–0.477 | 0.007 | 30.7 | 218 | 3 |
| essential512/slab | views | source | off | 0.270 | 0.267–0.295 | 0.007 | 36.4 | 260 | 9 |
| essential512/slab | views-inputs | deferred | off | 0.457 | 0.449–0.481 | 0.007 | 34.5 | 230 | 3 |
| essential512/slab | views-inputs | source | off | 0.280 | 0.278–0.294 | 0.007 | 36.4 | 276 | 9 |
| essential64/enum | materialized | deferred | off | 0.816 | 0.803–0.822 | 0.007 | 160.6 | 1110 | 3 |
| essential64/enum | views | deferred | off | 2.096 | 2.000–2.102 | 0.007 | 80.8 | 517 | 3 |
| essential64/enum | views | source | off | 0.618 | 0.614–0.633 | 0.007 | 112.4 | 698 | 3 |
| essential64/enum | views-inputs | deferred | off | 2.045 | 2.026–2.092 | 0.007 | 107.1 | 660 | 3 |
| essential64/enum | views-inputs | source | off | 0.709 | 0.702–0.789 | 0.007 | 113.8 | 856 | 3 |
| essential64/slab | materialized | deferred | off | 0.780 | 0.766–0.808 | 0.007 | 101.1 | 1110 | 3 |
| essential64/slab | views | deferred | off | 2.011 | 1.972–2.027 | 0.007 | 51.5 | 517 | 3 |
| essential64/slab | views | source | off | 0.653 | 0.625–0.680 | 0.007 | 73.6 | 698 | 3 |
| essential64/slab | views-inputs | deferred | off | 2.053 | 2.051–2.110 | 0.007 | 69.8 | 660 | 3 |
| essential64/slab | views-inputs | source | off | 0.704 | 0.699–0.727 | 0.007 | 78.2 | 856 | 3 |
| packed512/enum | materialized | deferred | off | 0.678 | 0.642–0.714 | 0.009 | 143.0 | 991 | 9 |

## full, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.256 | 0.254–0.310 | 0.252 | 78.3 | 132 | 9 |
| essential512/enum | materialized | deferred | off | 1.292 | 1.288–1.355 | 1.074 | 76.7 | 441 | 3 |
| essential512/enum | views | deferred | off | 1.721 | 1.694–1.785 | 1.517 | 50.8 | 277 | 3 |
| essential512/enum | views | source | off | 0.913 | 0.888–0.955 | 0.713 | 56.2 | 356 | 3 |
| essential512/enum | views-inputs | deferred | off | 1.958 | 1.936–2.148 | 1.907 | 44.3 | 268 | 3 |
| essential512/enum | views-inputs | source | off | 1.267 | 1.237–1.289 | 1.162 | 47.2 | 327 | 3 |
| essential512/slab | materialized | deferred | off | 1.223 | 1.197–1.280 | 1.055 | 52.1 | 441 | 9 |
| essential512/slab | views | deferred | off | 1.674 | 1.666–1.806 | 1.577 | 31.0 | 277 | 9 |
| essential512/slab | views | source | off | 0.943 | 0.889–0.971 | 0.752 | 34.8 | 356 | 9 |
| essential512/slab | views-inputs | deferred | off | 2.086 | 2.019–2.199 | 2.053 | 28.2 | 268 | 9 |
| essential512/slab | views-inputs | source | off | 1.293 | 1.266–1.369 | 1.231 | 30.1 | 327 | 9 |
| essential64/enum | materialized | deferred | off | 1.633 | 1.608–1.672 | 1.247 | 167.5 | 1033 | 3 |
| essential64/enum | views | deferred | off | 8.068 | 8.031–8.122 | 7.663 | 163.5 | 1034 | 3 |
| essential64/enum | views | source | off | 1.977 | 1.916–2.004 | 1.435 | 177.2 | 1196 | 3 |
| essential64/enum | views-inputs | deferred | off | 7.716 | 7.685–7.733 | 7.371 | 82.0 | 628 | 3 |
| essential64/enum | views-inputs | source | off | 1.515 | 1.472–1.546 | 1.208 | 90.5 | 757 | 3 |
| essential64/slab | materialized | deferred | off | 1.623 | 1.579–1.627 | 1.236 | 103.8 | 1033 | 3 |
| essential64/slab | views | deferred | off | 8.274 | 8.148–8.335 | 7.890 | 99.9 | 1034 | 3 |
| essential64/slab | views | source | off | 1.981 | 1.965–2.026 | 1.560 | 111.4 | 1196 | 3 |
| essential64/slab | views-inputs | deferred | off | 7.598 | 7.564–7.639 | 7.429 | 49.2 | 628 | 3 |
| essential64/slab | views-inputs | source | off | 1.526 | 1.511–1.655 | 1.282 | 59.6 | 757 | 3 |
| packed512/enum | materialized | deferred | off | 5.893 | 5.836–6.089 | 5.785 | 300.5 | 1300 | 9 |

## full, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.059 | 0.058–0.064 | 0.007 | 83.7 | 132 | 9 |
| essential512/enum | materialized | deferred | off | 0.379 | 0.356–0.418 | 0.007 | 82.1 | 441 | 3 |
| essential512/enum | views | deferred | off | 0.467 | 0.446–0.488 | 0.007 | 56.2 | 277 | 3 |
| essential512/enum | views | source | off | 0.301 | 0.298–0.319 | 0.007 | 61.6 | 356 | 3 |
| essential512/enum | views-inputs | deferred | off | 0.444 | 0.418–0.500 | 0.007 | 49.7 | 268 | 3 |
| essential512/enum | views-inputs | source | off | 0.312 | 0.290–0.469 | 0.007 | 52.6 | 327 | 3 |
| essential512/slab | materialized | deferred | off | 0.343 | 0.339–0.367 | 0.007 | 57.4 | 441 | 9 |
| essential512/slab | views | deferred | off | 0.430 | 0.427–0.458 | 0.007 | 36.4 | 277 | 9 |
| essential512/slab | views | source | off | 0.304 | 0.292–0.336 | 0.007 | 40.2 | 356 | 9 |
| essential512/slab | views-inputs | deferred | off | 0.433 | 0.419–0.545 | 0.007 | 33.5 | 268 | 9 |
| essential512/slab | views-inputs | source | off | 0.302 | 0.298–0.345 | 0.007 | 35.4 | 327 | 9 |
| essential64/enum | materialized | deferred | off | 0.584 | 0.559–0.616 | 0.007 | 172.8 | 1033 | 3 |
| essential64/enum | views | deferred | off | 1.824 | 1.823–1.890 | 0.007 | 168.9 | 1034 | 3 |
| essential64/enum | views | source | off | 0.736 | 0.735–0.758 | 0.007 | 182.6 | 1196 | 3 |
| essential64/enum | views-inputs | deferred | off | 1.603 | 1.562–1.669 | 0.007 | 87.4 | 628 | 3 |
| essential64/enum | views-inputs | source | off | 0.474 | 0.472–0.483 | 0.007 | 95.8 | 757 | 3 |
| essential64/slab | materialized | deferred | off | 0.551 | 0.544–0.567 | 0.007 | 109.1 | 1033 | 3 |
| essential64/slab | views | deferred | off | 1.923 | 1.879–1.934 | 0.007 | 105.3 | 1034 | 3 |
| essential64/slab | views | source | off | 0.785 | 0.759–0.848 | 0.007 | 116.7 | 1196 | 3 |
| essential64/slab | views-inputs | deferred | off | 1.564 | 1.525–1.673 | 0.007 | 54.6 | 628 | 3 |
| essential64/slab | views-inputs | source | off | 0.470 | 0.468–0.492 | 0.007 | 65.0 | 757 | 3 |
| packed512/enum | materialized | deferred | off | 1.190 | 1.164–1.288 | 0.007 | 305.9 | 1300 | 9 |

## full, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 13.969 | 13.631–14.179 | 13.646 | 4564.8 | 137 | 9 |
| essential512/enum | materialized | deferred | off | 9.628 | 9.598–9.711 | 7.542 | 758.9 | 3681 | 3 |
| essential512/enum | views | deferred | off | 173.215 | 173.123–174.072 | 172.818 | 369.9 | 1963 | 3 |
| essential512/enum | views | source | off | 10.556 | 10.549–10.612 | 9.599 | 422.5 | 2367 | 3 |
| essential512/enum | views-inputs | deferred | off | 173.894 | 173.312–175.600 | 172.634 | 385.2 | 2460 | 3 |
| essential512/enum | views-inputs | source | off | 11.105 | 11.076–11.134 | 10.031 | 541.0 | 2966 | 3 |
| essential512/slab | materialized | deferred | off | 9.501 | 9.410–9.860 | 7.625 | 496.8 | 3681 | 9 |
| essential512/slab | views | deferred | off | 172.472 | 172.215–173.081 | 173.541 | 243.0 | 1963 | 3 |
| essential512/slab | views | source | off | 10.937 | 10.756–11.304 | 9.893 | 271.6 | 2367 | 9 |
| essential512/slab | views-inputs | deferred | off | 175.061 | 174.705–176.877 | 175.116 | 242.1 | 2460 | 3 |
| essential512/slab | views-inputs | source | off | 11.211 | 11.060–11.456 | 10.194 | 342.8 | 2966 | 9 |
| essential64/enum | materialized | deferred | off | 16.407 | 14.007–22.693 | 12.611 | 1113.7 | 7286 | 3 |
| essential64/enum | views | deferred | off | 247.463 | 247.458–247.630 | 246.796 | 403.0 | 3374 | 3 |
| essential64/enum | views | source | off | 14.907 | 14.687–15.670 | 12.793 | 608.6 | 4129 | 3 |
| essential64/enum | views-inputs | deferred | off | 245.505 | 243.978–246.494 | 239.586 | 784.3 | 5126 | 3 |
| essential64/enum | views-inputs | source | off | 15.792 | 15.346–15.836 | 13.689 | 843.5 | 5840 | 3 |
| essential64/slab | materialized | deferred | off | 13.519 | 13.484–13.533 | 10.920 | 651.3 | 7286 | 3 |
| essential64/slab | views | deferred | off | 244.905 | 244.874–246.669 | 243.012 | 257.4 | 3374 | 3 |
| essential64/slab | views | source | off | 14.752 | 14.583–14.758 | 12.966 | 381.3 | 4129 | 3 |
| essential64/slab | views-inputs | deferred | off | 243.730 | 243.412–244.956 | 244.380 | 506.6 | 5126 | 3 |
| essential64/slab | views-inputs | source | off | 15.243 | 15.186–15.291 | 13.100 | 559.9 | 5840 | 3 |
| packed512/enum | materialized | deferred | off | 8.104 | 8.021–8.276 | 7.620 | 808.6 | 7197 | 9 |

## full, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.719 | 2.585–2.978 | 0.015 | 4570.2 | 137 | 9 |
| essential512/enum | materialized | deferred | off | 3.392 | 3.327–3.411 | 0.020 | 764.3 | 3681 | 3 |
| essential512/enum | views | deferred | off | 33.251 | 33.251–33.786 | 0.013 | 375.3 | 1963 | 3 |
| essential512/enum | views | source | off | 2.930 | 2.844–3.015 | 0.017 | 427.8 | 2367 | 3 |
| essential512/enum | views-inputs | deferred | off | 33.123 | 33.110–33.158 | 0.013 | 390.6 | 2460 | 3 |
| essential512/enum | views-inputs | source | off | 3.123 | 3.009–3.225 | 0.016 | 546.4 | 2966 | 3 |
| essential512/slab | materialized | deferred | off | 3.351 | 3.318–3.539 | 0.019 | 502.2 | 3681 | 9 |
| essential512/slab | views | deferred | off | 32.855 | 32.817–32.869 | 0.012 | 248.4 | 1963 | 3 |
| essential512/slab | views | source | off | 2.948 | 2.885–8.975 | 0.017 | 277.0 | 2367 | 9 |
| essential512/slab | views-inputs | deferred | off | 33.571 | 33.382–34.270 | 0.016 | 247.5 | 2460 | 3 |
| essential512/slab | views-inputs | source | off | 3.035 | 2.994–3.260 | 0.016 | 348.2 | 2966 | 9 |
| essential64/enum | materialized | deferred | off | 4.884 | 4.705–4.888 | 0.018 | 1119.1 | 7286 | 3 |
| essential64/enum | views | deferred | off | 47.658 | 47.612–48.130 | 0.016 | 408.3 | 3374 | 3 |
| essential64/enum | views | source | off | 4.331 | 4.201–4.350 | 0.016 | 614.0 | 4129 | 3 |
| essential64/enum | views-inputs | deferred | off | 46.739 | 46.713–46.989 | 0.016 | 789.7 | 5126 | 3 |
| essential64/enum | views-inputs | source | off | 4.698 | 4.521–4.743 | 0.017 | 848.9 | 5840 | 3 |
| essential64/slab | materialized | deferred | off | 4.613 | 4.597–4.750 | 0.016 | 656.6 | 7286 | 3 |
| essential64/slab | views | deferred | off | 46.867 | 46.836–47.742 | 0.017 | 262.8 | 3374 | 3 |
| essential64/slab | views | source | off | 4.249 | 4.162–4.290 | 0.016 | 386.7 | 4129 | 3 |
| essential64/slab | views-inputs | deferred | off | 47.902 | 47.394–48.870 | 0.015 | 511.9 | 5126 | 3 |
| essential64/slab | views-inputs | source | off | 4.675 | 4.525–4.750 | 0.017 | 565.2 | 5840 | 3 |
| packed512/enum | materialized | deferred | off | 1.920 | 1.869–1.998 | 0.038 | 814.0 | 7197 | 9 |

## full, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 13.806 | 13.694–14.150 | 13.873 | 4564.8 | 137 | 9 |
| essential512/enum | materialized | deferred | off | 7.981 | 7.803–8.195 | 6.495 | 835.1 | 3908 | 3 |
| essential512/enum | views | deferred | off | 138.184 | 136.232–139.668 | 134.740 | 1301.0 | 6677 | 3 |
| essential512/enum | views | source | off | 19.543 | 19.424–20.776 | 15.893 | 1363.6 | 7164 | 3 |
| essential512/enum | views-inputs | deferred | off | 123.390 | 123.313–123.949 | 122.760 | 432.3 | 2627 | 3 |
| essential512/enum | views-inputs | source | off | 8.526 | 8.435–8.660 | 7.358 | 490.2 | 3262 | 3 |
| essential512/slab | materialized | deferred | off | 7.798 | 7.737–8.117 | 6.390 | 510.0 | 3908 | 9 |
| essential512/slab | views | deferred | off | 141.769 | 140.688–142.893 | 138.114 | 867.0 | 6677 | 9 |
| essential512/slab | views | source | off | 20.121 | 19.667–20.731 | 16.539 | 897.5 | 7164 | 9 |
| essential512/slab | views-inputs | deferred | off | 129.954 | 127.926–131.090 | 129.193 | 295.7 | 2627 | 9 |
| essential512/slab | views-inputs | source | off | 8.505 | 8.415–8.727 | 7.357 | 325.9 | 3262 | 9 |
| essential64/enum | materialized | deferred | off | 9.319 | 9.284–9.358 | 6.679 | 1445.5 | 9371 | 3 |
| essential64/enum | views | deferred | off | 95.843 | 95.709–96.865 | 92.641 | 1443.3 | 11374 | 3 |
| essential64/enum | views | source | off | 21.221 | 21.105–21.360 | 17.644 | 1500.3 | 12948 | 3 |
| essential64/enum | views-inputs | deferred | off | 17.611 | 17.602–17.967 | 16.095 | 722.2 | 6471 | 3 |
| essential64/enum | views-inputs | source | off | 8.847 | 8.824–8.857 | 6.896 | 754.5 | 6985 | 3 |
| essential64/slab | materialized | deferred | off | 9.779 | 9.548–9.980 | 7.359 | 919.8 | 9371 | 3 |
| essential64/slab | views | deferred | off | 98.628 | 97.721–98.641 | 95.855 | 919.8 | 11374 | 3 |
| essential64/slab | views | source | off | 22.043 | 21.929–22.975 | 18.084 | 981.0 | 12948 | 3 |
| essential64/slab | views-inputs | deferred | off | 17.690 | 17.610–18.127 | 15.868 | 460.9 | 6471 | 3 |
| essential64/slab | views-inputs | source | off | 8.944 | 8.934–8.973 | 7.076 | 490.8 | 6985 | 3 |
| packed512/enum | materialized | deferred | off | 16.943 | 16.857–17.248 | 16.464 | 1665.2 | 8004 | 9 |

## full, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.648 | 2.616–2.687 | 0.015 | 4570.2 | 137 | 9 |
| essential512/enum | materialized | deferred | off | 2.640 | 2.622–2.642 | 0.015 | 840.5 | 3908 | 3 |
| essential512/enum | views | deferred | off | 28.608 | 28.272–28.744 | 0.016 | 1306.4 | 6677 | 3 |
| essential512/enum | views | source | off | 6.626 | 6.429–7.056 | 0.016 | 1369.0 | 7164 | 3 |
| essential512/enum | views-inputs | deferred | off | 23.852 | 23.822–23.861 | 0.014 | 437.7 | 2627 | 3 |
| essential512/enum | views-inputs | source | off | 2.548 | 2.536–2.821 | 0.016 | 495.6 | 3262 | 3 |
| essential512/slab | materialized | deferred | off | 2.571 | 2.507–2.859 | 0.018 | 515.4 | 3908 | 9 |
| essential512/slab | views | deferred | off | 28.969 | 28.504–29.499 | 0.016 | 872.4 | 6677 | 9 |
| essential512/slab | views | source | off | 6.577 | 6.387–6.754 | 0.018 | 902.8 | 7164 | 9 |
| essential512/slab | views-inputs | deferred | off | 24.907 | 24.671–25.393 | 0.017 | 301.0 | 2627 | 9 |
| essential512/slab | views-inputs | source | off | 2.496 | 2.409–2.534 | 0.015 | 331.3 | 3262 | 9 |
| essential64/enum | materialized | deferred | off | 3.970 | 3.820–3.979 | 0.017 | 1450.8 | 9371 | 3 |
| essential64/enum | views | deferred | off | 20.413 | 20.344–20.421 | 0.012 | 1448.7 | 11374 | 3 |
| essential64/enum | views | source | off | 7.023 | 6.946–7.107 | 0.015 | 1505.6 | 12948 | 3 |
| essential64/enum | views-inputs | deferred | off | 4.771 | 4.732–4.994 | 0.017 | 727.6 | 6471 | 3 |
| essential64/enum | views-inputs | source | off | 3.303 | 3.247–3.376 | 0.015 | 759.9 | 6985 | 3 |
| essential64/slab | materialized | deferred | off | 3.931 | 3.840–4.002 | 0.017 | 925.2 | 9371 | 3 |
| essential64/slab | views | deferred | off | 20.951 | 20.743–21.243 | 0.016 | 925.2 | 11374 | 3 |
| essential64/slab | views | source | off | 7.084 | 7.053–7.297 | 0.016 | 986.4 | 12948 | 3 |
| essential64/slab | views-inputs | deferred | off | 4.895 | 4.725–4.946 | 0.017 | 466.2 | 6471 | 3 |
| essential64/slab | views-inputs | source | off | 3.313 | 3.263–3.315 | 0.013 | 496.2 | 6985 | 3 |
| packed512/enum | materialized | deferred | off | 3.640 | 3.493–3.763 | 0.023 | 1670.5 | 8004 | 9 |

## legal-product, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.252 | 0.250–0.280 | 0.248 | 79.4 | 134 | 9 |
| essential512/enum | materialized | deferred | off | 2.676 | 2.630–2.688 | 1.380 | 407.6 | 2087 | 3 |
| essential512/enum | views | deferred | off | 4.408 | 4.302–4.449 | 3.748 | 198.5 | 996 | 3 |
| essential512/enum | views | source | off | 2.446 | 2.314–2.450 | 1.454 | 296.7 | 1434 | 3 |
| essential512/enum | views-inputs | deferred | off | 4.908 | 4.901–4.912 | 3.920 | 294.6 | 1508 | 3 |
| essential512/enum | views-inputs | source | off | 2.909 | 2.877–2.985 | 1.682 | 412.0 | 1845 | 3 |
| essential512/slab | materialized | deferred | off | 2.566 | 2.510–2.729 | 1.322 | 250.1 | 2087 | 9 |
| essential512/slab | views | deferred | off | 4.334 | 4.287–4.410 | 3.811 | 125.3 | 996 | 3 |
| essential512/slab | views | source | off | 2.361 | 2.244–2.443 | 1.514 | 204.4 | 1434 | 9 |
| essential512/slab | views-inputs | deferred | off | 4.969 | 4.949–4.978 | 3.964 | 212.0 | 1508 | 3 |
| essential512/slab | views-inputs | source | off | 2.891 | 2.829–3.152 | 1.716 | 265.4 | 1845 | 9 |
| essential64/enum | materialized | deferred | off | 5.386 | 5.256–5.521 | 3.045 | 680.6 | 4861 | 3 |
| essential64/enum | views | deferred | off | 17.730 | 17.713–17.751 | 16.682 | 340.7 | 2088 | 3 |
| essential64/enum | views | source | off | 4.792 | 4.788–4.814 | 3.328 | 495.6 | 3023 | 3 |
| essential64/enum | views-inputs | deferred | off | 18.666 | 18.533–18.720 | 16.897 | 511.5 | 3473 | 3 |
| essential64/enum | views-inputs | source | off | 6.300 | 6.200–6.435 | 3.954 | 709.1 | 4229 | 3 |
| essential64/slab | materialized | deferred | off | 5.336 | 5.327–5.363 | 3.110 | 437.1 | 4861 | 3 |
| essential64/slab | views | deferred | off | 17.890 | 17.853–17.990 | 16.900 | 218.8 | 2088 | 3 |
| essential64/slab | views | source | off | 4.733 | 4.688–4.777 | 3.315 | 345.7 | 3023 | 3 |
| essential64/slab | views-inputs | deferred | off | 19.558 | 19.475–19.848 | 17.478 | 360.9 | 3473 | 3 |
| essential64/slab | views-inputs | source | off | 6.137 | 6.082–6.137 | 3.910 | 467.5 | 4229 | 3 |
| packed512/enum | materialized | deferred | off | 4.237 | 4.172–4.370 | 4.011 | 509.6 | 2936 | 9 |

## legal-product, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.058 | 0.057–0.064 | 0.007 | 84.8 | 134 | 9 |
| essential512/enum | materialized | deferred | off | 1.572 | 1.481–1.622 | 0.007 | 413.0 | 2087 | 3 |
| essential512/enum | views | deferred | off | 1.258 | 1.247–1.288 | 0.007 | 203.9 | 996 | 3 |
| essential512/enum | views | source | off | 1.093 | 1.081–1.096 | 0.007 | 302.0 | 1434 | 3 |
| essential512/enum | views-inputs | deferred | off | 1.697 | 1.652–1.706 | 0.008 | 299.9 | 1508 | 3 |
| essential512/enum | views-inputs | source | off | 1.522 | 1.499–1.560 | 0.007 | 417.3 | 1845 | 3 |
| essential512/slab | materialized | deferred | off | 1.583 | 1.474–1.616 | 0.007 | 255.5 | 2087 | 9 |
| essential512/slab | views | deferred | off | 1.241 | 1.233–1.329 | 0.007 | 130.7 | 996 | 3 |
| essential512/slab | views | source | off | 1.071 | 1.063–1.145 | 0.007 | 209.8 | 1434 | 9 |
| essential512/slab | views-inputs | deferred | off | 1.718 | 1.714–1.798 | 0.007 | 217.4 | 1508 | 3 |
| essential512/slab | views-inputs | source | off | 1.473 | 1.450–1.682 | 0.007 | 270.7 | 1845 | 9 |
| essential64/enum | materialized | deferred | off | 2.774 | 2.768–2.786 | 0.007 | 686.0 | 4861 | 3 |
| essential64/enum | views | deferred | off | 4.079 | 4.064–4.168 | 0.007 | 346.1 | 2088 | 3 |
| essential64/enum | views | source | off | 2.198 | 2.160–2.254 | 0.007 | 501.0 | 3023 | 3 |
| essential64/enum | views-inputs | deferred | off | 4.856 | 4.847–4.888 | 0.007 | 516.9 | 3473 | 3 |
| essential64/enum | views-inputs | source | off | 2.878 | 2.789–2.895 | 0.007 | 714.4 | 4229 | 3 |
| essential64/slab | materialized | deferred | off | 2.730 | 2.710–2.803 | 0.007 | 442.4 | 4861 | 3 |
| essential64/slab | views | deferred | off | 4.090 | 4.058–4.182 | 0.007 | 224.1 | 2088 | 3 |
| essential64/slab | views | source | off | 2.028 | 1.968–2.080 | 0.007 | 351.0 | 3023 | 3 |
| essential64/slab | views-inputs | deferred | off | 5.192 | 5.030–5.209 | 0.007 | 366.3 | 3473 | 3 |
| essential64/slab | views-inputs | source | off | 2.792 | 2.789–2.825 | 0.009 | 472.9 | 4229 | 3 |
| packed512/enum | materialized | deferred | off | 0.939 | 0.934–1.044 | 0.012 | 515.0 | 2936 | 9 |

## legal-product, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.252 | 0.250–0.272 | 0.247 | 79.4 | 134 | 9 |
| essential512/enum | materialized | deferred | off | 3.341 | 3.340–3.416 | 1.569 | 492.3 | 2829 | 3 |
| essential512/enum | views | deferred | off | 5.576 | 5.553–5.635 | 5.058 | 132.5 | 747 | 3 |
| essential512/enum | views | source | off | 2.175 | 2.097–2.202 | 1.399 | 247.4 | 1235 | 3 |
| essential512/enum | views-inputs | deferred | off | 5.251 | 5.233–5.516 | 3.937 | 459.8 | 2257 | 3 |
| essential512/enum | views-inputs | source | off | 3.392 | 3.388–3.589 | 1.833 | 485.0 | 2467 | 3 |
| essential512/slab | materialized | deferred | off | 3.376 | 3.267–3.471 | 1.642 | 300.8 | 2829 | 9 |
| essential512/slab | views | deferred | off | 5.605 | 5.542–5.636 | 5.162 | 87.9 | 747 | 3 |
| essential512/slab | views | source | off | 2.129 | 2.069–2.184 | 1.415 | 158.2 | 1235 | 9 |
| essential512/slab | views-inputs | deferred | off | 5.255 | 5.243–5.284 | 3.946 | 300.8 | 2257 | 3 |
| essential512/slab | views-inputs | source | off | 3.515 | 3.366–3.686 | 1.900 | 316.0 | 2467 | 9 |
| essential64/enum | materialized | deferred | off | 3.167 | 3.134–3.277 | 1.530 | 716.2 | 4397 | 3 |
| essential64/enum | views | deferred | off | 16.078 | 16.060–16.094 | 15.587 | 179.4 | 1451 | 3 |
| essential64/enum | views | source | off | 3.744 | 3.564–3.822 | 2.552 | 378.9 | 2446 | 3 |
| essential64/enum | views-inputs | deferred | off | 14.551 | 14.406–14.555 | 13.346 | 549.0 | 3455 | 3 |
| essential64/enum | views-inputs | source | off | 3.531 | 3.433–3.607 | 1.834 | 713.9 | 3830 | 3 |
| essential64/slab | materialized | deferred | off | 3.360 | 3.181–3.470 | 1.562 | 466.8 | 4397 | 3 |
| essential64/slab | views | deferred | off | 16.157 | 16.096–16.284 | 15.610 | 117.5 | 1451 | 3 |
| essential64/slab | views | source | off | 3.713 | 3.580–3.801 | 2.583 | 248.9 | 2446 | 3 |
| essential64/slab | views-inputs | deferred | off | 14.274 | 14.179–14.395 | 12.868 | 382.2 | 3455 | 3 |
| essential64/slab | views-inputs | source | off | 3.584 | 3.443–3.657 | 1.857 | 458.3 | 3830 | 3 |
| packed512/enum | materialized | deferred | off | 6.320 | 6.273–6.539 | 6.014 | 622.5 | 3985 | 9 |

## legal-product, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 0.058 | 0.057–0.067 | 0.007 | 84.8 | 134 | 9 |
| essential512/enum | materialized | deferred | off | 1.959 | 1.943–2.018 | 0.007 | 497.6 | 2829 | 3 |
| essential512/enum | views | deferred | off | 1.345 | 1.326–1.348 | 0.007 | 137.9 | 747 | 3 |
| essential512/enum | views | source | off | 0.964 | 0.950–1.001 | 0.009 | 252.7 | 1235 | 3 |
| essential512/enum | views-inputs | deferred | off | 2.091 | 2.070–2.316 | 0.007 | 465.2 | 2257 | 3 |
| essential512/enum | views-inputs | source | off | 1.856 | 1.845–2.000 | 0.007 | 490.4 | 2467 | 3 |
| essential512/slab | materialized | deferred | off | 2.008 | 1.928–2.108 | 0.007 | 306.2 | 2829 | 9 |
| essential512/slab | views | deferred | off | 1.408 | 1.401–1.437 | 0.007 | 93.3 | 747 | 3 |
| essential512/slab | views | source | off | 0.960 | 0.950–1.130 | 0.007 | 163.6 | 1235 | 9 |
| essential512/slab | views-inputs | deferred | off | 2.043 | 2.037–2.191 | 0.007 | 306.2 | 2257 | 3 |
| essential512/slab | views-inputs | source | off | 1.832 | 1.795–1.894 | 0.007 | 321.4 | 2467 | 9 |
| essential64/enum | materialized | deferred | off | 2.003 | 1.935–2.073 | 0.007 | 721.6 | 4397 | 3 |
| essential64/enum | views | deferred | off | 3.479 | 3.424–3.669 | 0.007 | 184.8 | 1451 | 3 |
| essential64/enum | views | source | off | 1.510 | 1.484–1.559 | 0.007 | 384.2 | 2446 | 3 |
| essential64/enum | views-inputs | deferred | off | 3.957 | 3.941–3.995 | 0.007 | 554.4 | 3455 | 3 |
| essential64/enum | views-inputs | source | off | 1.892 | 1.878–2.009 | 0.007 | 719.3 | 3830 | 3 |
| essential64/slab | materialized | deferred | off | 1.927 | 1.900–1.996 | 0.007 | 472.2 | 4397 | 3 |
| essential64/slab | views | deferred | off | 3.474 | 3.422–3.523 | 0.007 | 122.9 | 1451 | 3 |
| essential64/slab | views | source | off | 1.506 | 1.499–1.614 | 0.008 | 254.2 | 2446 | 3 |
| essential64/slab | views-inputs | deferred | off | 3.820 | 3.706–3.834 | 0.007 | 387.5 | 3455 | 3 |
| essential64/slab | views-inputs | source | off | 2.010 | 1.945–2.016 | 0.007 | 463.7 | 3830 | 3 |
| packed512/enum | materialized | deferred | off | 1.423 | 1.374–1.448 | 0.009 | 627.9 | 3985 | 9 |

## legal-product, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 13.803 | 13.624–14.151 | 13.847 | 4564.8 | 137 | 9 |
| essential512/enum | materialized | deferred | off | 24.290 | 23.877–24.913 | 12.266 | 3798.3 | 20047 | 3 |
| essential512/enum | views | deferred | off | 172.008 | 169.617–172.133 | 164.743 | 1516.0 | 8273 | 3 |
| essential512/enum | views | source | off | 17.002 | 16.969–17.092 | 11.071 | 1992.3 | 10459 | 3 |
| essential512/enum | views-inputs | deferred | off | 175.926 | 173.496–177.101 | 164.810 | 2832.9 | 14586 | 3 |
| essential512/enum | views-inputs | source | off | 23.603 | 23.335–24.525 | 14.232 | 2947.9 | 16302 | 3 |
| essential512/slab | materialized | deferred | off | 24.138 | 23.550–26.214 | 12.563 | 2645.1 | 20047 | 9 |
| essential512/slab | views | deferred | off | 165.621 | 164.632–166.097 | 161.221 | 936.0 | 8273 | 3 |
| essential512/slab | views | source | off | 17.543 | 17.229–18.385 | 11.343 | 1383.8 | 10459 | 9 |
| essential512/slab | views-inputs | deferred | off | 177.271 | 176.737–177.355 | 168.172 | 1810.3 | 14586 | 3 |
| essential512/slab | views-inputs | source | off | 23.378 | 22.971–24.527 | 13.904 | 1871.3 | 16302 | 9 |
| essential64/enum | materialized | deferred | off | 34.318 | 31.783–34.544 | 18.822 | 5541.8 | 31357 | 3 |
| essential64/enum | views | deferred | off | 371.286 | 371.199–372.364 | 367.983 | 1680.8 | 12228 | 3 |
| essential64/enum | views | source | off | 25.516 | 25.449–26.151 | 18.097 | 2961.5 | 15410 | 3 |
| essential64/enum | views-inputs | deferred | off | 376.921 | 375.143–377.184 | 366.642 | 3346.9 | 23398 | 3 |
| essential64/enum | views-inputs | source | off | 33.101 | 32.263–33.977 | 21.221 | 4476.5 | 25864 | 3 |
| essential64/slab | materialized | deferred | off | 31.211 | 31.171–31.443 | 19.184 | 3560.4 | 31357 | 3 |
| essential64/slab | views | deferred | off | 374.225 | 372.258–374.815 | 365.910 | 1164.8 | 12228 | 3 |
| essential64/slab | views | source | off | 24.377 | 24.356–24.452 | 17.786 | 1963.4 | 15410 | 3 |
| essential64/slab | views-inputs | deferred | off | 393.944 | 391.722–397.302 | 379.312 | 2328.9 | 23398 | 3 |
| essential64/slab | views-inputs | source | off | 32.769 | 32.552–35.314 | 21.778 | 3149.9 | 25864 | 3 |
| packed512/enum | materialized | deferred | off | 15.473 | 15.208–16.434 | 13.618 | 2899.3 | 23658 | 9 |

## legal-product, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.640 | 2.613–2.663 | 0.016 | 4570.2 | 137 | 9 |
| essential512/enum | materialized | deferred | off | 14.005 | 13.885–14.152 | 0.017 | 3803.7 | 20047 | 3 |
| essential512/enum | views | deferred | off | 35.397 | 35.286–35.444 | 0.016 | 1521.4 | 8273 | 3 |
| essential512/enum | views | source | off | 8.216 | 8.132–8.231 | 0.013 | 1997.7 | 10459 | 3 |
| essential512/enum | views-inputs | deferred | off | 40.042 | 39.879–40.788 | 0.015 | 2838.3 | 14586 | 3 |
| essential512/enum | views-inputs | source | off | 12.269 | 11.999–12.564 | 0.014 | 2953.2 | 16302 | 3 |
| essential512/slab | materialized | deferred | off | 13.909 | 13.495–15.143 | 0.017 | 2650.5 | 20047 | 9 |
| essential512/slab | views | deferred | off | 36.369 | 35.790–36.459 | 0.018 | 941.4 | 8273 | 3 |
| essential512/slab | views | source | off | 8.177 | 7.847–8.925 | 0.017 | 1389.2 | 10459 | 9 |
| essential512/slab | views-inputs | deferred | off | 40.219 | 39.647–42.032 | 0.016 | 1815.7 | 14586 | 3 |
| essential512/slab | views-inputs | source | off | 11.927 | 11.855–12.113 | 0.017 | 1876.6 | 16302 | 9 |
| essential64/enum | materialized | deferred | off | 16.394 | 16.329–16.980 | 0.016 | 5547.1 | 31357 | 3 |
| essential64/enum | views | deferred | off | 75.020 | 74.171–78.077 | 0.017 | 1686.2 | 12228 | 3 |
| essential64/enum | views | source | off | 10.645 | 10.603–11.057 | 0.016 | 2966.9 | 15410 | 3 |
| essential64/enum | views-inputs | deferred | off | 79.394 | 78.912–79.804 | 0.016 | 3352.3 | 23398 | 3 |
| essential64/enum | views-inputs | source | off | 15.659 | 15.259–15.733 | 0.016 | 4481.8 | 25864 | 3 |
| essential64/slab | materialized | deferred | off | 15.796 | 15.786–25.960 | 0.017 | 3565.8 | 31357 | 3 |
| essential64/slab | views | deferred | off | 74.361 | 74.281–75.254 | 0.013 | 1170.2 | 12228 | 3 |
| essential64/slab | views | source | off | 10.324 | 10.157–10.368 | 0.014 | 1968.7 | 15410 | 3 |
| essential64/slab | views-inputs | deferred | off | 84.491 | 83.353–84.922 | 0.017 | 2334.3 | 23398 | 3 |
| essential64/slab | views-inputs | source | off | 15.083 | 15.056–15.112 | 0.017 | 3155.3 | 25864 | 3 |
| packed512/enum | materialized | deferred | off | 4.287 | 4.181–4.449 | 0.080 | 2904.7 | 23658 | 9 |

## legal-product, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 13.409 | 13.326–13.538 | 13.521 | 4564.8 | 137 | 9 |
| essential512/enum | materialized | deferred | off | 25.803 | 25.473–25.860 | 8.968 | 6633.5 | 33764 | 3 |
| essential512/enum | views | deferred | off | 358.345 | 355.293–359.383 | 352.989 | 1647.2 | 7644 | 3 |
| essential512/enum | views | source | off | 35.607 | 35.428–35.857 | 28.389 | 2602.7 | 14258 | 3 |
| essential512/enum | views-inputs | deferred | off | 293.497 | 290.819–293.594 | 276.848 | 4000.0 | 26433 | 3 |
| essential512/enum | views-inputs | source | off | 26.487 | 26.168–29.074 | 10.511 | 6499.2 | 29466 | 3 |
| essential512/slab | materialized | deferred | off | 25.811 | 24.736–26.634 | 9.008 | 4306.4 | 33764 | 9 |
| essential512/slab | views | deferred | off | 359.541 | 358.694–361.372 | 355.637 | 1064.4 | 7644 | 3 |
| essential512/slab | views | source | off | 36.586 | 36.113–37.688 | 29.403 | 1666.1 | 14258 | 9 |
| essential512/slab | views-inputs | deferred | off | 291.336 | 284.808–291.890 | 273.687 | 2920.2 | 26433 | 3 |
| essential512/slab | views-inputs | source | off | 25.579 | 25.441–26.570 | 10.616 | 4306.4 | 29466 | 9 |
| essential64/enum | materialized | deferred | off | 18.547 | 18.502–19.402 | 7.895 | 6046.2 | 40396 | 3 |
| essential64/enum | views | deferred | off | 601.914 | 601.504–604.136 | 596.943 | 1772.2 | 13094 | 3 |
| essential64/enum | views | source | off | 30.357 | 30.326–31.180 | 24.336 | 3256.9 | 20861 | 3 |
| essential64/enum | views-inputs | deferred | off | 583.320 | 583.258–584.725 | 573.765 | 4666.5 | 30661 | 3 |
| essential64/enum | views-inputs | source | off | 20.446 | 20.016–21.710 | 11.233 | 6095.5 | 33894 | 3 |
| essential64/slab | materialized | deferred | off | 17.820 | 17.631–18.082 | 7.759 | 3872.2 | 40396 | 3 |
| essential64/slab | views | deferred | off | 615.412 | 612.151–620.204 | 622.362 | 1219.2 | 13094 | 3 |
| essential64/slab | views | source | off | 31.130 | 30.646–32.176 | 24.220 | 2163.6 | 20861 | 3 |
| essential64/slab | views-inputs | deferred | off | 576.294 | 575.339–580.833 | 569.574 | 2888.5 | 30661 | 3 |
| essential64/slab | views-inputs | source | off | 21.606 | 20.852–21.905 | 11.766 | 3933.2 | 33894 | 3 |
| packed512/enum | materialized | deferred | off | 25.409 | 24.957–25.609 | 22.742 | 5079.7 | 37394 | 9 |

## legal-product, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | deferred | off | 2.584 | 2.543–2.696 | 0.018 | 4570.2 | 137 | 9 |
| essential512/enum | materialized | deferred | off | 18.305 | 17.681–18.920 | 0.016 | 6638.9 | 33764 | 3 |
| essential512/enum | views | deferred | off | 70.189 | 69.612–72.756 | 0.017 | 1652.6 | 7644 | 3 |
| essential512/enum | views | source | off | 12.655 | 12.413–12.901 | 0.016 | 2608.1 | 14258 | 3 |
| essential512/enum | views-inputs | deferred | off | 67.692 | 66.555–67.778 | 0.018 | 4005.4 | 26433 | 3 |
| essential512/enum | views-inputs | source | off | 17.281 | 17.242–17.394 | 0.017 | 6504.6 | 29466 | 3 |
| essential512/slab | materialized | deferred | off | 18.143 | 17.962–18.859 | 0.017 | 4311.7 | 33764 | 9 |
| essential512/slab | views | deferred | off | 70.052 | 69.967–71.094 | 0.017 | 1069.8 | 7644 | 3 |
| essential512/slab | views | source | off | 13.488 | 13.081–25.787 | 0.018 | 1671.5 | 14258 | 9 |
| essential512/slab | views-inputs | deferred | off | 66.117 | 63.665–66.506 | 0.016 | 2925.6 | 26433 | 3 |
| essential512/slab | views-inputs | source | off | 17.103 | 16.999–19.220 | 0.018 | 4311.7 | 29466 | 9 |
| essential64/enum | materialized | deferred | off | 11.964 | 11.706–12.291 | 0.017 | 6051.6 | 40396 | 3 |
| essential64/enum | views | deferred | off | 115.875 | 115.813–118.151 | 0.014 | 1777.6 | 13094 | 3 |
| essential64/enum | views | source | off | 11.679 | 10.986–12.067 | 0.016 | 3262.2 | 20861 | 3 |
| essential64/enum | views-inputs | deferred | off | 116.549 | 115.389–119.193 | 0.020 | 4671.8 | 30661 | 3 |
| essential64/enum | views-inputs | source | off | 11.402 | 11.277–11.730 | 0.017 | 6100.9 | 33894 | 3 |
| essential64/slab | materialized | deferred | off | 11.894 | 11.677–12.030 | 0.016 | 3877.6 | 40396 | 3 |
| essential64/slab | views | deferred | off | 121.451 | 120.597–123.379 | 0.016 | 1224.6 | 13094 | 3 |
| essential64/slab | views | source | off | 11.024 | 10.881–12.094 | 0.017 | 2169.0 | 20861 | 3 |
| essential64/slab | views-inputs | deferred | off | 116.680 | 115.606–117.275 | 0.015 | 2893.8 | 30661 | 3 |
| essential64/slab | views-inputs | source | off | 11.938 | 11.635–11.949 | 0.020 | 3938.6 | 33894 | 3 |
| packed512/enum | materialized | deferred | off | 6.684 | 6.382–6.817 | 0.030 | 5085.1 | 37394 | 9 |

704 cases; 1024 matched comparisons; 270 retained processes.

- [Raw evidence](results/scoped-smoke.json).
- [Raw evidence](results/scoped-initial-sweep.json).
- [Raw evidence](results/scoped-focused-repeat.json).
- [Raw evidence](results/scoped-full-normalization-repeat.json).
