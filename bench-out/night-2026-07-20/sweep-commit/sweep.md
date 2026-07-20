T8 commit-size sweep — judgment spans by touched-parent count (ns)
world: windowed twin, ephemeral; ambient 16384 parents x 8 children/parent; seed 1; 8 samples/cell
arms: delta = today's hash-order source probes; sorted = key-sorted probe order (hash-graded child ids); win = the already-sorted window walk, both arms

  size | src p50 delta src p50 sorted sorted/delta | src min delta src min sorted | win p50 delta win p50 sorted
     4 |          3709           3708       1.000x |          2708           2458 |          8917           8750
    16 |         12000          11875       0.990x |         10708          10083 |         35333          34167
    64 |         33708          33125       0.983x |         31292          29875 |        121125         126292
   256 |        115125         114333       0.993x |        111750         110000 |        485000         500375
  1024 |        349750         308042       0.881x |        337292         301000 |       1794667        1792292
  4096 |       1181083         997000       0.844x |       1160292         977542 |       6427125        6471958
