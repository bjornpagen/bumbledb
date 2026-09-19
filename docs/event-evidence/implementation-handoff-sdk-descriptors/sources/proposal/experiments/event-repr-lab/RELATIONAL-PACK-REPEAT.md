# Native relational Pack measurements

11-sample medians in milliseconds. Speedup = complete / factorized.
Every pair holds the carrier, legal support, environment, layout and typed program fixed.

| Carrier | Width | Layout | Fanout | Program | Memo | Fresh complete | Fresh factored | Speedup | Warm complete | Warm factored | Speedup |
| --- | ---: | --- | ---: | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| dense | 5 | face-major | 2 | compose | False | 1.3903 | 0.4275 | 3.25× | 1.3497 | 0.3952 | 3.42× |
| dense | 5 | face-major | 2 | compose | True | 0.8115 | 0.3608 | 2.25× | 0.0087 | 0.0193 | 0.45× |
| dense | 5 | face-major | 2 | residual | False | 1.5302 | 0.4747 | 3.22× | 1.4407 | 0.4119 | 3.50× |
| dense | 5 | face-major | 2 | residual | True | 0.9914 | 0.4863 | 2.04× | 0.0091 | 0.0189 | 0.48× |
| dense | 5 | face-major | 8 | compose | False | 21.8396 | 0.8132 | 26.85× | 21.8213 | 0.7996 | 27.29× |
| dense | 5 | face-major | 8 | compose | True | 2.5631 | 0.4482 | 5.72× | 0.0753 | 0.0270 | 2.79× |
| dense | 5 | face-major | 8 | residual | False | 23.5036 | 0.8785 | 26.75× | 23.2337 | 0.8031 | 28.93× |
| dense | 5 | face-major | 8 | residual | True | 4.5441 | 0.5859 | 7.76× | 0.0800 | 0.0271 | 2.95× |
| dense | 5 | bit-major | 2 | compose | False | 1.3947 | 0.4182 | 3.34× | 1.3544 | 0.3824 | 3.54× |
| dense | 5 | bit-major | 2 | compose | True | 0.8055 | 0.3448 | 2.34× | 0.0088 | 0.0192 | 0.46× |
| dense | 5 | bit-major | 2 | residual | False | 1.5775 | 0.4805 | 3.28× | 1.4682 | 0.4121 | 3.56× |
| dense | 5 | bit-major | 2 | residual | True | 0.9511 | 0.4856 | 1.96× | 0.0091 | 0.0193 | 0.47× |
| dense | 5 | bit-major | 8 | compose | False | 21.9240 | 0.8287 | 26.45× | 21.9371 | 0.7855 | 27.93× |
| dense | 5 | bit-major | 8 | compose | True | 2.5234 | 0.4631 | 5.45× | 0.0750 | 0.0268 | 2.80× |
| dense | 5 | bit-major | 8 | residual | False | 23.3667 | 0.8835 | 26.45× | 23.2218 | 0.8201 | 28.32× |
| dense | 5 | bit-major | 8 | residual | True | 4.5132 | 0.6080 | 7.42× | 0.0797 | 0.0272 | 2.93× |
| packed512 | 5 | face-major | 2 | compose | False | 20.8229 | 6.7504 | 3.08× | 16.4406 | 3.9842 | 4.13× |
| packed512 | 5 | face-major | 2 | compose | True | 16.0688 | 6.3490 | 2.53× | 0.0187 | 0.0291 | 0.64× |
| packed512 | 5 | face-major | 2 | residual | False | 11.5843 | 4.1125 | 2.82× | 7.1126 | 1.6310 | 4.36× |
| packed512 | 5 | face-major | 2 | residual | True | 8.8996 | 4.1576 | 2.14× | 0.0190 | 0.0288 | 0.66× |
| packed512 | 5 | face-major | 8 | compose | False | 273.7233 | 1.5788 | 173.38× | 259.6646 | 1.1452 | 226.74× |
| packed512 | 5 | face-major | 8 | compose | True | 39.5024 | 0.5260 | 75.10× | 0.0850 | 0.0372 | 2.29× |
| packed512 | 5 | face-major | 8 | residual | False | 127.6901 | 1.2584 | 101.47× | 113.4840 | 0.7991 | 142.02× |
| packed512 | 5 | face-major | 8 | residual | True | 32.7868 | 0.5810 | 56.43× | 0.0897 | 0.0375 | 2.40× |
| packed512 | 5 | bit-major | 2 | compose | False | 2.6726 | 1.1686 | 2.29× | 1.3975 | 0.4071 | 3.43× |
| packed512 | 5 | bit-major | 2 | compose | True | 1.9004 | 1.0134 | 1.88× | 0.0460 | 0.0569 | 0.81× |
| packed512 | 5 | bit-major | 2 | residual | False | 6.5970 | 2.7855 | 2.37× | 3.2294 | 0.8348 | 3.87× |
| packed512 | 5 | bit-major | 2 | residual | True | 5.1969 | 2.7383 | 1.90× | 0.0507 | 0.0616 | 0.82× |
| packed512 | 5 | bit-major | 8 | compose | False | 25.5861 | 1.2933 | 19.78× | 22.0157 | 0.3677 | 59.88× |
| packed512 | 5 | bit-major | 8 | compose | True | 5.0476 | 0.9950 | 5.07× | 0.1134 | 0.0653 | 1.74× |
| packed512 | 5 | bit-major | 8 | residual | False | 63.3571 | 1.8675 | 33.93× | 49.4539 | 0.3777 | 130.92× |
| packed512 | 5 | bit-major | 8 | residual | True | 19.4183 | 1.6013 | 12.13× | 0.1135 | 0.0633 | 1.79× |

## Measurement boundary

Carriers: dense, packed512. Widths: [5]; layouts: face-major, bit-major; fanouts: [2, 8].
two legal state domains selected by one shared environment; sixteen groups.
This is a bounded structured transition/obligation fixture, not a full Coup simulator.

The complete path evaluates the typed operation for every matching row pair.
The factored path reduces each branch and joins summary rows. Both use actual
Free Join and validate all participating values. Role checks reject Events that
depend on the scratch face. Group presence is separate from relation nonemptiness.

Query timing includes validation, role admission, reduction, root staging, summary
planning/image/COLT construction, final joining and exact output counts. Input
setup and fresh arena cloning are separate; input COLTs are primed. Warm queries
reuse learned roots/memos but reconstruct summaries. Every fresh trial and the
last warm result are checked pointwise against an independent matrix oracle.

Retained Event/memo and COLT estimates are separate. Staged capacity sums root
and resolved-role vectors across groups, including short-lived vectors; it is
not peak memory. Maps, images and planner allocations are additional.

[Raw samples](results/relation-pack-repeat.json) · [Matched values and ranges](results/relation-pack-repeat-analysis.json)
· [Laws, query certificate and implementation](RELATIONAL-PACK.md)
