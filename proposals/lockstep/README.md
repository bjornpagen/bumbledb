# lockstep — every fact has one writer

The hard refactor pass after the cutover. The pre-release audit of the
tree found no protocol bugs — the six representations held. What it
found was a second species entirely: **facts that exist in more than one
place.** The version lives in fourteen manifests and one of them sat two
releases behind without any gate noticing. The battery lives in three
spellings (a doc, a CI file, human memory) and the spelling humans run
does not reach the crate that matters. The build lives in two cargo
workspaces, so "test everything" is a sentence with a footnote. A digest
is 32 bytes in Rust and in the TS chain, but a hex string in the TS
manifest. The deletion proof is a one-time transcript instead of a
standing gate, so a deleted grammar's *name* survived in six call sites.

Same doctrine, one level up: these are not process failures to be fixed
with more diligence. Each is a representation with two copies, and every
two-copy representation eventually disagrees — the audit caught four
already disagreeing. The fix is never "sync them better." It is **one
writer per fact**, and the duplicates become derivations or deletions.

| Doc | Decision | Dissolves |
| --- | --- | --- |
| [00-thesis.md](00-thesis.md) | The audit items are shadows of five duplicated facts | — |
| [10-one-workspace.md](10-one-workspace.md) | One cargo workspace: `bumbledb-log` joins the root; the second `[workspace]`, `Cargo.lock`, and battery blind spot die | the fake-green class |
| [20-one-version.md](20-one-version.md) | One version fact: `workspace.package` inheritance + a roster-driven gate over every npm manifest; 0.19.0; the runbook rewritten | the skew class |
| [30-one-battery.md](30-one-battery.md) | One battery artifact: a single script CI, docs, and humans all invoke; benches measure, tests assert | the three-spellings class |
| [40-one-identity.md](40-one-identity.md) | One digest representation: branded 32 bytes end-to-end in both drivers; hex only where humans read; names stop spelling dead grammars | the two-spellings-of-identity class |
| [50-proof-as-gate.md](50-proof-as-gate.md) | The deletion table becomes a standing census roster; the missing rulings get written; the 141-row audit and the receipt | the one-time-transcript class |
| [DISPATCH.md](DISPATCH.md) | The copy-paste orchestrator prompt for the whole pass | — |

**Relation to `settlement/`**: the canon ([settlement/00-canon.md](../settlement/00-canon.md)),
the traceability table, and RULINGS remain the law and the proof
artifacts — and this pass cuts settlement down to exactly those three.
The endgame's open items (green once, proof, bump, receipt) are absorbed
here and upgraded from tasks into representations; the one-encoding doc
is landed and gets absorbed into canon's text; then
`settlement/10-endgame.md`, `settlement/20-one-encoding.md`, and
`settlement/DISPATCH.md` are **deleted by this pass**
([50 §4](50-proof-as-gate.md)). The proposals directory obeys the same
one-writer invariant as the code: one law, one open campaign, zero stale
dispatches.

Reading order: [00-thesis.md](00-thesis.md) first;
[10-one-workspace.md](10-one-workspace.md) is load-bearing for 20 and 30
and lands first; [50-proof-as-gate.md](50-proof-as-gate.md) closes the
pass with the receipt.
