# Same-binary dependency-directed projection after Free Join

Every query returns 64 Events: original, Possible, Guaranteed and Ambiguous
for sixteen groups. Fresh/warm totals include exact original-support counts.
Every returned Event is checked pointwise and canonically reimported outside
timing. Construction is separate. Bytes estimate retained carrier/cache memory.
They exclude allocator overhead, verification clones, result vectors and the join engine.
Executable: `e09c99985bf2e46b2c0dfd9a3d05323794440dda01b50f709913b0c837c08afa`.

Only entirely successful processes contribute timing rows. Partial rows from
capped or failed processes remain in the raw evidence but cannot supply a
complete-query comparison. A later incomplete process invalidates an older
successful process for that exact job; no failed result is a fast answer.

## fibred, width 4, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 1.289 | 0.080 | 0.066 | 0.013 | 0.001 | 0.077–0.136 | 0.075 | 305.2 | 277 | 3 |
| packed512/enum/staged/joint | joint | 1.297 | 0.245 | 0.140 | 0.071 | 0.034 | 0.233–0.373 | 0.077 | 629.8 | 4401 | 3 |
| prefix512/slab/ite/active | faces | 0.908 | 0.593 | 0.152 | 0.426 | 0.015 | 0.578–0.604 | 0.038 | 223.7 | 1736 | 3 |
| prefix512/slab/ite/joint | faces | 0.912 | 14.425 | 0.147 | 14.266 | 0.012 | 13.816–14.493 | 0.080 | 3110.9 | 20158 | 3 |
| prefix512/slab/ite/witness | faces | 0.952 | 0.587 | 0.169 | 0.398 | 0.018 | 0.566–1.318 | 0.060 | 223.7 | 1669 | 3 |
| prefix512/slab/staged/active | faces | 1.399 | 0.792 | 0.169 | 0.605 | 0.016 | 0.792–0.792 | 0.052 | 365.5 | 2403 | 1 |
| prefix512/slab/staged/joint | faces | 1.441 | 23.072 | 0.175 | 22.878 | 0.018 | 23.072–23.072 | 0.114 | 4553.0 | 33845 | 1 |
| prefix512/slab/staged/witness | faces | 1.494 | 0.727 | 0.189 | 0.520 | 0.016 | 0.727–0.727 | 0.054 | 335.0 | 2290 | 1 |
| prefix64/slab/ite/active | faces | 2.459 | 2.069 | 0.412 | 1.569 | 0.088 | 2.069–2.069 | 0.123 | 758.1 | 7316 | 1 |
| prefix64/slab/ite/joint | faces | 2.538 | 22.551 | 0.424 | 22.022 | 0.103 | 22.551–22.551 | 0.179 | 4228.7 | 41558 | 1 |
| prefix64/slab/ite/witness | faces | 2.539 | 1.735 | 0.426 | 1.209 | 0.099 | 1.735–1.735 | 0.132 | 605.7 | 6831 | 1 |
| retraction512/slab/ite/active | faces | 0.946 | 0.594 | 0.154 | 0.421 | 0.017 | 0.594–0.594 | 0.050 | 218.2 | 1688 | 1 |
| retraction512/slab/ite/joint | faces | 0.980 | 16.056 | 0.155 | 15.879 | 0.021 | 16.056–16.056 | 0.100 | 3022.7 | 21209 | 1 |
| retraction512/slab/ite/witness | faces | 0.976 | 0.641 | 0.155 | 0.467 | 0.017 | 0.641–0.641 | 0.048 | 218.2 | 1745 | 1 |
| retraction64/slab/ite/active | faces | 2.632 | 2.275 | 0.538 | 1.639 | 0.097 | 2.275–2.275 | 0.137 | 761.1 | 7496 | 1 |
| retraction64/slab/ite/joint | faces | 2.571 | 24.448 | 0.528 | 23.808 | 0.110 | 24.448–24.448 | 0.157 | 4310.3 | 42815 | 1 |
| retraction64/slab/ite/witness | faces | 2.711 | 2.433 | 0.535 | 1.799 | 0.098 | 2.433–2.433 | 0.139 | 761.1 | 7801 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 1.289 | 0.041 | 0.029 | 0.010 | 0.001 | 0.040–0.044 | 0.019 | 314.1 | 277 | 3 |
| packed512/enum/staged/joint | joint | 1.297 | 0.225 | 0.128 | 0.065 | 0.033 | 0.225–0.239 | 0.073 | 638.7 | 4401 | 3 |
| prefix512/slab/ite/active | faces | 0.908 | 0.576 | 0.148 | 0.416 | 0.012 | 0.573–0.605 | 0.037 | 232.6 | 1736 | 3 |
| prefix512/slab/ite/joint | faces | 0.912 | 13.809 | 0.139 | 13.644 | 0.012 | 13.692–13.905 | 0.071 | 3119.8 | 20158 | 3 |
| prefix512/slab/ite/witness | faces | 0.952 | 1.349 | 0.818 | 0.516 | 0.014 | 1.235–1.876 | 0.053 | 232.6 | 1669 | 3 |
| prefix512/slab/staged/active | faces | 1.399 | 0.739 | 0.148 | 0.578 | 0.012 | 0.739–0.739 | 0.040 | 374.4 | 2403 | 1 |
| prefix512/slab/staged/joint | faces | 1.441 | 22.409 | 0.166 | 22.231 | 0.012 | 22.409–22.409 | 0.090 | 4562.0 | 33845 | 1 |
| prefix512/slab/staged/witness | faces | 1.494 | 0.661 | 0.159 | 0.490 | 0.012 | 0.661–0.661 | 0.044 | 344.0 | 2290 | 1 |
| prefix64/slab/ite/active | faces | 2.459 | 1.996 | 0.395 | 1.521 | 0.081 | 1.996–1.996 | 0.115 | 767.0 | 7316 | 1 |
| prefix64/slab/ite/joint | faces | 2.538 | 21.793 | 0.476 | 21.233 | 0.083 | 21.793–21.793 | 0.137 | 4237.7 | 41558 | 1 |
| prefix64/slab/ite/witness | faces | 2.539 | 1.725 | 0.401 | 1.225 | 0.099 | 1.725–1.725 | 0.119 | 614.7 | 6831 | 1 |
| retraction512/slab/ite/active | faces | 0.946 | 0.567 | 0.142 | 0.412 | 0.012 | 0.567–0.567 | 0.036 | 227.1 | 1688 | 1 |
| retraction512/slab/ite/joint | faces | 0.980 | 15.491 | 0.144 | 15.334 | 0.013 | 15.491–15.491 | 0.094 | 3031.6 | 21209 | 1 |
| retraction512/slab/ite/witness | faces | 0.976 | 0.612 | 0.143 | 0.457 | 0.012 | 0.612–0.612 | 0.035 | 227.1 | 1745 | 1 |
| retraction64/slab/ite/active | faces | 2.632 | 2.176 | 0.508 | 1.582 | 0.085 | 2.176–2.176 | 0.147 | 770.0 | 7496 | 1 |
| retraction64/slab/ite/joint | faces | 2.571 | 23.946 | 0.511 | 23.349 | 0.086 | 23.946–23.946 | 0.147 | 4319.2 | 42815 | 1 |
| retraction64/slab/ite/witness | faces | 2.711 | 2.406 | 0.509 | 1.795 | 0.102 | 2.406–2.406 | 0.119 | 770.0 | 7801 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 1.262 | 0.082 | 0.066 | 0.015 | 0.001 | 0.075–0.090 | 0.075 | 305.2 | 277 | 3 |
| packed512/enum/staged/joint | joint | 0.272 | 0.228 | 0.156 | 0.043 | 0.029 | 0.208–0.269 | 0.063 | 638.9 | 3967 | 3 |
| prefix512/slab/ite/active | faces | 0.928 | 0.619 | 0.157 | 0.450 | 0.012 | 0.601–0.642 | 0.037 | 262.8 | 1727 | 3 |
| prefix512/slab/ite/joint | faces | 0.920 | 1.653 | 0.141 | 1.501 | 0.012 | 1.592–1.694 | 0.231 | 458.4 | 3653 | 3 |
| prefix512/slab/ite/witness | faces | 0.888 | 0.525 | 0.146 | 0.368 | 0.012 | 0.523–0.553 | 0.038 | 210.5 | 1639 | 3 |
| prefix512/slab/staged/active | faces | 1.333 | 0.834 | 0.182 | 0.634 | 0.016 | 0.834–0.834 | 0.048 | 346.3 | 2388 | 1 |
| prefix512/slab/staged/joint | faces | 1.343 | 2.534 | 0.171 | 2.345 | 0.016 | 2.534–2.534 | 0.260 | 684.1 | 5407 | 1 |
| prefix512/slab/staged/witness | faces | 1.406 | 0.752 | 0.180 | 0.553 | 0.017 | 0.752–0.752 | 0.054 | 315.8 | 2274 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, non-suffix/x, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 1.262 | 0.041 | 0.029 | 0.010 | 0.001 | 0.039–0.043 | 0.019 | 314.1 | 277 | 3 |
| packed512/enum/staged/joint | joint | 0.272 | 0.210 | 0.141 | 0.041 | 0.029 | 0.204–0.256 | 0.057 | 647.8 | 3967 | 3 |
| prefix512/slab/ite/active | faces | 0.928 | 0.588 | 0.143 | 0.437 | 0.012 | 0.588–0.606 | 0.034 | 271.7 | 1727 | 3 |
| prefix512/slab/ite/joint | faces | 0.920 | 1.583 | 0.145 | 1.423 | 0.011 | 1.562–1.671 | 0.221 | 467.3 | 3653 | 3 |
| prefix512/slab/ite/witness | faces | 0.888 | 0.513 | 0.141 | 0.361 | 0.011 | 0.512–0.525 | 0.038 | 219.5 | 1639 | 3 |
| prefix512/slab/staged/active | faces | 1.333 | 0.816 | 0.164 | 0.639 | 0.012 | 0.816–0.816 | 0.053 | 355.2 | 2388 | 1 |
| prefix512/slab/staged/joint | faces | 1.343 | 2.578 | 0.161 | 2.401 | 0.015 | 2.578–2.578 | 0.236 | 693.1 | 5407 | 1 |
| prefix512/slab/staged/witness | faces | 1.406 | 0.699 | 0.156 | 0.531 | 0.011 | 0.699–0.699 | 0.036 | 324.8 | 2274 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, suffix-1/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix64/slab/ite/active | faces | 2.578 | 0.686 | 0.430 | 0.130 | 0.125 | 0.686–0.686 | 0.242 | 510.3 | 4962 | 1 |
| prefix64/slab/ite/joint | faces | 2.517 | 0.675 | 0.426 | 0.127 | 0.120 | 0.675–0.675 | 0.240 | 510.3 | 4962 | 1 |
| prefix64/slab/ite/witness | faces | 2.503 | 0.699 | 0.442 | 0.130 | 0.126 | 0.699–0.699 | 0.305 | 510.3 | 4962 | 1 |
| retraction512/slab/ite/active | faces | 0.962 | 0.671 | 0.147 | 0.506 | 0.017 | 0.671–0.671 | 0.048 | 218.2 | 1755 | 1 |
| retraction512/slab/ite/joint | faces | 0.985 | 14.273 | 0.155 | 14.095 | 0.021 | 14.273–14.273 | 0.351 | 3022.7 | 19937 | 1 |
| retraction512/slab/ite/witness | faces | 0.974 | 0.716 | 0.157 | 0.539 | 0.018 | 0.716–0.716 | 0.048 | 308.5 | 1805 | 1 |
| retraction64/slab/ite/active | faces | 2.639 | 2.553 | 0.537 | 1.911 | 0.104 | 2.553–2.553 | 0.279 | 761.1 | 7937 | 1 |
| retraction64/slab/ite/joint | faces | 2.703 | 24.339 | 0.589 | 23.630 | 0.118 | 24.339–24.339 | 1.216 | 4249.3 | 40087 | 1 |
| retraction64/slab/ite/witness | faces | 2.658 | 2.704 | 0.541 | 2.062 | 0.100 | 2.704–2.704 | 0.279 | 761.1 | 8035 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 4, suffix-1/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix64/slab/ite/active | faces | 2.578 | 0.658 | 0.421 | 0.123 | 0.113 | 0.658–0.658 | 0.222 | 519.3 | 4962 | 1 |
| prefix64/slab/ite/joint | faces | 2.517 | 0.649 | 0.408 | 0.126 | 0.114 | 0.649–0.649 | 0.234 | 519.3 | 4962 | 1 |
| prefix64/slab/ite/witness | faces | 2.503 | 0.656 | 0.411 | 0.130 | 0.114 | 0.656–0.656 | 0.229 | 519.3 | 4962 | 1 |
| retraction512/slab/ite/active | faces | 0.962 | 0.647 | 0.143 | 0.483 | 0.019 | 0.647–0.647 | 0.042 | 227.1 | 1755 | 1 |
| retraction512/slab/ite/joint | faces | 0.985 | 13.931 | 0.145 | 13.774 | 0.012 | 13.931–13.931 | 0.331 | 3031.6 | 19937 | 1 |
| retraction512/slab/ite/witness | faces | 0.974 | 0.671 | 0.143 | 0.515 | 0.012 | 0.671–0.671 | 0.040 | 317.5 | 1805 | 1 |
| retraction64/slab/ite/active | faces | 2.639 | 2.445 | 0.513 | 1.839 | 0.093 | 2.445–2.445 | 0.251 | 770.0 | 7937 | 1 |
| retraction64/slab/ite/joint | faces | 2.703 | 23.103 | 0.517 | 22.494 | 0.092 | 23.103–23.103 | 1.104 | 4258.3 | 40087 | 1 |
| retraction64/slab/ite/witness | faces | 2.658 | 2.642 | 0.533 | 2.018 | 0.090 | 2.642–2.642 | 0.252 | 770.0 | 8035 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 460.791 | 3.793 | 3.114 | 0.612 | 0.063 | 3.565–4.420 | 3.694 | 41852.6 | 636 | 3 |
| packed512/enum/staged/joint | joint | 64.869 | 1.465 | 0.675 | 0.568 | 0.216 | 1.432–1.470 | 0.790 | 6651.9 | 47190 | 3 |
| prefix512/slab/ite/active | faces | 14.682 | 12.469 | 1.001 | 11.384 | 0.132 | 12.269–12.730 | 0.216 | 4318.9 | 37611 | 3 |
| prefix512/slab/ite/joint | faces | 14.142 | 371.689 | 0.990 | 370.520 | 0.185 | 371.054–387.981 | 0.275 | 83841.7 | 708405 | 3 |
| prefix512/slab/ite/witness | faces | 136.722 | 12.018 | 1.118 | 10.840 | 0.144 | 11.819–31.848 | 0.228 | 4318.9 | 35380 | 3 |
| prefix512/slab/staged/active | faces | 25.208 | 18.940 | 1.137 | 17.665 | 0.138 | 18.940–18.940 | 0.230 | 8076.0 | 66516 | 1 |
| prefix512/slab/staged/joint | faces | 25.853 | 1733.752 | 1.041 | 1732.508 | 0.202 | 1733.752–1733.752 | 0.431 | 162947.5 | 1363847 | 1 |
| prefix512/slab/staged/witness | faces | 25.032 | 15.862 | 1.100 | 14.614 | 0.145 | 15.862–15.862 | 0.231 | 8076.0 | 61226 | 1 |
| prefix64/slab/ite/active | faces | 16.635 | 18.267 | 1.317 | 16.649 | 0.301 | 18.267–18.267 | 0.397 | 9368.8 | 86464 | 1 |
| prefix64/slab/ite/joint | faces | 16.800 | 307.473 | 1.386 | 305.723 | 0.363 | 307.473–307.473 | 0.492 | 86802.6 | 839852 | 1 |
| prefix64/slab/ite/witness | faces | 16.655 | 15.824 | 1.342 | 14.182 | 0.299 | 15.824–15.824 | 0.403 | 6948.9 | 80231 | 1 |
| retraction512/slab/ite/active | faces | 14.945 | 13.510 | 1.421 | 11.919 | 0.168 | 13.510–13.510 | 0.246 | 4408.3 | 38679 | 1 |
| retraction512/slab/ite/joint | faces | 16.086 | 411.128 | 1.382 | 409.529 | 0.216 | 411.128–411.128 | 0.337 | 78436.5 | 673536 | 1 |
| retraction512/slab/ite/witness | faces | 15.171 | 15.510 | 1.360 | 13.986 | 0.164 | 15.510–15.510 | 0.242 | 6622.8 | 41728 | 1 |
| retraction64/slab/ite/active | faces | 24.003 | 20.316 | 2.059 | 17.708 | 0.548 | 20.316–20.316 | 0.584 | 7160.7 | 91649 | 1 |
| retraction64/slab/ite/joint | faces | 24.389 | 413.840 | 2.070 | 411.197 | 0.572 | 413.840–413.840 | 0.674 | 92504.6 | 804725 | 1 |
| retraction64/slab/ite/witness | faces | 24.410 | 24.153 | 2.052 | 21.623 | 0.476 | 24.153–24.153 | 0.583 | 9126.5 | 96018 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 460.791 | 1.446 | 0.909 | 0.480 | 0.062 | 1.445–1.470 | 0.515 | 41861.5 | 636 | 3 |
| packed512/enum/staged/joint | joint | 64.869 | 1.467 | 0.673 | 0.570 | 0.224 | 1.452–1.662 | 0.779 | 6660.8 | 47190 | 3 |
| prefix512/slab/ite/active | faces | 14.682 | 12.402 | 0.974 | 11.298 | 0.130 | 12.097–12.590 | 0.205 | 4327.8 | 37611 | 3 |
| prefix512/slab/ite/joint | faces | 14.142 | 392.599 | 1.020 | 391.363 | 0.177 | 368.885–404.967 | 0.275 | 83850.6 | 708405 | 3 |
| prefix512/slab/ite/witness | faces | 136.722 | 10.758 | 1.022 | 9.606 | 0.132 | 10.462–11.159 | 0.198 | 4327.8 | 35380 | 3 |
| prefix512/slab/staged/active | faces | 25.208 | 18.772 | 1.040 | 17.597 | 0.135 | 18.772–18.772 | 0.221 | 8085.0 | 66516 | 1 |
| prefix512/slab/staged/joint | faces | 25.853 | 1722.688 | 0.967 | 1721.521 | 0.199 | 1722.688–1722.688 | 0.403 | 162956.4 | 1363847 | 1 |
| prefix512/slab/staged/witness | faces | 25.032 | 15.502 | 1.005 | 14.357 | 0.139 | 15.502–15.502 | 0.209 | 8085.0 | 61226 | 1 |
| prefix64/slab/ite/active | faces | 16.635 | 17.136 | 1.327 | 15.505 | 0.303 | 17.136–17.136 | 0.384 | 9377.8 | 86464 | 1 |
| prefix64/slab/ite/joint | faces | 16.800 | 306.124 | 1.349 | 304.413 | 0.361 | 306.124–306.124 | 0.456 | 86811.5 | 839852 | 1 |
| prefix64/slab/ite/witness | faces | 16.655 | 15.726 | 1.326 | 14.094 | 0.305 | 15.726–15.726 | 0.377 | 6957.9 | 80231 | 1 |
| retraction512/slab/ite/active | faces | 14.945 | 13.224 | 1.313 | 11.754 | 0.157 | 13.224–13.224 | 0.234 | 4417.3 | 38679 | 1 |
| retraction512/slab/ite/joint | faces | 16.086 | 408.870 | 1.408 | 407.237 | 0.223 | 408.870–408.870 | 0.303 | 78445.5 | 673536 | 1 |
| retraction512/slab/ite/witness | faces | 15.171 | 15.520 | 1.429 | 13.932 | 0.159 | 15.520–15.520 | 0.228 | 6631.8 | 41728 | 1 |
| retraction64/slab/ite/active | faces | 24.003 | 20.203 | 2.031 | 17.693 | 0.479 | 20.203–20.203 | 0.562 | 7169.6 | 91649 | 1 |
| retraction64/slab/ite/joint | faces | 24.389 | 410.181 | 2.039 | 407.582 | 0.558 | 410.181–410.181 | 0.646 | 92513.5 | 804725 | 1 |
| retraction64/slab/ite/witness | faces | 24.410 | 23.835 | 2.102 | 21.257 | 0.476 | 23.835–23.835 | 0.570 | 9135.5 | 96018 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 463.251 | 3.518 | 2.888 | 0.567 | 0.064 | 3.343–4.087 | 3.717 | 41852.6 | 636 | 3 |
| packed512/enum/staged/joint | joint | 2.159 | 0.573 | 0.395 | 0.105 | 0.075 | 0.558–0.603 | 0.227 | 2402.4 | 18095 | 3 |
| prefix512/slab/ite/active | faces | 10.217 | 7.166 | 1.198 | 5.837 | 0.130 | 7.037–7.427 | 0.537 | 2764.0 | 26325 | 3 |
| prefix512/slab/ite/joint | faces | 10.507 | 8.604 | 1.185 | 7.288 | 0.131 | 8.539–8.684 | 0.724 | 4092.3 | 31114 | 3 |
| prefix512/slab/ite/witness | faces | 10.305 | 6.254 | 1.231 | 4.871 | 0.135 | 6.076–6.290 | 0.471 | 2642.1 | 24270 | 3 |
| prefix512/slab/staged/active | faces | 19.788 | 10.873 | 1.302 | 9.432 | 0.138 | 10.873–10.873 | 0.564 | 5060.3 | 50014 | 1 |
| prefix512/slab/staged/joint | faces | 18.945 | 14.777 | 1.348 | 13.282 | 0.146 | 14.777–14.777 | 0.775 | 7010.0 | 59034 | 1 |
| prefix512/slab/staged/witness | faces | 18.272 | 9.287 | 1.313 | 7.833 | 0.141 | 9.287–9.287 | 0.483 | 5060.3 | 46993 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, non-suffix/x, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 463.251 | 1.480 | 0.923 | 0.472 | 0.062 | 1.414–1.731 | 0.513 | 41861.5 | 636 | 3 |
| packed512/enum/staged/joint | joint | 2.159 | 0.595 | 0.418 | 0.103 | 0.074 | 0.574–0.687 | 0.208 | 2411.3 | 18095 | 3 |
| prefix512/slab/ite/active | faces | 10.217 | 7.346 | 1.241 | 5.957 | 0.129 | 7.321–7.386 | 0.553 | 2772.9 | 26325 | 3 |
| prefix512/slab/ite/joint | faces | 10.507 | 8.584 | 1.191 | 7.276 | 0.131 | 8.578–8.819 | 0.719 | 4101.3 | 31114 | 3 |
| prefix512/slab/ite/witness | faces | 10.305 | 5.985 | 1.182 | 4.667 | 0.127 | 5.962–6.062 | 0.508 | 2651.1 | 24270 | 3 |
| prefix512/slab/staged/active | faces | 19.788 | 10.758 | 1.241 | 9.387 | 0.129 | 10.758–10.758 | 0.554 | 5069.3 | 50014 | 1 |
| prefix512/slab/staged/joint | faces | 18.945 | 14.500 | 1.356 | 13.012 | 0.132 | 14.500–14.500 | 0.760 | 7019.0 | 59034 | 1 |
| prefix512/slab/staged/witness | faces | 18.272 | 9.281 | 1.278 | 7.873 | 0.129 | 9.281–9.281 | 0.479 | 5069.3 | 46993 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, suffix-1/x, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix64/slab/ite/active | faces | 16.842 | 2.013 | 1.304 | 0.355 | 0.353 | 2.013–2.013 | 0.721 | 4692.7 | 46969 | 1 |
| prefix64/slab/ite/joint | faces | 17.649 | 2.110 | 1.374 | 0.374 | 0.362 | 2.110–2.110 | 0.734 | 4692.7 | 46969 | 1 |
| prefix64/slab/ite/witness | faces | 16.803 | 2.030 | 1.313 | 0.356 | 0.360 | 2.030–2.030 | 0.726 | 4692.7 | 46969 | 1 |
| retraction512/slab/ite/active | faces | 15.324 | 14.228 | 1.420 | 12.625 | 0.183 | 14.228–14.228 | 0.564 | 4424.4 | 39708 | 1 |
| retraction512/slab/ite/joint | faces | 15.001 | 404.907 | 1.386 | 403.307 | 0.213 | 404.907–404.907 | 4.022 | 60172.4 | 637135 | 1 |
| retraction512/slab/ite/witness | faces | 15.371 | 16.070 | 1.426 | 14.460 | 0.183 | 16.070–16.070 | 0.586 | 6622.8 | 41723 | 1 |
| retraction64/slab/ite/active | faces | 23.967 | 23.699 | 2.047 | 21.135 | 0.516 | 23.699–23.699 | 1.254 | 9126.5 | 94990 | 1 |
| retraction64/slab/ite/joint | faces | 24.447 | 455.082 | 2.121 | 452.393 | 0.568 | 455.082–455.082 | 4.944 | 70726.0 | 777405 | 1 |
| retraction64/slab/ite/witness | faces | 24.272 | 25.566 | 2.051 | 23.013 | 0.501 | 25.566–25.566 | 1.149 | 11848.8 | 97465 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## fibred, width 6, suffix-1/x, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| prefix64/slab/ite/active | faces | 16.842 | 2.098 | 1.346 | 0.385 | 0.366 | 2.098–2.098 | 0.709 | 4701.6 | 46969 | 1 |
| prefix64/slab/ite/joint | faces | 17.649 | 1.987 | 1.268 | 0.362 | 0.357 | 1.987–1.987 | 0.713 | 4701.6 | 46969 | 1 |
| prefix64/slab/ite/witness | faces | 16.803 | 2.066 | 1.348 | 0.361 | 0.356 | 2.066–2.066 | 2.945 | 4701.6 | 46969 | 1 |
| retraction512/slab/ite/active | faces | 15.324 | 14.521 | 1.385 | 12.954 | 0.178 | 14.521–14.521 | 0.551 | 4433.4 | 39708 | 1 |
| retraction512/slab/ite/joint | faces | 15.001 | 432.123 | 1.409 | 430.496 | 0.215 | 432.123–432.123 | 3.880 | 60181.4 | 637135 | 1 |
| retraction512/slab/ite/witness | faces | 15.371 | 16.282 | 1.381 | 14.714 | 0.186 | 16.282–16.282 | 0.608 | 6631.8 | 41723 | 1 |
| retraction64/slab/ite/active | faces | 23.967 | 24.971 | 2.233 | 22.218 | 0.518 | 24.971–24.971 | 1.232 | 9135.5 | 94990 | 1 |
| retraction64/slab/ite/joint | faces | 24.447 | 431.559 | 2.101 | 428.888 | 0.569 | 431.559–431.559 | 4.990 | 70734.9 | 777405 | 1 |
| retraction64/slab/ite/witness | faces | 24.272 | 25.706 | 2.183 | 22.985 | 0.534 | 25.706–25.706 | 1.171 | 11857.8 | 97465 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 48/48.

## full, width 4, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 0.052 | 0.159 | 0.128 | 0.021 | 0.007 | 0.159–0.159 | 0.131 | 91.4 | 157 | 1 |
| packed512/enum/staged/joint | joint | 0.545 | 0.101 | 0.075 | 0.014 | 0.010 | 0.101–0.101 | 0.029 | 105.0 | 590 | 1 |
| prefix512/slab/ite/active | faces | 0.075 | 0.169 | 0.131 | 0.035 | 0.002 | 0.169–0.169 | 0.042 | 21.6 | 169 | 1 |
| prefix512/slab/ite/joint | faces | 0.155 | 0.449 | 0.333 | 0.102 | 0.012 | 0.449–0.449 | 0.151 | 21.6 | 169 | 1 |
| prefix512/slab/ite/witness | faces | 0.156 | 0.611 | 0.503 | 0.094 | 0.012 | 0.611–0.611 | 0.127 | 21.6 | 169 | 1 |
| retraction512/slab/ite/active | faces | 0.065 | 0.176 | 0.133 | 0.038 | 0.004 | 0.176–0.176 | 0.044 | 21.6 | 169 | 1 |
| retraction512/slab/ite/joint | faces | 0.072 | 0.181 | 0.140 | 0.036 | 0.004 | 0.181–0.181 | 0.043 | 21.6 | 169 | 1 |
| retraction512/slab/ite/witness | faces | 0.068 | 0.176 | 0.135 | 0.038 | 0.002 | 0.176–0.176 | 0.043 | 21.6 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 0.052 | 0.094 | 0.072 | 0.020 | 0.002 | 0.094–0.094 | 0.046 | 100.3 | 157 | 1 |
| packed512/enum/staged/joint | joint | 0.545 | 0.070 | 0.053 | 0.009 | 0.008 | 0.070–0.070 | 0.026 | 114.0 | 590 | 1 |
| prefix512/slab/ite/active | faces | 0.075 | 0.145 | 0.114 | 0.031 | 0.000 | 0.145–0.145 | 0.058 | 30.5 | 169 | 1 |
| prefix512/slab/ite/joint | faces | 0.155 | 0.167 | 0.132 | 0.034 | 0.001 | 0.167–0.167 | 0.042 | 30.5 | 169 | 1 |
| prefix512/slab/ite/witness | faces | 0.156 | 0.161 | 0.127 | 0.034 | 0.000 | 0.161–0.161 | 0.041 | 30.5 | 169 | 1 |
| retraction512/slab/ite/active | faces | 0.065 | 0.147 | 0.116 | 0.031 | 0.000 | 0.147–0.147 | 0.039 | 30.5 | 169 | 1 |
| retraction512/slab/ite/joint | faces | 0.072 | 0.147 | 0.115 | 0.031 | 0.000 | 0.147–0.147 | 0.039 | 30.5 | 169 | 1 |
| retraction512/slab/ite/witness | faces | 0.068 | 0.147 | 0.115 | 0.031 | 0.000 | 0.147–0.147 | 0.039 | 30.5 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 0.025 | 0.065 | 0.055 | 0.008 | 0.001 | 0.065–0.065 | 0.048 | 91.4 | 157 | 1 |
| packed512/enum/staged/joint | joint | 0.021 | 0.052 | 0.040 | 0.007 | 0.004 | 0.052–0.052 | 0.019 | 36.2 | 157 | 1 |
| prefix512/slab/ite/active | faces | 0.066 | 0.169 | 0.130 | 0.036 | 0.002 | 0.169–0.169 | 0.046 | 21.6 | 169 | 1 |
| prefix512/slab/ite/joint | faces | 0.071 | 0.180 | 0.141 | 0.036 | 0.002 | 0.180–0.180 | 0.043 | 21.6 | 169 | 1 |
| prefix512/slab/ite/witness | faces | 0.065 | 0.171 | 0.130 | 0.038 | 0.002 | 0.171–0.171 | 0.043 | 21.6 | 169 | 1 |
| retraction512/slab/ite/active | faces | 0.071 | 0.179 | 0.137 | 0.038 | 0.002 | 0.179–0.179 | 0.048 | 21.6 | 169 | 1 |
| retraction512/slab/ite/joint | faces | 0.073 | 0.212 | 0.170 | 0.039 | 0.002 | 0.212–0.212 | 0.091 | 21.6 | 169 | 1 |
| retraction512/slab/ite/witness | faces | 0.070 | 0.175 | 0.133 | 0.039 | 0.002 | 0.175–0.175 | 0.043 | 21.6 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 4, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 0.025 | 0.042 | 0.033 | 0.008 | 0.001 | 0.042–0.042 | 0.017 | 100.3 | 157 | 1 |
| packed512/enum/staged/joint | joint | 0.021 | 0.035 | 0.029 | 0.004 | 0.003 | 0.035–0.035 | 0.014 | 45.1 | 157 | 1 |
| prefix512/slab/ite/active | faces | 0.066 | 0.149 | 0.118 | 0.031 | 0.000 | 0.149–0.149 | 0.039 | 30.5 | 169 | 1 |
| prefix512/slab/ite/joint | faces | 0.071 | 0.147 | 0.116 | 0.031 | 0.000 | 0.147–0.147 | 0.039 | 30.5 | 169 | 1 |
| prefix512/slab/ite/witness | faces | 0.065 | 0.147 | 0.116 | 0.031 | 0.000 | 0.147–0.147 | 0.039 | 30.5 | 169 | 1 |
| retraction512/slab/ite/active | faces | 0.071 | 0.156 | 0.123 | 0.033 | 0.000 | 0.156–0.156 | 0.042 | 30.5 | 169 | 1 |
| retraction512/slab/ite/joint | faces | 0.073 | 0.158 | 0.124 | 0.034 | 0.000 | 0.158–0.158 | 0.044 | 30.5 | 169 | 1 |
| retraction512/slab/ite/witness | faces | 0.070 | 0.150 | 0.119 | 0.031 | 0.000 | 0.150–0.150 | 0.039 | 30.5 | 169 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 0.729 | 2.267 | 1.683 | 0.548 | 0.036 | 2.267–2.267 | 5.331 | 5351.2 | 161 | 1 |
| packed512/enum/staged/joint | joint | 30.211 | 0.232 | 0.172 | 0.030 | 0.029 | 0.232–0.232 | 0.083 | 367.9 | 2289 | 1 |
| prefix512/slab/ite/active | faces | 0.559 | 0.681 | 0.506 | 0.174 | 0.000 | 0.681–0.681 | 0.190 | 137.4 | 1030 | 1 |
| prefix512/slab/ite/joint | faces | 0.569 | 0.675 | 0.500 | 0.173 | 0.001 | 0.675–0.675 | 0.208 | 137.4 | 1030 | 1 |
| prefix512/slab/ite/witness | faces | 0.604 | 0.680 | 0.502 | 0.176 | 0.001 | 0.680–0.680 | 0.205 | 137.4 | 1030 | 1 |
| retraction512/slab/ite/active | faces | 0.552 | 0.652 | 0.483 | 0.168 | 0.000 | 0.652–0.652 | 0.181 | 137.4 | 1030 | 1 |
| retraction512/slab/ite/joint | faces | 0.570 | 0.672 | 0.488 | 0.183 | 0.000 | 0.672–0.672 | 0.196 | 137.4 | 1030 | 1 |
| retraction512/slab/ite/witness | faces | 0.566 | 0.672 | 0.499 | 0.172 | 0.000 | 0.672–0.672 | 0.195 | 137.4 | 1030 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 0.729 | 10.215 | 9.635 | 0.545 | 0.034 | 10.215–10.215 | 0.592 | 5360.2 | 161 | 1 |
| packed512/enum/staged/joint | joint | 30.211 | 0.219 | 0.166 | 0.027 | 0.026 | 0.219–0.219 | 0.085 | 376.8 | 2289 | 1 |
| prefix512/slab/ite/active | faces | 0.559 | 0.668 | 0.493 | 0.174 | 0.000 | 0.668–0.668 | 0.186 | 146.4 | 1030 | 1 |
| prefix512/slab/ite/joint | faces | 0.569 | 0.704 | 0.531 | 0.172 | 0.001 | 0.704–0.704 | 0.202 | 146.4 | 1030 | 1 |
| prefix512/slab/ite/witness | faces | 0.604 | 0.680 | 0.507 | 0.172 | 0.001 | 0.680–0.680 | 0.193 | 146.4 | 1030 | 1 |
| retraction512/slab/ite/active | faces | 0.552 | 0.683 | 0.513 | 0.169 | 0.000 | 0.683–0.683 | 0.196 | 146.4 | 1030 | 1 |
| retraction512/slab/ite/joint | faces | 0.570 | 0.671 | 0.490 | 0.180 | 0.000 | 0.671–0.671 | 0.187 | 146.4 | 1030 | 1 |
| retraction512/slab/ite/witness | faces | 0.566 | 0.662 | 0.494 | 0.167 | 0.000 | 0.662–0.662 | 0.176 | 146.4 | 1030 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 0.735 | 2.244 | 1.660 | 0.549 | 0.033 | 2.244–2.244 | 2.046 | 5351.2 | 161 | 1 |
| packed512/enum/staged/joint | joint | 0.374 | 0.131 | 0.087 | 0.022 | 0.022 | 0.131–0.131 | 0.060 | 215.9 | 1238 | 1 |
| prefix512/slab/ite/active | faces | 0.532 | 0.839 | 0.624 | 0.214 | 0.000 | 0.839–0.839 | 0.235 | 166.8 | 1254 | 1 |
| prefix512/slab/ite/joint | faces | 0.541 | 0.870 | 0.648 | 0.221 | 0.000 | 0.870–0.870 | 0.242 | 166.8 | 1254 | 1 |
| prefix512/slab/ite/witness | faces | 0.537 | 0.855 | 0.637 | 0.217 | 0.000 | 0.855–0.855 | 0.257 | 166.8 | 1254 | 1 |
| retraction512/slab/ite/active | faces | 0.605 | 0.935 | 0.690 | 0.243 | 0.001 | 0.935–0.935 | 0.238 | 166.8 | 1254 | 1 |
| retraction512/slab/ite/joint | faces | 0.626 | 1.503 | 1.271 | 0.229 | 0.001 | 1.503–1.503 | 0.400 | 166.8 | 1254 | 1 |
| retraction512/slab/ite/witness | faces | 0.541 | 0.860 | 0.635 | 0.225 | 0.000 | 0.860–0.860 | 0.240 | 166.8 | 1254 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## full, width 6, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 0.735 | 1.013 | 0.463 | 0.518 | 0.031 | 1.013–1.013 | 0.557 | 5360.2 | 161 | 1 |
| packed512/enum/staged/joint | joint | 0.374 | 0.146 | 0.111 | 0.019 | 0.015 | 0.146–0.146 | 0.053 | 224.9 | 1238 | 1 |
| prefix512/slab/ite/active | faces | 0.532 | 0.839 | 0.629 | 0.209 | 0.000 | 0.839–0.839 | 0.245 | 175.8 | 1254 | 1 |
| prefix512/slab/ite/joint | faces | 0.541 | 0.856 | 0.639 | 0.215 | 0.001 | 0.856–0.856 | 0.236 | 175.8 | 1254 | 1 |
| prefix512/slab/ite/witness | faces | 0.537 | 0.864 | 0.639 | 0.223 | 0.000 | 0.864–0.864 | 0.231 | 175.8 | 1254 | 1 |
| retraction512/slab/ite/active | faces | 0.605 | 0.840 | 0.630 | 0.209 | 0.000 | 0.840–0.840 | 0.236 | 175.8 | 1254 | 1 |
| retraction512/slab/ite/joint | faces | 0.626 | 1.064 | 0.674 | 0.386 | 0.002 | 1.064–1.064 | 0.583 | 175.8 | 1254 | 1 |
| retraction512/slab/ite/witness | faces | 0.541 | 0.856 | 0.638 | 0.217 | 0.000 | 0.856–0.856 | 0.233 | 175.8 | 1254 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 0.941 | 0.160 | 0.128 | 0.027 | 0.002 | 0.160–0.160 | 0.128 | 115.2 | 200 | 1 |
| packed512/enum/staged/joint | joint | 0.744 | 0.167 | 0.096 | 0.046 | 0.024 | 0.167–0.167 | 0.061 | 290.7 | 1877 | 1 |
| prefix512/slab/ite/active | faces | 0.576 | 0.179 | 0.131 | 0.031 | 0.015 | 0.179–0.179 | 0.072 | 107.8 | 759 | 1 |
| prefix512/slab/ite/joint | faces | 0.599 | 0.189 | 0.136 | 0.033 | 0.018 | 0.189–0.189 | 0.068 | 107.8 | 759 | 1 |
| prefix512/slab/ite/witness | faces | 0.557 | 0.185 | 0.135 | 0.031 | 0.017 | 0.185–0.185 | 0.067 | 107.8 | 759 | 1 |
| retraction512/slab/ite/active | faces | 1.256 | 0.451 | 0.268 | 0.161 | 0.019 | 0.451–0.451 | 0.062 | 103.9 | 751 | 1 |
| retraction512/slab/ite/joint | faces | 0.532 | 1.038 | 0.113 | 0.914 | 0.010 | 1.038–1.038 | 0.295 | 188.3 | 1626 | 1 |
| retraction512/slab/ite/witness | faces | 0.522 | 0.194 | 0.117 | 0.066 | 0.009 | 0.194–0.194 | 0.056 | 103.9 | 751 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 0.941 | 0.094 | 0.070 | 0.022 | 0.002 | 0.094–0.094 | 0.047 | 124.1 | 200 | 1 |
| packed512/enum/staged/joint | joint | 0.744 | 0.134 | 0.086 | 0.029 | 0.019 | 0.134–0.134 | 0.049 | 299.7 | 1877 | 1 |
| prefix512/slab/ite/active | faces | 0.576 | 0.170 | 0.129 | 0.031 | 0.009 | 0.170–0.170 | 0.058 | 116.8 | 759 | 1 |
| prefix512/slab/ite/joint | faces | 0.599 | 0.176 | 0.133 | 0.032 | 0.010 | 0.176–0.176 | 0.187 | 116.8 | 759 | 1 |
| prefix512/slab/ite/witness | faces | 0.557 | 0.192 | 0.141 | 0.036 | 0.010 | 0.192–0.192 | 0.048 | 116.8 | 759 | 1 |
| retraction512/slab/ite/active | faces | 1.256 | 0.187 | 0.116 | 0.065 | 0.005 | 0.187–0.187 | 0.051 | 112.9 | 751 | 1 |
| retraction512/slab/ite/joint | faces | 0.532 | 1.147 | 0.113 | 1.028 | 0.006 | 1.147–1.147 | 0.298 | 197.2 | 1626 | 1 |
| retraction512/slab/ite/witness | faces | 0.522 | 0.182 | 0.113 | 0.065 | 0.005 | 0.182–0.182 | 0.044 | 112.9 | 751 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 0.589 | 0.067 | 0.050 | 0.015 | 0.001 | 0.067–0.067 | 0.049 | 115.2 | 200 | 1 |
| packed512/enum/staged/joint | joint | 0.222 | 0.166 | 0.110 | 0.036 | 0.018 | 0.166–0.166 | 0.042 | 331.7 | 2075 | 1 |
| prefix512/slab/ite/active | faces | 0.578 | 0.179 | 0.130 | 0.032 | 0.016 | 0.179–0.179 | 0.060 | 92.8 | 664 | 1 |
| prefix512/slab/ite/joint | faces | 0.508 | 0.172 | 0.124 | 0.033 | 0.014 | 0.172–0.172 | 0.050 | 92.8 | 664 | 1 |
| prefix512/slab/ite/witness | faces | 0.505 | 0.173 | 0.127 | 0.032 | 0.013 | 0.173–0.173 | 0.052 | 92.8 | 664 | 1 |
| retraction512/slab/ite/active | faces | 0.457 | 0.190 | 0.115 | 0.064 | 0.009 | 0.190–0.190 | 0.051 | 90.3 | 656 | 1 |
| retraction512/slab/ite/joint | faces | 0.462 | 0.681 | 0.115 | 0.551 | 0.013 | 0.681–0.681 | 0.259 | 168.6 | 1080 | 1 |
| retraction512/slab/ite/witness | faces | 0.481 | 0.203 | 0.123 | 0.067 | 0.011 | 0.203–0.203 | 0.054 | 90.3 | 656 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 4, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 0.589 | 0.039 | 0.029 | 0.009 | 0.001 | 0.039–0.039 | 0.017 | 124.1 | 200 | 1 |
| packed512/enum/staged/joint | joint | 0.222 | 0.126 | 0.088 | 0.024 | 0.014 | 0.126–0.126 | 0.037 | 340.7 | 2075 | 1 |
| prefix512/slab/ite/active | faces | 0.578 | 0.157 | 0.119 | 0.029 | 0.008 | 0.157–0.157 | 0.066 | 101.7 | 664 | 1 |
| prefix512/slab/ite/joint | faces | 0.508 | 0.151 | 0.113 | 0.030 | 0.008 | 0.151–0.151 | 0.061 | 101.7 | 664 | 1 |
| prefix512/slab/ite/witness | faces | 0.505 | 0.156 | 0.118 | 0.030 | 0.008 | 0.156–0.156 | 0.046 | 101.7 | 664 | 1 |
| retraction512/slab/ite/active | faces | 0.457 | 0.173 | 0.108 | 0.061 | 0.005 | 0.173–0.173 | 0.043 | 99.2 | 656 | 1 |
| retraction512/slab/ite/joint | faces | 0.462 | 0.624 | 0.109 | 0.510 | 0.005 | 0.624–0.624 | 0.248 | 177.6 | 1080 | 1 |
| retraction512/slab/ite/witness | faces | 0.481 | 0.182 | 0.114 | 0.062 | 0.005 | 0.182–0.182 | 0.052 | 99.2 | 656 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, bit-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 270.406 | 2.353 | 1.749 | 0.569 | 0.035 | 2.353–2.353 | 2.190 | 17043.7 | 517 | 1 |
| packed512/enum/staged/joint | joint | 34.704 | 1.520 | 0.957 | 0.320 | 0.242 | 1.520–1.520 | 0.569 | 5355.9 | 31982 | 1 |
| prefix512/slab/ite/active | faces | 6.805 | 0.798 | 0.513 | 0.157 | 0.127 | 0.798–0.798 | 0.303 | 1431.5 | 9539 | 1 |
| prefix512/slab/ite/joint | faces | 7.242 | 0.841 | 0.547 | 0.162 | 0.132 | 0.841–0.841 | 0.308 | 1431.5 | 9539 | 1 |
| prefix512/slab/ite/witness | faces | 6.644 | 0.803 | 0.521 | 0.152 | 0.130 | 0.803–0.803 | 0.308 | 1431.5 | 9539 | 1 |
| retraction512/slab/ite/active | faces | 7.097 | 1.292 | 0.748 | 0.482 | 0.060 | 1.292–1.292 | 9.025 | 1560.6 | 9815 | 1 |
| retraction512/slab/ite/joint | faces | 6.983 | 10.106 | 0.716 | 9.329 | 0.060 | 10.106–10.106 | 3.076 | 2139.5 | 16955 | 1 |
| retraction512/slab/ite/witness | faces | 6.892 | 1.255 | 0.722 | 0.477 | 0.056 | 1.255–1.255 | 0.365 | 1560.6 | 9815 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, bit-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 270.406 | 1.077 | 0.507 | 0.535 | 0.033 | 1.077–1.077 | 0.586 | 17052.6 | 517 | 1 |
| packed512/enum/staged/joint | joint | 34.704 | 1.469 | 0.915 | 0.312 | 0.241 | 1.469–1.469 | 0.529 | 5364.8 | 31982 | 1 |
| prefix512/slab/ite/active | faces | 6.805 | 0.777 | 0.507 | 0.151 | 0.119 | 0.777–0.777 | 0.292 | 1440.4 | 9539 | 1 |
| prefix512/slab/ite/joint | faces | 7.242 | 0.775 | 0.502 | 0.151 | 0.121 | 0.775–0.775 | 0.291 | 1440.4 | 9539 | 1 |
| prefix512/slab/ite/witness | faces | 6.644 | 0.805 | 0.524 | 0.154 | 0.126 | 0.805–0.805 | 0.402 | 1440.4 | 9539 | 1 |
| retraction512/slab/ite/active | faces | 7.097 | 1.342 | 0.778 | 0.505 | 0.059 | 1.342–1.342 | 0.392 | 1569.6 | 9815 | 1 |
| retraction512/slab/ite/joint | faces | 6.983 | 10.143 | 0.715 | 9.374 | 0.053 | 10.143–10.143 | 3.087 | 2148.4 | 16955 | 1 |
| retraction512/slab/ite/witness | faces | 6.892 | 1.244 | 0.721 | 0.468 | 0.054 | 1.244–1.244 | 0.365 | 1569.6 | 9815 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, face-major, memo False

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 224.600 | 2.186 | 1.621 | 0.532 | 0.032 | 2.186–2.186 | 1.992 | 17043.7 | 517 | 1 |
| packed512/enum/staged/joint | joint | 1.385 | 0.356 | 0.248 | 0.062 | 0.046 | 0.356–0.356 | 0.128 | 1282.0 | 10792 | 1 |
| prefix512/slab/ite/active | faces | 4.454 | 0.979 | 0.638 | 0.208 | 0.133 | 0.979–0.979 | 0.360 | 914.0 | 6457 | 1 |
| prefix512/slab/ite/joint | faces | 4.453 | 0.957 | 0.623 | 0.206 | 0.128 | 0.957–0.957 | 0.351 | 914.0 | 6457 | 1 |
| prefix512/slab/ite/witness | faces | 4.396 | 0.953 | 0.626 | 0.202 | 0.124 | 0.953–0.953 | 0.359 | 914.0 | 6457 | 1 |
| retraction512/slab/ite/active | faces | 4.551 | 1.314 | 0.796 | 0.464 | 0.054 | 1.314–1.314 | 0.353 | 902.0 | 7015 | 1 |
| retraction512/slab/ite/joint | faces | 4.625 | 1.924 | 0.834 | 1.031 | 0.060 | 1.924–1.924 | 0.637 | 1062.4 | 7618 | 1 |
| retraction512/slab/ite/witness | faces | 4.713 | 1.330 | 0.812 | 0.457 | 0.060 | 1.330–1.330 | 0.360 | 902.0 | 7015 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

## holes, width 6, suffix-half/xy, face-major, memo True

| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| dense-dispatched/enum/staged/joint | joint | 224.600 | 1.000 | 0.462 | 0.507 | 0.030 | 1.000–1.000 | 0.549 | 17052.6 | 517 | 1 |
| packed512/enum/staged/joint | joint | 1.385 | 0.343 | 0.233 | 0.058 | 0.048 | 0.343–0.343 | 0.104 | 1291.0 | 10792 | 1 |
| prefix512/slab/ite/active | faces | 4.454 | 1.029 | 0.699 | 0.209 | 0.121 | 1.029–1.029 | 0.352 | 922.9 | 6457 | 1 |
| prefix512/slab/ite/joint | faces | 4.453 | 0.935 | 0.614 | 0.201 | 0.119 | 0.935–0.935 | 0.338 | 922.9 | 6457 | 1 |
| prefix512/slab/ite/witness | faces | 4.396 | 0.979 | 0.658 | 0.202 | 0.119 | 0.979–0.979 | 0.341 | 922.9 | 6457 | 1 |
| retraction512/slab/ite/active | faces | 4.551 | 1.288 | 0.786 | 0.450 | 0.051 | 1.288–1.288 | 0.346 | 910.9 | 7015 | 1 |
| retraction512/slab/ite/joint | faces | 4.625 | 1.864 | 0.819 | 0.990 | 0.054 | 1.864–1.864 | 0.599 | 1071.3 | 7618 | 1 |
| retraction512/slab/ite/witness | faces | 4.713 | 1.345 | 0.825 | 0.465 | 0.055 | 1.345–1.345 | 0.360 | 910.9 | 7015 | 1 |

Groups changed by Possible: 16/16; by Guaranteed: 16/16. Nonconstant readout outputs: 32/48.

264 configurations; 816 matched comparisons; 74 retained processes.

- [Raw evidence](results/projection-first-smoke.json).
- [Raw evidence](results/projection-cutoff-screen.json).
- [Raw evidence](results/projection-normalization-control.json).
- [Raw evidence](results/projection-domain-controls.json).
- [Raw evidence](results/projection-readout-repeat.json).
