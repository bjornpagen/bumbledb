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
| essential512/enum | materialized | assignments | 0.898 | 0.898–0.898 | 0.620 | 64.9 | 329 | 1 |
| essential512/enum | views | assignments | 1.992 | 1.992–1.992 | 1.899 | 32.5 | 216 | 1 |
| essential512/enum | views | words | 2.136 | 2.136–2.136 | 1.938 | 32.5 | 216 | 1 |
| essential512/enum | views-inputs | assignments | 2.139 | 2.139–2.139 | 1.945 | 42.0 | 230 | 1 |
| essential512/enum | views-inputs | words | 2.086 | 2.086–2.086 | 2.020 | 42.0 | 230 | 1 |
| essential64/enum | materialized | assignments | 1.711 | 1.711–1.711 | 1.196 | 125.2 | 829 | 1 |
| essential64/enum | views | assignments | 3.934 | 3.934–3.934 | 3.687 | 79.6 | 486 | 1 |
| essential64/enum | views | words | 3.716 | 3.716–3.716 | 3.990 | 79.6 | 486 | 1 |
| essential64/enum | views-inputs | assignments | 3.967 | 3.967–3.967 | 3.570 | 80.3 | 616 | 1 |
| essential64/enum | views-inputs | words | 3.899 | 3.899–3.899 | 3.570 | 80.3 | 616 | 1 |

## product_free_join, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 0.281 | 0.281–0.281 | 0.009 | 72.3 | 329 | 1 |
| essential512/enum | views | assignments | 0.427 | 0.427–0.427 | 0.007 | 38.6 | 216 | 1 |
| essential512/enum | views | words | 0.447 | 0.447–0.447 | 0.008 | 38.6 | 216 | 1 |
| essential512/enum | views-inputs | assignments | 0.442 | 0.442–0.442 | 0.007 | 49.0 | 230 | 1 |
| essential512/enum | views-inputs | words | 0.493 | 0.493–0.493 | 0.008 | 49.0 | 230 | 1 |
| essential64/enum | materialized | assignments | 0.563 | 0.563–0.563 | 0.009 | 132.6 | 829 | 1 |
| essential64/enum | views | assignments | 0.902 | 0.902–0.902 | 0.010 | 85.6 | 486 | 1 |
| essential64/enum | views | words | 0.877 | 0.877–0.877 | 0.014 | 85.6 | 486 | 1 |
| essential64/enum | views-inputs | assignments | 0.895 | 0.895–0.895 | 0.009 | 87.3 | 616 | 1 |
| essential64/enum | views-inputs | words | 0.909 | 0.909–0.909 | 0.008 | 87.3 | 616 | 1 |

## product_free_join, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 1.246 | 1.246–1.246 | 1.057 | 88.6 | 456 | 1 |
| essential512/enum | views | assignments | 4.018 | 4.018–4.018 | 1.490 | 48.8 | 291 | 1 |
| essential512/enum | views | words | 1.601 | 1.601–1.601 | 1.449 | 48.8 | 291 | 1 |
| essential512/enum | views-inputs | assignments | 2.004 | 2.004–2.004 | 1.873 | 41.6 | 292 | 1 |
| essential512/enum | views-inputs | words | 1.995 | 1.995–1.995 | 1.871 | 41.6 | 292 | 1 |
| essential64/enum | materialized | assignments | 1.379 | 1.379–1.379 | 1.030 | 102.0 | 851 | 1 |
| essential64/enum | views | assignments | 8.171 | 8.171–8.171 | 8.332 | 177.0 | 1018 | 1 |
| essential64/enum | views | words | 14.050 | 14.050–14.050 | 7.641 | 177.0 | 1018 | 1 |
| essential64/enum | views-inputs | assignments | 5.623 | 5.623–5.623 | 5.557 | 98.5 | 716 | 1 |
| essential64/enum | views-inputs | words | 5.454 | 5.454–5.454 | 5.177 | 98.5 | 716 | 1 |

## product_free_join, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 0.389 | 0.389–0.389 | 0.009 | 96.0 | 456 | 1 |
| essential512/enum | views | assignments | 0.446 | 0.446–0.446 | 0.008 | 54.8 | 291 | 1 |
| essential512/enum | views | words | 0.436 | 0.436–0.436 | 0.007 | 54.8 | 291 | 1 |
| essential512/enum | views-inputs | assignments | 0.410 | 0.410–0.410 | 0.008 | 48.6 | 292 | 1 |
| essential512/enum | views-inputs | words | 0.435 | 0.435–0.435 | 0.009 | 48.6 | 292 | 1 |
| essential64/enum | materialized | assignments | 0.478 | 0.478–0.478 | 0.009 | 109.4 | 851 | 1 |
| essential64/enum | views | assignments | 1.883 | 1.883–1.883 | 0.010 | 183.1 | 1018 | 1 |
| essential64/enum | views | words | 2.632 | 2.632–2.632 | 0.013 | 183.1 | 1018 | 1 |
| essential64/enum | views-inputs | assignments | 1.307 | 1.307–1.307 | 0.010 | 105.5 | 716 | 1 |
| essential64/enum | views-inputs | words | 1.232 | 1.232–1.232 | 0.008 | 105.5 | 716 | 1 |

## product_free_join, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 7.858 | 7.858–7.858 | 5.927 | 583.9 | 3112 | 1 |
| essential512/enum | views | assignments | 61.729 | 61.729–61.729 | 60.568 | 311.2 | 1792 | 1 |
| essential512/enum | views | words | 64.140 | 64.140–64.140 | 63.295 | 311.2 | 1792 | 1 |
| essential512/enum | views-inputs | assignments | 66.223 | 66.223–66.223 | 64.564 | 400.2 | 2342 | 1 |
| essential512/enum | views-inputs | words | 65.610 | 65.610–65.610 | 64.886 | 400.2 | 2342 | 1 |
| essential64/enum | materialized | assignments | 10.529 | 10.529–10.529 | 8.859 | 826.7 | 5582 | 1 |
| essential64/enum | views | assignments | 36.457 | 36.457–36.457 | 34.892 | 408.4 | 3102 | 1 |
| essential64/enum | views | words | 35.621 | 35.621–35.621 | 35.255 | 408.4 | 3102 | 1 |
| essential64/enum | views-inputs | assignments | 36.328 | 36.328–36.328 | 35.025 | 796.2 | 4643 | 1 |
| essential64/enum | views-inputs | words | 35.867 | 35.867–35.867 | 34.296 | 796.2 | 4643 | 1 |

## product_free_join, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 2.548 | 2.548–2.548 | 0.022 | 591.3 | 3112 | 1 |
| essential512/enum | views | assignments | 12.057 | 12.057–12.057 | 0.017 | 317.3 | 1792 | 1 |
| essential512/enum | views | words | 12.877 | 12.877–12.877 | 0.019 | 317.3 | 1792 | 1 |
| essential512/enum | views-inputs | assignments | 12.826 | 12.826–12.826 | 0.016 | 407.2 | 2342 | 1 |
| essential512/enum | views-inputs | words | 13.677 | 13.677–13.677 | 0.020 | 407.2 | 2342 | 1 |
| essential64/enum | materialized | assignments | 3.414 | 3.414–3.414 | 0.021 | 834.1 | 5582 | 1 |
| essential64/enum | views | assignments | 7.954 | 7.954–7.954 | 0.019 | 414.4 | 3102 | 1 |
| essential64/enum | views | words | 7.767 | 7.767–7.767 | 0.018 | 414.4 | 3102 | 1 |
| essential64/enum | views-inputs | assignments | 8.156 | 8.156–8.156 | 0.019 | 803.2 | 4643 | 1 |
| essential64/enum | views-inputs | words | 7.991 | 7.991–7.991 | 0.028 | 803.2 | 4643 | 1 |

## product_free_join, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 7.314 | 7.314–7.314 | 5.975 | 512.2 | 3227 | 1 |
| essential512/enum | views | assignments | 283.406 | 283.406–283.406 | 337.099 | 1011.5 | 5565 | 1 |
| essential512/enum | views | words | 135.426 | 135.426–135.426 | 132.457 | 1011.5 | 5565 | 1 |
| essential512/enum | views-inputs | assignments | 102.286 | 102.286–102.286 | 99.662 | 461.9 | 2645 | 1 |
| essential512/enum | views-inputs | words | 99.953 | 99.953–99.953 | 98.297 | 461.9 | 2645 | 1 |
| essential64/enum | materialized | assignments | 6.718 | 6.718–6.718 | 3.836 | 1087.0 | 7382 | 1 |
| essential64/enum | views | assignments | 110.983 | 110.983–110.983 | 108.337 | 1537.1 | 9887 | 1 |
| essential64/enum | views | words | 170.847 | 170.847–170.847 | 118.481 | 1537.1 | 9887 | 1 |
| essential64/enum | views-inputs | assignments | 17.109 | 17.109–17.109 | 14.953 | 777.2 | 6468 | 1 |
| essential64/enum | views-inputs | words | 16.540 | 16.540–16.540 | 15.286 | 777.2 | 6468 | 1 |

## product_free_join, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 2.283 | 2.283–2.283 | 0.019 | 519.6 | 3227 | 1 |
| essential512/enum | views | assignments | 84.008 | 84.008–84.008 | 0.018 | 1017.5 | 5565 | 1 |
| essential512/enum | views | words | 27.311 | 27.311–27.311 | 0.017 | 1017.5 | 5565 | 1 |
| essential512/enum | views-inputs | assignments | 19.576 | 19.576–19.576 | 0.017 | 468.9 | 2645 | 1 |
| essential512/enum | views-inputs | words | 19.774 | 19.774–19.774 | 0.019 | 468.9 | 2645 | 1 |
| essential64/enum | materialized | assignments | 3.177 | 3.177–3.177 | 0.020 | 1094.4 | 7382 | 1 |
| essential64/enum | views | assignments | 23.069 | 23.069–23.069 | 0.018 | 1543.2 | 9887 | 1 |
| essential64/enum | views | words | 20.285 | 20.285–20.285 | 0.018 | 1543.2 | 9887 | 1 |
| essential64/enum | views-inputs | assignments | 4.695 | 4.695–4.695 | 0.017 | 784.1 | 6468 | 1 |
| essential64/enum | views-inputs | words | 4.753 | 4.753–4.753 | 0.013 | 784.1 | 6468 | 1 |

## relations_free_join, 12 coordinates, bit-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 0.508 | 0.508–0.508 | 0.251 | 66.9 | 345 | 1 |
| essential512/enum | views | assignments | 0.749 | 0.749–0.749 | 0.650 | 22.7 | 130 | 1 |
| essential512/enum | views | words | 0.772 | 0.772–0.772 | 0.651 | 22.7 | 130 | 1 |
| essential512/enum | views-inputs | assignments | 0.772 | 0.772–0.772 | 0.625 | 22.2 | 121 | 1 |
| essential512/enum | views-inputs | words | 0.766 | 0.766–0.766 | 0.631 | 22.2 | 121 | 1 |
| essential64/enum | materialized | assignments | 1.069 | 1.069–1.069 | 0.617 | 167.9 | 1159 | 1 |
| essential64/enum | views | assignments | 0.970 | 0.970–0.970 | 0.745 | 79.5 | 495 | 1 |
| essential64/enum | views | words | 0.975 | 0.975–0.975 | 0.793 | 79.5 | 495 | 1 |
| essential64/enum | views-inputs | assignments | 1.457 | 1.457–1.457 | 0.805 | 79.4 | 519 | 1 |
| essential64/enum | views-inputs | words | 0.949 | 0.949–0.949 | 0.786 | 79.4 | 519 | 1 |

## relations_free_join, 12 coordinates, bit-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 0.344 | 0.344–0.344 | 0.019 | 79.9 | 345 | 1 |
| essential512/enum | views | assignments | 0.416 | 0.416–0.416 | 0.010 | 34.5 | 130 | 1 |
| essential512/enum | views | words | 0.430 | 0.430–0.430 | 0.010 | 34.5 | 130 | 1 |
| essential512/enum | views-inputs | assignments | 0.393 | 0.393–0.393 | 0.010 | 34.4 | 121 | 1 |
| essential512/enum | views-inputs | words | 0.397 | 0.397–0.397 | 0.011 | 34.4 | 121 | 1 |
| essential64/enum | materialized | assignments | 0.725 | 0.725–0.725 | 0.018 | 180.9 | 1159 | 1 |
| essential64/enum | views | assignments | 0.534 | 0.534–0.534 | 0.018 | 91.2 | 495 | 1 |
| essential64/enum | views | words | 0.552 | 0.552–0.552 | 0.012 | 91.2 | 495 | 1 |
| essential64/enum | views-inputs | assignments | 0.581 | 0.581–0.581 | 0.012 | 91.5 | 519 | 1 |
| essential64/enum | views-inputs | words | 0.570 | 0.570–0.570 | 0.010 | 91.5 | 519 | 1 |

## relations_free_join, 12 coordinates, face-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 0.792 | 0.792–0.792 | 0.484 | 99.1 | 510 | 1 |
| essential512/enum | views | assignments | 0.796 | 0.796–0.796 | 0.521 | 47.5 | 251 | 1 |
| essential512/enum | views | words | 0.757 | 0.757–0.757 | 0.564 | 47.5 | 251 | 1 |
| essential512/enum | views-inputs | assignments | 0.766 | 0.766–0.766 | 0.652 | 22.8 | 156 | 1 |
| essential512/enum | views-inputs | words | 0.765 | 0.765–0.765 | 0.680 | 22.8 | 156 | 1 |
| essential64/enum | materialized | assignments | 1.481 | 1.481–1.481 | 0.680 | 210.8 | 1342 | 1 |
| essential64/enum | views | assignments | 3.413 | 3.413–3.413 | 2.753 | 196.9 | 1499 | 1 |
| essential64/enum | views | words | 3.612 | 3.612–3.612 | 3.029 | 196.9 | 1499 | 1 |
| essential64/enum | views-inputs | assignments | 2.853 | 2.853–2.853 | 2.459 | 96.6 | 544 | 1 |
| essential64/enum | views-inputs | words | 3.002 | 3.002–3.002 | 2.524 | 96.6 | 544 | 1 |

## relations_free_join, 12 coordinates, face-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 0.511 | 0.511–0.511 | 0.016 | 112.1 | 510 | 1 |
| essential512/enum | views | assignments | 0.428 | 0.428–0.428 | 0.010 | 59.2 | 251 | 1 |
| essential512/enum | views | words | 0.453 | 0.453–0.453 | 0.011 | 59.2 | 251 | 1 |
| essential512/enum | views-inputs | assignments | 0.407 | 0.407–0.407 | 0.010 | 34.9 | 156 | 1 |
| essential512/enum | views-inputs | words | 0.430 | 0.430–0.430 | 0.010 | 34.9 | 156 | 1 |
| essential64/enum | materialized | assignments | 1.124 | 1.124–1.124 | 0.013 | 223.8 | 1342 | 1 |
| essential64/enum | views | assignments | 1.984 | 1.984–1.984 | 0.022 | 208.7 | 1499 | 1 |
| essential64/enum | views | words | 2.116 | 2.116–2.116 | 0.011 | 208.7 | 1499 | 1 |
| essential64/enum | views-inputs | assignments | 1.570 | 1.570–1.570 | 0.013 | 108.8 | 544 | 1 |
| essential64/enum | views-inputs | words | 1.596 | 1.596–1.596 | 0.017 | 108.8 | 544 | 1 |

## relations_free_join, 18 coordinates, bit-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 3.463 | 3.463–3.463 | 1.699 | 450.5 | 2668 | 1 |
| essential512/enum | views | assignments | 15.306 | 15.306–15.306 | 14.776 | 195.5 | 1011 | 1 |
| essential512/enum | views | words | 15.239 | 15.239–15.239 | 14.580 | 195.5 | 1011 | 1 |
| essential512/enum | views-inputs | assignments | 14.364 | 14.364–14.364 | 13.798 | 195.0 | 1024 | 1 |
| essential512/enum | views-inputs | words | 14.686 | 14.686–14.686 | 13.930 | 195.0 | 1024 | 1 |
| essential64/enum | materialized | assignments | 5.082 | 5.082–5.082 | 3.010 | 879.5 | 5699 | 1 |
| essential64/enum | views | assignments | 7.047 | 7.047–7.047 | 10.580 | 415.1 | 2640 | 1 |
| essential64/enum | views | words | 7.295 | 7.295–7.295 | 6.158 | 415.1 | 2640 | 1 |
| essential64/enum | views-inputs | assignments | 7.912 | 7.912–7.912 | 5.912 | 414.0 | 2863 | 1 |
| essential64/enum | views-inputs | words | 6.724 | 6.724–6.724 | 6.005 | 414.0 | 2863 | 1 |

## relations_free_join, 18 coordinates, bit-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 2.360 | 2.360–2.360 | 0.029 | 463.5 | 2668 | 1 |
| essential512/enum | views | assignments | 7.944 | 7.944–7.944 | 0.020 | 207.2 | 1011 | 1 |
| essential512/enum | views | words | 7.928 | 7.928–7.928 | 0.023 | 207.2 | 1011 | 1 |
| essential512/enum | views-inputs | assignments | 7.355 | 7.355–7.355 | 0.022 | 207.5 | 1024 | 1 |
| essential512/enum | views-inputs | words | 7.585 | 7.585–7.585 | 0.020 | 207.5 | 1024 | 1 |
| essential64/enum | materialized | assignments | 3.382 | 3.382–3.382 | 0.025 | 892.5 | 5699 | 1 |
| essential64/enum | views | assignments | 11.446 | 11.446–11.446 | 0.023 | 426.9 | 2640 | 1 |
| essential64/enum | views | words | 3.997 | 3.997–3.997 | 0.022 | 426.9 | 2640 | 1 |
| essential64/enum | views-inputs | assignments | 3.979 | 3.979–3.979 | 0.024 | 426.5 | 2863 | 1 |
| essential64/enum | views-inputs | words | 3.891 | 3.891–3.891 | 0.024 | 426.5 | 2863 | 1 |

## relations_free_join, 18 coordinates, face-major, memo False

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 10.325 | 10.325–10.325 | 6.156 | 1043.6 | 5273 | 1 |
| essential512/enum | views | assignments | 159.496 | 159.496–159.496 | 153.466 | 1722.5 | 8088 | 1 |
| essential512/enum | views | words | 98.284 | 98.284–98.284 | 92.757 | 1722.5 | 8088 | 1 |
| essential512/enum | views-inputs | assignments | 44.054 | 44.054–44.054 | 43.227 | 263.4 | 1642 | 1 |
| essential512/enum | views-inputs | words | 43.434 | 43.434–43.434 | 43.866 | 263.4 | 1642 | 1 |
| essential64/enum | materialized | assignments | 26.102 | 26.102–26.102 | 16.377 | 1866.6 | 9513 | 1 |
| essential64/enum | views | assignments | 42.597 | 42.597–42.597 | 37.721 | 2402.2 | 14862 | 1 |
| essential64/enum | views | words | 35.523 | 35.523–35.523 | 30.110 | 2402.2 | 14862 | 1 |
| essential64/enum | views-inputs | assignments | 39.296 | 39.296–39.296 | 35.976 | 911.1 | 4924 | 1 |
| essential64/enum | views-inputs | words | 39.897 | 39.897–39.897 | 36.601 | 911.1 | 4924 | 1 |

## relations_free_join, 18 coordinates, face-major, memo True

| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| essential512/enum | materialized | assignments | 7.451 | 7.451–7.451 | 0.022 | 1056.6 | 5273 | 1 |
| essential512/enum | views | assignments | 161.405 | 161.405–161.405 | 0.022 | 1734.3 | 8088 | 1 |
| essential512/enum | views | words | 49.439 | 49.439–49.439 | 0.024 | 1734.3 | 8088 | 1 |
| essential512/enum | views-inputs | assignments | 21.935 | 21.935–21.935 | 0.021 | 275.9 | 1642 | 1 |
| essential512/enum | views-inputs | words | 22.586 | 22.586–22.586 | 0.020 | 275.9 | 1642 | 1 |
| essential64/enum | materialized | assignments | 17.204 | 17.204–17.204 | 0.024 | 1879.6 | 9513 | 1 |
| essential64/enum | views | assignments | 23.749 | 23.749–23.749 | 0.025 | 2414.0 | 14862 | 1 |
| essential64/enum | views | words | 20.760 | 20.760–20.760 | 0.022 | 2414.0 | 14862 | 1 |
| essential64/enum | views-inputs | assignments | 21.112 | 21.112–21.112 | 0.021 | 923.7 | 4924 | 1 |
| essential64/enum | views-inputs | words | 21.804 | 21.804–21.804 | 0.024 | 923.7 | 4924 | 1 |

160 cases; 256 matched comparisons; 46 retained processes.

- [Raw evidence](results/view2-smoke.json).
