# In-place route mechanism attempt 4 — correct, performance inconclusive

Session 50142 exited 0 at 07:02:25 UTC. p2-wide-4/ has frozen source,
driver, build output, executable and all 108 sample blocks; independent
expected answers passed throughout. Test binary SHA256:
374518fb01639438b38a588ccb08c32b90d20be672be588f47a8ef2f1b3d1789.
The temporary hook was removed and gate-5 source fingerprint revalidated:
1ced1b6ce8fbd415371de00be3f5a4d4899e50feffb1968a7a28a1c6014b0e78.
fold_row.rs SHA256 also matches its pre-hook value:
7d0ce6cd000faa905a62ffbcaeb744f569a84b3182d7a11137cf8d50d203752e.

A=preallocated staged control, B=taken-cache control, C=production in-place
emit_batch. Geometric centers of two rounds, all samples retained:

| Chunk | Key/group | C/A median | C/B median | C/A mean | C/B mean |
|---:|---:|---:|---:|---:|---:|
| 1 | 1/1 | 0.9918 | 1.1044 | 0.9863 | 1.1050 |
| 1 | 4/1 | 0.9338 | 1.1226 | 0.9345 | 1.1121 |
| 1 | 4/4 | 0.8970 | 1.0301 | 0.8979 | 1.0309 |
| 1 | 16/1 | 0.7642 | 1.1102 | 0.7715 | 1.0988 |
| 1 | 16/16 | 0.9160 | 1.1266 | 0.9010 | 1.1161 |
| 1 | 64/1 | 0.5035 | 1.0554 | 0.4995 | 1.0321 |
| 1 | 64/64 | 0.8137 | 1.0087 | 0.8142 | 1.0094 |
| 1 | 128/1 | 0.3698 | 1.0541 | 0.3717 | 1.0585 |
| 1 | 128/128 | 0.8172 | 1.0117 | 0.8053 | 1.0105 |
| 128 | 1/1 | 0.9505 | 0.9924 | 0.9555 | 0.9994 |
| 128 | 4/1 | 0.8314 | 0.9598 | 0.8284 | 0.9588 |
| 128 | 4/4 | 0.8560 | 1.0136 | 0.8564 | 1.0123 |
| 128 | 16/1 | 0.6036 | 0.9601 | 0.5995 | 0.9456 |
| 128 | 16/16 | 0.8431 | 1.0254 | 0.8399 | 1.0162 |
| 128 | 64/1 | 0.3008 | 0.9746 | 0.3022 | 0.9762 |
| 128 | 64/64 | 0.7993 | 0.9963 | 0.8019 | 0.9957 |
| 128 | 128/1 | 0.1620 | 0.9743 | 0.1619 | 0.9714 |
| 128 | 128/128 | 0.7922 | 1.0112 | 0.7902 | 1.0109 |

The wide cliff remains absent, but this does NOT prove removal of cache
take/restore improves singleton batches. C is ~10-12% slower than B in several
small singleton cases. The controls deliberately omitted emit_batch dispatch;
that conservative advantage was documented, but it confounds a causal claim
about cache movement when the sizes are this small. It does not establish that
the old complete gate-4 binary is faster than gate 5. Do not accept or reject
the production refinement from this unequal-dispatch ratio alone.

Eleven of 108 clock-boundary blocks are flagged and remain in the table:
chunk 1: 4/4 C0, 16/1 B1, 64/1 B0, 128/1 A0, 128/128 C1 and B1;
chunk 128: 4/1 B1, 16/1 C0, 16/16 B1, 64/1 C1, 64/64 A1.
Even unflagged boundaries do not exclude interior pauses. Capacity reports
are retained payload for these semantic-Elided fixtures, not RSS/Pi results.

## Matched-dispatch attempt 5 — complete

The working p2_wide.rs/p2-wide.py now add a matched in-place row-loop control,
separating it from production dispatch. New roles/order are A=staged, B=taken,
C=matched in-place, D=production emit_batch, in ABCD/DCBA order (144 blocks).
B and C both refresh shape once per batch, fold the same rows, check progress
and omit the same known dispatch; D/C measures the additional production
dispatch/code-generation boundary separately. All arms share the current
numeric helper and have independent expected answers. This is still not an
impersonation of a previously frozen binary's complete machine code.

Session 56009 completed p2-wide.py 5 5, exit 0 at 07:07:20 UTC.
The frozen executable SHA256 is
5f5a398f8bf142bbeac6b8e9468e078df6ad59649b99c0d1041f240cf421000f.
All 144 sample blocks and independent expected outputs passed. Sixteen boundary
windows are flagged and retained in p2-wide-5/measure.log; none is normalized
or omitted. All arms share the current arithmetic helper, not the old complete
binary's machine code. The temporary hook was removed and gate-5 source and
fold_row.rs hashes above revalidated, including at the next continuation.

Descriptive geometric centers of round medians, singleton chunks:

| Keys/groups | In-place/taken C/B | Production/in-place D/C |
|---:|---:|---:|
| 1/1 | 0.9829 | 1.1268 |
| 4/1 | 0.9617 | 1.1664 |
| 4/4 | 0.9717 | 1.0872 |
| 16/1 | 0.9628 | 1.1543 |
| 16/16 | 0.9924 | 1.0242 |
| 64/64 | 0.9973 | 1.0131 |
| 128/128 | 1.0084 | 1.0007 |

128-row chunk ratios are mostly near unity. The wide-row cliff stays absent.
Cache movement is not the primary singleton overhead; additional production
dispatch/code generation is a larger signal. This does not yet accept P2.

Static ordinary gate-5 inspection completed in p2-codegen-1/: the cache check
reserves 112 stack bytes and saves six register pairs before testing for a hit;
even a hit restores the entire frame. emit_batch reserves 160 bytes and calls
it. This matches the saved o4 trace's 6.23% nearest-exclusive attribution to
refresh_shape_cache and 3.12% to emit_batch (historical, not predicted savings).

Selected next change: inline only the exact slot-count/layout-content check,
and keep the existing rebuild body in a non-inlined helper. No pointer-identity
shortcut or singleton dispatch special case. A regression changes layout words
at the same address, crossing variable/constant group and outer/leaf inputs.
Gate 6 will freeze this isolated refinement before any new targeted timing.
No full trace, commit, push, release or publication has been launched.
