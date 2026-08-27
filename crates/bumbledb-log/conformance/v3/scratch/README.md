# ckpt-scratch corpus

Body goldens for the scratch lease at `~lease/ckpt-scratch` — a known
path in the reserved namespace, no LIST. The body is 33 bytes exactly:
version byte `3`, then the 32-byte checkpoint digest. Any other length
or version parses to nothing on both drivers (the scratch is a hint; an
unreadable hint is silence, not an error).

Sidecar:

```json
{ "kind": "scratch", "expect": "ok", "value": "<64 hex digest>", "hex": "<66 hex body>" }
{ "kind": "scratch", "expect": "refusal", "hex": "…" }
```

A refusal sidecar carries no refusal name: the parse yields
`None`/`null`, an untyped nothing, on both drivers.
