# id-lease counter corpus

Body goldens for the counter object at `ids/{relation:08x}/{field:04x}`
(`ok_birth` is the one birth body: a lease width of 4096). The body is
a canonical decimal ASCII u64: digits only, nonempty, no leading zero
unless the value is exactly `0`, no byte after the digits, nothing past
`u64::MAX`. Birth writes `4096`; each CAS writes the new end as bare
decimal. A non-canonical body is the typed `Counter` refusal, never a
value — refusal, not retry, because no repetition mends a disagreement
about what the bytes say.

Sidecar:

```json
{ "kind": "counter", "expect": "ok", "value": "4096", "hex": "34303936" }
{ "kind": "counter", "expect": "refusal", "refusal": "Counter", "hex": "…" }
```

`value` is a decimal string (corpus law: a u64 is never a JSON number).
`hex` is the exact `.bin` bytes.
