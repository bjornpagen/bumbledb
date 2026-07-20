# churn — degradation over cycles

- crate version: 0.1.0
- engine rev: ec0b9c75f013ce85c3aa4fce0c055ae7c46e0d49
- timestamp: 2026-07-20T13:06:49Z
- host: Apple M2 Max
- shared machine: boost qos-user-interactive — load 1/5/15 2.23 1.77 2.00 (start) → 1.61 1.59 1.66 (end)
- config: scale S, seed 1, 10000 cycles, sample every 250, vacuum every 500, analyze every 500

## run steady (churn=64 updates=32 growth=0, working set 100000)

### ours-durable (bumbledb)

| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | freelist | pages |
|---|---|---|---|---|---|---|---|---|
| 250 | churn_point=583 churn_balance=27125 churn_window=19041 | 44.41 | 0 | 95944704 | 294 | 123999 | - | - |
| 500 | churn_point=583 churn_balance=19833 churn_window=19666 | 44.49 | 0 | 95944704 | 544 | 147999 | - | - |
| 750 | churn_point=583 churn_balance=15000 churn_window=19500 | 44.56 | 0 | 96927744 | 794 | 171999 | - | - |
| 1000 | churn_point=541 churn_balance=12208 churn_window=18917 | 43.98 | 0 | 96927744 | 1044 | 195999 | - | - |
| 1250 | churn_point=583 churn_balance=11083 churn_window=19709 | 43.36 | 0 | 96927744 | 1294 | 219999 | - | - |
| 1500 | churn_point=500 churn_balance=7875 churn_window=20209 | 43.86 | 0 | 96927744 | 1544 | 243999 | - | - |
| 1750 | churn_point=583 churn_balance=6792 churn_window=20083 | 43.78 | 0 | 96927744 | 1794 | 267999 | - | - |
| 2000 | churn_point=583 churn_balance=5125 churn_window=19750 | 43.70 | 0 | 96927744 | 2044 | 291999 | - | - |
| 2250 | churn_point=583 churn_balance=4500 churn_window=19250 | 43.80 | 0 | 96927744 | 2294 | 315999 | - | - |
| 2500 | churn_point=583 churn_balance=3125 churn_window=19875 | 43.79 | 0 | 96927744 | 2544 | 339999 | - | - |
| 2750 | churn_point=542 churn_balance=2500 churn_window=19500 | 43.33 | 0 | 96927744 | 2794 | 363999 | - | - |
| 3000 | churn_point=542 churn_balance=1917 churn_window=21792 | 40.98 | 0 | 96927744 | 3044 | 387999 | - | - |
| 3250 | churn_point=542 churn_balance=1708 churn_window=20333 | 41.85 | 0 | 96927744 | 3294 | 411999 | - | - |
| 3500 | churn_point=542 churn_balance=1292 churn_window=20500 | 42.07 | 0 | 96927744 | 3544 | 435999 | - | - |
| 3750 | churn_point=541 churn_balance=1166 churn_window=21667 | 41.87 | 0 | 96927744 | 3794 | 459999 | - | - |
| 4000 | churn_point=542 churn_balance=958 churn_window=20000 | 38.75 | 0 | 96927744 | 4044 | 483999 | - | - |
| 4250 | churn_point=583 churn_balance=792 churn_window=20958 | 40.74 | 0 | 96927744 | 4294 | 507999 | - | - |
| 4500 | churn_point=541 churn_balance=667 churn_window=20209 | 44.54 | 0 | 96927744 | 4544 | 531999 | - | - |
| 4750 | churn_point=542 churn_balance=625 churn_window=21250 | 43.84 | 0 | 96927744 | 4794 | 555999 | - | - |
| 5000 | churn_point=542 churn_balance=542 churn_window=20333 | 44.18 | 0 | 96927744 | 5044 | 579999 | - | - |
| 5250 | churn_point=584 churn_balance=542 churn_window=21833 | 43.77 | 0 | 96927744 | 5294 | 603999 | - | - |
| 5500 | churn_point=542 churn_balance=424 churn_window=20208 | 44.33 | 0 | 96927744 | 5544 | 627999 | - | - |
| 5750 | churn_point=583 churn_balance=432 churn_window=21125 | 44.16 | 0 | 96927744 | 5794 | 651999 | - | - |
| 6000 | churn_point=625 churn_balance=377 churn_window=20667 | 44.04 | 0 | 96927744 | 6044 | 675999 | - | - |
| 6250 | churn_point=542 churn_balance=382 churn_window=22667 | 44.14 | 0 | 96927744 | 6294 | 699999 | - | - |
| 6500 | churn_point=542 churn_balance=348 churn_window=20500 | 44.67 | 0 | 96927744 | 6544 | 723999 | - | - |
| 6750 | churn_point=625 churn_balance=372 churn_window=22375 | 44.07 | 0 | 96927744 | 6794 | 747999 | - | - |
| 7000 | churn_point=542 churn_balance=330 churn_window=20458 | 44.46 | 0 | 96927744 | 7044 | 771999 | - | - |
| 7250 | churn_point=584 churn_balance=364 churn_window=22167 | 44.11 | 0 | 96927744 | 7294 | 795999 | - | - |
| 7500 | churn_point=541 churn_balance=333 churn_window=20167 | 44.58 | 0 | 96927744 | 7544 | 819999 | - | - |
| 7750 | churn_point=583 churn_balance=333 churn_window=22042 | 44.44 | 0 | 96927744 | 7794 | 843999 | - | - |
| 8000 | churn_point=583 churn_balance=320 churn_window=21459 | 44.59 | 0 | 96927744 | 8044 | 867999 | - | - |
| 8250 | churn_point=583 churn_balance=317 churn_window=22041 | 44.19 | 0 | 96927744 | 8294 | 891999 | - | - |
| 8500 | churn_point=542 churn_balance=286 churn_window=20833 | 41.92 | 0 | 96927744 | 8544 | 915999 | - | - |
| 8750 | churn_point=583 churn_balance=320 churn_window=23375 | 42.30 | 0 | 96927744 | 8794 | 939999 | - | - |
| 9000 | churn_point=583 churn_balance=283 churn_window=21167 | 42.06 | 0 | 96927744 | 9044 | 963999 | - | - |
| 9250 | churn_point=584 churn_balance=335 churn_window=23959 | 42.63 | 0 | 96927744 | 9294 | 987999 | - | - |
| 9500 | churn_point=708 churn_balance=276 churn_window=21416 | 41.94 | 0 | 96927744 | 9544 | 1011999 | - | - |
| 9750 | churn_point=583 churn_balance=302 churn_window=24000 | 42.88 | 0 | 96927744 | 9794 | 1035999 | - | - |
| 10000 | churn_point=583 churn_balance=281 churn_window=21459 | 43.08 | 0 | 96927744 | 10044 | 1059999 | - | - |

### sqlite-bare (sqlite)

| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | freelist | pages |
|---|---|---|---|---|---|---|---|---|
| 250 | churn_point=1167 churn_balance=5355000 churn_window=276917 | 55.59 | 0 | 14753792 | - | - | 0 | 3602 |
| 500 | churn_point=1125 churn_balance=4427542 churn_window=366500 | 50.85 | 0 | 15958016 | - | - | 0 | 3896 |
| 750 | churn_point=1084 churn_balance=3619125 churn_window=422708 | 51.42 | 0 | 16916480 | - | - | 0 | 4130 |
| 1000 | churn_point=1167 churn_balance=2933917 churn_window=440208 | 48.73 | 0 | 17309696 | - | - | 59 | 4226 |
| 1250 | churn_point=1250 churn_balance=2407292 churn_window=477500 | 47.94 | 0 | 17309696 | - | - | 176 | 4226 |
| 1500 | churn_point=1166 churn_balance=1891375 churn_window=494458 | 47.51 | 0 | 17309696 | - | - | 156 | 4226 |
| 1750 | churn_point=1208 churn_balance=1533000 churn_window=503292 | 47.46 | 0 | 17309696 | - | - | 201 | 4226 |
| 2000 | churn_point=1125 churn_balance=1208459 churn_window=505959 | 47.51 | 0 | 17309696 | - | - | 194 | 4226 |
| 2250 | churn_point=1250 churn_balance=1020958 churn_window=512458 | 49.57 | 0 | 17309696 | - | - | 188 | 4226 |
| 2500 | churn_point=1166 churn_balance=768000 churn_window=520000 | 47.94 | 0 | 17309696 | - | - | 192 | 4226 |
| 2750 | churn_point=1125 churn_balance=615125 churn_window=525333 | 48.40 | 0 | 17309696 | - | - | 210 | 4226 |
| 3000 | churn_point=1166 churn_balance=492167 churn_window=546000 | 48.78 | 0 | 17309696 | - | - | 208 | 4226 |
| 3250 | churn_point=1125 churn_balance=415917 churn_window=547333 | 49.05 | 0 | 17309696 | - | - | 220 | 4226 |
| 3500 | churn_point=1167 churn_balance=326542 churn_window=546625 | 48.79 | 0 | 17309696 | - | - | 230 | 4226 |
| 3750 | churn_point=1167 churn_balance=283042 churn_window=586792 | 47.49 | 0 | 17309696 | - | - | 213 | 4226 |
| 4000 | churn_point=1125 churn_balance=221125 churn_window=593417 | 41.86 | 0 | 17309696 | - | - | 214 | 4226 |
| 4250 | churn_point=1167 churn_balance=189000 churn_window=558750 | 44.44 | 0 | 17309696 | - | - | 221 | 4226 |
| 4500 | churn_point=1125 churn_balance=156583 churn_window=559208 | 48.69 | 0 | 17309696 | - | - | 211 | 4226 |
| 4750 | churn_point=1250 churn_balance=137833 churn_window=585958 | 47.45 | 0 | 17309696 | - | - | 204 | 4226 |
| 5000 | churn_point=1166 churn_balance=108875 churn_window=547958 | 48.17 | 0 | 17309696 | - | - | 199 | 4226 |
| 5250 | churn_point=1125 churn_balance=101916 churn_window=570041 | 49.12 | 0 | 17309696 | - | - | 203 | 4226 |
| 5500 | churn_point=1125 churn_balance=86500 churn_window=553208 | 49.49 | 0 | 17309696 | - | - | 203 | 4226 |
| 5750 | churn_point=1209 churn_balance=86166 churn_window=587000 | 48.92 | 0 | 17309696 | - | - | 214 | 4226 |
| 6000 | churn_point=1084 churn_balance=73000 churn_window=545625 | 49.15 | 0 | 17309696 | - | - | 222 | 4226 |
| 6250 | churn_point=1250 churn_balance=73875 churn_window=581542 | 49.89 | 0 | 17309696 | - | - | 210 | 4226 |
| 6500 | churn_point=1166 churn_balance=66458 churn_window=563083 | 50.48 | 0 | 17309696 | - | - | 209 | 4226 |
| 6750 | churn_point=1208 churn_balance=70125 churn_window=569875 | 49.45 | 0 | 17309696 | - | - | 203 | 4226 |
| 7000 | churn_point=1166 churn_balance=62125 churn_window=548208 | 49.54 | 0 | 17309696 | - | - | 202 | 4226 |
| 7250 | churn_point=1208 churn_balance=66500 churn_window=583041 | 49.24 | 0 | 17309696 | - | - | 199 | 4226 |
| 7500 | churn_point=1125 churn_balance=61041 churn_window=549750 | 48.53 | 0 | 17309696 | - | - | 184 | 4226 |
| 7750 | churn_point=1208 churn_balance=63208 churn_window=591000 | 49.72 | 0 | 17309696 | - | - | 201 | 4226 |
| 8000 | churn_point=1125 churn_balance=58792 churn_window=567458 | 50.50 | 0 | 17309696 | - | - | 189 | 4226 |
| 8250 | churn_point=1208 churn_balance=58750 churn_window=580083 | 49.74 | 0 | 17309696 | - | - | 197 | 4226 |
| 8500 | churn_point=1167 churn_balance=50917 churn_window=548083 | 46.76 | 0 | 17309696 | - | - | 191 | 4226 |
| 8750 | churn_point=1167 churn_balance=58500 churn_window=601667 | 46.42 | 0 | 17309696 | - | - | 194 | 4226 |
| 9000 | churn_point=1125 churn_balance=51916 churn_window=557459 | 45.59 | 0 | 17309696 | - | - | 199 | 4226 |
| 9250 | churn_point=1333 churn_balance=57958 churn_window=616709 | 47.42 | 0 | 17309696 | - | - | 185 | 4226 |
| 9500 | churn_point=1125 churn_balance=48167 churn_window=560750 | 45.45 | 0 | 17309696 | - | - | 192 | 4226 |
| 9750 | churn_point=1209 churn_balance=50458 churn_window=612000 | 47.40 | 0 | 17309696 | - | - | 190 | 4226 |
| 10000 | churn_point=1125 churn_balance=46958 churn_window=561000 | 47.47 | 0 | 17309696 | - | - | 185 | 4226 |

### sqlite-maint (sqlite)

| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | freelist | pages |
|---|---|---|---|---|---|---|---|---|
| 250 | churn_point=1167 churn_balance=5397625 churn_window=276959 | 55.16 | 0 | 14753792 | - | - | 0 | 3602 |
| 500 | churn_point=1125 churn_balance=4421292 churn_window=647167 | 49.27 | 110477334 | 12836864 | - | - | 0 | 3134 |
| 750 | churn_point=1083 churn_balance=3618500 churn_window=679334 | 51.38 | 0 | 14827520 | - | - | 0 | 3620 |
| 1000 | churn_point=1125 churn_balance=2924250 churn_window=688709 | 49.47 | 107768541 | 12967936 | - | - | 0 | 3166 |
| 1250 | churn_point=1208 churn_balance=2364000 churn_window=718959 | 48.88 | 0 | 15065088 | - | - | 0 | 3678 |
| 1500 | churn_point=1125 churn_balance=1905792 churn_window=727167 | 48.00 | 109443251 | 13053952 | - | - | 0 | 3187 |
| 1750 | churn_point=1167 churn_balance=1528208 churn_window=746750 | 49.14 | 0 | 15093760 | - | - | 0 | 3685 |
| 2000 | churn_point=1166 churn_balance=1217292 churn_window=738500 | 48.85 | 106031791 | 13103104 | - | - | 0 | 3199 |
| 2250 | churn_point=1208 churn_balance=970000 churn_window=747917 | 49.60 | 0 | 15216640 | - | - | 0 | 3715 |
| 2500 | churn_point=1125 churn_balance=767417 churn_window=525375 | 47.95 | 103035458 | 13135872 | - | - | 0 | 3207 |
| 2750 | churn_point=1125 churn_balance=608208 churn_window=527292 | 46.99 | 0 | 15269888 | - | - | 0 | 3728 |
| 3000 | churn_point=1125 churn_balance=491875 churn_window=777625 | 47.60 | 108026499 | 13152256 | - | - | 0 | 3211 |
| 3250 | churn_point=1125 churn_balance=405542 churn_window=785250 | 47.12 | 0 | 15306752 | - | - | 0 | 3737 |
| 3500 | churn_point=1125 churn_balance=328875 churn_window=775667 | 45.60 | 108682583 | 13164544 | - | - | 0 | 3214 |
| 3750 | churn_point=1167 churn_balance=281792 churn_window=823875 | 44.29 | 0 | 15384576 | - | - | 0 | 3756 |
| 4000 | churn_point=1125 churn_balance=221500 churn_window=587166 | 40.08 | 109446667 | 13172736 | - | - | 0 | 3216 |
| 4250 | churn_point=1167 churn_balance=188583 churn_window=558750 | 42.63 | 0 | 15384576 | - | - | 0 | 3756 |
| 4500 | churn_point=1125 churn_balance=154708 churn_window=797583 | 48.20 | 110293042 | 13180928 | - | - | 0 | 3218 |
| 4750 | churn_point=1208 churn_balance=137500 churn_window=780583 | 47.00 | 0 | 15314944 | - | - | 0 | 3739 |
| 5000 | churn_point=1125 churn_balance=108834 churn_window=556791 | 45.82 | 106972292 | 13180928 | - | - | 0 | 3218 |
| 5250 | churn_point=1125 churn_balance=102708 churn_window=564792 | 46.05 | 0 | 15331328 | - | - | 0 | 3743 |
| 5500 | churn_point=1125 churn_balance=86000 churn_window=787542 | 47.29 | 104960249 | 13180928 | - | - | 0 | 3218 |
| 5750 | churn_point=1208 churn_balance=84791 churn_window=787917 | 46.09 | 0 | 15392768 | - | - | 0 | 3758 |
| 6000 | churn_point=1084 churn_balance=71959 churn_window=798458 | 46.23 | 108232458 | 13180928 | - | - | 0 | 3218 |
| 6250 | churn_point=1250 churn_balance=73750 churn_window=799375 | 46.43 | 0 | 15384576 | - | - | 0 | 3756 |
| 6500 | churn_point=1125 churn_balance=64417 churn_window=566166 | 46.99 | 108931749 | 13180928 | - | - | 0 | 3218 |
| 6750 | churn_point=1167 churn_balance=66250 churn_window=552292 | 46.34 | 0 | 15368192 | - | - | 0 | 3752 |
| 7000 | churn_point=1125 churn_balance=61125 churn_window=575875 | 46.70 | 109109749 | 13180928 | - | - | 0 | 3218 |
| 7250 | churn_point=1167 churn_balance=63708 churn_window=563750 | 45.70 | 0 | 15388672 | - | - | 0 | 3757 |
| 7500 | churn_point=1125 churn_balance=60209 churn_window=572625 | 46.88 | 108205834 | 13180928 | - | - | 0 | 3218 |
| 7750 | churn_point=1167 churn_balance=62375 churn_window=580708 | 47.29 | 0 | 15335424 | - | - | 0 | 3744 |
| 8000 | churn_point=1167 churn_balance=64708 churn_window=797166 | 47.02 | 112724584 | 13180928 | - | - | 0 | 3218 |
| 8250 | churn_point=1167 churn_balance=65250 churn_window=807791 | 47.19 | 0 | 15327232 | - | - | 0 | 3742 |
| 8500 | churn_point=1166 churn_balance=49084 churn_window=559084 | 43.68 | 104995125 | 13180928 | - | - | 0 | 3218 |
| 8750 | churn_point=1167 churn_balance=57334 churn_window=598917 | 43.11 | 0 | 15314944 | - | - | 0 | 3739 |
| 9000 | churn_point=1125 churn_balance=48375 churn_window=805625 | 43.72 | 109647834 | 13180928 | - | - | 0 | 3218 |
| 9250 | churn_point=1292 churn_balance=56500 churn_window=832333 | 44.23 | 0 | 15355904 | - | - | 0 | 3749 |
| 9500 | churn_point=1125 churn_balance=45000 churn_window=581834 | 42.94 | 109879459 | 13180928 | - | - | 0 | 3218 |
| 9750 | churn_point=1208 churn_balance=49917 churn_window=604042 | 45.20 | 0 | 15327232 | - | - | 0 | 3742 |
| 10000 | churn_point=1166 churn_balance=46333 churn_window=791375 | 44.14 | 106044167 | 13185024 | - | - | 0 | 3219 |

## run nosync (churn=64 updates=32 growth=0, working set 100000)

### ours-ephemeral (bumbledb)

| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | freelist | pages |
|---|---|---|---|---|---|---|---|---|
| 250 | churn_point=500 churn_balance=24916 churn_window=19083 | 309.05 | 0 | 95944704 | 294 | 123999 | - | - |
| 500 | churn_point=625 churn_balance=19667 churn_window=19291 | 302.58 | 0 | 95944704 | 544 | 147999 | - | - |
| 750 | churn_point=667 churn_balance=15416 churn_window=19375 | 306.20 | 0 | 96927744 | 794 | 171999 | - | - |
| 1000 | churn_point=542 churn_balance=12167 churn_window=19500 | 301.10 | 0 | 96927744 | 1044 | 195999 | - | - |
| 1250 | churn_point=625 churn_balance=9750 churn_window=19625 | 298.05 | 0 | 96927744 | 1294 | 219999 | - | - |
| 1500 | churn_point=625 churn_balance=7792 churn_window=19833 | 302.32 | 0 | 96927744 | 1544 | 243999 | - | - |
| 1750 | churn_point=542 churn_balance=6250 churn_window=19750 | 281.13 | 0 | 96927744 | 1794 | 267999 | - | - |
| 2000 | churn_point=666 churn_balance=5000 churn_window=18792 | 260.60 | 0 | 96927744 | 2044 | 291999 | - | - |
| 2250 | churn_point=708 churn_balance=3958 churn_window=19458 | 257.35 | 0 | 96927744 | 2294 | 315999 | - | - |
| 2500 | churn_point=625 churn_balance=3083 churn_window=19375 | 256.18 | 0 | 96927744 | 2544 | 339999 | - | - |
| 2750 | churn_point=625 churn_balance=2375 churn_window=19375 | 255.42 | 0 | 96927744 | 2794 | 363999 | - | - |
| 3000 | churn_point=583 churn_balance=1958 churn_window=20709 | 251.18 | 0 | 96927744 | 3044 | 387999 | - | - |
| 3250 | churn_point=500 churn_balance=1542 churn_window=20417 | 257.74 | 0 | 96927744 | 3294 | 411999 | - | - |
| 3500 | churn_point=583 churn_balance=1292 churn_window=20084 | 256.21 | 0 | 96927744 | 3544 | 435999 | - | - |
| 3750 | churn_point=541 churn_balance=1083 churn_window=21209 | 255.71 | 0 | 96927744 | 3794 | 459999 | - | - |
| 4000 | churn_point=667 churn_balance=916 churn_window=20542 | 256.10 | 0 | 96927744 | 4044 | 483999 | - | - |
| 4250 | churn_point=541 churn_balance=792 churn_window=21042 | 254.42 | 0 | 96927744 | 4294 | 507999 | - | - |
| 4500 | churn_point=583 churn_balance=667 churn_window=20292 | 256.64 | 0 | 96927744 | 4544 | 531999 | - | - |
| 4750 | churn_point=541 churn_balance=625 churn_window=20417 | 254.32 | 0 | 96927744 | 4794 | 555999 | - | - |
| 5000 | churn_point=500 churn_balance=542 churn_window=20208 | 257.89 | 0 | 96927744 | 5044 | 579999 | - | - |
| 5250 | churn_point=583 churn_balance=500 churn_window=20500 | 253.61 | 0 | 96927744 | 5294 | 603999 | - | - |
| 5500 | churn_point=625 churn_balance=419 churn_window=20333 | 257.08 | 0 | 96927744 | 5544 | 627999 | - | - |
| 5750 | churn_point=542 churn_balance=385 churn_window=20292 | 256.78 | 0 | 96927744 | 5794 | 651999 | - | - |
| 6000 | churn_point=500 churn_balance=375 churn_window=20417 | 256.97 | 0 | 96927744 | 6044 | 675999 | - | - |
| 6250 | churn_point=625 churn_balance=354 churn_window=20166 | 258.42 | 0 | 96927744 | 6294 | 699999 | - | - |
| 6500 | churn_point=583 churn_balance=341 churn_window=20667 | 254.77 | 0 | 96927744 | 6544 | 723999 | - | - |
| 6750 | churn_point=542 churn_balance=346 churn_window=20375 | 257.75 | 0 | 96927744 | 6794 | 747999 | - | - |
| 7000 | churn_point=584 churn_balance=333 churn_window=20541 | 258.53 | 0 | 96927744 | 7044 | 771999 | - | - |
| 7250 | churn_point=583 churn_balance=330 churn_window=21167 | 258.39 | 0 | 96927744 | 7294 | 795999 | - | - |
| 7500 | churn_point=500 churn_balance=335 churn_window=20750 | 259.18 | 0 | 96927744 | 7544 | 819999 | - | - |
| 7750 | churn_point=583 churn_balance=304 churn_window=21333 | 258.21 | 0 | 96927744 | 7794 | 843999 | - | - |
| 8000 | churn_point=583 churn_balance=296 churn_window=20541 | 259.57 | 0 | 96927744 | 8044 | 867999 | - | - |
| 8250 | churn_point=667 churn_balance=291 churn_window=20750 | 259.15 | 0 | 96927744 | 8294 | 891999 | - | - |
| 8500 | churn_point=625 churn_balance=281 churn_window=20375 | 259.62 | 0 | 96927744 | 8544 | 915999 | - | - |
| 8750 | churn_point=625 churn_balance=296 churn_window=21208 | 256.85 | 0 | 96927744 | 8794 | 939999 | - | - |
| 9000 | churn_point=542 churn_balance=281 churn_window=21083 | 260.21 | 0 | 96927744 | 9044 | 963999 | - | - |
| 9250 | churn_point=541 churn_balance=278 churn_window=20583 | 256.05 | 0 | 96927744 | 9294 | 987999 | - | - |
| 9500 | churn_point=583 churn_balance=263 churn_window=21292 | 256.81 | 0 | 96927744 | 9544 | 1011999 | - | - |
| 9750 | churn_point=625 churn_balance=276 churn_window=21834 | 256.81 | 0 | 96927744 | 9794 | 1035999 | - | - |
| 10000 | churn_point=625 churn_balance=281 churn_window=21250 | 256.70 | 0 | 96927744 | 10044 | 1059999 | - | - |

### sqlite-nosync (sqlite)

| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | freelist | pages |
|---|---|---|---|---|---|---|---|---|
| 250 | churn_point=1042 churn_balance=5458583 churn_window=277625 | 252.84 | 0 | 14753792 | - | - | 0 | 3602 |
| 500 | churn_point=1083 churn_balance=4494416 churn_window=356208 | 244.71 | 0 | 15958016 | - | - | 0 | 3896 |
| 750 | churn_point=1084 churn_balance=3635709 churn_window=421625 | 245.97 | 0 | 16916480 | - | - | 0 | 4130 |
| 1000 | churn_point=1083 churn_balance=2931333 churn_window=446792 | 243.62 | 0 | 17309696 | - | - | 59 | 4226 |
| 1250 | churn_point=1083 churn_balance=2358459 churn_window=476750 | 241.33 | 0 | 17309696 | - | - | 176 | 4226 |
| 1500 | churn_point=1083 churn_balance=1893833 churn_window=493541 | 243.08 | 0 | 17309696 | - | - | 156 | 4226 |
| 1750 | churn_point=1083 churn_balance=1519208 churn_window=502333 | 208.23 | 0 | 17309696 | - | - | 201 | 4226 |
| 2000 | churn_point=1083 churn_balance=1209250 churn_window=504667 | 221.56 | 0 | 17309696 | - | - | 194 | 4226 |
| 2250 | churn_point=1042 churn_balance=967208 churn_window=510042 | 222.59 | 0 | 17309696 | - | - | 188 | 4226 |
| 2500 | churn_point=1083 churn_balance=765292 churn_window=518084 | 219.66 | 0 | 17309696 | - | - | 192 | 4226 |
| 2750 | churn_point=1083 churn_balance=607375 churn_window=524792 | 219.61 | 0 | 17309696 | - | - | 210 | 4226 |
| 3000 | churn_point=1125 churn_balance=491000 churn_window=547625 | 217.51 | 0 | 17309696 | - | - | 208 | 4226 |
| 3250 | churn_point=1083 churn_balance=399792 churn_window=546417 | 221.81 | 0 | 17309696 | - | - | 220 | 4226 |
| 3500 | churn_point=1083 churn_balance=326916 churn_window=548125 | 222.36 | 0 | 17309696 | - | - | 230 | 4226 |
| 3750 | churn_point=1083 churn_balance=263250 churn_window=567250 | 221.44 | 0 | 17309696 | - | - | 213 | 4226 |
| 4000 | churn_point=1083 churn_balance=217458 churn_window=561666 | 221.72 | 0 | 17309696 | - | - | 214 | 4226 |
| 4250 | churn_point=1125 churn_balance=181208 churn_window=561125 | 220.85 | 0 | 17309696 | - | - | 221 | 4226 |
| 4500 | churn_point=1084 churn_balance=154250 churn_window=558500 | 220.21 | 0 | 17309696 | - | - | 211 | 4226 |
| 4750 | churn_point=1125 churn_balance=126375 churn_window=560500 | 217.74 | 0 | 17309696 | - | - | 204 | 4226 |
| 5000 | churn_point=1084 churn_balance=109250 churn_window=551000 | 220.23 | 0 | 17309696 | - | - | 199 | 4226 |
| 5250 | churn_point=1083 churn_balance=97792 churn_window=561667 | 193.86 | 0 | 17309696 | - | - | 203 | 4226 |
| 5500 | churn_point=1083 churn_balance=86084 churn_window=552750 | 220.41 | 0 | 17309696 | - | - | 203 | 4226 |
| 5750 | churn_point=1083 churn_balance=79042 churn_window=560041 | 221.12 | 0 | 17309696 | - | - | 214 | 4226 |
| 6000 | churn_point=1083 churn_balance=71500 churn_window=559750 | 221.17 | 0 | 17309696 | - | - | 222 | 4226 |
| 6250 | churn_point=1125 churn_balance=68083 churn_window=555000 | 222.69 | 0 | 17309696 | - | - | 210 | 4226 |
| 6500 | churn_point=1083 churn_balance=65042 churn_window=564416 | 221.81 | 0 | 17309696 | - | - | 209 | 4226 |
| 6750 | churn_point=1083 churn_balance=64292 churn_window=560750 | 223.83 | 0 | 17309696 | - | - | 203 | 4226 |
| 7000 | churn_point=1084 churn_balance=62833 churn_window=567041 | 222.04 | 0 | 17309696 | - | - | 202 | 4226 |
| 7250 | churn_point=1083 churn_balance=61541 churn_window=570750 | 223.00 | 0 | 17309696 | - | - | 199 | 4226 |
| 7500 | churn_point=1125 churn_balance=60791 churn_window=567041 | 222.80 | 0 | 17309696 | - | - | 184 | 4226 |
| 7750 | churn_point=1083 churn_balance=58625 churn_window=571625 | 223.83 | 0 | 17309696 | - | - | 201 | 4226 |
| 8000 | churn_point=1125 churn_balance=55917 churn_window=568584 | 225.15 | 0 | 17309696 | - | - | 189 | 4226 |
| 8250 | churn_point=1084 churn_balance=53750 churn_window=567583 | 224.49 | 0 | 17309696 | - | - | 197 | 4226 |
| 8500 | churn_point=1084 churn_balance=50458 churn_window=561542 | 224.95 | 0 | 17309696 | - | - | 191 | 4226 |
| 8750 | churn_point=1125 churn_balance=53417 churn_window=573083 | 203.37 | 0 | 17309696 | - | - | 194 | 4226 |
| 9000 | churn_point=1083 churn_balance=49708 churn_window=571250 | 223.28 | 0 | 17309696 | - | - | 199 | 4226 |
| 9250 | churn_point=1083 churn_balance=48666 churn_window=565750 | 221.96 | 0 | 17309696 | - | - | 185 | 4226 |
| 9500 | churn_point=1084 churn_balance=46375 churn_window=577083 | 221.92 | 0 | 17309696 | - | - | 192 | 4226 |
| 9750 | churn_point=1084 churn_balance=46417 churn_window=586000 | 222.05 | 0 | 17309696 | - | - | 190 | 4226 |
| 10000 | churn_point=1125 churn_balance=46708 churn_window=575125 | 220.88 | 0 | 17309696 | - | - | 185 | 4226 |

## run delete-heavy (churn=512 updates=0 growth=0, working set 100000)

### ours-durable (bumbledb)

| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | freelist | pages |
|---|---|---|---|---|---|---|---|---|
| 250 | churn_point=542 churn_balance=9209 churn_window=20875 | 20.98 | 0 | 141639680 | 294 | 227999 | - | - |
| 500 | churn_point=541 churn_balance=2625 churn_window=20958 | 19.99 | 0 | 141770752 | 544 | 355999 | - | - |
| 750 | churn_point=542 churn_balance=917 churn_window=20250 | 20.06 | 0 | 141770752 | 794 | 483999 | - | - |
| 1000 | churn_point=584 churn_balance=450 churn_window=21208 | 19.98 | 0 | 141770752 | 1044 | 611999 | - | - |
| 1250 | churn_point=542 churn_balance=338 churn_window=20167 | 20.17 | 0 | 141770752 | 1294 | 739999 | - | - |
| 1500 | churn_point=542 churn_balance=286 churn_window=19709 | 20.14 | 0 | 141770752 | 1544 | 867999 | - | - |
| 1750 | churn_point=583 churn_balance=294 churn_window=22125 | 19.88 | 0 | 141770752 | 1794 | 995999 | - | - |
| 2000 | churn_point=542 churn_balance=296 churn_window=21000 | 19.87 | 0 | 141770752 | 2044 | 1123999 | - | - |
| 2250 | churn_point=583 churn_balance=283 churn_window=20875 | 19.82 | 0 | 141770752 | 2294 | 1251999 | - | - |
| 2500 | churn_point=542 churn_balance=286 churn_window=20250 | 19.78 | 0 | 141770752 | 2544 | 1379999 | - | - |
| 2750 | churn_point=500 churn_balance=263 churn_window=20416 | 19.81 | 0 | 141770752 | 2794 | 1507999 | - | - |
| 3000 | churn_point=583 churn_balance=281 churn_window=20708 | 19.71 | 0 | 141770752 | 3044 | 1635999 | - | - |
| 3250 | churn_point=583 churn_balance=283 churn_window=20209 | 19.93 | 0 | 141770752 | 3294 | 1763999 | - | - |
| 3500 | churn_point=542 churn_balance=291 churn_window=20416 | 19.98 | 0 | 141885440 | 3544 | 1891999 | - | - |
| 3750 | churn_point=542 churn_balance=289 churn_window=20250 | 19.79 | 0 | 141885440 | 3794 | 2019999 | - | - |
| 4000 | churn_point=541 churn_balance=278 churn_window=21209 | 18.08 | 0 | 141885440 | 4044 | 2147999 | - | - |
| 4250 | churn_point=542 churn_balance=291 churn_window=19750 | 18.76 | 0 | 141885440 | 4294 | 2275999 | - | - |
| 4500 | churn_point=542 churn_balance=289 churn_window=20708 | 20.09 | 0 | 141885440 | 4544 | 2403999 | - | - |
| 4750 | churn_point=542 churn_balance=291 churn_window=19125 | 19.85 | 0 | 141885440 | 4794 | 2531999 | - | - |
| 5000 | churn_point=500 churn_balance=276 churn_window=19708 | 20.14 | 0 | 142131200 | 5044 | 2659999 | - | - |
| 5250 | churn_point=541 churn_balance=283 churn_window=19834 | 20.04 | 0 | 142131200 | 5294 | 2787999 | - | - |
| 5500 | churn_point=542 churn_balance=286 churn_window=20250 | 19.78 | 0 | 142131200 | 5544 | 2915999 | - | - |
| 5750 | churn_point=541 churn_balance=278 churn_window=20750 | 20.01 | 0 | 142131200 | 5794 | 3043999 | - | - |
| 6000 | churn_point=500 churn_balance=260 churn_window=19916 | 20.05 | 0 | 142131200 | 6044 | 3171999 | - | - |
| 6250 | churn_point=542 churn_balance=281 churn_window=20125 | 20.00 | 0 | 142131200 | 6294 | 3299999 | - | - |
| 6500 | churn_point=542 churn_balance=270 churn_window=20625 | 19.99 | 0 | 142131200 | 6544 | 3427999 | - | - |
| 6750 | churn_point=583 churn_balance=281 churn_window=19875 | 20.20 | 0 | 142131200 | 6794 | 3555999 | - | - |
| 7000 | churn_point=542 churn_balance=283 churn_window=20166 | 19.91 | 0 | 142131200 | 7044 | 3683999 | - | - |
| 7250 | churn_point=583 churn_balance=283 churn_window=21125 | 19.90 | 0 | 142131200 | 7294 | 3811999 | - | - |
| 7500 | churn_point=542 churn_balance=260 churn_window=21333 | 20.05 | 0 | 142163968 | 7544 | 3939999 | - | - |
| 7750 | churn_point=542 churn_balance=265 churn_window=20375 | 19.97 | 0 | 142163968 | 7794 | 4067999 | - | - |
| 8000 | churn_point=542 churn_balance=286 churn_window=19875 | 19.89 | 0 | 142163968 | 8044 | 4195999 | - | - |
| 8250 | churn_point=500 churn_balance=257 churn_window=19792 | 19.95 | 0 | 142163968 | 8294 | 4323999 | - | - |
| 8500 | churn_point=583 churn_balance=263 churn_window=20458 | 19.83 | 0 | 142196736 | 8544 | 4451999 | - | - |
| 8750 | churn_point=541 churn_balance=257 churn_window=20042 | 20.06 | 0 | 142196736 | 8794 | 4579999 | - | - |
| 9000 | churn_point=542 churn_balance=268 churn_window=20292 | 20.02 | 0 | 142311424 | 9044 | 4707999 | - | - |
| 9250 | churn_point=542 churn_balance=263 churn_window=20333 | 19.88 | 0 | 142311424 | 9294 | 4835999 | - | - |
| 9500 | churn_point=542 churn_balance=263 churn_window=20875 | 20.01 | 0 | 142311424 | 9544 | 4963999 | - | - |
| 9750 | churn_point=542 churn_balance=270 churn_window=20542 | 19.90 | 0 | 142344192 | 9794 | 5091999 | - | - |
| 10000 | churn_point=583 churn_balance=289 churn_window=20208 | 19.84 | 0 | 142344192 | 10044 | 5219999 | - | - |

### sqlite-bare (sqlite)

| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | freelist | pages |
|---|---|---|---|---|---|---|---|---|
| 250 | churn_point=1125 churn_balance=2197125 churn_window=502083 | 28.20 | 0 | 17346560 | - | - | 153 | 4235 |
| 500 | churn_point=1167 churn_balance=667291 churn_window=551292 | 27.68 | 0 | 17346560 | - | - | 235 | 4235 |
| 750 | churn_point=1167 churn_balance=219458 churn_window=554875 | 27.71 | 0 | 17346560 | - | - | 221 | 4235 |
| 1000 | churn_point=1167 churn_balance=92792 churn_window=568125 | 27.58 | 0 | 17346560 | - | - | 230 | 4235 |
| 1250 | churn_point=1166 churn_balance=63125 churn_window=555875 | 27.58 | 0 | 17346560 | - | - | 226 | 4235 |
| 1500 | churn_point=1167 churn_balance=50208 churn_window=546125 | 27.70 | 0 | 17346560 | - | - | 210 | 4235 |
| 1750 | churn_point=1167 churn_balance=49916 churn_window=580791 | 27.49 | 0 | 17346560 | - | - | 203 | 4235 |
| 2000 | churn_point=1166 churn_balance=52292 churn_window=572291 | 27.54 | 0 | 17346560 | - | - | 210 | 4235 |
| 2250 | churn_point=1167 churn_balance=49708 churn_window=560458 | 27.59 | 0 | 17346560 | - | - | 191 | 4235 |
| 2500 | churn_point=1167 churn_balance=50166 churn_window=556667 | 27.57 | 0 | 17346560 | - | - | 197 | 4235 |
| 2750 | churn_point=1166 churn_balance=44625 churn_window=554875 | 27.49 | 0 | 17346560 | - | - | 189 | 4235 |
| 3000 | churn_point=1167 churn_balance=50542 churn_window=559375 | 27.62 | 0 | 17346560 | - | - | 185 | 4235 |
| 3250 | churn_point=1167 churn_balance=50458 churn_window=548833 | 27.56 | 0 | 17346560 | - | - | 194 | 4235 |
| 3500 | churn_point=1167 churn_balance=54834 churn_window=556666 | 27.46 | 0 | 17346560 | - | - | 170 | 4235 |
| 3750 | churn_point=1167 churn_balance=52250 churn_window=553917 | 27.62 | 0 | 17346560 | - | - | 182 | 4235 |
| 4000 | churn_point=1167 churn_balance=47042 churn_window=571292 | 27.45 | 0 | 17346560 | - | - | 150 | 4235 |
| 4250 | churn_point=1166 churn_balance=53584 churn_window=562792 | 27.36 | 0 | 17346560 | - | - | 137 | 4235 |
| 4500 | churn_point=1167 churn_balance=52125 churn_window=584167 | 27.33 | 0 | 17346560 | - | - | 114 | 4235 |
| 4750 | churn_point=1167 churn_balance=53125 churn_window=550750 | 27.35 | 0 | 17346560 | - | - | 121 | 4235 |
| 5000 | churn_point=1167 churn_balance=48333 churn_window=565542 | 27.35 | 0 | 17346560 | - | - | 113 | 4235 |
| 5250 | churn_point=1167 churn_balance=51416 churn_window=573375 | 27.59 | 0 | 17346560 | - | - | 110 | 4235 |
| 5500 | churn_point=1167 churn_balance=51917 churn_window=571625 | 27.18 | 0 | 17346560 | - | - | 107 | 4235 |
| 5750 | churn_point=1125 churn_balance=48875 churn_window=572500 | 27.43 | 0 | 17346560 | - | - | 114 | 4235 |
| 6000 | churn_point=1167 churn_balance=43750 churn_window=562375 | 27.45 | 0 | 17346560 | - | - | 103 | 4235 |
| 6250 | churn_point=1125 churn_balance=49125 churn_window=562625 | 27.41 | 0 | 17346560 | - | - | 121 | 4235 |
| 6500 | churn_point=1166 churn_balance=46750 churn_window=579708 | 27.44 | 0 | 17346560 | - | - | 106 | 4235 |
| 6750 | churn_point=1167 churn_balance=49334 churn_window=573000 | 27.47 | 0 | 17346560 | - | - | 121 | 4235 |
| 7000 | churn_point=1125 churn_balance=52125 churn_window=577000 | 27.54 | 0 | 17346560 | - | - | 108 | 4235 |
| 7250 | churn_point=1167 churn_balance=51375 churn_window=591292 | 27.48 | 0 | 17346560 | - | - | 115 | 4235 |
| 7500 | churn_point=1167 churn_balance=44292 churn_window=593458 | 27.55 | 0 | 17346560 | - | - | 99 | 4235 |
| 7750 | churn_point=1166 churn_balance=46958 churn_window=584333 | 27.40 | 0 | 17346560 | - | - | 107 | 4235 |
| 8000 | churn_point=1166 churn_balance=51500 churn_window=568958 | 27.37 | 0 | 17346560 | - | - | 116 | 4235 |
| 8250 | churn_point=1167 churn_balance=43958 churn_window=574416 | 27.41 | 0 | 17346560 | - | - | 105 | 4235 |
| 8500 | churn_point=1125 churn_balance=44458 churn_window=559667 | 27.46 | 0 | 17346560 | - | - | 100 | 4235 |
| 8750 | churn_point=1167 churn_balance=43125 churn_window=572125 | 27.69 | 0 | 17346560 | - | - | 110 | 4235 |
| 9000 | churn_point=1167 churn_balance=46417 churn_window=572958 | 27.58 | 0 | 17346560 | - | - | 94 | 4235 |
| 9250 | churn_point=1166 churn_balance=47041 churn_window=565959 | 27.42 | 0 | 17346560 | - | - | 106 | 4235 |
| 9500 | churn_point=1167 churn_balance=46292 churn_window=586875 | 27.59 | 0 | 17346560 | - | - | 107 | 4235 |
| 9750 | churn_point=1208 churn_balance=47500 churn_window=577625 | 27.13 | 0 | 17346560 | - | - | 98 | 4235 |
| 10000 | churn_point=1166 churn_balance=52500 churn_window=575375 | 27.25 | 0 | 17346560 | - | - | 98 | 4235 |
