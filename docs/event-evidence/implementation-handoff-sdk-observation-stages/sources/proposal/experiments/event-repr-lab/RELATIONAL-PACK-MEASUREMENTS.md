# Native relational Pack measurements

7-sample medians in milliseconds. Speedup = complete / factorized.
Every pair holds the carrier, legal support, environment, layout and typed program fixed.

| Carrier | Width | Layout | Fanout | Program | Memo | Fresh complete | Fresh factored | Speedup | Warm complete | Warm factored | Speedup |
| --- | ---: | --- | ---: | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| dense | 4 | face-major | 2 | compose | False | 0.2002 | 0.0839 | 2.39× | 0.1832 | 0.0675 | 2.71× |
| dense | 4 | face-major | 2 | compose | True | 0.1173 | 0.0687 | 1.71× | 0.0065 | 0.0171 | 0.38× |
| dense | 4 | face-major | 2 | residual | False | 0.2205 | 0.0874 | 2.52× | 0.2011 | 0.0723 | 2.78× |
| dense | 4 | face-major | 2 | residual | True | 0.1490 | 0.0901 | 1.65× | 0.0068 | 0.0167 | 0.41× |
| dense | 4 | face-major | 8 | compose | False | 2.9017 | 0.1667 | 17.41× | 2.9378 | 0.1496 | 19.63× |
| dense | 4 | face-major | 8 | compose | True | 0.4208 | 0.0982 | 4.28× | 0.0749 | 0.0250 | 3.00× |
| dense | 4 | face-major | 8 | residual | False | 3.2880 | 0.1599 | 20.57× | 3.2418 | 0.1399 | 23.17× |
| dense | 4 | face-major | 8 | residual | True | 0.7267 | 0.1127 | 6.45× | 0.0777 | 0.0257 | 3.02× |
| dense | 4 | bit-major | 2 | compose | False | 0.2068 | 0.0808 | 2.56× | 0.1797 | 0.0680 | 2.64× |
| dense | 4 | bit-major | 2 | compose | True | 0.1157 | 0.0663 | 1.74× | 0.0066 | 0.0169 | 0.39× |
| dense | 4 | bit-major | 2 | residual | False | 0.2152 | 0.0889 | 2.42× | 0.2034 | 0.0724 | 2.81× |
| dense | 4 | bit-major | 2 | residual | True | 0.1468 | 0.0893 | 1.64× | 0.0070 | 0.0167 | 0.42× |
| dense | 4 | bit-major | 8 | compose | False | 2.9328 | 0.1712 | 17.13× | 3.0097 | 0.1486 | 20.25× |
| dense | 4 | bit-major | 8 | compose | True | 0.4480 | 0.0968 | 4.63× | 0.0740 | 0.0248 | 2.99× |
| dense | 4 | bit-major | 8 | residual | False | 3.2655 | 0.1510 | 21.63× | 3.2383 | 0.1384 | 23.40× |
| dense | 4 | bit-major | 8 | residual | True | 0.7422 | 0.1180 | 6.29× | 0.0765 | 0.0257 | 2.98× |
| dense | 5 | face-major | 2 | compose | False | 1.3830 | 0.4316 | 3.20× | 1.3469 | 0.3959 | 3.40× |
| dense | 5 | face-major | 2 | compose | True | 0.8195 | 0.3691 | 2.22× | 0.0087 | 0.0190 | 0.46× |
| dense | 5 | face-major | 2 | residual | False | 1.5445 | 0.5000 | 3.09× | 1.4610 | 0.4219 | 3.46× |
| dense | 5 | face-major | 2 | residual | True | 0.9762 | 0.4858 | 2.01× | 0.0090 | 0.0301 | 0.30× |
| dense | 5 | face-major | 8 | compose | False | 21.9524 | 0.8160 | 26.90× | 21.8116 | 0.7949 | 27.44× |
| dense | 5 | face-major | 8 | compose | True | 2.6369 | 0.4557 | 5.79× | 0.0760 | 0.0270 | 2.82× |
| dense | 5 | face-major | 8 | residual | False | 23.2716 | 0.8751 | 26.59× | 23.1628 | 0.8149 | 28.42× |
| dense | 5 | face-major | 8 | residual | True | 4.4245 | 0.5861 | 7.55× | 0.0816 | 0.0272 | 3.00× |
| dense | 5 | bit-major | 2 | compose | False | 1.3797 | 0.4578 | 3.01× | 1.3486 | 0.3978 | 3.39× |
| dense | 5 | bit-major | 2 | compose | True | 0.8225 | 0.3763 | 2.19× | 0.0088 | 0.0192 | 0.46× |
| dense | 5 | bit-major | 2 | residual | False | 1.5339 | 0.4770 | 3.22× | 1.4424 | 0.4128 | 3.49× |
| dense | 5 | bit-major | 2 | residual | True | 0.9610 | 0.4940 | 1.95× | 0.0091 | 0.0191 | 0.47× |
| dense | 5 | bit-major | 8 | compose | False | 21.9106 | 1.0138 | 21.61× | 21.8517 | 0.8074 | 27.06× |
| dense | 5 | bit-major | 8 | compose | True | 2.5735 | 0.4505 | 5.71× | 0.0763 | 0.0274 | 2.79× |
| dense | 5 | bit-major | 8 | residual | False | 23.3396 | 0.9146 | 25.52× | 23.5429 | 0.8189 | 28.75× |
| dense | 5 | bit-major | 8 | residual | True | 4.5412 | 0.6069 | 7.48× | 0.0796 | 0.0271 | 2.93× |
| packed512 | 4 | face-major | 2 | compose | False | 6.4963 | 2.3230 | 2.80× | 5.0542 | 1.4083 | 3.59× |
| packed512 | 4 | face-major | 2 | compose | True | 4.7317 | 2.1261 | 2.23× | 0.0138 | 0.0241 | 0.57× |
| packed512 | 4 | face-major | 2 | residual | False | 6.3169 | 2.1692 | 2.91× | 4.9736 | 1.2353 | 4.03× |
| packed512 | 4 | face-major | 2 | residual | True | 4.2403 | 2.0830 | 2.04× | 0.0141 | 0.0242 | 0.58× |
| packed512 | 4 | face-major | 8 | compose | False | 83.9263 | 1.2190 | 68.85× | 80.4880 | 0.9704 | 82.95× |
| packed512 | 4 | face-major | 8 | compose | True | 10.4949 | 0.3700 | 28.37× | 0.0813 | 0.0329 | 2.47× |
| packed512 | 4 | face-major | 8 | residual | False | 82.9920 | 1.2187 | 68.10× | 79.2840 | 0.8797 | 90.13× |
| packed512 | 4 | face-major | 8 | residual | True | 15.5503 | 0.3923 | 39.64× | 0.0826 | 0.0321 | 2.57× |
| packed512 | 4 | bit-major | 2 | compose | False | 1.3035 | 0.5274 | 2.47× | 0.9497 | 0.2622 | 3.62× |
| packed512 | 4 | bit-major | 2 | compose | True | 0.7435 | 0.5206 | 1.43× | 0.0180 | 0.0278 | 0.65× |
| packed512 | 4 | bit-major | 2 | residual | False | 1.6288 | 0.6188 | 2.63× | 1.1439 | 0.3020 | 3.79× |
| packed512 | 4 | bit-major | 2 | residual | True | 1.0982 | 0.6850 | 1.60× | 0.0154 | 0.0257 | 0.60× |
| packed512 | 4 | bit-major | 8 | compose | False | 16.0092 | 0.5527 | 28.96× | 15.0801 | 0.2598 | 58.05× |
| packed512 | 4 | bit-major | 8 | compose | True | 2.0513 | 0.3156 | 6.50× | 0.0821 | 0.0336 | 2.44× |
| packed512 | 4 | bit-major | 8 | residual | False | 19.7621 | 0.5797 | 34.09× | 17.7576 | 0.2586 | 68.68× |
| packed512 | 4 | bit-major | 8 | residual | True | 4.1453 | 0.3741 | 11.08× | 0.0836 | 0.0338 | 2.47× |
| packed512 | 5 | face-major | 2 | compose | False | 20.9267 | 6.6514 | 3.15× | 16.3330 | 4.0265 | 4.06× |
| packed512 | 5 | face-major | 2 | compose | True | 16.0967 | 6.3678 | 2.53× | 0.0185 | 0.0289 | 0.64× |
| packed512 | 5 | face-major | 2 | residual | False | 11.6551 | 4.0798 | 2.86× | 7.0925 | 1.6433 | 4.32× |
| packed512 | 5 | face-major | 2 | residual | True | 8.9967 | 4.1338 | 2.18× | 0.0192 | 0.0284 | 0.68× |
| packed512 | 5 | face-major | 8 | compose | False | 277.2597 | 1.6205 | 171.10× | 262.4255 | 1.1540 | 227.41× |
| packed512 | 5 | face-major | 8 | compose | True | 39.4980 | 0.5186 | 76.16× | 0.0852 | 0.0372 | 2.29× |
| packed512 | 5 | face-major | 8 | residual | False | 127.2387 | 1.2690 | 100.27× | 111.9958 | 0.7937 | 141.11× |
| packed512 | 5 | face-major | 8 | residual | True | 31.9327 | 0.5760 | 55.44× | 0.0872 | 0.0377 | 2.32× |
| packed512 | 5 | bit-major | 2 | compose | False | 2.7408 | 1.2406 | 2.21× | 1.4236 | 0.4128 | 3.45× |
| packed512 | 5 | bit-major | 2 | compose | True | 1.8588 | 1.0837 | 1.72× | 0.0455 | 0.0565 | 0.81× |
| packed512 | 5 | bit-major | 2 | residual | False | 6.7635 | 2.8539 | 2.37× | 3.1195 | 0.8367 | 3.73× |
| packed512 | 5 | bit-major | 2 | residual | True | 5.1562 | 2.7550 | 1.87× | 0.0501 | 0.0949 | 0.53× |
| packed512 | 5 | bit-major | 8 | compose | False | 26.8207 | 1.3407 | 20.01× | 22.3498 | 0.3769 | 59.30× |
| packed512 | 5 | bit-major | 8 | compose | True | 5.1815 | 0.9945 | 5.21× | 0.1146 | 0.0651 | 1.76× |
| packed512 | 5 | bit-major | 8 | residual | False | 64.8649 | 1.9397 | 33.44× | 51.1302 | 0.3878 | 131.85× |
| packed512 | 5 | bit-major | 8 | residual | True | 19.2993 | 1.5479 | 12.47× | 0.1142 | 0.0630 | 1.81× |
| essential512 | 4 | face-major | 2 | compose | False | 10.6715 | 4.8251 | 2.21× | 2.1042 | 0.5801 | 3.63× |
| essential512 | 4 | face-major | 2 | compose | True | 9.9328 | 4.7990 | 2.07× | 0.0064 | 0.0163 | 0.39× |
| essential512 | 4 | face-major | 2 | residual | False | 9.4640 | 5.0690 | 1.87× | 1.8127 | 0.5368 | 3.38× |
| essential512 | 4 | face-major | 2 | residual | True | 8.8688 | 5.0721 | 1.75× | 0.0067 | 0.0166 | 0.40× |
| essential512 | 4 | face-major | 8 | compose | False | 53.5808 | 2.0150 | 26.59× | 32.9475 | 0.4158 | 79.24× |
| essential512 | 4 | face-major | 8 | compose | True | 23.7846 | 1.7638 | 13.48× | 0.0729 | 0.0258 | 2.83× |
| essential512 | 4 | face-major | 8 | residual | False | 49.3495 | 2.3085 | 21.38× | 28.8606 | 0.4214 | 68.49× |
| essential512 | 4 | face-major | 8 | residual | True | 26.2940 | 2.0656 | 12.73× | 0.0756 | 0.0255 | 2.96× |
| essential512 | 4 | bit-major | 2 | compose | False | 4.4939 | 2.4295 | 1.85× | 1.0306 | 0.2773 | 3.72× |
| essential512 | 4 | bit-major | 2 | compose | True | 4.0451 | 2.3255 | 1.74× | 0.0063 | 0.0167 | 0.38× |
| essential512 | 4 | bit-major | 2 | residual | False | 6.0845 | 3.2401 | 1.88× | 1.2891 | 0.3365 | 3.83× |
| essential512 | 4 | bit-major | 2 | residual | True | 5.4113 | 3.2679 | 1.66× | 0.0065 | 0.0166 | 0.39× |
| essential512 | 4 | bit-major | 8 | compose | False | 157.0109 | 2.3057 | 68.10× | 109.5998 | 0.2695 | 406.74× |
| essential512 | 4 | bit-major | 8 | compose | True | 11.0987 | 2.2135 | 5.01× | 0.1931 | 0.0696 | 2.77× |
| essential512 | 4 | bit-major | 8 | residual | False | 136.2366 | 3.0890 | 44.10× | 32.7080 | 0.2717 | 120.36× |
| essential512 | 4 | bit-major | 8 | residual | True | 74.6309 | 3.0349 | 24.59× | 0.0776 | 0.0260 | 2.98× |
| essential512 | 5 | face-major | 2 | compose | False | 36.8415 | 18.3384 | 2.01× | 6.9778 | 1.9551 | 3.57× |
| essential512 | 5 | face-major | 2 | compose | True | 34.4168 | 18.3813 | 1.87× | 0.0064 | 0.0168 | 0.38× |
| essential512 | 5 | face-major | 2 | residual | False | 34.2643 | 18.1884 | 1.88× | 5.4917 | 1.6217 | 3.39× |
| essential512 | 5 | face-major | 2 | residual | True | 32.0568 | 17.4142 | 1.84× | 0.0071 | 0.0170 | 0.42× |
| essential512 | 5 | face-major | 8 | compose | False | 175.2283 | 3.0908 | 56.69× | 112.0642 | 0.6326 | 177.16× |
| essential512 | 5 | face-major | 8 | compose | True | 73.3698 | 2.7225 | 26.95× | 0.0740 | 0.0255 | 2.91× |
| essential512 | 5 | face-major | 8 | residual | False | 171.7832 | 3.6881 | 46.58× | 88.4267 | 0.8786 | 100.64× |
| essential512 | 5 | face-major | 8 | residual | True | 103.4822 | 3.4104 | 30.34× | 0.0785 | 0.0264 | 2.98× |
| essential512 | 5 | bit-major | 2 | compose | False | 12.4299 | 6.7239 | 1.85× | 2.4877 | 0.6595 | 3.77× |
| essential512 | 5 | bit-major | 2 | compose | True | 11.4761 | 6.6778 | 1.72× | 0.0064 | 0.0166 | 0.38× |
| essential512 | 5 | bit-major | 2 | residual | False | 35.0032 | 17.6327 | 1.99× | 6.8433 | 1.7390 | 3.94× |
| essential512 | 5 | bit-major | 2 | residual | True | 31.8022 | 17.7131 | 1.80× | 0.0070 | 0.0166 | 0.42× |
| essential512 | 5 | bit-major | 8 | compose | False | 67.5563 | 6.3063 | 10.71× | 40.0046 | 0.5198 | 76.96× |
| essential512 | 5 | bit-major | 8 | compose | True | 31.2210 | 6.0043 | 5.20× | 0.0741 | 0.0252 | 2.94× |
| essential512 | 5 | bit-major | 8 | residual | False | 210.1711 | 13.1485 | 15.98× | 112.2680 | 0.7597 | 147.78× |
| essential512 | 5 | bit-major | 8 | residual | True | 112.0940 | 12.7015 | 8.83× | 0.0768 | 0.0254 | 3.02× |
| retraction512 | 4 | face-major | 2 | compose | False | 2.5818 | 0.8116 | 3.18× | 1.6615 | 0.4451 | 3.73× |
| retraction512 | 4 | face-major | 2 | compose | True | 2.1506 | 0.8159 | 2.64× | 0.0092 | 0.0196 | 0.47× |
| retraction512 | 4 | face-major | 2 | residual | False | 1.7793 | 0.7028 | 2.53× | 1.2025 | 0.3715 | 3.24× |
| retraction512 | 4 | face-major | 2 | residual | True | 1.4387 | 0.6865 | 2.10× | 0.0095 | 0.0194 | 0.49× |
| retraction512 | 4 | face-major | 8 | compose | False | 28.7240 | 0.4471 | 64.24× | 26.4937 | 0.2393 | 110.70× |
| retraction512 | 4 | face-major | 8 | compose | True | 5.6328 | 0.3514 | 16.03× | 0.0751 | 0.0275 | 2.73× |
| retraction512 | 4 | face-major | 8 | residual | False | 20.8283 | 0.4707 | 44.25× | 18.8564 | 0.2466 | 76.47× |
| retraction512 | 4 | face-major | 8 | residual | True | 6.2698 | 0.4020 | 15.60× | 0.0781 | 0.0279 | 2.80× |
| retraction512 | 4 | bit-major | 2 | compose | False | 1.8933 | 0.7065 | 2.68× | 1.0122 | 0.3214 | 3.15× |
| retraction512 | 4 | bit-major | 2 | compose | True | 1.6193 | 0.7767 | 2.08× | 0.0091 | 0.0193 | 0.47× |
| retraction512 | 4 | bit-major | 2 | residual | False | 1.6711 | 0.6159 | 2.71× | 0.8521 | 0.2471 | 3.45× |
| retraction512 | 4 | bit-major | 2 | residual | True | 1.4342 | 0.6147 | 2.33× | 0.0092 | 0.0188 | 0.49× |
| retraction512 | 4 | bit-major | 8 | compose | False | 18.6251 | 0.4889 | 38.10× | 16.7984 | 0.2258 | 74.40× |
| retraction512 | 4 | bit-major | 8 | compose | True | 4.3357 | 0.3710 | 11.69× | 0.0757 | 0.0276 | 2.74× |
| retraction512 | 4 | bit-major | 8 | residual | False | 16.4364 | 0.5242 | 31.35× | 13.7156 | 0.2232 | 61.44× |
| retraction512 | 4 | bit-major | 8 | residual | True | 5.9420 | 0.3955 | 15.03× | 0.0793 | 0.0280 | 2.83× |
| retraction512 | 5 | face-major | 2 | compose | False | 7.9731 | 3.0543 | 2.61× | 5.4315 | 1.5941 | 3.41× |
| retraction512 | 5 | face-major | 2 | compose | True | 6.4306 | 3.0263 | 2.12× | 0.0179 | 0.0282 | 0.63× |
| retraction512 | 5 | face-major | 2 | residual | False | 6.3507 | 2.3217 | 2.74× | 4.7444 | 1.3717 | 3.46× |
| retraction512 | 5 | face-major | 2 | residual | True | 5.0160 | 2.4472 | 2.05× | 0.0180 | 0.0283 | 0.64× |
| retraction512 | 5 | face-major | 8 | compose | False | 96.5846 | 1.4822 | 65.16× | 90.8939 | 0.5603 | 162.21× |
| retraction512 | 5 | face-major | 8 | compose | True | 16.9381 | 1.2336 | 13.73× | 0.0871 | 0.0366 | 2.38× |
| retraction512 | 5 | face-major | 8 | residual | False | 83.8139 | 1.8553 | 45.18× | 79.0132 | 0.7930 | 99.64× |
| retraction512 | 5 | face-major | 8 | residual | True | 23.2601 | 1.5475 | 15.03× | 0.0897 | 0.0361 | 2.49× |
| retraction512 | 5 | bit-major | 2 | compose | False | 5.3712 | 2.4740 | 2.17× | 3.0551 | 1.0493 | 2.91× |
| retraction512 | 5 | bit-major | 2 | compose | True | 4.4516 | 2.3720 | 1.88× | 0.0180 | 0.0284 | 0.64× |
| retraction512 | 5 | bit-major | 2 | residual | False | 8.6049 | 3.4254 | 2.51× | 4.8354 | 1.4404 | 3.36× |
| retraction512 | 5 | bit-major | 2 | residual | True | 7.1676 | 3.3080 | 2.17× | 0.0181 | 0.0278 | 0.65× |
| retraction512 | 5 | bit-major | 8 | compose | False | 87.6959 | 2.4167 | 36.29× | 60.2824 | 1.2668 | 47.59× |
| retraction512 | 5 | bit-major | 8 | compose | True | 14.6999 | 1.9083 | 7.70× | 0.0972 | 0.0362 | 2.68× |
| retraction512 | 5 | bit-major | 8 | residual | False | 96.9417 | 2.6379 | 36.75× | 81.0928 | 1.5839 | 51.20× |
| retraction512 | 5 | bit-major | 8 | residual | True | 33.1645 | 2.1923 | 15.13× | 0.0898 | 0.0366 | 2.45× |
| range-shannon | 4 | face-major | 2 | compose | False | 3.9128 | 2.3562 | 1.66× | 0.8387 | 0.2683 | 3.13× |
| range-shannon | 4 | face-major | 2 | compose | True | 3.4461 | 2.3313 | 1.48× | 0.0325 | 0.0435 | 0.75× |
| range-shannon | 4 | face-major | 2 | residual | False | 2.6999 | 1.5945 | 1.69× | 0.4176 | 0.1382 | 3.02× |
| range-shannon | 4 | face-major | 2 | residual | True | 2.4012 | 1.6355 | 1.47× | 0.0243 | 0.0345 | 0.70× |
| range-shannon | 4 | face-major | 8 | compose | False | 19.5759 | 0.9344 | 20.95× | 13.4861 | 0.1090 | 123.76× |
| range-shannon | 4 | face-major | 8 | compose | True | 7.6562 | 0.8185 | 9.35× | 0.0880 | 0.0388 | 2.26× |
| range-shannon | 4 | face-major | 8 | residual | False | 11.9842 | 0.9100 | 13.17× | 6.6864 | 0.1001 | 66.80× |
| range-shannon | 4 | face-major | 8 | residual | True | 6.1135 | 0.8681 | 7.04× | 0.0902 | 0.0397 | 2.27× |
| range-shannon | 4 | bit-major | 2 | compose | False | 4.7584 | 2.5582 | 1.86× | 1.2890 | 0.4716 | 2.73× |
| range-shannon | 4 | bit-major | 2 | compose | True | 4.0396 | 2.5396 | 1.59× | 0.0981 | 0.1077 | 0.91× |
| range-shannon | 4 | bit-major | 2 | residual | False | 4.5866 | 2.4122 | 1.90× | 1.8953 | 0.5252 | 3.61× |
| range-shannon | 4 | bit-major | 2 | residual | True | 3.6236 | 2.3338 | 1.55× | 0.0620 | 0.0719 | 0.86× |
| range-shannon | 4 | bit-major | 8 | compose | False | 25.5807 | 1.7450 | 14.66× | 18.8687 | 0.3051 | 61.85× |
| range-shannon | 4 | bit-major | 8 | compose | True | 8.0042 | 1.4506 | 5.52× | 0.1158 | 0.0672 | 1.72× |
| range-shannon | 4 | bit-major | 8 | residual | False | 37.2277 | 1.9080 | 19.51× | 29.7689 | 0.3020 | 98.56× |
| range-shannon | 4 | bit-major | 8 | residual | True | 10.5081 | 1.6784 | 6.26× | 0.1183 | 0.0695 | 1.70× |
| range-shannon | 5 | face-major | 2 | compose | False | 10.2320 | 6.5279 | 1.57× | 2.2533 | 0.6343 | 3.55× |
| range-shannon | 5 | face-major | 2 | compose | True | 9.6882 | 6.2086 | 1.56× | 0.0647 | 0.0722 | 0.90× |
| range-shannon | 5 | face-major | 2 | residual | False | 8.5800 | 5.3535 | 1.60× | 0.9534 | 0.2913 | 3.27× |
| range-shannon | 5 | face-major | 2 | residual | True | 8.2835 | 5.3090 | 1.56× | 0.0453 | 0.0559 | 0.81× |
| range-shannon | 5 | face-major | 8 | compose | False | 51.9837 | 1.9987 | 26.01× | 35.3455 | 0.1949 | 181.34× |
| range-shannon | 5 | face-major | 8 | compose | True | 18.9752 | 1.8269 | 10.39× | 0.0971 | 0.0489 | 1.99× |
| range-shannon | 5 | face-major | 8 | residual | False | 36.5564 | 2.0641 | 17.71× | 14.6909 | 0.1538 | 95.54× |
| range-shannon | 5 | face-major | 8 | residual | True | 22.6468 | 1.8570 | 12.20× | 0.1002 | 0.0490 | 2.04× |
| range-shannon | 5 | bit-major | 2 | compose | False | 8.9352 | 4.6528 | 1.92× | 2.3285 | 0.8417 | 2.77× |
| range-shannon | 5 | bit-major | 2 | compose | True | 8.1438 | 4.3587 | 1.87× | 0.1873 | 0.2022 | 0.93× |
| range-shannon | 5 | bit-major | 2 | residual | False | 17.8201 | 9.3849 | 1.90× | 6.0428 | 1.6766 | 3.60× |
| range-shannon | 5 | bit-major | 2 | residual | True | 15.1329 | 9.3496 | 1.62× | 0.2646 | 0.2742 | 0.97× |
| range-shannon | 5 | bit-major | 8 | compose | False | 47.0252 | 3.3299 | 14.12× | 32.3005 | 0.4606 | 70.12× |
| range-shannon | 5 | bit-major | 8 | compose | True | 16.7298 | 2.8896 | 5.79× | 0.1524 | 0.1035 | 1.47× |
| range-shannon | 5 | bit-major | 8 | residual | False | 133.0691 | 5.0170 | 26.52× | 95.1986 | 0.4520 | 210.61× |
| range-shannon | 5 | bit-major | 8 | residual | True | 46.7576 | 4.5550 | 10.27× | 0.1517 | 0.0997 | 1.52× |
| range-group | 4 | face-major | 2 | compose | False | 4.4101 | 2.6041 | 1.69× | 0.8905 | 0.2778 | 3.21× |
| range-group | 4 | face-major | 2 | compose | True | 3.8036 | 2.5460 | 1.49× | 0.0325 | 0.0429 | 0.76× |
| range-group | 4 | face-major | 2 | residual | False | 2.8450 | 1.7603 | 1.62× | 0.4169 | 0.1384 | 3.01× |
| range-group | 4 | face-major | 2 | residual | True | 2.5518 | 1.7093 | 1.49× | 0.0237 | 0.0334 | 0.71× |
| range-group | 4 | face-major | 8 | compose | False | 20.8033 | 1.0447 | 19.91× | 14.2805 | 0.1092 | 130.74× |
| range-group | 4 | face-major | 8 | compose | True | 7.6707 | 0.9315 | 8.23× | 0.0860 | 0.0391 | 2.20× |
| range-group | 4 | face-major | 8 | residual | False | 12.3730 | 1.0044 | 12.32× | 6.7368 | 0.1031 | 65.33× |
| range-group | 4 | face-major | 8 | residual | True | 6.3992 | 0.9360 | 6.84× | 0.0900 | 0.0396 | 2.27× |
| range-group | 4 | bit-major | 2 | compose | False | 5.1418 | 2.8868 | 1.78× | 1.3079 | 0.4831 | 2.71× |
| range-group | 4 | bit-major | 2 | compose | True | 4.5667 | 2.7468 | 1.66× | 0.0983 | 0.1084 | 0.91× |
| range-group | 4 | bit-major | 2 | residual | False | 4.8225 | 2.4776 | 1.95× | 1.9825 | 0.5387 | 3.68× |
| range-group | 4 | bit-major | 2 | residual | True | 3.9921 | 2.5849 | 1.54× | 0.0624 | 0.0742 | 0.84× |
| range-group | 4 | bit-major | 8 | compose | False | 27.1666 | 1.9171 | 14.17× | 19.4460 | 0.3092 | 62.89× |
| range-group | 4 | bit-major | 8 | compose | True | 8.6316 | 1.5753 | 5.48× | 0.1156 | 0.0668 | 1.73× |
| range-group | 4 | bit-major | 8 | residual | False | 38.4250 | 2.1167 | 18.15× | 30.7555 | 0.3089 | 99.58× |
| range-group | 4 | bit-major | 8 | residual | True | 10.9621 | 1.8247 | 6.01× | 0.1195 | 0.0688 | 1.74× |
| range-group | 5 | face-major | 2 | compose | False | 10.9445 | 6.6825 | 1.64× | 2.3866 | 0.6652 | 3.59× |
| range-group | 5 | face-major | 2 | compose | True | 10.0195 | 6.5314 | 1.53× | 0.0587 | 0.0717 | 0.82× |
| range-group | 5 | face-major | 2 | residual | False | 9.2553 | 5.7039 | 1.62× | 0.9524 | 0.2917 | 3.26× |
| range-group | 5 | face-major | 2 | residual | True | 8.8207 | 5.6345 | 1.57× | 0.0434 | 0.0537 | 0.81× |
| range-group | 5 | face-major | 8 | compose | False | 54.6532 | 2.1612 | 25.29× | 37.2715 | 0.1991 | 187.19× |
| range-group | 5 | face-major | 8 | compose | True | 20.2833 | 2.0731 | 9.78× | 0.0990 | 0.0487 | 2.03× |
| range-group | 5 | face-major | 8 | residual | False | 39.5797 | 2.1835 | 18.13× | 15.3775 | 0.1573 | 97.76× |
| range-group | 5 | face-major | 8 | residual | True | 24.6912 | 2.0371 | 12.12× | 0.1015 | 0.0483 | 2.10× |
| range-group | 5 | bit-major | 2 | compose | False | 9.6541 | 4.9537 | 1.95× | 2.2701 | 0.8661 | 2.62× |
| range-group | 5 | bit-major | 2 | compose | True | 8.0196 | 4.7757 | 1.68× | 0.1882 | 0.2075 | 0.91× |
| range-group | 5 | bit-major | 2 | residual | False | 19.0340 | 10.3516 | 1.84× | 6.1422 | 1.7274 | 3.56× |
| range-group | 5 | bit-major | 2 | residual | True | 15.9778 | 9.9247 | 1.61× | 0.2557 | 0.2719 | 0.94× |
| range-group | 5 | bit-major | 8 | compose | False | 49.7675 | 3.6316 | 13.70× | 34.7730 | 0.4720 | 73.67× |
| range-group | 5 | bit-major | 8 | compose | True | 20.0808 | 3.1980 | 6.28× | 0.1534 | 0.1045 | 1.47× |
| range-group | 5 | bit-major | 8 | residual | False | 136.6260 | 5.2691 | 25.93× | 97.1588 | 0.4554 | 213.37× |
| range-group | 5 | bit-major | 8 | residual | True | 49.2410 | 4.8370 | 10.18× | 0.1591 | 0.1028 | 1.55× |

## Measurement boundary

Carriers: dense, packed512, essential512, retraction512, range-shannon, range-group. Widths: [4, 5]; layouts: face-major, bit-major; fanouts: [2, 8].
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

[Raw samples](results/relation-pack-comparison.json) · [Matched values and ranges](results/relation-pack-analysis.json)
· [Laws, query certificate and implementation](RELATIONAL-PACK.md)
