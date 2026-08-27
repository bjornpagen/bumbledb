# Key-grammar corpus

The StoreKey accept and refuse sets as data. Both drivers walk these:
every `accept` spelling parses as a key, every `refuse` spelling is
refused.

| Path | What |
| --- | --- |
| `grammar.json` | Named key spellings: `accept` entries parse, `refuse` entries refuse. `why` names the refusing rule. |
| `tilde-family.json` | The reserved tilde table — ASCII `~`, its lookalikes, and the NFKC preimage of U+007E, one closed set. |

## Grammar

- A key is nonempty slash-joined segments: no leading, trailing, or
  doubled slash; no `.` or `..` segment.
- A segment containing a control, format, line-separator,
  paragraph-separator, or space-separator code point (Cc, Cf, Zl, Zp,
  Zs) is refused. A format character cannot hide a reserved prefix or
  a `.lock` suffix because it cannot appear at all.
- A segment whose first code point is in `tilde-family.json` is
  reserved; `~tmp` and `~lease` live there and no StoreKey can spell
  them. Elsewhere in a segment the family is ordinary text.
- A segment ending `.lock` is refused; the match is exact bytes,
  case-sensitive (`a.LOCK` is a key).

## The tilde walk

Beyond `grammar.json`'s named cases, a suite derives from
`tilde-family.json`: for every code point `P` in the table, `{P}x` and
`a/{P}x` refuse, and `x{P}` parses. The table is the one refusal set —
`ts-log/src/keys.ts` reads it at module init, and the Rust
`segment_ok` first-code-point check spells the same fifteen.

## Refusal vocabulary

`why` is `empty | slash | dot | reserved | lookalike | control |
format | separator | lock`.
