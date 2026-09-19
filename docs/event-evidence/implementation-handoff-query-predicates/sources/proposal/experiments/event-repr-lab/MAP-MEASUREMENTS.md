# Matched packed-terminal permutation experiment

One binary; one representation per candidate; two selectable renaming algorithms.
All eighty full relation-query outputs are identical and checked against matrices.
Cells are medians of per-process sample medians. Every process has equal weight;
there is no selection of the fastest process. Ratios are descriptive, not confidence intervals.

Inputs: [tail-map-controls.json](results/tail-map-controls.json), [tail-focused-repeat.json](results/tail-focused-repeat.json).

Binary SHA-256: `d09d8ae49f4e8b85c0872e8da16d7dd3144b34fa85b390847c7d1d3f718b7a89`.

## 12 coordinates, fresh arena, outer memo=true

| Candidate | Layout | Recursive (ms) | Local when valid (ms) | Recursive / local | Recursive arena (MB) | Local arena (MB) |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| packed64 | face-major | 0.249 | 0.221 | 1.126 | 0.163 | 0.163 |
| packed64 | bit-major | 0.268 | 0.203 | 1.319 | 0.160 | 0.146 |
| packed64 | pair-major | 0.321 | 0.247 | 1.297 | 0.164 | 0.150 |
| packed256 | face-major | 0.322 | 0.329 | 0.979 | 0.179 | 0.179 |
| packed256 | bit-major | 0.315 | 0.305 | 1.032 | 0.144 | 0.144 |
| packed256 | pair-major | 0.396 | 0.310 | 1.279 | 0.180 | 0.163 |
| packed512 | face-major | 0.495 | 0.477 | 1.039 | 0.292 | 0.292 |
| packed512 | bit-major | 0.458 | 0.252 | 1.820 | 0.197 | 0.086 |
| packed512 | pair-major | 0.467 | 0.903 | 0.518 | 0.246 | 0.246 |
| packed4096 | face-major | 3.001 | 1.790 | 1.677 | 2.248 | 0.153 |
| packed4096 | bit-major | 2.530 | 1.782 | 1.419 | 1.152 | 0.153 |
| packed4096 | pair-major | 2.747 | 1.733 | 1.585 | 1.625 | 0.153 |

## 12 coordinates, fresh arena, outer memo=false

| Candidate | Layout | Recursive (ms) | Local when valid (ms) | Recursive / local | Recursive arena (MB) | Local arena (MB) |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| packed64 | face-major | 0.445 | 0.461 | 0.964 | 0.150 | 0.150 |
| packed64 | bit-major | 0.444 | 0.380 | 1.170 | 0.147 | 0.133 |
| packed64 | pair-major | 0.534 | 0.402 | 1.330 | 0.151 | 0.137 |
| packed256 | face-major | 0.845 | 0.799 | 1.057 | 0.166 | 0.166 |
| packed256 | bit-major | 0.687 | 0.674 | 1.018 | 0.131 | 0.131 |
| packed256 | pair-major | 0.865 | 0.752 | 1.151 | 0.167 | 0.150 |
| packed512 | face-major | 1.474 | 1.379 | 1.069 | 0.279 | 0.279 |
| packed512 | bit-major | 1.221 | 0.812 | 1.504 | 0.184 | 0.073 |
| packed512 | pair-major | 1.374 | 1.605 | 0.856 | 0.233 | 0.233 |
| packed4096 | face-major | 10.820 | 7.470 | 1.448 | 2.235 | 0.140 |
| packed4096 | bit-major | 9.818 | 7.793 | 1.260 | 1.139 | 0.140 |
| packed4096 | pair-major | 10.113 | 7.436 | 1.360 | 1.612 | 0.140 |

## 18 coordinates, fresh arena, outer memo=true

| Candidate | Layout | Recursive (ms) | Local when valid (ms) | Recursive / local | Recursive arena (MB) | Local arena (MB) |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| packed64 | face-major | 2.398 | 2.299 | 1.043 | 1.091 | 1.091 |
| packed64 | bit-major | 1.021 | 0.808 | 1.263 | 0.560 | 0.539 |
| packed64 | pair-major | 1.219 | 1.019 | 1.196 | 0.741 | 0.720 |
| packed256 | face-major | 2.072 | 2.078 | 0.997 | 0.987 | 0.987 |
| packed256 | bit-major | 1.360 | 1.322 | 1.028 | 0.639 | 0.639 |
| packed256 | pair-major | 1.696 | 1.398 | 1.213 | 0.817 | 0.683 |
| packed512 | face-major | 2.308 | 2.267 | 1.018 | 1.195 | 1.195 |
| packed512 | bit-major | 2.012 | 0.767 | 2.622 | 0.801 | 0.510 |
| packed512 | pair-major | 1.980 | 2.225 | 0.890 | 0.858 | 0.858 |
| packed4096 | face-major | 11.247 | 11.216 | 1.003 | 6.960 | 6.960 |
| packed4096 | bit-major | 8.010 | 2.932 | 2.732 | 4.113 | 0.902 |
| packed4096 | pair-major | 8.465 | 2.984 | 2.837 | 4.116 | 0.905 |

## 18 coordinates, fresh arena, outer memo=false

| Candidate | Layout | Recursive (ms) | Local when valid (ms) | Recursive / local | Recursive arena (MB) | Local arena (MB) |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| packed64 | face-major | 4.200 | 3.808 | 1.103 | 1.078 | 1.078 |
| packed64 | bit-major | 1.616 | 1.277 | 1.265 | 0.547 | 0.526 |
| packed64 | pair-major | 1.918 | 1.614 | 1.189 | 0.728 | 0.707 |
| packed256 | face-major | 4.384 | 4.293 | 1.021 | 0.974 | 0.974 |
| packed256 | bit-major | 2.353 | 2.511 | 0.937 | 0.626 | 0.626 |
| packed256 | pair-major | 3.031 | 2.488 | 1.218 | 0.804 | 0.670 |
| packed512 | face-major | 5.501 | 5.469 | 1.006 | 1.182 | 1.182 |
| packed512 | bit-major | 3.841 | 1.510 | 2.544 | 0.788 | 0.497 |
| packed512 | pair-major | 4.000 | 6.175 | 0.648 | 0.845 | 0.845 |
| packed4096 | face-major | 27.934 | 27.988 | 0.998 | 6.947 | 6.947 |
| packed4096 | bit-major | 18.656 | 8.819 | 2.115 | 4.100 | 0.889 |
| packed4096 | pair-major | 19.710 | 8.780 | 2.245 | 4.103 | 0.892 |

The local policy retains recursive rebuilding for maps that cross the terminal cut.
It still pays invariant checking, map preparation, literal construction and public support normalization.
Warm samples, imports, counts, load averages and process RSS remain in the raw inputs.
