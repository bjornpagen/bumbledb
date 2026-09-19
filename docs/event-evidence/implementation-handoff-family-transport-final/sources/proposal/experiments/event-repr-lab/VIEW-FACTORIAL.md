# Mapped product factorial experiment

Same executable and answers. Each process uses one local readout kernel
and one product strategy. `views` fuses output renaming; `views-inputs`
renames the canonical output after projection in the working coordinate order.
Local word/assignment pairs must retain exactly identical nodes, cache
and arena byte estimates. Resident bytes omit transient workspace and allocator overhead.
Times are milliseconds; KB means 1,000 bytes. Query includes actual Free Join,
canonical construction, and exact count. Fixture construction, cloning, setup
and full bitset verification are outside. These two query kinds return different outputs.
Executable: `304c3e347eebc94c0cbdbfa49283491f18f50a59924d4e45389ceabf7a13a934`.

## product_free_join, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 0.194 | 0.186–0.300 | 0.181 | 76.6 | 129 | 9 |
| essential512/enum | materialized | assignments | 0.759 | 0.737–0.819 | 0.614 | 64.9 | 329 | 9 |
| essential512/enum | views | assignments | 1.944 | 1.905–2.125 | 1.892 | 32.5 | 216 | 9 |
| essential512/enum | views | words | 1.965 | 1.944–1.996 | 1.895 | 32.5 | 216 | 9 |
| essential512/enum | views-inputs | assignments | 2.105 | 2.032–2.186 | 1.977 | 42.0 | 230 | 9 |
| essential512/enum | views-inputs | words | 2.056 | 1.956–2.109 | 1.893 | 42.0 | 230 | 9 |
| essential512/slab | materialized | assignments | 0.777 | 0.747–0.831 | 0.632 | 41.8 | 329 | 9 |
| essential512/slab | views | assignments | 1.942 | 1.915–2.040 | 1.910 | 20.1 | 216 | 9 |
| essential512/slab | views | words | 2.040 | 2.001–2.116 | 1.954 | 20.1 | 216 | 9 |
| essential512/slab | views-inputs | assignments | 2.171 | 2.074–2.283 | 2.039 | 23.9 | 230 | 9 |
| essential512/slab | views-inputs | words | 2.186 | 2.146–2.327 | 2.033 | 23.9 | 230 | 9 |
| essential64/enum | materialized | assignments | 1.558 | 1.513–1.613 | 1.157 | 125.2 | 829 | 5 |
| essential64/enum | views | assignments | 3.872 | 3.862–4.229 | 3.765 | 79.6 | 486 | 5 |
| essential64/enum | views | words | 3.778 | 3.750–4.167 | 3.625 | 79.6 | 486 | 5 |
| essential64/enum | views-inputs | assignments | 3.934 | 3.861–4.013 | 3.716 | 80.3 | 616 | 5 |
| essential64/enum | views-inputs | words | 3.886 | 3.806–3.999 | 3.703 | 80.3 | 616 | 5 |
| essential64/slab | materialized | assignments | 1.702 | 1.610–1.778 | 1.329 | 84.6 | 829 | 5 |
| essential64/slab | views | assignments | 3.872 | 3.770–4.001 | 3.684 | 48.2 | 486 | 5 |
| essential64/slab | views | words | 3.892 | 3.774–3.963 | 3.643 | 48.2 | 486 | 5 |
| essential64/slab | views-inputs | assignments | 3.971 | 3.869–3.998 | 3.672 | 48.2 | 616 | 5 |
| essential64/slab | views-inputs | words | 3.975 | 3.940–4.049 | 3.766 | 48.2 | 616 | 5 |
| packed512/enum | materialized | assignments | 1.498 | 1.450–1.536 | 1.452 | 63.6 | 548 | 9 |

## product_free_join, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 0.039 | 0.039–0.052 | 0.009 | 84.0 | 129 | 9 |
| essential512/enum | materialized | assignments | 0.264 | 0.245–0.289 | 0.009 | 72.3 | 329 | 9 |
| essential512/enum | views | assignments | 0.439 | 0.425–0.513 | 0.007 | 38.6 | 216 | 9 |
| essential512/enum | views | words | 0.412 | 0.405–0.472 | 0.007 | 38.6 | 216 | 9 |
| essential512/enum | views-inputs | assignments | 0.445 | 0.429–0.501 | 0.007 | 49.0 | 230 | 9 |
| essential512/enum | views-inputs | words | 0.428 | 0.424–0.469 | 0.007 | 49.0 | 230 | 9 |
| essential512/slab | materialized | assignments | 0.259 | 0.252–0.276 | 0.009 | 49.2 | 329 | 9 |
| essential512/slab | views | assignments | 0.426 | 0.410–0.459 | 0.007 | 26.2 | 216 | 9 |
| essential512/slab | views | words | 0.440 | 0.427–0.496 | 0.007 | 26.2 | 216 | 9 |
| essential512/slab | views-inputs | assignments | 0.453 | 0.440–0.494 | 0.007 | 30.9 | 230 | 9 |
| essential512/slab | views-inputs | words | 0.473 | 0.452–0.488 | 0.008 | 30.9 | 230 | 9 |
| essential64/enum | materialized | assignments | 0.550 | 0.535–0.575 | 0.009 | 132.6 | 829 | 5 |
| essential64/enum | views | assignments | 0.849 | 0.826–0.928 | 0.008 | 85.6 | 486 | 5 |
| essential64/enum | views | words | 0.809 | 0.794–0.882 | 0.007 | 85.6 | 486 | 5 |
| essential64/enum | views-inputs | assignments | 0.888 | 0.854–0.934 | 0.007 | 87.3 | 616 | 5 |
| essential64/enum | views-inputs | words | 0.885 | 0.858–0.984 | 0.008 | 87.3 | 616 | 5 |
| essential64/slab | materialized | assignments | 0.580 | 0.576–0.619 | 0.009 | 92.0 | 829 | 5 |
| essential64/slab | views | assignments | 0.850 | 0.833–0.871 | 0.007 | 54.2 | 486 | 5 |
| essential64/slab | views | words | 0.836 | 0.809–0.850 | 0.007 | 54.2 | 486 | 5 |
| essential64/slab | views-inputs | assignments | 0.857 | 0.849–0.915 | 0.008 | 55.1 | 616 | 5 |
| essential64/slab | views-inputs | words | 0.910 | 0.895–1.001 | 0.008 | 55.1 | 616 | 5 |
| packed512/enum | materialized | assignments | 0.222 | 0.209–0.240 | 0.010 | 70.9 | 548 | 9 |

## product_free_join, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 0.193 | 0.180–0.204 | 0.178 | 76.6 | 129 | 9 |
| essential512/enum | materialized | assignments | 1.156 | 1.118–1.235 | 0.989 | 88.6 | 456 | 9 |
| essential512/enum | views | assignments | 1.642 | 1.619–1.788 | 1.480 | 48.8 | 291 | 9 |
| essential512/enum | views | words | 1.605 | 1.561–1.742 | 1.437 | 48.8 | 291 | 9 |
| essential512/enum | views-inputs | assignments | 1.919 | 1.874–2.041 | 1.826 | 41.6 | 292 | 9 |
| essential512/enum | views-inputs | words | 1.884 | 1.871–2.009 | 1.858 | 41.6 | 292 | 9 |
| essential512/slab | materialized | assignments | 1.170 | 1.137–1.336 | 1.011 | 49.4 | 456 | 9 |
| essential512/slab | views | assignments | 1.589 | 1.550–1.622 | 1.422 | 31.0 | 291 | 9 |
| essential512/slab | views | words | 1.623 | 1.582–1.768 | 1.462 | 31.0 | 291 | 9 |
| essential512/slab | views-inputs | assignments | 1.959 | 1.886–2.067 | 1.864 | 23.0 | 292 | 9 |
| essential512/slab | views-inputs | words | 1.930 | 1.878–1.988 | 1.849 | 23.0 | 292 | 9 |
| essential64/enum | materialized | assignments | 1.373 | 1.339–1.426 | 1.028 | 102.0 | 851 | 5 |
| essential64/enum | views | assignments | 8.274 | 8.207–8.318 | 7.758 | 177.0 | 1018 | 5 |
| essential64/enum | views | words | 8.095 | 8.007–8.336 | 7.619 | 177.0 | 1018 | 5 |
| essential64/enum | views-inputs | assignments | 5.393 | 5.328–5.618 | 5.245 | 98.5 | 716 | 5 |
| essential64/enum | views-inputs | words | 5.576 | 5.480–5.634 | 5.320 | 98.5 | 716 | 5 |
| essential64/slab | materialized | assignments | 1.372 | 1.339–1.414 | 1.077 | 65.7 | 851 | 5 |
| essential64/slab | views | assignments | 8.362 | 8.341–8.488 | 7.925 | 107.2 | 1018 | 5 |
| essential64/slab | views | words | 13.130 | 8.769–14.151 | 8.970 | 107.2 | 1018 | 5 |
| essential64/slab | views-inputs | assignments | 5.546 | 5.442–5.567 | 5.283 | 63.8 | 716 | 5 |
| essential64/slab | views-inputs | words | 5.540 | 5.464–5.648 | 5.280 | 63.8 | 716 | 5 |
| packed512/enum | materialized | assignments | 4.533 | 4.416–4.608 | 4.443 | 190.8 | 990 | 9 |

## product_free_join, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 0.040 | 0.038–0.055 | 0.009 | 84.0 | 129 | 9 |
| essential512/enum | materialized | assignments | 0.334 | 0.331–0.366 | 0.009 | 96.0 | 456 | 9 |
| essential512/enum | views | assignments | 0.425 | 0.422–0.493 | 0.008 | 54.8 | 291 | 9 |
| essential512/enum | views | words | 0.413 | 0.403–0.465 | 0.007 | 54.8 | 291 | 9 |
| essential512/enum | views-inputs | assignments | 0.394 | 0.390–0.436 | 0.007 | 48.6 | 292 | 9 |
| essential512/enum | views-inputs | words | 0.419 | 0.392–0.449 | 0.007 | 48.6 | 292 | 9 |
| essential512/slab | materialized | assignments | 0.360 | 0.336–0.403 | 0.008 | 56.8 | 456 | 9 |
| essential512/slab | views | assignments | 0.419 | 0.407–0.443 | 0.007 | 37.0 | 291 | 9 |
| essential512/slab | views | words | 0.412 | 0.403–0.428 | 0.007 | 37.0 | 291 | 9 |
| essential512/slab | views-inputs | assignments | 0.419 | 0.394–0.450 | 0.007 | 30.0 | 292 | 9 |
| essential512/slab | views-inputs | words | 0.413 | 0.391–0.469 | 0.007 | 30.0 | 292 | 9 |
| essential64/enum | materialized | assignments | 0.470 | 0.464–0.493 | 0.009 | 109.4 | 851 | 5 |
| essential64/enum | views | assignments | 1.954 | 1.911–2.045 | 0.007 | 183.1 | 1018 | 5 |
| essential64/enum | views | words | 1.914 | 1.851–1.934 | 0.007 | 183.1 | 1018 | 5 |
| essential64/enum | views-inputs | assignments | 1.230 | 1.198–1.269 | 0.007 | 105.5 | 716 | 5 |
| essential64/enum | views-inputs | words | 1.177 | 1.162–1.201 | 0.007 | 105.5 | 716 | 5 |
| essential64/slab | materialized | assignments | 0.472 | 0.451–0.497 | 0.009 | 73.1 | 851 | 5 |
| essential64/slab | views | assignments | 1.909 | 1.878–1.967 | 0.007 | 113.3 | 1018 | 5 |
| essential64/slab | views | words | 1.981 | 1.936–2.689 | 0.007 | 113.3 | 1018 | 5 |
| essential64/slab | views-inputs | assignments | 1.181 | 1.178–1.275 | 0.007 | 70.8 | 716 | 5 |
| essential64/slab | views-inputs | words | 1.246 | 1.218–1.269 | 0.007 | 70.8 | 716 | 5 |
| packed512/enum | materialized | assignments | 0.773 | 0.744–0.830 | 0.009 | 198.2 | 990 | 9 |

## product_free_join, 12 coordinates, pair-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 0.179 | 0.177–0.197 | 0.179 | 76.6 | 129 | 5 |
| essential512/enum | materialized | assignments | 0.986 | 0.979–1.093 | 2.065 | 68.8 | 427 | 5 |
| essential512/enum | views | assignments | 1.910 | 1.834–2.002 | 1.774 | 46.4 | 270 | 5 |
| essential512/enum | views | words | 1.908 | 1.872–2.000 | 1.770 | 46.4 | 270 | 5 |
| essential512/enum | views-inputs | assignments | 1.873 | 1.815–2.027 | 1.739 | 44.4 | 286 | 5 |
| essential512/enum | views-inputs | words | 1.914 | 1.873–2.017 | 1.782 | 44.4 | 286 | 5 |
| essential512/slab | materialized | assignments | 1.110 | 1.018–1.152 | 0.819 | 41.8 | 427 | 5 |
| essential512/slab | views | assignments | 1.981 | 1.932–2.107 | 1.845 | 31.0 | 270 | 5 |
| essential512/slab | views | words | 1.939 | 1.919–2.038 | 1.831 | 31.0 | 270 | 5 |
| essential512/slab | views-inputs | assignments | 1.884 | 1.878–1.976 | 1.797 | 25.8 | 286 | 5 |
| essential512/slab | views-inputs | words | 1.885 | 1.868–2.042 | 1.824 | 25.8 | 286 | 5 |
| essential64/enum | materialized | assignments | 1.830 | 1.778–1.844 | 1.260 | 183.0 | 1034 | 5 |
| essential64/enum | views | assignments | 8.454 | 8.378–8.766 | 8.156 | 99.6 | 757 | 5 |
| essential64/enum | views | words | 8.540 | 8.368–8.732 | 8.143 | 99.6 | 757 | 5 |
| essential64/enum | views-inputs | assignments | 8.357 | 8.305–8.447 | 7.974 | 100.1 | 816 | 5 |
| essential64/enum | views-inputs | words | 8.650 | 8.445–8.842 | 8.234 | 100.1 | 816 | 5 |
| essential64/slab | materialized | assignments | 1.839 | 1.795–3.544 | 1.360 | 115.9 | 1034 | 5 |
| essential64/slab | views | assignments | 8.429 | 8.326–8.528 | 8.197 | 63.8 | 757 | 5 |
| essential64/slab | views | words | 8.543 | 8.417–8.857 | 8.318 | 63.8 | 757 | 5 |
| essential64/slab | views-inputs | assignments | 8.504 | 8.450–8.676 | 8.182 | 63.8 | 816 | 5 |
| essential64/slab | views-inputs | words | 8.658 | 8.343–8.799 | 8.381 | 63.8 | 816 | 5 |
| packed512/enum | materialized | assignments | 3.614 | 3.581–3.721 | 3.528 | 190.0 | 1086 | 5 |

## product_free_join, 12 coordinates, pair-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 0.041 | 0.039–0.047 | 0.009 | 84.0 | 129 | 5 |
| essential512/enum | materialized | assignments | 0.344 | 0.332–1.627 | 0.009 | 76.2 | 427 | 5 |
| essential512/enum | views | assignments | 0.443 | 0.430–0.467 | 0.007 | 52.5 | 270 | 5 |
| essential512/enum | views | words | 0.447 | 0.429–0.473 | 0.007 | 52.5 | 270 | 5 |
| essential512/enum | views-inputs | assignments | 0.416 | 0.411–0.445 | 0.007 | 51.3 | 286 | 5 |
| essential512/enum | views-inputs | words | 0.429 | 0.426–0.447 | 0.008 | 51.3 | 286 | 5 |
| essential512/slab | materialized | assignments | 0.384 | 0.337–0.402 | 0.009 | 49.2 | 427 | 5 |
| essential512/slab | views | assignments | 0.450 | 0.439–0.455 | 0.007 | 37.0 | 270 | 5 |
| essential512/slab | views | words | 0.444 | 0.440–0.476 | 0.008 | 37.0 | 270 | 5 |
| essential512/slab | views-inputs | assignments | 0.412 | 0.407–0.427 | 0.008 | 32.8 | 286 | 5 |
| essential512/slab | views-inputs | words | 0.428 | 0.414–0.447 | 0.007 | 32.8 | 286 | 5 |
| essential64/enum | materialized | assignments | 0.652 | 0.639–0.701 | 0.009 | 190.4 | 1034 | 5 |
| essential64/enum | views | assignments | 1.765 | 1.745–1.793 | 0.007 | 105.7 | 757 | 5 |
| essential64/enum | views | words | 1.779 | 1.749–1.825 | 0.007 | 105.7 | 757 | 5 |
| essential64/enum | views-inputs | assignments | 1.772 | 1.722–1.918 | 0.007 | 107.1 | 816 | 5 |
| essential64/enum | views-inputs | words | 1.818 | 1.816–1.889 | 0.008 | 107.1 | 816 | 5 |
| essential64/slab | materialized | assignments | 0.691 | 0.686–0.706 | 0.009 | 123.3 | 1034 | 5 |
| essential64/slab | views | assignments | 1.805 | 1.761–1.816 | 0.007 | 69.9 | 757 | 5 |
| essential64/slab | views | words | 1.824 | 1.757–1.867 | 0.007 | 69.9 | 757 | 5 |
| essential64/slab | views-inputs | assignments | 1.774 | 1.757–1.806 | 0.007 | 70.8 | 816 | 5 |
| essential64/slab | views-inputs | words | 1.831 | 1.809–1.855 | 0.008 | 70.8 | 816 | 5 |
| packed512/enum | materialized | assignments | 0.600 | 0.581–0.631 | 0.009 | 197.4 | 1086 | 5 |

## product_free_join, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 9.285 | 9.123–9.739 | 16.060 | 4335.1 | 130 | 9 |
| essential512/enum | materialized | assignments | 7.280 | 7.152–7.585 | 5.791 | 583.9 | 3112 | 9 |
| essential512/enum | views | assignments | 62.539 | 62.221–63.776 | 61.459 | 311.2 | 1792 | 9 |
| essential512/enum | views | words | 62.211 | 61.092–63.042 | 61.348 | 311.2 | 1792 | 9 |
| essential512/enum | views-inputs | assignments | 66.096 | 65.867–68.383 | 64.683 | 400.2 | 2342 | 9 |
| essential512/enum | views-inputs | words | 65.149 | 64.372–66.334 | 63.215 | 400.2 | 2342 | 9 |
| essential512/slab | materialized | assignments | 7.441 | 7.148–7.793 | 6.041 | 356.7 | 3112 | 9 |
| essential512/slab | views | assignments | 62.510 | 62.398–63.567 | 62.215 | 231.5 | 1792 | 9 |
| essential512/slab | views | words | 64.175 | 63.532–83.804 | 65.902 | 231.5 | 1792 | 9 |
| essential512/slab | views-inputs | assignments | 68.036 | 67.576–68.642 | 67.352 | 260.1 | 2342 | 9 |
| essential512/slab | views-inputs | words | 66.377 | 64.830–69.444 | 64.122 | 260.1 | 2342 | 9 |
| essential64/enum | materialized | assignments | 10.403 | 10.201–10.592 | 8.946 | 826.7 | 5582 | 5 |
| essential64/enum | views | assignments | 36.925 | 36.575–37.476 | 35.138 | 408.4 | 3102 | 5 |
| essential64/enum | views | words | 35.846 | 35.638–36.407 | 34.542 | 408.4 | 3102 | 5 |
| essential64/enum | views-inputs | assignments | 36.446 | 35.937–37.111 | 34.418 | 796.2 | 4643 | 5 |
| essential64/enum | views-inputs | words | 36.497 | 35.968–37.141 | 34.675 | 796.2 | 4643 | 5 |
| essential64/slab | materialized | assignments | 11.995 | 11.342–23.103 | 9.810 | 543.6 | 5582 | 5 |
| essential64/slab | views | assignments | 37.119 | 36.815–37.336 | 36.346 | 264.4 | 3102 | 5 |
| essential64/slab | views | words | 36.221 | 35.518–36.364 | 34.962 | 264.4 | 3102 | 5 |
| essential64/slab | views-inputs | assignments | 36.301 | 36.219–36.617 | 34.141 | 509.9 | 4643 | 5 |
| essential64/slab | views-inputs | words | 37.069 | 36.526–37.210 | 35.325 | 509.9 | 4643 | 5 |
| packed512/enum | materialized | assignments | 3.286 | 3.170–3.374 | 2.844 | 676.8 | 5073 | 9 |

## product_free_join, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 1.397 | 1.387–1.722 | 0.018 | 4342.5 | 130 | 9 |
| essential512/enum | materialized | assignments | 2.528 | 2.422–2.629 | 0.009 | 591.3 | 3112 | 9 |
| essential512/enum | views | assignments | 12.332 | 12.129–12.677 | 0.009 | 317.3 | 1792 | 9 |
| essential512/enum | views | words | 12.247 | 12.106–12.516 | 0.007 | 317.3 | 1792 | 9 |
| essential512/enum | views-inputs | assignments | 12.894 | 12.821–13.328 | 0.007 | 407.2 | 2342 | 9 |
| essential512/enum | views-inputs | words | 13.053 | 12.658–13.370 | 0.007 | 407.2 | 2342 | 9 |
| essential512/slab | materialized | assignments | 2.501 | 2.459–2.694 | 0.009 | 364.1 | 3112 | 9 |
| essential512/slab | views | assignments | 12.409 | 12.157–12.611 | 0.007 | 237.6 | 1792 | 9 |
| essential512/slab | views | words | 13.047 | 12.768–13.173 | 0.007 | 237.6 | 1792 | 9 |
| essential512/slab | views-inputs | assignments | 13.432 | 13.251–13.799 | 0.007 | 267.0 | 2342 | 9 |
| essential512/slab | views-inputs | words | 12.891 | 12.663–13.072 | 0.007 | 267.0 | 2342 | 9 |
| essential64/enum | materialized | assignments | 3.453 | 3.358–3.595 | 0.008 | 834.1 | 5582 | 5 |
| essential64/enum | views | assignments | 7.872 | 7.619–8.002 | 0.007 | 414.4 | 3102 | 5 |
| essential64/enum | views | words | 7.695 | 7.459–7.837 | 0.007 | 414.4 | 3102 | 5 |
| essential64/enum | views-inputs | assignments | 8.140 | 7.941–8.280 | 0.008 | 803.2 | 4643 | 5 |
| essential64/enum | views-inputs | words | 7.997 | 7.921–8.314 | 0.007 | 803.2 | 4643 | 5 |
| essential64/slab | materialized | assignments | 4.281 | 3.990–20.483 | 0.009 | 551.0 | 5582 | 5 |
| essential64/slab | views | assignments | 8.055 | 7.711–8.111 | 0.007 | 270.5 | 3102 | 5 |
| essential64/slab | views | words | 7.759 | 7.672–7.800 | 0.007 | 270.5 | 3102 | 5 |
| essential64/slab | views-inputs | assignments | 7.989 | 7.898–8.060 | 0.007 | 516.9 | 4643 | 5 |
| essential64/slab | views-inputs | words | 8.193 | 8.066–8.539 | 0.008 | 516.9 | 4643 | 5 |
| packed512/enum | materialized | assignments | 0.729 | 0.702–0.770 | 0.024 | 684.2 | 5073 | 9 |

## product_free_join, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 8.708 | 8.629–9.149 | 8.743 | 4335.1 | 130 | 9 |
| essential512/enum | materialized | assignments | 7.279 | 7.060–7.511 | 5.874 | 512.2 | 3227 | 9 |
| essential512/enum | views | assignments | 138.100 | 134.312–148.946 | 133.394 | 1011.5 | 5565 | 9 |
| essential512/enum | views | words | 135.782 | 135.109–138.484 | 134.865 | 1011.5 | 5565 | 9 |
| essential512/enum | views-inputs | assignments | 98.270 | 97.772–98.954 | 97.340 | 461.9 | 2645 | 9 |
| essential512/enum | views-inputs | words | 98.031 | 96.985–99.715 | 97.003 | 461.9 | 2645 | 9 |
| essential512/slab | materialized | assignments | 7.133 | 6.985–7.489 | 5.923 | 360.3 | 3227 | 9 |
| essential512/slab | views | assignments | 135.938 | 133.725–136.908 | 133.413 | 735.4 | 5565 | 9 |
| essential512/slab | views | words | 136.016 | 133.870–136.698 | 132.614 | 735.4 | 5565 | 9 |
| essential512/slab | views-inputs | assignments | 98.703 | 97.983–99.298 | 97.506 | 268.8 | 2645 | 9 |
| essential512/slab | views-inputs | words | 99.871 | 98.680–101.825 | 98.619 | 268.8 | 2645 | 9 |
| essential64/enum | materialized | assignments | 6.485 | 6.315–6.779 | 3.843 | 1087.0 | 7382 | 5 |
| essential64/enum | views | assignments | 114.062 | 112.947–114.793 | 110.362 | 1537.1 | 9887 | 5 |
| essential64/enum | views | words | 95.610 | 94.293–95.667 | 90.920 | 1537.1 | 9887 | 5 |
| essential64/enum | views-inputs | assignments | 16.806 | 16.461–17.071 | 14.656 | 777.2 | 6468 | 5 |
| essential64/enum | views-inputs | words | 16.724 | 16.647–16.882 | 14.756 | 777.2 | 6468 | 5 |
| essential64/slab | materialized | assignments | 6.601 | 6.414–6.651 | 4.017 | 639.0 | 7382 | 5 |
| essential64/slab | views | assignments | 115.108 | 114.583–116.716 | 111.806 | 979.9 | 9887 | 5 |
| essential64/slab | views | words | 109.698 | 97.369–201.225 | 141.341 | 979.9 | 9887 | 5 |
| essential64/slab | views-inputs | assignments | 17.162 | 17.030–17.412 | 14.929 | 495.4 | 6468 | 5 |
| essential64/slab | views-inputs | words | 17.018 | 16.826–17.317 | 15.221 | 495.4 | 6468 | 5 |
| packed512/enum | materialized | assignments | 12.330 | 12.027–12.469 | 11.702 | 994.3 | 6218 | 9 |

## product_free_join, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 1.332 | 1.303–1.397 | 0.017 | 4342.5 | 130 | 9 |
| essential512/enum | materialized | assignments | 2.275 | 2.186–2.442 | 0.009 | 519.6 | 3227 | 9 |
| essential512/enum | views | assignments | 28.402 | 27.834–29.190 | 0.007 | 1017.5 | 5565 | 9 |
| essential512/enum | views | words | 28.323 | 27.990–29.125 | 0.007 | 1017.5 | 5565 | 9 |
| essential512/enum | views-inputs | assignments | 19.259 | 19.066–19.694 | 0.007 | 468.9 | 2645 | 9 |
| essential512/enum | views-inputs | words | 19.502 | 19.112–53.404 | 0.007 | 468.9 | 2645 | 9 |
| essential512/slab | materialized | assignments | 2.212 | 2.166–2.297 | 0.009 | 367.7 | 3227 | 9 |
| essential512/slab | views | assignments | 28.384 | 27.903–45.309 | 0.007 | 741.4 | 5565 | 9 |
| essential512/slab | views | words | 27.869 | 27.492–33.629 | 0.007 | 741.4 | 5565 | 9 |
| essential512/slab | views-inputs | assignments | 19.312 | 19.037–19.484 | 0.008 | 275.7 | 2645 | 9 |
| essential512/slab | views-inputs | words | 19.257 | 19.025–19.727 | 0.007 | 275.7 | 2645 | 9 |
| essential64/enum | materialized | assignments | 3.055 | 2.951–3.060 | 0.009 | 1094.4 | 7382 | 5 |
| essential64/enum | views | assignments | 24.018 | 23.706–24.577 | 0.007 | 1543.2 | 9887 | 5 |
| essential64/enum | views | words | 20.574 | 20.014–20.653 | 0.007 | 1543.2 | 9887 | 5 |
| essential64/enum | views-inputs | assignments | 4.883 | 4.749–4.911 | 0.008 | 784.1 | 6468 | 5 |
| essential64/enum | views-inputs | words | 4.874 | 4.786–4.947 | 0.007 | 784.1 | 6468 | 5 |
| essential64/slab | materialized | assignments | 3.007 | 2.909–3.077 | 0.009 | 646.4 | 7382 | 5 |
| essential64/slab | views | assignments | 24.408 | 24.190–24.624 | 0.007 | 986.0 | 9887 | 5 |
| essential64/slab | views | words | 20.848 | 20.803–21.213 | 0.008 | 986.0 | 9887 | 5 |
| essential64/slab | views-inputs | assignments | 4.842 | 4.754–4.899 | 0.007 | 502.4 | 6468 | 5 |
| essential64/slab | views-inputs | words | 4.911 | 4.706–5.062 | 0.008 | 502.4 | 6468 | 5 |
| packed512/enum | materialized | assignments | 2.113 | 2.074–2.260 | 0.015 | 1001.7 | 6218 | 9 |

## product_free_join, 18 coordinates, pair-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 8.957 | 8.858–9.136 | 8.989 | 4335.1 | 130 | 5 |
| essential512/enum | materialized | assignments | 8.155 | 7.943–9.933 | 6.238 | 639.7 | 3479 | 5 |
| essential512/enum | views | assignments | 63.215 | 62.352–64.305 | 61.510 | 418.4 | 2378 | 5 |
| essential512/enum | views | words | 62.995 | 62.608–65.325 | 106.529 | 418.4 | 2378 | 5 |
| essential512/enum | views-inputs | assignments | 62.908 | 62.056–63.122 | 61.956 | 416.1 | 2643 | 5 |
| essential512/enum | views-inputs | words | 64.807 | 64.694–64.856 | 64.129 | 416.1 | 2643 | 5 |
| essential512/slab | materialized | assignments | 7.977 | 7.855–8.127 | 6.413 | 405.1 | 3479 | 5 |
| essential512/slab | views | assignments | 65.616 | 65.107–65.986 | 64.527 | 270.9 | 2378 | 5 |
| essential512/slab | views | words | 64.636 | 63.839–64.818 | 63.870 | 270.9 | 2378 | 5 |
| essential512/slab | views-inputs | assignments | 63.686 | 63.276–64.517 | 65.662 | 270.9 | 2643 | 5 |
| essential512/slab | views-inputs | words | 63.273 | 63.046–63.400 | 62.428 | 270.9 | 2643 | 5 |
| essential64/enum | materialized | assignments | 10.589 | 10.514–10.880 | 8.313 | 872.8 | 6490 | 5 |
| essential64/enum | views | assignments | 60.769 | 60.408–61.004 | 60.497 | 581.2 | 4275 | 5 |
| essential64/enum | views | words | 60.880 | 60.068–61.407 | 60.016 | 581.2 | 4275 | 5 |
| essential64/enum | views-inputs | assignments | 60.506 | 60.254–60.894 | 59.109 | 842.1 | 5496 | 5 |
| essential64/enum | views-inputs | words | 61.737 | 61.339–63.110 | 58.867 | 842.1 | 5496 | 5 |
| essential64/slab | materialized | assignments | 11.411 | 11.097–25.300 | 9.273 | 574.6 | 6490 | 5 |
| essential64/slab | views | assignments | 60.835 | 60.514–61.922 | 69.841 | 340.9 | 4275 | 5 |
| essential64/slab | views | words | 61.603 | 60.495–62.404 | 108.762 | 340.9 | 4275 | 5 |
| essential64/slab | views-inputs | assignments | 61.562 | 60.938–61.924 | 60.213 | 551.8 | 5496 | 5 |
| essential64/slab | views-inputs | words | 62.626 | 62.133–63.519 | 61.913 | 551.8 | 5496 | 5 |
| packed512/enum | materialized | assignments | 15.463 | 15.199–15.798 | 15.017 | 965.8 | 6500 | 5 |

## product_free_join, 18 coordinates, pair-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 1.356 | 1.315–1.370 | 0.017 | 4342.5 | 130 | 5 |
| essential512/enum | materialized | assignments | 2.779 | 2.707–2.919 | 0.009 | 647.0 | 3479 | 5 |
| essential512/enum | views | assignments | 12.772 | 12.383–12.934 | 0.008 | 424.5 | 2378 | 5 |
| essential512/enum | views | words | 13.059 | 12.835–13.188 | 0.007 | 424.5 | 2378 | 5 |
| essential512/enum | views-inputs | assignments | 12.845 | 12.437–13.012 | 0.008 | 423.1 | 2643 | 5 |
| essential512/enum | views-inputs | words | 12.992 | 12.946–13.130 | 0.008 | 423.1 | 2643 | 5 |
| essential512/slab | materialized | assignments | 2.648 | 2.620–2.775 | 0.009 | 412.5 | 3479 | 5 |
| essential512/slab | views | assignments | 12.927 | 12.486–13.020 | 0.008 | 277.0 | 2378 | 5 |
| essential512/slab | views | words | 13.176 | 12.796–13.255 | 0.007 | 277.0 | 2378 | 5 |
| essential512/slab | views-inputs | assignments | 12.919 | 12.779–13.004 | 0.007 | 277.9 | 2643 | 5 |
| essential512/slab | views-inputs | words | 12.722 | 12.646–12.896 | 0.007 | 277.9 | 2643 | 5 |
| essential64/enum | materialized | assignments | 3.836 | 3.649–3.876 | 0.009 | 880.2 | 6490 | 5 |
| essential64/enum | views | assignments | 12.600 | 12.518–12.823 | 0.007 | 587.2 | 4275 | 5 |
| essential64/enum | views | words | 12.570 | 12.519–13.072 | 0.009 | 587.2 | 4275 | 5 |
| essential64/enum | views-inputs | assignments | 12.840 | 12.695–12.947 | 0.007 | 849.1 | 5496 | 5 |
| essential64/enum | views-inputs | words | 12.836 | 12.650–13.113 | 0.008 | 849.1 | 5496 | 5 |
| essential64/slab | materialized | assignments | 4.170 | 3.941–7.893 | 0.009 | 582.0 | 6490 | 5 |
| essential64/slab | views | assignments | 12.897 | 12.635–13.468 | 0.007 | 347.0 | 4275 | 5 |
| essential64/slab | views | words | 12.790 | 12.652–20.678 | 0.007 | 347.0 | 4275 | 5 |
| essential64/slab | views-inputs | assignments | 12.817 | 12.719–13.124 | 0.008 | 558.8 | 5496 | 5 |
| essential64/slab | views-inputs | words | 13.186 | 13.145–13.655 | 0.008 | 558.8 | 5496 | 5 |
| packed512/enum | materialized | assignments | 2.636 | 2.531–2.699 | 0.022 | 973.2 | 6500 | 5 |

## relations_free_join, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 0.149 | 0.142–0.168 | 0.140 | 82.5 | 140 | 9 |
| essential512/enum | materialized | assignments | 0.446 | 0.418–0.502 | 0.230 | 66.9 | 345 | 9 |
| essential512/enum | views | assignments | 0.697 | 0.668–0.756 | 0.605 | 22.7 | 130 | 9 |
| essential512/enum | views | words | 0.704 | 0.674–0.747 | 0.599 | 22.7 | 130 | 9 |
| essential512/enum | views-inputs | assignments | 0.723 | 0.686–0.772 | 0.634 | 22.2 | 121 | 9 |
| essential512/enum | views-inputs | words | 0.718 | 0.696–0.872 | 0.630 | 22.2 | 121 | 9 |
| essential512/slab | materialized | assignments | 0.447 | 0.418–0.503 | 0.229 | 43.7 | 345 | 9 |
| essential512/slab | views | assignments | 0.712 | 0.669–0.758 | 0.616 | 13.1 | 130 | 9 |
| essential512/slab | views | words | 0.740 | 0.701–0.773 | 0.632 | 13.1 | 130 | 9 |
| essential512/slab | views-inputs | assignments | 0.725 | 0.703–0.783 | 0.632 | 13.1 | 121 | 9 |
| essential512/slab | views-inputs | words | 0.740 | 0.702–0.797 | 0.620 | 13.1 | 121 | 9 |
| essential64/enum | materialized | assignments | 0.972 | 0.916–1.068 | 0.551 | 167.9 | 1159 | 9 |
| essential64/enum | views | assignments | 0.941 | 0.877–0.982 | 0.740 | 79.5 | 495 | 9 |
| essential64/enum | views | words | 0.989 | 0.893–1.028 | 0.763 | 79.5 | 495 | 9 |
| essential64/enum | views-inputs | assignments | 0.964 | 0.910–1.008 | 0.736 | 79.4 | 519 | 9 |
| essential64/enum | views-inputs | words | 0.929 | 0.871–1.046 | 0.722 | 79.4 | 519 | 9 |
| essential64/slab | materialized | assignments | 0.953 | 0.936–1.113 | 0.580 | 106.1 | 1159 | 9 |
| essential64/slab | views | assignments | 0.895 | 0.887–1.034 | 0.765 | 46.7 | 495 | 9 |
| essential64/slab | views | words | 0.896 | 0.866–0.976 | 0.737 | 46.7 | 495 | 9 |
| essential64/slab | views-inputs | assignments | 0.876 | 0.856–0.939 | 0.715 | 46.7 | 519 | 9 |
| essential64/slab | views-inputs | words | 0.929 | 0.894–0.977 | 0.714 | 46.7 | 519 | 9 |
| packed512/enum | materialized | assignments | 0.827 | 0.765–0.869 | 0.763 | 74.3 | 482 | 9 |

## relations_free_join, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 0.055 | 0.054–0.068 | 0.010 | 95.5 | 140 | 9 |
| essential512/enum | materialized | assignments | 0.336 | 0.308–0.351 | 0.009 | 79.9 | 345 | 9 |
| essential512/enum | views | assignments | 0.385 | 0.368–0.423 | 0.008 | 34.5 | 130 | 9 |
| essential512/enum | views | words | 0.378 | 0.371–0.398 | 0.008 | 34.5 | 130 | 9 |
| essential512/enum | views-inputs | assignments | 0.387 | 0.370–0.420 | 0.008 | 34.4 | 121 | 9 |
| essential512/enum | views-inputs | words | 0.407 | 0.381–0.485 | 0.008 | 34.4 | 121 | 9 |
| essential512/slab | materialized | assignments | 0.314 | 0.307–0.367 | 0.009 | 56.7 | 345 | 9 |
| essential512/slab | views | assignments | 0.388 | 0.371–0.443 | 0.009 | 24.9 | 130 | 9 |
| essential512/slab | views | words | 0.390 | 0.379–0.436 | 0.008 | 24.9 | 130 | 9 |
| essential512/slab | views-inputs | assignments | 0.403 | 0.380–0.410 | 0.008 | 25.2 | 121 | 9 |
| essential512/slab | views-inputs | words | 0.384 | 0.370–0.442 | 0.008 | 25.2 | 121 | 9 |
| essential64/enum | materialized | assignments | 0.663 | 0.656–0.738 | 0.009 | 180.9 | 1159 | 9 |
| essential64/enum | views | assignments | 0.508 | 0.493–0.541 | 0.008 | 91.2 | 495 | 9 |
| essential64/enum | views | words | 0.538 | 0.498–0.615 | 0.008 | 91.2 | 495 | 9 |
| essential64/enum | views-inputs | assignments | 0.574 | 0.540–5.768 | 0.023 | 91.5 | 519 | 9 |
| essential64/enum | views-inputs | words | 0.518 | 0.511–0.590 | 0.009 | 91.5 | 519 | 9 |
| essential64/slab | materialized | assignments | 0.674 | 0.636–0.725 | 0.009 | 119.1 | 1159 | 9 |
| essential64/slab | views | assignments | 0.522 | 0.494–0.562 | 0.008 | 58.5 | 495 | 9 |
| essential64/slab | views | words | 0.506 | 0.501–0.589 | 0.008 | 58.5 | 495 | 9 |
| essential64/slab | views-inputs | assignments | 0.528 | 0.511–0.591 | 0.009 | 58.9 | 519 | 9 |
| essential64/slab | views-inputs | words | 0.513 | 0.497–0.533 | 0.010 | 58.9 | 519 | 9 |
| packed512/enum | materialized | assignments | 0.239 | 0.221–0.284 | 0.017 | 87.3 | 482 | 9 |

## relations_free_join, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 0.148 | 0.146–0.170 | 0.144 | 82.5 | 140 | 9 |
| essential512/enum | materialized | assignments | 0.721 | 0.714–0.806 | 0.455 | 99.1 | 510 | 9 |
| essential512/enum | views | assignments | 0.678 | 0.663–0.715 | 0.510 | 47.5 | 251 | 9 |
| essential512/enum | views | words | 0.683 | 0.656–0.761 | 0.508 | 47.5 | 251 | 9 |
| essential512/enum | views-inputs | assignments | 0.724 | 0.688–0.774 | 0.627 | 22.8 | 156 | 9 |
| essential512/enum | views-inputs | words | 0.732 | 0.700–0.800 | 0.645 | 22.8 | 156 | 9 |
| essential512/slab | materialized | assignments | 0.735 | 0.711–0.806 | 0.458 | 65.4 | 510 | 9 |
| essential512/slab | views | assignments | 0.734 | 0.677–0.759 | 0.519 | 31.0 | 251 | 9 |
| essential512/slab | views | words | 0.695 | 0.664–0.736 | 0.517 | 31.0 | 251 | 9 |
| essential512/slab | views-inputs | assignments | 0.739 | 0.712–0.776 | 0.636 | 13.1 | 156 | 9 |
| essential512/slab | views-inputs | words | 0.709 | 0.701–0.799 | 0.635 | 13.1 | 156 | 9 |
| essential64/enum | materialized | assignments | 1.499 | 1.463–1.562 | 0.695 | 210.8 | 1342 | 5 |
| essential64/enum | views | assignments | 3.522 | 3.351–3.661 | 2.976 | 196.9 | 1499 | 5 |
| essential64/enum | views | words | 3.589 | 3.488–3.749 | 2.904 | 196.9 | 1499 | 5 |
| essential64/enum | views-inputs | assignments | 2.758 | 2.699–2.768 | 2.358 | 96.6 | 544 | 5 |
| essential64/enum | views-inputs | words | 2.766 | 2.752–2.973 | 2.472 | 96.6 | 544 | 5 |
| essential64/slab | materialized | assignments | 1.607 | 1.522–1.655 | 0.772 | 138.8 | 1342 | 5 |
| essential64/slab | views | assignments | 3.531 | 3.472–3.591 | 2.935 | 127.3 | 1499 | 5 |
| essential64/slab | views | words | 3.509 | 3.445–3.614 | 2.908 | 127.3 | 1499 | 5 |
| essential64/slab | views-inputs | assignments | 2.795 | 2.704–2.835 | 2.502 | 62.9 | 544 | 5 |
| essential64/slab | views-inputs | words | 2.806 | 2.787–2.836 | 2.480 | 62.9 | 544 | 5 |
| packed512/enum | materialized | assignments | 1.413 | 1.383–1.457 | 1.303 | 291.1 | 1326 | 9 |

## relations_free_join, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 0.057 | 0.053–0.069 | 0.010 | 95.5 | 140 | 9 |
| essential512/enum | materialized | assignments | 0.527 | 0.487–0.683 | 0.010 | 112.1 | 510 | 9 |
| essential512/enum | views | assignments | 0.405 | 0.398–0.433 | 0.008 | 59.2 | 251 | 9 |
| essential512/enum | views | words | 0.419 | 0.405–0.465 | 0.008 | 59.2 | 251 | 9 |
| essential512/enum | views-inputs | assignments | 0.386 | 0.371–0.442 | 0.008 | 34.9 | 156 | 9 |
| essential512/enum | views-inputs | words | 0.387 | 0.377–0.453 | 0.008 | 34.9 | 156 | 9 |
| essential512/slab | materialized | assignments | 0.494 | 0.485–0.539 | 0.010 | 78.4 | 510 | 9 |
| essential512/slab | views | assignments | 0.421 | 0.409–0.479 | 0.008 | 42.7 | 251 | 9 |
| essential512/slab | views | words | 0.423 | 0.405–0.464 | 0.008 | 42.7 | 251 | 9 |
| essential512/slab | views-inputs | assignments | 0.392 | 0.377–0.434 | 0.008 | 25.2 | 156 | 9 |
| essential512/slab | views-inputs | words | 0.379 | 0.377–0.393 | 0.009 | 25.2 | 156 | 9 |
| essential64/enum | materialized | assignments | 1.127 | 1.073–1.168 | 0.009 | 223.8 | 1342 | 5 |
| essential64/enum | views | assignments | 2.037 | 2.026–2.117 | 0.009 | 208.7 | 1499 | 5 |
| essential64/enum | views | words | 2.049 | 2.029–2.065 | 0.008 | 208.7 | 1499 | 5 |
| essential64/enum | views-inputs | assignments | 1.529 | 1.475–1.567 | 0.009 | 108.8 | 544 | 5 |
| essential64/enum | views-inputs | words | 1.537 | 1.505–1.579 | 0.009 | 108.8 | 544 | 5 |
| essential64/slab | materialized | assignments | 1.119 | 1.075–1.166 | 0.010 | 151.8 | 1342 | 5 |
| essential64/slab | views | assignments | 2.046 | 2.017–2.105 | 0.009 | 139.1 | 1499 | 5 |
| essential64/slab | views | words | 2.090 | 1.991–2.122 | 0.008 | 139.1 | 1499 | 5 |
| essential64/slab | views-inputs | assignments | 1.553 | 1.531–1.604 | 0.009 | 75.0 | 544 | 5 |
| essential64/slab | views-inputs | words | 1.525 | 1.496–1.551 | 0.008 | 75.0 | 544 | 5 |
| packed512/enum | materialized | assignments | 0.498 | 0.474–0.528 | 0.012 | 304.1 | 1326 | 9 |

## relations_free_join, 12 coordinates, pair-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 0.147 | 0.146–0.221 | 0.144 | 82.5 | 140 | 5 |
| essential512/enum | materialized | assignments | 0.578 | 0.551–0.622 | 0.312 | 75.4 | 430 | 5 |
| essential512/enum | views | assignments | 0.767 | 0.746–0.793 | 0.643 | 34.3 | 189 | 5 |
| essential512/enum | views | words | 0.797 | 0.719–1.110 | 0.638 | 34.3 | 189 | 5 |
| essential512/enum | views-inputs | assignments | 0.764 | 0.724–0.830 | 0.660 | 22.4 | 128 | 5 |
| essential512/enum | views-inputs | words | 0.727 | 0.701–0.811 | 0.662 | 22.4 | 128 | 5 |
| essential512/slab | materialized | assignments | 0.555 | 0.535–0.611 | 0.308 | 47.5 | 430 | 5 |
| essential512/slab | views | assignments | 0.768 | 0.722–0.806 | 0.635 | 22.0 | 189 | 5 |
| essential512/slab | views | words | 0.768 | 0.728–0.986 | 0.626 | 22.0 | 189 | 5 |
| essential512/slab | views-inputs | assignments | 0.773 | 0.715–0.833 | 0.650 | 13.1 | 128 | 5 |
| essential512/slab | views-inputs | words | 0.741 | 0.716–0.758 | 0.646 | 13.1 | 128 | 5 |
| essential64/enum | materialized | assignments | 1.390 | 1.275–1.428 | 0.750 | 211.6 | 1392 | 5 |
| essential64/enum | views | assignments | 2.910 | 2.887–3.035 | 2.573 | 98.3 | 714 | 5 |
| essential64/enum | views | words | 2.875 | 2.847–2.952 | 2.444 | 98.3 | 714 | 5 |
| essential64/enum | views-inputs | assignments | 2.780 | 2.753–2.864 | 2.453 | 98.7 | 672 | 5 |
| essential64/enum | views-inputs | words | 2.729 | 2.700–2.978 | 2.408 | 98.7 | 672 | 5 |
| essential64/slab | materialized | assignments | 1.363 | 1.333–1.478 | 0.694 | 138.8 | 1392 | 5 |
| essential64/slab | views | assignments | 3.006 | 2.924–3.212 | 2.659 | 62.4 | 714 | 5 |
| essential64/slab | views | words | 2.772 | 2.729–2.939 | 2.488 | 62.4 | 714 | 5 |
| essential64/slab | views-inputs | assignments | 2.815 | 2.748–3.133 | 2.480 | 62.9 | 672 | 5 |
| essential64/slab | views-inputs | words | 2.748 | 2.695–2.813 | 2.415 | 62.9 | 672 | 5 |
| packed512/enum | materialized | assignments | 1.396 | 1.350–1.407 | 1.254 | 190.0 | 1107 | 5 |

## relations_free_join, 12 coordinates, pair-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 0.060 | 0.057–0.066 | 0.010 | 95.5 | 140 | 5 |
| essential512/enum | materialized | assignments | 0.414 | 0.392–0.475 | 0.009 | 88.4 | 430 | 5 |
| essential512/enum | views | assignments | 0.423 | 0.413–0.448 | 0.008 | 46.0 | 189 | 5 |
| essential512/enum | views | words | 0.438 | 0.430–0.441 | 0.008 | 46.0 | 189 | 5 |
| essential512/enum | views-inputs | assignments | 0.436 | 0.400–7.506 | 0.009 | 34.5 | 128 | 5 |
| essential512/enum | views-inputs | words | 0.420 | 0.401–0.437 | 0.009 | 34.5 | 128 | 5 |
| essential512/slab | materialized | assignments | 0.410 | 0.389–0.453 | 0.010 | 60.5 | 430 | 5 |
| essential512/slab | views | assignments | 0.428 | 0.412–0.447 | 0.008 | 33.8 | 189 | 5 |
| essential512/slab | views | words | 0.447 | 0.437–0.470 | 0.009 | 33.8 | 189 | 5 |
| essential512/slab | views-inputs | assignments | 0.403 | 0.395–0.413 | 0.008 | 25.2 | 128 | 5 |
| essential512/slab | views-inputs | words | 0.401 | 0.392–0.434 | 0.009 | 25.2 | 128 | 5 |
| essential64/enum | materialized | assignments | 1.039 | 1.002–1.082 | 0.010 | 224.6 | 1392 | 5 |
| essential64/enum | views | assignments | 1.619 | 1.558–1.742 | 0.008 | 110.1 | 714 | 5 |
| essential64/enum | views | words | 1.536 | 1.509–1.618 | 0.008 | 110.1 | 714 | 5 |
| essential64/enum | views-inputs | assignments | 1.492 | 1.473–1.506 | 0.008 | 110.8 | 672 | 5 |
| essential64/enum | views-inputs | words | 1.493 | 1.478–1.512 | 0.009 | 110.8 | 672 | 5 |
| essential64/slab | materialized | assignments | 0.994 | 0.946–1.047 | 0.010 | 151.8 | 1392 | 5 |
| essential64/slab | views | assignments | 1.582 | 1.561–1.647 | 0.008 | 74.2 | 714 | 5 |
| essential64/slab | views | words | 1.496 | 1.486–1.551 | 0.008 | 74.2 | 714 | 5 |
| essential64/slab | views-inputs | assignments | 1.533 | 1.474–1.566 | 0.012 | 75.0 | 672 | 5 |
| essential64/slab | views-inputs | words | 1.492 | 1.457–1.524 | 0.008 | 75.0 | 672 | 5 |
| packed512/enum | materialized | assignments | 0.489 | 0.473–0.520 | 0.014 | 203.0 | 1107 | 5 |

## relations_free_join, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 6.978 | 6.853–7.270 | 6.997 | 5810.7 | 175 | 9 |
| essential512/enum | materialized | assignments | 3.258 | 3.190–3.403 | 1.661 | 450.5 | 2668 | 9 |
| essential512/enum | views | assignments | 15.315 | 14.811–15.452 | 14.377 | 195.5 | 1011 | 9 |
| essential512/enum | views | words | 15.037 | 14.720–15.851 | 14.110 | 195.5 | 1011 | 9 |
| essential512/enum | views-inputs | assignments | 14.584 | 14.355–14.717 | 13.919 | 195.0 | 1024 | 9 |
| essential512/enum | views-inputs | words | 14.551 | 14.194–14.799 | 14.191 | 195.0 | 1024 | 9 |
| essential512/slab | materialized | assignments | 3.190 | 3.089–3.412 | 1.635 | 288.6 | 2668 | 9 |
| essential512/slab | views | assignments | 14.971 | 14.660–15.635 | 14.522 | 131.2 | 1011 | 9 |
| essential512/slab | views | words | 15.066 | 14.921–15.300 | 14.567 | 131.2 | 1011 | 9 |
| essential512/slab | views-inputs | assignments | 14.427 | 14.211–14.827 | 13.816 | 131.2 | 1024 | 9 |
| essential512/slab | views-inputs | words | 14.254 | 14.173–14.721 | 13.751 | 131.2 | 1024 | 9 |
| essential64/enum | materialized | assignments | 4.749 | 4.575–4.995 | 2.690 | 879.5 | 5699 | 9 |
| essential64/enum | views | assignments | 7.065 | 6.947–7.330 | 6.231 | 415.1 | 2640 | 9 |
| essential64/enum | views | words | 7.046 | 6.940–7.248 | 5.938 | 415.1 | 2640 | 9 |
| essential64/enum | views-inputs | assignments | 6.943 | 6.700–18.059 | 5.522 | 414.0 | 2863 | 9 |
| essential64/enum | views-inputs | words | 6.549 | 6.471–6.960 | 5.551 | 414.0 | 2863 | 9 |
| essential64/slab | materialized | assignments | 4.751 | 4.573–5.133 | 2.665 | 595.7 | 5699 | 9 |
| essential64/slab | views | assignments | 6.993 | 6.873–7.154 | 5.999 | 275.3 | 2640 | 9 |
| essential64/slab | views | words | 7.154 | 6.924–7.529 | 6.008 | 275.3 | 2640 | 9 |
| essential64/slab | views-inputs | assignments | 6.588 | 6.436–6.893 | 5.570 | 264.4 | 2863 | 9 |
| essential64/slab | views-inputs | words | 6.631 | 6.447–6.911 | 5.864 | 264.4 | 2863 | 9 |
| packed512/enum | materialized | assignments | 1.498 | 1.441–1.577 | 1.179 | 499.4 | 3815 | 9 |

## relations_free_join, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 2.534 | 2.421–2.588 | 0.049 | 5823.7 | 175 | 9 |
| essential512/enum | materialized | assignments | 2.385 | 2.334–2.483 | 0.012 | 463.5 | 2668 | 9 |
| essential512/enum | views | assignments | 7.771 | 7.623–7.920 | 0.009 | 207.2 | 1011 | 9 |
| essential512/enum | views | words | 7.771 | 7.579–8.028 | 0.010 | 207.2 | 1011 | 9 |
| essential512/enum | views-inputs | assignments | 7.526 | 7.446–8.109 | 0.009 | 207.5 | 1024 | 9 |
| essential512/enum | views-inputs | words | 7.476 | 7.356–7.712 | 0.009 | 207.5 | 1024 | 9 |
| essential512/slab | materialized | assignments | 2.338 | 2.273–2.463 | 0.010 | 301.6 | 2668 | 9 |
| essential512/slab | views | assignments | 8.076 | 7.791–15.667 | 0.013 | 143.0 | 1011 | 9 |
| essential512/slab | views | words | 7.764 | 7.661–7.917 | 0.009 | 143.0 | 1011 | 9 |
| essential512/slab | views-inputs | assignments | 7.414 | 7.347–7.559 | 0.009 | 143.8 | 1024 | 9 |
| essential512/slab | views-inputs | words | 7.537 | 7.346–7.669 | 0.009 | 143.8 | 1024 | 9 |
| essential64/enum | materialized | assignments | 3.408 | 3.275–15.992 | 0.010 | 892.5 | 5699 | 9 |
| essential64/enum | views | assignments | 3.967 | 3.850–4.216 | 0.009 | 426.9 | 2640 | 9 |
| essential64/enum | views | words | 3.956 | 3.866–3.995 | 0.009 | 426.9 | 2640 | 9 |
| essential64/enum | views-inputs | assignments | 3.773 | 3.577–3.963 | 0.009 | 426.5 | 2863 | 9 |
| essential64/enum | views-inputs | words | 3.759 | 3.684–4.019 | 0.009 | 426.5 | 2863 | 9 |
| essential64/slab | materialized | assignments | 3.302 | 3.223–3.537 | 0.010 | 608.7 | 5699 | 9 |
| essential64/slab | views | assignments | 3.895 | 3.805–4.094 | 0.008 | 287.0 | 2640 | 9 |
| essential64/slab | views | words | 3.917 | 3.870–4.468 | 0.009 | 287.0 | 2640 | 9 |
| essential64/slab | views-inputs | assignments | 3.723 | 3.628–3.961 | 0.008 | 277.0 | 2863 | 9 |
| essential64/slab | views-inputs | words | 3.773 | 3.622–3.908 | 0.009 | 277.0 | 2863 | 9 |
| packed512/enum | materialized | assignments | 0.751 | 0.707–0.776 | 0.040 | 512.4 | 3815 | 9 |

## relations_free_join, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 6.972 | 6.824–7.472 | 7.160 | 5810.7 | 175 | 9 |
| essential512/enum | materialized | assignments | 10.259 | 9.743–20.027 | 5.716 | 1043.6 | 5273 | 9 |
| essential512/enum | views | assignments | 158.915 | 154.066–169.728 | 157.929 | 1722.5 | 8088 | 9 |
| essential512/enum | views | words | 95.739 | 94.428–96.502 | 90.092 | 1722.5 | 8088 | 9 |
| essential512/enum | views-inputs | assignments | 43.576 | 42.982–44.049 | 42.173 | 263.4 | 1642 | 9 |
| essential512/enum | views-inputs | words | 44.090 | 43.840–44.560 | 42.905 | 263.4 | 1642 | 9 |
| essential512/slab | materialized | assignments | 10.048 | 9.843–10.280 | 5.923 | 655.1 | 5273 | 9 |
| essential512/slab | views | assignments | 173.536 | 171.946–180.121 | 171.722 | 1096.4 | 8088 | 9 |
| essential512/slab | views | words | 96.582 | 95.269–97.155 | 91.684 | 1096.4 | 8088 | 9 |
| essential512/slab | views-inputs | assignments | 43.848 | 43.410–44.168 | 42.933 | 182.3 | 1642 | 9 |
| essential512/slab | views-inputs | words | 43.602 | 43.387–43.988 | 43.267 | 182.3 | 1642 | 9 |
| essential64/enum | materialized | assignments | 26.363 | 26.244–26.905 | 18.152 | 1866.6 | 9513 | 5 |
| essential64/enum | views | assignments | 43.142 | 42.796–52.383 | 37.292 | 2402.2 | 14862 | 5 |
| essential64/enum | views | words | 36.527 | 35.958–37.765 | 31.231 | 2402.2 | 14862 | 5 |
| essential64/enum | views-inputs | assignments | 39.709 | 39.027–40.105 | 36.595 | 911.1 | 4924 | 5 |
| essential64/enum | views-inputs | words | 40.247 | 39.775–40.824 | 36.614 | 911.1 | 4924 | 5 |
| essential64/slab | materialized | assignments | 27.627 | 25.875–27.653 | 16.751 | 1308.0 | 9513 | 5 |
| essential64/slab | views | assignments | 46.782 | 44.529–53.338 | 47.664 | 1500.5 | 14862 | 5 |
| essential64/slab | views | words | 35.553 | 34.936–36.510 | 29.972 | 1500.5 | 14862 | 5 |
| essential64/slab | views-inputs | assignments | 40.150 | 39.701–40.378 | 36.636 | 631.4 | 4924 | 5 |
| essential64/slab | views-inputs | words | 40.136 | 39.036–40.311 | 36.108 | 631.4 | 4924 | 5 |
| packed512/enum | materialized | assignments | 5.464 | 5.186–5.730 | 4.697 | 1200.6 | 7882 | 9 |

## relations_free_join, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 2.530 | 2.377–2.661 | 0.050 | 5823.7 | 175 | 9 |
| essential512/enum | materialized | assignments | 7.096 | 6.850–7.611 | 0.010 | 1056.6 | 5273 | 9 |
| essential512/enum | views | assignments | 80.734 | 79.085–83.240 | 0.009 | 1734.3 | 8088 | 9 |
| essential512/enum | views | words | 49.126 | 48.689–49.715 | 0.008 | 1734.3 | 8088 | 9 |
| essential512/enum | views-inputs | assignments | 21.952 | 21.777–22.301 | 0.008 | 275.9 | 1642 | 9 |
| essential512/enum | views-inputs | words | 22.060 | 21.890–23.057 | 0.009 | 275.9 | 1642 | 9 |
| essential512/slab | materialized | assignments | 6.981 | 6.783–7.108 | 0.010 | 668.1 | 5273 | 9 |
| essential512/slab | views | assignments | 90.416 | 88.593–135.186 | 0.009 | 1108.2 | 8088 | 9 |
| essential512/slab | views | words | 49.502 | 48.821–51.942 | 0.009 | 1108.2 | 8088 | 9 |
| essential512/slab | views-inputs | assignments | 22.571 | 22.050–22.854 | 0.009 | 194.9 | 1642 | 9 |
| essential512/slab | views-inputs | words | 22.557 | 22.143–22.719 | 0.008 | 194.9 | 1642 | 9 |
| essential64/enum | materialized | assignments | 18.405 | 17.269–18.732 | 0.010 | 1879.6 | 9513 | 5 |
| essential64/enum | views | assignments | 24.209 | 23.883–24.751 | 0.009 | 2414.0 | 14862 | 5 |
| essential64/enum | views | words | 21.608 | 21.152–22.176 | 0.009 | 2414.0 | 14862 | 5 |
| essential64/enum | views-inputs | assignments | 21.346 | 21.179–21.948 | 0.009 | 923.7 | 4924 | 5 |
| essential64/enum | views-inputs | words | 21.701 | 21.342–21.763 | 0.009 | 923.7 | 4924 | 5 |
| essential64/slab | materialized | assignments | 18.001 | 17.288–18.161 | 0.010 | 1321.0 | 9513 | 5 |
| essential64/slab | views | assignments | 26.143 | 25.017–38.931 | 0.009 | 1512.3 | 14862 | 5 |
| essential64/slab | views | words | 20.684 | 20.519–21.512 | 0.009 | 1512.3 | 14862 | 5 |
| essential64/slab | views-inputs | assignments | 21.410 | 21.307–22.049 | 0.009 | 644.0 | 4924 | 5 |
| essential64/slab | views-inputs | words | 21.423 | 21.274–21.842 | 0.009 | 644.0 | 4924 | 5 |
| packed512/enum | materialized | assignments | 2.331 | 2.242–2.379 | 0.032 | 1213.6 | 7882 | 9 |

## relations_free_join, 18 coordinates, pair-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 7.305 | 7.243–7.684 | 7.119 | 5810.7 | 175 | 5 |
| essential512/enum | materialized | assignments | 4.199 | 4.121–4.279 | 2.180 | 543.8 | 3223 | 5 |
| essential512/enum | views | assignments | 16.411 | 16.091–16.573 | 15.189 | 235.1 | 1496 | 5 |
| essential512/enum | views | words | 16.352 | 16.108–40.467 | 15.001 | 235.1 | 1496 | 5 |
| essential512/enum | views-inputs | assignments | 15.969 | 15.141–29.144 | 14.747 | 217.2 | 1192 | 5 |
| essential512/enum | views-inputs | words | 15.079 | 15.017–15.277 | 14.205 | 217.2 | 1192 | 5 |
| essential512/slab | materialized | assignments | 4.031 | 3.987–4.133 | 2.200 | 358.5 | 3223 | 5 |
| essential512/slab | views | assignments | 16.143 | 16.027–16.328 | 15.038 | 151.0 | 1496 | 5 |
| essential512/slab | views | words | 16.313 | 16.233–16.489 | 15.235 | 151.0 | 1496 | 5 |
| essential512/slab | views-inputs | assignments | 15.379 | 15.199–15.627 | 14.433 | 151.0 | 1192 | 5 |
| essential512/slab | views-inputs | words | 15.040 | 14.726–15.274 | 14.484 | 151.0 | 1192 | 5 |
| essential64/enum | materialized | assignments | 6.786 | 6.691–15.078 | 3.527 | 927.0 | 6713 | 5 |
| essential64/enum | views | assignments | 18.705 | 18.426–18.861 | 16.756 | 588.5 | 3722 | 5 |
| essential64/enum | views | words | 18.031 | 17.787–18.225 | 16.158 | 588.5 | 3722 | 5 |
| essential64/enum | views-inputs | assignments | 16.165 | 15.788–16.862 | 14.839 | 440.3 | 3522 | 5 |
| essential64/enum | views-inputs | words | 16.710 | 16.377–17.618 | 14.890 | 440.3 | 3522 | 5 |
| essential64/slab | materialized | assignments | 6.574 | 6.375–7.741 | 3.623 | 631.4 | 6713 | 5 |
| essential64/slab | views | assignments | 18.596 | 18.175–19.400 | 16.755 | 354.0 | 3722 | 5 |
| essential64/slab | views | words | 18.032 | 17.795–18.457 | 19.365 | 354.0 | 3722 | 5 |
| essential64/slab | views-inputs | assignments | 16.568 | 16.261–17.038 | 14.403 | 293.1 | 3522 | 5 |
| essential64/slab | views-inputs | words | 16.071 | 15.925–16.496 | 14.737 | 293.1 | 3522 | 5 |
| packed512/enum | materialized | assignments | 4.000 | 3.868–4.985 | 3.477 | 851.2 | 5840 | 5 |

## relations_free_join, 18 coordinates, pair-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum | materialized | assignments | 2.445 | 2.401–2.525 | 0.051 | 5823.7 | 175 | 5 |
| essential512/enum | materialized | assignments | 3.111 | 3.088–3.310 | 0.010 | 556.8 | 3223 | 5 |
| essential512/enum | views | assignments | 8.480 | 8.297–8.660 | 0.009 | 246.9 | 1496 | 5 |
| essential512/enum | views | words | 8.430 | 8.314–8.474 | 0.009 | 246.9 | 1496 | 5 |
| essential512/enum | views-inputs | assignments | 8.076 | 7.938–8.171 | 0.009 | 229.8 | 1192 | 5 |
| essential512/enum | views-inputs | words | 7.936 | 7.696–8.243 | 0.009 | 229.8 | 1192 | 5 |
| essential512/slab | materialized | assignments | 2.896 | 2.877–2.974 | 0.010 | 371.5 | 3223 | 5 |
| essential512/slab | views | assignments | 8.853 | 8.399–29.164 | 0.009 | 162.7 | 1496 | 5 |
| essential512/slab | views | words | 8.498 | 8.429–8.688 | 0.009 | 162.7 | 1496 | 5 |
| essential512/slab | views-inputs | assignments | 7.815 | 7.741–7.866 | 0.009 | 163.5 | 1192 | 5 |
| essential512/slab | views-inputs | words | 7.676 | 7.644–7.844 | 0.009 | 163.5 | 1192 | 5 |
| essential64/enum | materialized | assignments | 4.763 | 4.535–4.866 | 0.011 | 940.0 | 6713 | 5 |
| essential64/enum | views | assignments | 9.881 | 9.816–10.078 | 0.009 | 600.3 | 3722 | 5 |
| essential64/enum | views | words | 9.817 | 9.610–9.953 | 0.009 | 600.3 | 3722 | 5 |
| essential64/enum | views-inputs | assignments | 8.689 | 8.581–8.955 | 0.009 | 452.9 | 3522 | 5 |
| essential64/enum | views-inputs | words | 8.853 | 8.632–8.961 | 0.009 | 452.9 | 3522 | 5 |
| essential64/slab | materialized | assignments | 4.722 | 4.576–4.994 | 0.010 | 644.3 | 6713 | 5 |
| essential64/slab | views | assignments | 9.912 | 9.888–10.064 | 0.008 | 365.8 | 3722 | 5 |
| essential64/slab | views | words | 9.865 | 9.805–10.109 | 0.009 | 365.8 | 3722 | 5 |
| essential64/slab | views-inputs | assignments | 8.758 | 8.567–8.859 | 0.009 | 305.7 | 3522 | 5 |
| essential64/slab | views-inputs | words | 8.761 | 8.586–9.075 | 0.009 | 305.7 | 3522 | 5 |
| packed512/enum | materialized | assignments | 1.996 | 1.941–2.042 | 0.034 | 864.1 | 5840 | 5 |

528 cases; 768 matched comparisons; 292 retained processes.

- [Raw evidence](results/view2-smoke.json).
- [Raw evidence](results/view2-initial-sweep.json).
- [Raw evidence](results/view2-focused-repeat.json).
- [Raw evidence](results/view2-small-repeat.json).
- [Raw evidence](results/view2-acceptance.json).
