# churn — degradation over cycles

- crate version: 0.1.0
- engine rev: 1e9d39adff5fd82f1ea7bdad0c53238608eb240d
- timestamp: 2026-07-24T15:03:28Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 2.79 2.88 2.99 (start) → 1.35 1.70 1.96 (end)
- config: scale S, seed 1, 10000 cycles, sample every 250, vacuum every 500, analyze every 500

## run steady (churn=64 updates=32 growth=0, working set 100000)

### ours-durable (bumbledb)

| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | freelist | pages |
|---|---|---|---|---|---|---|---|---|
| 250 | churn_point=281 churn_balance=28750 churn_window=20375 | 48.54 | 0 | 80543744 | 294 | 123999 | - | - |
| 500 | churn_point=257 churn_balance=20125 churn_window=20916 | 48.63 | 0 | 80543744 | 544 | 147999 | - | - |
| 750 | churn_point=255 churn_balance=16375 churn_window=20208 | 48.42 | 0 | 80543744 | 794 | 171999 | - | - |
| 1000 | churn_point=255 churn_balance=12500 churn_window=21000 | 46.59 | 0 | 80543744 | 1044 | 195999 | - | - |
| 1250 | churn_point=281 churn_balance=10833 churn_window=20750 | 44.86 | 0 | 80543744 | 1294 | 219999 | - | - |
| 1500 | churn_point=260 churn_balance=8042 churn_window=21333 | 45.18 | 0 | 80543744 | 1544 | 243999 | - | - |
| 1750 | churn_point=276 churn_balance=6584 churn_window=21167 | 48.94 | 0 | 80543744 | 1794 | 267999 | - | - |
| 2000 | churn_point=270 churn_balance=5250 churn_window=21417 | 49.21 | 0 | 80543744 | 2044 | 291999 | - | - |
| 2250 | churn_point=270 churn_balance=4250 churn_window=22667 | 48.80 | 0 | 80543744 | 2294 | 315999 | - | - |
| 2500 | churn_point=250 churn_balance=3250 churn_window=21958 | 48.84 | 0 | 80543744 | 2544 | 339999 | - | - |
| 2750 | churn_point=286 churn_balance=2583 churn_window=22250 | 48.69 | 0 | 80543744 | 2794 | 363999 | - | - |
| 3000 | churn_point=252 churn_balance=2041 churn_window=22750 | 48.88 | 0 | 80543744 | 3044 | 387999 | - | - |
| 3250 | churn_point=276 churn_balance=1792 churn_window=23208 | 48.27 | 0 | 80543744 | 3294 | 411999 | - | - |
| 3500 | churn_point=268 churn_balance=1375 churn_window=22959 | 48.53 | 0 | 80543744 | 3544 | 435999 | - | - |
| 3750 | churn_point=296 churn_balance=1292 churn_window=22833 | 48.46 | 0 | 80543744 | 3794 | 459999 | - | - |
| 4000 | churn_point=244 churn_balance=958 churn_window=21792 | 48.50 | 0 | 80543744 | 4044 | 483999 | - | - |
| 4250 | churn_point=270 churn_balance=875 churn_window=22667 | 48.41 | 0 | 80543744 | 4294 | 507999 | - | - |
| 4500 | churn_point=276 churn_balance=708 churn_window=21792 | 48.08 | 0 | 80543744 | 4544 | 531999 | - | - |
| 4750 | churn_point=278 churn_balance=625 churn_window=21625 | 48.41 | 0 | 80543744 | 4794 | 555999 | - | - |
| 5000 | churn_point=263 churn_balance=583 churn_window=21834 | 48.58 | 0 | 80543744 | 5044 | 579999 | - | - |
| 5250 | churn_point=257 churn_balance=542 churn_window=22708 | 48.91 | 0 | 80543744 | 5294 | 603999 | - | - |
| 5500 | churn_point=260 churn_balance=440 churn_window=22292 | 48.47 | 0 | 80543744 | 5544 | 627999 | - | - |
| 5750 | churn_point=270 churn_balance=432 churn_window=23625 | 48.63 | 0 | 80543744 | 5794 | 651999 | - | - |
| 6000 | churn_point=286 churn_balance=401 churn_window=23250 | 48.07 | 0 | 80543744 | 6044 | 675999 | - | - |
| 6250 | churn_point=304 churn_balance=414 churn_window=23625 | 44.64 | 0 | 80543744 | 6294 | 699999 | - | - |
| 6500 | churn_point=255 churn_balance=356 churn_window=22917 | 47.83 | 0 | 80543744 | 6544 | 723999 | - | - |
| 6750 | churn_point=276 churn_balance=364 churn_window=24083 | 48.27 | 0 | 80543744 | 6794 | 747999 | - | - |
| 7000 | churn_point=268 churn_balance=341 churn_window=22917 | 48.65 | 0 | 80543744 | 7044 | 771999 | - | - |
| 7250 | churn_point=281 churn_balance=348 churn_window=23500 | 48.73 | 0 | 80543744 | 7294 | 795999 | - | - |
| 7500 | churn_point=250 churn_balance=330 churn_window=22750 | 48.34 | 0 | 80543744 | 7544 | 819999 | - | - |
| 7750 | churn_point=289 churn_balance=359 churn_window=23375 | 48.62 | 0 | 80543744 | 7794 | 843999 | - | - |
| 8000 | churn_point=265 churn_balance=315 churn_window=22000 | 48.35 | 0 | 80543744 | 8044 | 867999 | - | - |
| 8250 | churn_point=273 churn_balance=317 churn_window=22541 | 48.48 | 0 | 80543744 | 8294 | 891999 | - | - |
| 8500 | churn_point=268 churn_balance=307 churn_window=21625 | 48.52 | 0 | 80543744 | 8544 | 915999 | - | - |
| 8750 | churn_point=299 churn_balance=346 churn_window=23958 | 48.21 | 0 | 80543744 | 8794 | 939999 | - | - |
| 9000 | churn_point=273 churn_balance=302 churn_window=22209 | 48.87 | 0 | 80543744 | 9044 | 963999 | - | - |
| 9250 | churn_point=309 churn_balance=346 churn_window=23459 | 48.33 | 0 | 80543744 | 9294 | 987999 | - | - |
| 9500 | churn_point=250 churn_balance=307 churn_window=21667 | 48.80 | 0 | 80543744 | 9544 | 1011999 | - | - |
| 9750 | churn_point=289 churn_balance=335 churn_window=22709 | 48.18 | 0 | 80543744 | 9794 | 1035999 | - | - |
| 10000 | churn_point=270 churn_balance=315 churn_window=22291 | 48.79 | 0 | 80543744 | 10044 | 1059999 | - | - |

### sqlite-bare (sqlite)

| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | freelist | pages |
|---|---|---|---|---|---|---|---|---|
| 250 | churn_point=1209 churn_balance=5340125 churn_window=278834 | 55.44 | 0 | 14774272 | - | - | 0 | 3607 |
| 500 | churn_point=1125 churn_balance=4489083 churn_window=380625 | 53.14 | 0 | 15884288 | - | - | 0 | 3878 |
| 750 | churn_point=1166 churn_balance=3648458 churn_window=406500 | 49.66 | 0 | 16924672 | - | - | 0 | 4132 |
| 1000 | churn_point=1083 churn_balance=2953959 churn_window=449625 | 46.81 | 0 | 17383424 | - | - | 45 | 4244 |
| 1250 | churn_point=1291 churn_balance=2382584 churn_window=467209 | 47.84 | 0 | 17383424 | - | - | 193 | 4244 |
| 1500 | churn_point=1292 churn_balance=1903666 churn_window=487458 | 47.12 | 0 | 17383424 | - | - | 161 | 4244 |
| 1750 | churn_point=1125 churn_balance=1556334 churn_window=503542 | 47.29 | 0 | 17383424 | - | - | 204 | 4244 |
| 2000 | churn_point=1083 churn_balance=1238292 churn_window=536708 | 48.00 | 0 | 17383424 | - | - | 205 | 4244 |
| 2250 | churn_point=1125 churn_balance=1028583 churn_window=533875 | 48.38 | 0 | 17383424 | - | - | 209 | 4244 |
| 2500 | churn_point=1084 churn_balance=805209 churn_window=545875 | 49.19 | 0 | 17383424 | - | - | 214 | 4244 |
| 2750 | churn_point=1125 churn_balance=654250 churn_window=561166 | 48.27 | 0 | 17383424 | - | - | 248 | 4244 |
| 3000 | churn_point=1084 churn_balance=518167 churn_window=554333 | 50.46 | 0 | 17383424 | - | - | 239 | 4244 |
| 3250 | churn_point=1167 churn_balance=456333 churn_window=559916 | 49.73 | 0 | 17383424 | - | - | 236 | 4244 |
| 3500 | churn_point=1125 churn_balance=342666 churn_window=562292 | 49.03 | 0 | 17383424 | - | - | 237 | 4244 |
| 3750 | churn_point=1250 churn_balance=303792 churn_window=559667 | 49.78 | 0 | 17383424 | - | - | 236 | 4244 |
| 4000 | churn_point=1125 churn_balance=236250 churn_window=549708 | 49.69 | 0 | 17383424 | - | - | 254 | 4244 |
| 4250 | churn_point=1208 churn_balance=200583 churn_window=550375 | 49.12 | 0 | 17383424 | - | - | 244 | 4244 |
| 4500 | churn_point=1083 churn_balance=158084 churn_window=545625 | 49.31 | 0 | 17383424 | - | - | 243 | 4244 |
| 4750 | churn_point=1125 churn_balance=140084 churn_window=538834 | 48.82 | 0 | 17383424 | - | - | 236 | 4244 |
| 5000 | churn_point=1375 churn_balance=113708 churn_window=544000 | 50.66 | 0 | 17383424 | - | - | 240 | 4244 |
| 5250 | churn_point=1125 churn_balance=106208 churn_window=553084 | 49.75 | 0 | 17383424 | - | - | 244 | 4244 |
| 5500 | churn_point=1125 churn_balance=93625 churn_window=562625 | 49.01 | 0 | 17383424 | - | - | 226 | 4244 |
| 5750 | churn_point=1208 churn_balance=84083 churn_window=569083 | 50.49 | 0 | 17383424 | - | - | 234 | 4244 |
| 6000 | churn_point=1125 churn_balance=75916 churn_window=589417 | 48.98 | 0 | 17383424 | - | - | 239 | 4244 |
| 6250 | churn_point=1208 churn_balance=78958 churn_window=574583 | 43.70 | 0 | 17383424 | - | - | 229 | 4244 |
| 6500 | churn_point=1125 churn_balance=65667 churn_window=571875 | 49.59 | 0 | 17383424 | - | - | 216 | 4244 |
| 6750 | churn_point=1167 churn_balance=67459 churn_window=588666 | 48.90 | 0 | 17383424 | - | - | 229 | 4244 |
| 7000 | churn_point=1084 churn_balance=59625 churn_window=570875 | 50.09 | 0 | 17383424 | - | - | 217 | 4244 |
| 7250 | churn_point=1208 churn_balance=61750 churn_window=584208 | 49.06 | 0 | 17383424 | - | - | 226 | 4244 |
| 7500 | churn_point=1084 churn_balance=57250 churn_window=550041 | 49.52 | 0 | 17383424 | - | - | 226 | 4244 |
| 7750 | churn_point=1208 churn_balance=61125 churn_window=570500 | 49.48 | 0 | 17383424 | - | - | 232 | 4244 |
| 8000 | churn_point=1084 churn_balance=53583 churn_window=553167 | 48.69 | 0 | 17383424 | - | - | 222 | 4244 |
| 8250 | churn_point=1125 churn_balance=54792 churn_window=555458 | 49.79 | 0 | 17383424 | - | - | 221 | 4244 |
| 8500 | churn_point=1083 churn_balance=51708 churn_window=547958 | 48.73 | 0 | 17383424 | - | - | 222 | 4244 |
| 8750 | churn_point=1250 churn_balance=55708 churn_window=586250 | 48.88 | 0 | 17383424 | - | - | 219 | 4244 |
| 9000 | churn_point=1084 churn_balance=50875 churn_window=541500 | 49.24 | 0 | 17383424 | - | - | 220 | 4244 |
| 9250 | churn_point=1250 churn_balance=60209 churn_window=580916 | 49.75 | 0 | 17383424 | - | - | 214 | 4244 |
| 9500 | churn_point=1084 churn_balance=51583 churn_window=532792 | 49.45 | 0 | 17383424 | - | - | 232 | 4244 |
| 9750 | churn_point=1250 churn_balance=55125 churn_window=558333 | 49.91 | 0 | 17383424 | - | - | 221 | 4244 |
| 10000 | churn_point=1084 churn_balance=52208 churn_window=545209 | 51.16 | 0 | 17383424 | - | - | 211 | 4244 |

### sqlite-maint (sqlite)

| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | freelist | pages |
|---|---|---|---|---|---|---|---|---|
| 250 | churn_point=1208 churn_balance=5329541 churn_window=277459 | 54.81 | 0 | 14774272 | - | - | 0 | 3607 |
| 500 | churn_point=1084 churn_balance=4488833 churn_window=643791 | 51.38 | 110614417 | 12832768 | - | - | 0 | 3133 |
| 750 | churn_point=1084 churn_balance=3651334 churn_window=664958 | 52.74 | 0 | 14778368 | - | - | 0 | 3608 |
| 1000 | churn_point=1083 churn_balance=2994458 churn_window=439708 | 48.81 | 108634624 | 12963840 | - | - | 0 | 3165 |
| 1250 | churn_point=1250 churn_balance=2374875 churn_window=463916 | 49.52 | 0 | 15093760 | - | - | 0 | 3685 |
| 1500 | churn_point=1125 churn_balance=1910625 churn_window=721167 | 47.74 | 104007374 | 13049856 | - | - | 0 | 3186 |
| 1750 | churn_point=1083 churn_balance=1555625 churn_window=745375 | 48.65 | 0 | 15175680 | - | - | 0 | 3705 |
| 2000 | churn_point=1084 churn_balance=1236208 churn_window=755625 | 48.55 | 110410750 | 13103104 | - | - | 0 | 3199 |
| 2250 | churn_point=1125 churn_balance=1019667 churn_window=771000 | 48.64 | 0 | 15179776 | - | - | 0 | 3706 |
| 2500 | churn_point=1084 churn_balance=793000 churn_window=544750 | 48.63 | 109708459 | 13135872 | - | - | 0 | 3207 |
| 2750 | churn_point=1084 churn_balance=661834 churn_window=554375 | 47.73 | 0 | 15278080 | - | - | 0 | 3730 |
| 3000 | churn_point=1084 churn_balance=515458 churn_window=783709 | 48.08 | 106218417 | 13152256 | - | - | 0 | 3211 |
| 3250 | churn_point=1167 churn_balance=437375 churn_window=796458 | 47.22 | 0 | 15241216 | - | - | 0 | 3721 |
| 3500 | churn_point=1084 churn_balance=339500 churn_window=558334 | 48.79 | 104005083 | 13164544 | - | - | 0 | 3214 |
| 3750 | churn_point=1208 churn_balance=301459 churn_window=550708 | 46.66 | 0 | 15302656 | - | - | 0 | 3736 |
| 4000 | churn_point=1125 churn_balance=229375 churn_window=547875 | 47.43 | 113729208 | 13168640 | - | - | 0 | 3215 |
| 4250 | churn_point=1208 churn_balance=199792 churn_window=550125 | 45.91 | 0 | 15392768 | - | - | 0 | 3758 |
| 4500 | churn_point=1083 churn_balance=157084 churn_window=784708 | 46.37 | 111607125 | 13176832 | - | - | 0 | 3217 |
| 4750 | churn_point=1125 churn_balance=139250 churn_window=780084 | 45.95 | 0 | 15376384 | - | - | 0 | 3754 |
| 5000 | churn_point=1125 churn_balance=112958 churn_window=539834 | 47.53 | 106624125 | 13180928 | - | - | 0 | 3218 |
| 5250 | churn_point=1125 churn_balance=104542 churn_window=545042 | 47.49 | 0 | 15351808 | - | - | 0 | 3748 |
| 5500 | churn_point=1083 churn_balance=87666 churn_window=780667 | 47.77 | 111098416 | 13180928 | - | - | 0 | 3218 |
| 5750 | churn_point=1167 churn_balance=82125 churn_window=795917 | 47.08 | 0 | 15388672 | - | - | 0 | 3757 |
| 6000 | churn_point=1167 churn_balance=74500 churn_window=582208 | 47.14 | 118123209 | 13180928 | - | - | 0 | 3218 |
| 6250 | churn_point=1167 churn_balance=77583 churn_window=567000 | 40.75 | 0 | 15376384 | - | - | 0 | 3754 |
| 6500 | churn_point=1083 churn_balance=65208 churn_window=569750 | 46.95 | 106002625 | 13185024 | - | - | 0 | 3219 |
| 6750 | churn_point=1167 churn_balance=66792 churn_window=587500 | 46.31 | 0 | 15355904 | - | - | 0 | 3749 |
| 7000 | churn_point=1083 churn_balance=57875 churn_window=572750 | 48.14 | 103889959 | 13180928 | - | - | 0 | 3218 |
| 7250 | churn_point=1167 churn_balance=60500 churn_window=564709 | 47.68 | 0 | 15339520 | - | - | 0 | 3745 |
| 7500 | churn_point=1084 churn_balance=56458 churn_window=792084 | 47.30 | 107109000 | 13185024 | - | - | 0 | 3219 |
| 7750 | churn_point=1167 churn_balance=61042 churn_window=781583 | 46.32 | 0 | 15400960 | - | - | 0 | 3760 |
| 8000 | churn_point=1083 churn_balance=60917 churn_window=550292 | 46.80 | 107542458 | 13180928 | - | - | 0 | 3218 |
| 8250 | churn_point=1125 churn_balance=59375 churn_window=545750 | 46.99 | 0 | 15343616 | - | - | 0 | 3746 |
| 8500 | churn_point=1083 churn_balance=58792 churn_window=563458 | 46.44 | 107779208 | 13180928 | - | - | 0 | 3218 |
| 8750 | churn_point=1250 churn_balance=60458 churn_window=570917 | 46.39 | 0 | 15405056 | - | - | 0 | 3761 |
| 9000 | churn_point=1083 churn_balance=49167 churn_window=540375 | 46.37 | 112053625 | 13180928 | - | - | 0 | 3218 |
| 9250 | churn_point=1208 churn_balance=55375 churn_window=548750 | 48.20 | 0 | 15339520 | - | - | 0 | 3745 |
| 9500 | churn_point=1084 churn_balance=56833 churn_window=531791 | 47.23 | 103076084 | 13180928 | - | - | 0 | 3218 |
| 9750 | churn_point=1209 churn_balance=59792 churn_window=538875 | 46.19 | 0 | 15351808 | - | - | 0 | 3748 |
| 10000 | churn_point=1084 churn_balance=59375 churn_window=527500 | 48.20 | 104992500 | 13185024 | - | - | 0 | 3219 |

## run nosync (churn=64 updates=32 growth=0, working set 100000)

### ours-ephemeral (bumbledb)

| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | freelist | pages |
|---|---|---|---|---|---|---|---|---|
| 250 | churn_point=244 churn_balance=25291 churn_window=20542 | 327.43 | 0 | 80543744 | 294 | 123999 | - | - |
| 500 | churn_point=265 churn_balance=19916 churn_window=20792 | 334.09 | 0 | 80543744 | 544 | 147999 | - | - |
| 750 | churn_point=247 churn_balance=15834 churn_window=20333 | 329.31 | 0 | 80543744 | 794 | 171999 | - | - |
| 1000 | churn_point=255 churn_balance=12584 churn_window=21416 | 329.24 | 0 | 80543744 | 1044 | 195999 | - | - |
| 1250 | churn_point=239 churn_balance=9958 churn_window=20959 | 329.61 | 0 | 80543744 | 1294 | 219999 | - | - |
| 1500 | churn_point=268 churn_balance=8125 churn_window=21125 | 307.84 | 0 | 80543744 | 1544 | 243999 | - | - |
| 1750 | churn_point=265 churn_balance=6833 churn_window=22041 | 303.53 | 0 | 80543744 | 1794 | 267999 | - | - |
| 2000 | churn_point=260 churn_balance=5250 churn_window=21959 | 287.34 | 0 | 80543744 | 2044 | 291999 | - | - |
| 2250 | churn_point=250 churn_balance=4167 churn_window=21750 | 288.10 | 0 | 80543744 | 2294 | 315999 | - | - |
| 2500 | churn_point=239 churn_balance=3291 churn_window=22291 | 288.72 | 0 | 80543744 | 2544 | 339999 | - | - |
| 2750 | churn_point=255 churn_balance=2500 churn_window=22458 | 288.80 | 0 | 80543744 | 2794 | 363999 | - | - |
| 3000 | churn_point=252 churn_balance=2042 churn_window=22500 | 286.91 | 0 | 80543744 | 3044 | 387999 | - | - |
| 3250 | churn_point=252 churn_balance=1666 churn_window=22708 | 285.64 | 0 | 80543744 | 3294 | 411999 | - | - |
| 3500 | churn_point=268 churn_balance=1375 churn_window=22958 | 285.65 | 0 | 80543744 | 3544 | 435999 | - | - |
| 3750 | churn_point=260 churn_balance=1167 churn_window=21958 | 285.46 | 0 | 80543744 | 3794 | 459999 | - | - |
| 4000 | churn_point=242 churn_balance=958 churn_window=21792 | 284.18 | 0 | 80543744 | 4044 | 483999 | - | - |
| 4250 | churn_point=247 churn_balance=833 churn_window=22167 | 281.50 | 0 | 80543744 | 4294 | 507999 | - | - |
| 4500 | churn_point=273 churn_balance=708 churn_window=21500 | 275.37 | 0 | 80543744 | 4544 | 531999 | - | - |
| 4750 | churn_point=276 churn_balance=667 churn_window=21500 | 288.35 | 0 | 80543744 | 4794 | 555999 | - | - |
| 5000 | churn_point=257 churn_balance=583 churn_window=21417 | 274.63 | 0 | 80543744 | 5044 | 579999 | - | - |
| 5250 | churn_point=244 churn_balance=500 churn_window=21917 | 285.50 | 0 | 80543744 | 5294 | 603999 | - | - |
| 5500 | churn_point=247 churn_balance=500 churn_window=21958 | 281.46 | 0 | 80543744 | 5544 | 627999 | - | - |
| 5750 | churn_point=247 churn_balance=408 churn_window=22042 | 285.04 | 0 | 80543744 | 5794 | 651999 | - | - |
| 6000 | churn_point=265 churn_balance=390 churn_window=23041 | 285.82 | 0 | 80543744 | 6044 | 675999 | - | - |
| 6250 | churn_point=276 churn_balance=385 churn_window=23125 | 285.50 | 0 | 80543744 | 6294 | 699999 | - | - |
| 6500 | churn_point=255 churn_balance=364 churn_window=22750 | 285.65 | 0 | 80543744 | 6544 | 723999 | - | - |
| 6750 | churn_point=260 churn_balance=362 churn_window=22792 | 287.23 | 0 | 80543744 | 6794 | 747999 | - | - |
| 7000 | churn_point=260 churn_balance=346 churn_window=23125 | 272.25 | 0 | 80543744 | 7044 | 771999 | - | - |
| 7250 | churn_point=260 churn_balance=325 churn_window=22416 | 285.55 | 0 | 80543744 | 7294 | 795999 | - | - |
| 7500 | churn_point=247 churn_balance=338 churn_window=21708 | 285.23 | 0 | 80543744 | 7544 | 819999 | - | - |
| 7750 | churn_point=263 churn_balance=325 churn_window=21500 | 285.01 | 0 | 80543744 | 7794 | 843999 | - | - |
| 8000 | churn_point=268 churn_balance=325 churn_window=22000 | 284.04 | 0 | 80543744 | 8044 | 867999 | - | - |
| 8250 | churn_point=260 churn_balance=309 churn_window=22125 | 274.27 | 0 | 80543744 | 8294 | 891999 | - | - |
| 8500 | churn_point=265 churn_balance=307 churn_window=22000 | 285.96 | 0 | 80543744 | 8544 | 915999 | - | - |
| 8750 | churn_point=283 churn_balance=312 churn_window=21750 | 272.56 | 0 | 80543744 | 8794 | 939999 | - | - |
| 9000 | churn_point=273 churn_balance=317 churn_window=22250 | 284.45 | 0 | 80543744 | 9044 | 963999 | - | - |
| 9250 | churn_point=273 churn_balance=307 churn_window=21542 | 281.23 | 0 | 80543744 | 9294 | 987999 | - | - |
| 9500 | churn_point=244 churn_balance=309 churn_window=21250 | 282.66 | 0 | 80543744 | 9544 | 1011999 | - | - |
| 9750 | churn_point=257 churn_balance=299 churn_window=21500 | 277.24 | 0 | 80543744 | 9794 | 1035999 | - | - |
| 10000 | churn_point=268 churn_balance=307 churn_window=21250 | 284.39 | 0 | 80543744 | 10044 | 1059999 | - | - |

### sqlite-nosync (sqlite)

| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | freelist | pages |
|---|---|---|---|---|---|---|---|---|
| 250 | churn_point=1083 churn_balance=5434083 churn_window=284375 | 243.36 | 0 | 14774272 | - | - | 0 | 3607 |
| 500 | churn_point=1084 churn_balance=4496458 churn_window=374042 | 241.79 | 0 | 15884288 | - | - | 0 | 3878 |
| 750 | churn_point=1125 churn_balance=3674000 churn_window=405125 | 237.20 | 0 | 16924672 | - | - | 0 | 4132 |
| 1000 | churn_point=1084 churn_balance=2996250 churn_window=443750 | 235.81 | 0 | 17383424 | - | - | 45 | 4244 |
| 1250 | churn_point=1083 churn_balance=2390167 churn_window=469917 | 238.00 | 0 | 17383424 | - | - | 193 | 4244 |
| 1500 | churn_point=1084 churn_balance=1946208 churn_window=493667 | 224.80 | 0 | 17383424 | - | - | 161 | 4244 |
| 1750 | churn_point=1084 churn_balance=1603083 churn_window=517125 | 224.61 | 0 | 17383424 | - | - | 204 | 4244 |
| 2000 | churn_point=1083 churn_balance=1247709 churn_window=519041 | 174.07 | 0 | 17383424 | - | - | 205 | 4244 |
| 2250 | churn_point=1083 churn_balance=999333 churn_window=520875 | 220.40 | 0 | 17383424 | - | - | 209 | 4244 |
| 2500 | churn_point=1083 churn_balance=809292 churn_window=530208 | 221.23 | 0 | 17383424 | - | - | 214 | 4244 |
| 2750 | churn_point=1084 churn_balance=638333 churn_window=544708 | 220.56 | 0 | 17383424 | - | - | 248 | 4244 |
| 3000 | churn_point=1083 churn_balance=528458 churn_window=552875 | 222.37 | 0 | 17383424 | - | - | 239 | 4244 |
| 3250 | churn_point=1083 churn_balance=431500 churn_window=558042 | 220.92 | 0 | 17383424 | - | - | 236 | 4244 |
| 3500 | churn_point=1375 churn_balance=347417 churn_window=570167 | 220.25 | 0 | 17383424 | - | - | 237 | 4244 |
| 3750 | churn_point=1083 churn_balance=282625 churn_window=569209 | 221.91 | 0 | 17383424 | - | - | 236 | 4244 |
| 4000 | churn_point=1084 churn_balance=233125 churn_window=558750 | 220.98 | 0 | 17383424 | - | - | 254 | 4244 |
| 4250 | churn_point=1083 churn_balance=194125 churn_window=545291 | 220.21 | 0 | 17383424 | - | - | 244 | 4244 |
| 4500 | churn_point=1083 churn_balance=157875 churn_window=543291 | 217.31 | 0 | 17383424 | - | - | 243 | 4244 |
| 4750 | churn_point=1083 churn_balance=134917 churn_window=540125 | 222.63 | 0 | 17383424 | - | - | 236 | 4244 |
| 5000 | churn_point=1083 churn_balance=113833 churn_window=557250 | 216.85 | 0 | 17383424 | - | - | 240 | 4244 |
| 5250 | churn_point=1083 churn_balance=103042 churn_window=543416 | 221.62 | 0 | 17383424 | - | - | 244 | 4244 |
| 5500 | churn_point=1083 churn_balance=88834 churn_window=556292 | 208.61 | 0 | 17383424 | - | - | 226 | 4244 |
| 5750 | churn_point=1083 churn_balance=78250 churn_window=583542 | 222.95 | 0 | 17383424 | - | - | 234 | 4244 |
| 6000 | churn_point=1084 churn_balance=74167 churn_window=575833 | 222.76 | 0 | 17383424 | - | - | 239 | 4244 |
| 6250 | churn_point=1083 churn_balance=71917 churn_window=570167 | 223.11 | 0 | 17383424 | - | - | 229 | 4244 |
| 6500 | churn_point=1083 churn_balance=66458 churn_window=569458 | 222.50 | 0 | 17383424 | - | - | 216 | 4244 |
| 6750 | churn_point=1083 churn_balance=63917 churn_window=591917 | 221.77 | 0 | 17383424 | - | - | 229 | 4244 |
| 7000 | churn_point=1083 churn_balance=58542 churn_window=570875 | 215.31 | 0 | 17383424 | - | - | 217 | 4244 |
| 7250 | churn_point=1083 churn_balance=56125 churn_window=566333 | 221.82 | 0 | 17383424 | - | - | 226 | 4244 |
| 7500 | churn_point=1083 churn_balance=58167 churn_window=563334 | 221.79 | 0 | 17383424 | - | - | 226 | 4244 |
| 7750 | churn_point=1083 churn_balance=56875 churn_window=550083 | 222.07 | 0 | 17383424 | - | - | 232 | 4244 |
| 8000 | churn_point=1083 churn_balance=55875 churn_window=567167 | 220.32 | 0 | 17383424 | - | - | 222 | 4244 |
| 8250 | churn_point=1084 churn_balance=52250 churn_window=546000 | 216.23 | 0 | 17383424 | - | - | 221 | 4244 |
| 8500 | churn_point=1042 churn_balance=52250 churn_window=544958 | 221.65 | 0 | 17383424 | - | - | 222 | 4244 |
| 8750 | churn_point=1083 churn_balance=48875 churn_window=560167 | 216.18 | 0 | 17383424 | - | - | 219 | 4244 |
| 9000 | churn_point=1083 churn_balance=51125 churn_window=544417 | 220.60 | 0 | 17383424 | - | - | 220 | 4244 |
| 9250 | churn_point=1083 churn_balance=52000 churn_window=551041 | 187.66 | 0 | 17383424 | - | - | 214 | 4244 |
| 9500 | churn_point=1083 churn_balance=51791 churn_window=534625 | 221.41 | 0 | 17383424 | - | - | 232 | 4244 |
| 9750 | churn_point=1083 churn_balance=50541 churn_window=531042 | 218.60 | 0 | 17383424 | - | - | 221 | 4244 |
| 10000 | churn_point=1084 churn_balance=52708 churn_window=523709 | 223.86 | 0 | 17383424 | - | - | 211 | 4244 |

## run delete-heavy (churn=512 updates=0 growth=0, working set 100000)

### ours-durable (bumbledb)

| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | freelist | pages |
|---|---|---|---|---|---|---|---|---|
| 250 | churn_point=265 churn_balance=9250 churn_window=21208 | 24.26 | 0 | 112427008 | 294 | 227999 | - | - |
| 500 | churn_point=247 churn_balance=2791 churn_window=21958 | 23.38 | 0 | 112607232 | 544 | 355999 | - | - |
| 750 | churn_point=273 churn_balance=917 churn_window=22125 | 23.41 | 0 | 112607232 | 794 | 483999 | - | - |
| 1000 | churn_point=257 churn_balance=500 churn_window=21625 | 23.30 | 0 | 112607232 | 1044 | 611999 | - | - |
| 1250 | churn_point=239 churn_balance=354 churn_window=22209 | 22.59 | 0 | 112607232 | 1294 | 739999 | - | - |
| 1500 | churn_point=289 churn_balance=317 churn_window=22500 | 22.56 | 0 | 112787456 | 1544 | 867999 | - | - |
| 1750 | churn_point=270 churn_balance=312 churn_window=21833 | 22.72 | 0 | 112951296 | 1794 | 995999 | - | - |
| 2000 | churn_point=268 churn_balance=270 churn_window=22875 | 22.49 | 0 | 113295360 | 2044 | 1123999 | - | - |
| 2250 | churn_point=273 churn_balance=281 churn_window=21750 | 22.44 | 0 | 113426432 | 2294 | 1251999 | - | - |
| 2500 | churn_point=255 churn_balance=281 churn_window=21000 | 22.47 | 0 | 113541120 | 2544 | 1379999 | - | - |
| 2750 | churn_point=257 churn_balance=278 churn_window=21625 | 22.41 | 0 | 114163712 | 2794 | 1507999 | - | - |
| 3000 | churn_point=268 churn_balance=273 churn_window=21875 | 22.27 | 0 | 114163712 | 3044 | 1635999 | - | - |
| 3250 | churn_point=265 churn_balance=289 churn_window=22083 | 20.46 | 0 | 114196480 | 3294 | 1763999 | - | - |
| 3500 | churn_point=276 churn_balance=270 churn_window=22000 | 20.69 | 0 | 114278400 | 3544 | 1891999 | - | - |
| 3750 | churn_point=257 churn_balance=296 churn_window=22917 | 23.01 | 0 | 114278400 | 3794 | 2019999 | - | - |
| 4000 | churn_point=247 churn_balance=278 churn_window=21750 | 23.12 | 0 | 114278400 | 4044 | 2147999 | - | - |
| 4250 | churn_point=260 churn_balance=276 churn_window=20333 | 23.22 | 0 | 114278400 | 4294 | 2275999 | - | - |
| 4500 | churn_point=260 churn_balance=268 churn_window=22708 | 22.98 | 0 | 114278400 | 4544 | 2403999 | - | - |
| 4750 | churn_point=257 churn_balance=315 churn_window=21000 | 22.89 | 0 | 114671616 | 4794 | 2531999 | - | - |
| 5000 | churn_point=263 churn_balance=299 churn_window=23083 | 22.98 | 0 | 114671616 | 5044 | 2659999 | - | - |
| 5250 | churn_point=250 churn_balance=299 churn_window=22667 | 23.15 | 0 | 114671616 | 5294 | 2787999 | - | - |
| 5500 | churn_point=268 churn_balance=304 churn_window=21125 | 23.17 | 0 | 114671616 | 5544 | 2915999 | - | - |
| 5750 | churn_point=281 churn_balance=296 churn_window=23125 | 22.86 | 0 | 114671616 | 5794 | 3043999 | - | - |
| 6000 | churn_point=260 churn_balance=307 churn_window=22167 | 22.98 | 0 | 114704384 | 6044 | 3171999 | - | - |
| 6250 | churn_point=273 churn_balance=296 churn_window=21333 | 23.11 | 0 | 114753536 | 6294 | 3299999 | - | - |
| 6500 | churn_point=265 churn_balance=273 churn_window=22042 | 22.95 | 0 | 114868224 | 6544 | 3427999 | - | - |
| 6750 | churn_point=265 churn_balance=296 churn_window=20958 | 22.81 | 0 | 114900992 | 6794 | 3555999 | - | - |
| 7000 | churn_point=291 churn_balance=312 churn_window=22834 | 23.02 | 0 | 114900992 | 7044 | 3683999 | - | - |
| 7250 | churn_point=281 churn_balance=317 churn_window=22959 | 23.00 | 0 | 115015680 | 7294 | 3811999 | - | - |
| 7500 | churn_point=278 churn_balance=299 churn_window=22000 | 22.96 | 0 | 115015680 | 7544 | 3939999 | - | - |
| 7750 | churn_point=276 churn_balance=278 churn_window=26542 | 22.87 | 0 | 115343360 | 7794 | 4067999 | - | - |
| 8000 | churn_point=263 churn_balance=299 churn_window=21958 | 22.88 | 0 | 115343360 | 8044 | 4195999 | - | - |
| 8250 | churn_point=273 churn_balance=294 churn_window=23083 | 23.02 | 0 | 115343360 | 8294 | 4323999 | - | - |
| 8500 | churn_point=252 churn_balance=304 churn_window=21875 | 23.02 | 0 | 115343360 | 8544 | 4451999 | - | - |
| 8750 | churn_point=278 churn_balance=302 churn_window=20958 | 22.84 | 0 | 115343360 | 8794 | 4579999 | - | - |
| 9000 | churn_point=260 churn_balance=276 churn_window=21042 | 22.88 | 0 | 115343360 | 9044 | 4707999 | - | - |
| 9250 | churn_point=257 churn_balance=296 churn_window=23833 | 23.04 | 0 | 115343360 | 9294 | 4835999 | - | - |
| 9500 | churn_point=250 churn_balance=278 churn_window=22583 | 23.02 | 0 | 115343360 | 9544 | 4963999 | - | - |
| 9750 | churn_point=260 churn_balance=273 churn_window=21667 | 22.97 | 0 | 115343360 | 9794 | 5091999 | - | - |
| 10000 | churn_point=263 churn_balance=291 churn_window=21625 | 22.92 | 0 | 115343360 | 10044 | 5219999 | - | - |

### sqlite-bare (sqlite)

| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | freelist | pages |
|---|---|---|---|---|---|---|---|---|
| 250 | churn_point=1125 churn_balance=2231292 churn_window=482167 | 27.82 | 0 | 17362944 | - | - | 176 | 4239 |
| 500 | churn_point=1125 churn_balance=682292 churn_window=539583 | 27.19 | 0 | 17362944 | - | - | 209 | 4239 |
| 750 | churn_point=1125 churn_balance=217166 churn_window=560750 | 27.35 | 0 | 17362944 | - | - | 226 | 4239 |
| 1000 | churn_point=1125 churn_balance=100166 churn_window=549458 | 27.25 | 0 | 17362944 | - | - | 217 | 4239 |
| 1250 | churn_point=1084 churn_balance=62500 churn_window=546875 | 27.28 | 0 | 17362944 | - | - | 218 | 4239 |
| 1500 | churn_point=1250 churn_balance=51375 churn_window=577292 | 27.23 | 0 | 17362944 | - | - | 230 | 4239 |
| 1750 | churn_point=1166 churn_balance=50625 churn_window=561750 | 27.21 | 0 | 17362944 | - | - | 203 | 4239 |
| 2000 | churn_point=1083 churn_balance=43291 churn_window=577292 | 27.25 | 0 | 17362944 | - | - | 193 | 4239 |
| 2250 | churn_point=1125 churn_balance=42917 churn_window=555708 | 27.12 | 0 | 17362944 | - | - | 202 | 4239 |
| 2500 | churn_point=1125 churn_balance=45042 churn_window=538958 | 27.23 | 0 | 17362944 | - | - | 205 | 4239 |
| 2750 | churn_point=1125 churn_balance=43917 churn_window=549833 | 27.04 | 0 | 17362944 | - | - | 194 | 4239 |
| 3000 | churn_point=1125 churn_balance=45041 churn_window=539417 | 27.18 | 0 | 17362944 | - | - | 187 | 4239 |
| 3250 | churn_point=1125 churn_balance=47083 churn_window=537875 | 27.15 | 0 | 17362944 | - | - | 182 | 4239 |
| 3500 | churn_point=1125 churn_balance=43375 churn_window=548083 | 26.93 | 0 | 17362944 | - | - | 187 | 4239 |
| 3750 | churn_point=1125 churn_balance=47417 churn_window=565917 | 27.01 | 0 | 17362944 | - | - | 189 | 4239 |
| 4000 | churn_point=1125 churn_balance=46541 churn_window=557375 | 27.24 | 0 | 17362944 | - | - | 172 | 4239 |
| 4250 | churn_point=1084 churn_balance=41958 churn_window=543125 | 27.32 | 0 | 17362944 | - | - | 151 | 4239 |
| 4500 | churn_point=1292 churn_balance=44792 churn_window=570000 | 27.25 | 0 | 17362944 | - | - | 150 | 4239 |
| 4750 | churn_point=1125 churn_balance=51417 churn_window=569750 | 26.40 | 0 | 17362944 | - | - | 152 | 4239 |
| 5000 | churn_point=1125 churn_balance=49250 churn_window=576083 | 27.26 | 0 | 17362944 | - | - | 134 | 4239 |
| 5250 | churn_point=1125 churn_balance=49792 churn_window=589916 | 27.36 | 0 | 17362944 | - | - | 154 | 4239 |
| 5500 | churn_point=1125 churn_balance=50083 churn_window=562291 | 27.29 | 0 | 17362944 | - | - | 137 | 4239 |
| 5750 | churn_point=1125 churn_balance=50000 churn_window=587084 | 27.13 | 0 | 17362944 | - | - | 135 | 4239 |
| 6000 | churn_point=1125 churn_balance=52500 churn_window=581375 | 27.32 | 0 | 17362944 | - | - | 139 | 4239 |
| 6250 | churn_point=1125 churn_balance=49500 churn_window=564459 | 27.35 | 0 | 17362944 | - | - | 124 | 4239 |
| 6500 | churn_point=1125 churn_balance=45666 churn_window=586333 | 27.25 | 0 | 17362944 | - | - | 133 | 4239 |
| 6750 | churn_point=1125 churn_balance=49375 churn_window=559292 | 27.05 | 0 | 17362944 | - | - | 130 | 4239 |
| 7000 | churn_point=1167 churn_balance=49459 churn_window=584084 | 27.27 | 0 | 17362944 | - | - | 124 | 4239 |
| 7250 | churn_point=1208 churn_balance=53334 churn_window=641166 | 27.37 | 0 | 17362944 | - | - | 116 | 4239 |
| 7500 | churn_point=1125 churn_balance=51875 churn_window=582250 | 27.13 | 0 | 17362944 | - | - | 128 | 4239 |
| 7750 | churn_point=1125 churn_balance=46709 churn_window=586625 | 27.23 | 0 | 17362944 | - | - | 130 | 4239 |
| 8000 | churn_point=1125 churn_balance=50083 churn_window=580167 | 27.23 | 0 | 17362944 | - | - | 131 | 4239 |
| 8250 | churn_point=1125 churn_balance=48667 churn_window=593958 | 27.30 | 0 | 17362944 | - | - | 129 | 4239 |
| 8500 | churn_point=1125 churn_balance=50417 churn_window=580709 | 27.39 | 0 | 17362944 | - | - | 126 | 4239 |
| 8750 | churn_point=1166 churn_balance=54958 churn_window=554292 | 27.17 | 0 | 17362944 | - | - | 123 | 4239 |
| 9000 | churn_point=1125 churn_balance=44209 churn_window=551500 | 27.12 | 0 | 17362944 | - | - | 118 | 4239 |
| 9250 | churn_point=1125 churn_balance=50459 churn_window=604500 | 27.29 | 0 | 17362944 | - | - | 123 | 4239 |
| 9500 | churn_point=1125 churn_balance=47792 churn_window=571084 | 27.33 | 0 | 17362944 | - | - | 120 | 4239 |
| 9750 | churn_point=1125 churn_balance=43542 churn_window=572084 | 27.34 | 0 | 17362944 | - | - | 116 | 4239 |
| 10000 | churn_point=1083 churn_balance=49625 churn_window=567459 | 27.11 | 0 | 17362944 | - | - | 129 | 4239 |
