# Key-grammar corpus

The StoreKey accept and refuse sets as data. Both drivers walk these:
every `accept` spelling parses as a key, every `refuse` spelling is
refused.

| Path | What |
| --- | --- |
| `grammar.json` | Named key spellings: `accept` entries parse, `refuse` entries refuse. `why` names the refusing rule. |

## Grammar

- A key is nonempty slash-joined segments: no leading, trailing, or
  doubled slash; no `.` or `..` segment.
- A segment containing a control, format, line-separator,
  paragraph-separator, or space-separator code point (Cc, Cf, Zl, Zp,
  Zs) is refused. A format character cannot hide a reserved prefix or
  a `.lock` suffix because it cannot appear at all.
- A segment that starts with ASCII `~` is reserved; `~tmp` and
  `~lease` live there and no StoreKey can spell them. Elsewhere in a
  segment `~` is ordinary text.
- A segment ending `.lock` is refused; the match is exact bytes,
  case-sensitive (`a.LOCK` is a key).

## Refusal vocabulary

`why` is `empty | slash | dot | reserved | control | format |
separator | lock`.
