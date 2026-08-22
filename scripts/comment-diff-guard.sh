#!/usr/bin/env bash
# Comment-diff guard (proposals/purge/comment-gates.md hunk law):
# every touched file's non-comment token stream must be byte-equal
# across the compared revisions. Language-aware strip (string-literal
# awareness) for Rust / TypeScript / Lean / shell / TOML. A file that
# fails is reverted whole, not patched.
#
# Usage:
#   scripts/comment-diff-guard.sh <git-range>     # e.g. abc..HEAD
#   scripts/comment-diff-guard.sh --selftest
#   scripts/comment-diff-guard.sh --extract-comments <file>
set -euo pipefail

cd "$(dirname "$0")/.."

STRIPPER=$(mktemp)
trap 'rm -f "$STRIPPER"' EXIT

cat > "$STRIPPER" << 'PY'
#!/usr/bin/env python3
"""Language-aware comment strip / extract. Token stream = code bytes
after comments are removed, blank lines collapsed, trailing whitespace
stripped. Over-stripping code as a comment is equality-safe (both sides
lose the same bytes); under-stripping a real comment makes a valid
comment-only edit fail the guard, so the scanner is aggressive on
comment starters outside strings.
"""
from __future__ import annotations

import sys
from pathlib import Path


def lang_of(path: str) -> str | None:
    name = Path(path).name
    if name == "Cargo.toml" or name.endswith(".toml"):
        return "hash"
    suffix = Path(path).suffix
    return {
        ".rs": "rust",
        ".ts": "ts",
        ".tsx": "ts",
        ".js": "ts",
        ".mjs": "ts",
        ".cjs": "ts",
        ".lean": "lean",
        ".sh": "hash",
        ".bash": "hash",
    }.get(suffix)


def _is_ident(ch: str) -> bool:
    return ch.isalnum() or ch == "_"


def strip_or_extract(text: str, lang: str, extract: bool) -> str:
    if lang == "rust":
        return _c_family(text, ts=False, extract=extract)
    if lang == "ts":
        return _c_family(text, ts=True, extract=extract)
    if lang == "lean":
        return _lean(text, extract=extract)
    if lang == "hash":
        return _hash(text, extract=extract)
    raise SystemExit(f"comment-diff-guard: unknown lang {lang}")


def _emit_code(out: list[str], comments: list[str], ch: str, extract: bool) -> None:
    if not extract:
        out.append(ch)


def _emit_comment(comments: list[str], buf: list[str], line: int, extract: bool) -> None:
    if extract:
        comments.append(f"{line}:{''.join(buf)}")


def _c_family(text: str, ts: bool, extract: bool) -> str:
    n = len(text)
    i = 0
    line = 1
    out: list[str] = []
    comments: list[str] = []
    # states: code, line, block, dstr, squote, tstr
    state = "code"
    tstr_depth = 0
    raw_hashes = -1  # rust raw string: number of # after r
    comment_buf: list[str] = []
    comment_line = 1

    def start_comment(kind: str) -> None:
        nonlocal state, comment_buf, comment_line
        state = kind
        comment_buf = []
        comment_line = line

    def end_comment() -> None:
        nonlocal state
        _emit_comment(comments, comment_buf, comment_line, extract)
        comment_buf.clear()
        state = "code"

    while i < n:
        ch = text[i]
        nxt = text[i + 1] if i + 1 < n else ""

        if state == "line":
            if ch == "\n":
                end_comment()
                _emit_code(out, comments, ch, extract)
                line += 1
            else:
                comment_buf.append(ch)
            i += 1
            continue

        if state == "block":
            if ch == "*" and nxt == "/":
                i += 2
                end_comment()
                continue
            if ch == "\n":
                comment_buf.append(ch)
                _emit_comment(comments, comment_buf, comment_line, extract)
                comment_buf = []
                comment_line = line + 1
                line += 1
            else:
                comment_buf.append(ch)
            i += 1
            continue

        if state == "dstr":
            _emit_code(out, comments, ch, extract)
            if ch == "\\":
                if nxt:
                    _emit_code(out, comments, nxt, extract)
                    if nxt == "\n":
                        line += 1
                    i += 2
                    continue
            elif ch == '"':
                state = "code"
            elif ch == "\n":
                line += 1
            i += 1
            continue

        if state == "squote":
            _emit_code(out, comments, ch, extract)
            if ch == "\\":
                if nxt:
                    _emit_code(out, comments, nxt, extract)
                    if nxt == "\n":
                        line += 1
                    i += 2
                    continue
            elif ch == "'":
                state = "code"
            elif ch == "\n":
                line += 1
            i += 1
            continue

        if state == "tstr":
            _emit_code(out, comments, ch, extract)
            if ch == "\\":
                if nxt:
                    _emit_code(out, comments, nxt, extract)
                    if nxt == "\n":
                        line += 1
                    i += 2
                    continue
            elif ch == "`":
                state = "code"
            elif ch == "$" and nxt == "{":
                _emit_code(out, comments, nxt, extract)
                i += 2
                tstr_depth += 1
                state = "code"
                continue
            elif ch == "\n":
                line += 1
            i += 1
            continue

        if state == "raw":
            # rust raw string: close is " then raw_hashes times #
            _emit_code(out, comments, ch, extract)
            if ch == '"':
                ok = True
                for k in range(raw_hashes):
                    if i + 1 + k >= n or text[i + 1 + k] != "#":
                        ok = False
                        break
                if ok:
                    for k in range(raw_hashes):
                        _emit_code(out, comments, text[i + 1 + k], extract)
                    i += 1 + raw_hashes
                    raw_hashes = -1
                    state = "code"
                    continue
            if ch == "\n":
                line += 1
            i += 1
            continue

        # ---- code ----
        if ch == "\n":
            _emit_code(out, comments, ch, extract)
            line += 1
            i += 1
            if tstr_depth and False:
                pass
            continue

        # rust raw string / byte-raw: r#"..."# , br#"..."#
        if not ts and ch in "rb" and (ch != "b" or nxt in "r\""):
            # possible raw or byte string; only treat as raw if r#*" or br#*"
            j = i
            if text[j] == "b":
                j += 1
            if j < n and text[j] == "r":
                j += 1
                hashes = 0
                while j < n and text[j] == "#":
                    hashes += 1
                    j += 1
                if j < n and text[j] == '"':
                    for k in range(i, j + 1):
                        _emit_code(out, comments, text[k], extract)
                    i = j + 1
                    raw_hashes = hashes
                    state = "raw"
                    continue

        if ch == "/" and nxt == "/":
            start_comment("line")
            i += 2
            continue
        if ch == "/" and nxt == "*":
            start_comment("block")
            i += 2
            continue

        if ch == '"':
            _emit_code(out, comments, ch, extract)
            state = "dstr"
            i += 1
            continue

        if ts and ch == "'":
            _emit_code(out, comments, ch, extract)
            state = "squote"
            i += 1
            continue

        if ts and ch == "`":
            _emit_code(out, comments, ch, extract)
            state = "tstr"
            i += 1
            continue

        # rust lifetime or char: 'a  vs  'x'
        if not ts and ch == "'":
            _emit_code(out, comments, ch, extract)
            i += 1
            if i < n and _is_ident(text[i]):
                # lifetime 'foo or char 'x' — consume ident; if next is '
                # it is a char literal closer
                while i < n and _is_ident(text[i]):
                    _emit_code(out, comments, text[i], extract)
                    i += 1
                if i < n and text[i] == "'":
                    _emit_code(out, comments, text[i], extract)
                    i += 1
            elif i < n and text[i] == "\\":
                # '\n'
                _emit_code(out, comments, text[i], extract)
                i += 1
                if i < n:
                    _emit_code(out, comments, text[i], extract)
                    i += 1
                if i < n and text[i] == "'":
                    _emit_code(out, comments, text[i], extract)
                    i += 1
            continue

        # template interpolation closer
        if ts and ch == "}" and tstr_depth:
            _emit_code(out, comments, ch, extract)
            tstr_depth -= 1
            state = "tstr"
            i += 1
            continue

        _emit_code(out, comments, ch, extract)
        i += 1

    if state in ("line", "block") and comment_buf:
        _emit_comment(comments, comment_buf, comment_line, extract)

    if extract:
        return "\n".join(comments)
    return _normalize_code("".join(out))


def _lean(text: str, extract: bool) -> str:
    n = len(text)
    i = 0
    line = 1
    out: list[str] = []
    comments: list[str] = []
    state = "code"
    nest = 0
    comment_buf: list[str] = []
    comment_line = 1

    def end_comment() -> None:
        nonlocal state, nest
        _emit_comment(comments, comment_buf, comment_line, extract)
        comment_buf.clear()
        state = "code"
        nest = 0

    while i < n:
        ch = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if state == "line":
            if ch == "\n":
                end_comment()
                if not extract:
                    out.append(ch)
                line += 1
            else:
                comment_buf.append(ch)
            i += 1
            continue
        if state == "block":
            if ch == "-" and nxt == "/":
                nest -= 1
                i += 2
                if nest == 0:
                    end_comment()
                else:
                    comment_buf.append("-/")
                continue
            if ch == "/" and nxt == "-":
                nest += 1
                comment_buf.append("/-")
                i += 2
                continue
            if ch == "\n":
                comment_buf.append(ch)
                _emit_comment(comments, comment_buf, comment_line, extract)
                comment_buf.clear()
                comment_line = line + 1
                line += 1
            else:
                comment_buf.append(ch)
            i += 1
            continue
        if state == "dstr":
            if not extract:
                out.append(ch)
            if ch == "\\":
                if nxt:
                    if not extract:
                        out.append(nxt)
                    if nxt == "\n":
                        line += 1
                    i += 2
                    continue
            elif ch == '"':
                state = "code"
            elif ch == "\n":
                line += 1
            i += 1
            continue
        # code
        if ch == "-" and nxt == "-":
            state = "line"
            comment_buf = []
            comment_line = line
            i += 2
            continue
        if ch == "/" and nxt == "-":
            state = "block"
            nest = 1
            comment_buf = []
            comment_line = line
            i += 2
            continue
        if ch == '"':
            if not extract:
                out.append(ch)
            state = "dstr"
            i += 1
            continue
        if ch == "\n":
            if not extract:
                out.append(ch)
            line += 1
            i += 1
            continue
        if not extract:
            out.append(ch)
        i += 1

    if state in ("line", "block") and comment_buf:
        _emit_comment(comments, comment_buf, comment_line, extract)
    if extract:
        return "\n".join(comments)
    return _normalize_code("".join(out))


def _hash(text: str, extract: bool) -> str:
    n = len(text)
    i = 0
    line = 1
    out: list[str] = []
    comments: list[str] = []
    state = "code"
    comment_buf: list[str] = []
    comment_line = 1
    at_bol = True

    def end_comment() -> None:
        nonlocal state
        _emit_comment(comments, comment_buf, comment_line, extract)
        comment_buf.clear()
        state = "code"

    while i < n:
        ch = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if state == "line":
            if ch == "\n":
                end_comment()
                if not extract:
                    out.append(ch)
                line += 1
                at_bol = True
            else:
                comment_buf.append(ch)
            i += 1
            continue
        if state == "dstr":
            if not extract:
                out.append(ch)
            if ch == "\\":
                if nxt:
                    if not extract:
                        out.append(nxt)
                    if nxt == "\n":
                        line += 1
                    i += 2
                    continue
            elif ch == '"':
                state = "code"
            elif ch == "\n":
                line += 1
                at_bol = True
            i += 1
            continue
        if state == "squote":
            if not extract:
                out.append(ch)
            if ch == "'":
                state = "code"
            elif ch == "\n":
                line += 1
                at_bol = True
            i += 1
            continue
        # code
        if ch == "#" and (at_bol or (i > 0 and text[i - 1] in " \t")):
            # $# and ${# are not comments
            prev = text[i - 1] if i else ""
            if prev == "$":
                if not extract:
                    out.append(ch)
                at_bol = False
                i += 1
                continue
            state = "line"
            comment_buf = []
            comment_line = line
            i += 1
            continue
        if ch == '"':
            if not extract:
                out.append(ch)
            state = "dstr"
            at_bol = False
            i += 1
            continue
        if ch == "'":
            if not extract:
                out.append(ch)
            state = "squote"
            at_bol = False
            i += 1
            continue
        if ch == "\n":
            if not extract:
                out.append(ch)
            line += 1
            at_bol = True
            i += 1
            continue
        if ch not in " \t":
            at_bol = False
        if not extract:
            out.append(ch)
        i += 1

    if state == "line" and comment_buf:
        _emit_comment(comments, comment_buf, comment_line, extract)
    if extract:
        return "\n".join(comments)
    return _normalize_code("".join(out))


def _normalize_code(text: str) -> str:
    lines = [ln.rstrip() for ln in text.splitlines()]
    kept = [ln for ln in lines if ln != ""]
    if not kept:
        return ""
    return "\n".join(kept) + "\n"


def main() -> None:
    if len(sys.argv) < 2:
        raise SystemExit("usage: strip.py --strip|--extract <lang> <path|->")
    mode = sys.argv[1]
    lang = sys.argv[2]
    src = sys.argv[3] if len(sys.argv) > 3 else "-"
    text = sys.stdin.read() if src == "-" else Path(src).read_text(encoding="utf-8")
    extract = mode == "--extract"
    sys.stdout.write(strip_or_extract(text, lang, extract))


if __name__ == "__main__":
    main()
PY

fail=0

strip_file() {
  local lang="$1" file="$2"
  python3 "$STRIPPER" --strip "$lang" "$file"
}

extract_file() {
  local lang="$1" file="$2"
  python3 "$STRIPPER" --extract "$lang" "$file"
}

selftest() {
  local root="scripts/fixtures/comment-diff-guard"
  local lang file_a file_b stripped_a stripped_b
  local failed=0
  for pair in rust:rs ts:ts lean:lean hash:sh; do
    lang="${pair%%:*}"
    ext="${pair##*:}"
    file_a="$root/${lang}.before.${ext}"
    file_b="$root/${lang}.after.${ext}"
    if [ ! -f "$file_a" ] || [ ! -f "$file_b" ]; then
      echo "comment-diff-guard: FAIL — missing fixture pair $file_a / $file_b" >&2
      failed=1
      continue
    fi
    stripped_a=$(strip_file "$lang" "$file_a")
    stripped_b=$(strip_file "$lang" "$file_b")
    if [ "$stripped_a" != "$stripped_b" ]; then
      echo "comment-diff-guard: FAIL — $lang fixture pair token streams differ" >&2
      diff -u <(printf '%s' "$stripped_a") <(printf '%s' "$stripped_b") >&2 || true
      failed=1
    else
      echo "comment-diff-guard: OK — $lang fixture pair"
    fi
  done

  # Negative: a code-token change must be detected.
  local neg="$root/rust.changed.rs"
  if [ ! -f "$neg" ]; then
    echo "comment-diff-guard: FAIL — missing negative fixture $neg" >&2
    failed=1
  else
    stripped_a=$(strip_file rust "$root/rust.before.rs")
    stripped_b=$(strip_file rust "$neg")
    if [ "$stripped_a" = "$stripped_b" ]; then
      echo "comment-diff-guard: FAIL — negative fixture was not detected as a code change" >&2
      failed=1
    else
      echo "comment-diff-guard: OK — rust negative fixture detected"
    fi
  fi

  if [ "$failed" -ne 0 ]; then
    echo "comment-diff-guard: FAIL — selftest" >&2
    exit 1
  fi
  echo "comment-diff-guard: OK — selftest (4 language pairs + negative)"
}

range_guard() {
  local range="$1"
  local files file lang before after tmp_a tmp_b
  if ! git rev-parse --verify "${range%%..*}" >/dev/null 2>&1; then
    echo "comment-diff-guard: FAIL — cannot resolve range '$range'" >&2
    exit 1
  fi
  files=$(git diff --name-only "$range")
  if [ -z "$files" ]; then
    echo "comment-diff-guard: OK — no files in $range"
    return 0
  fi
  local scanned=0
  while IFS= read -r file; do
    [ -n "$file" ] || continue
    lang=$(python3 -c "import sys; sys.path.insert(0, ''); from pathlib import Path
# inline lang_of
p=sys.argv[1]
name=Path(p).name
suf=Path(p).suffix
if name=='Cargo.toml' or suf=='.toml':
    print('hash')
elif suf in {'.rs'}: print('rust')
elif suf in {'.ts','.tsx','.js','.mjs','.cjs'}: print('ts')
elif suf=='.lean': print('lean')
elif suf in {'.sh','.bash'}: print('hash')
else: print('')
" "$file")
    if [ -z "$lang" ]; then
      continue
    fi
    scanned=$((scanned + 1))
    tmp_a=$(mktemp)
    tmp_b=$(mktemp)
    if ! git show "${range%%..*}:$file" > "$tmp_a" 2>/dev/null; then
      : > "$tmp_a"
    fi
    if ! git show "${range##*..}:$file" > "$tmp_b" 2>/dev/null; then
      : > "$tmp_b"
    fi
    before=$(strip_file "$lang" "$tmp_a")
    after=$(strip_file "$lang" "$tmp_b")
    rm -f "$tmp_a" "$tmp_b"
    if [ "$before" != "$after" ]; then
      echo "comment-diff-guard: FAIL — $file token stream changed in $range" >&2
      fail=1
    fi
  done <<< "$files"
  if [ "$scanned" -eq 0 ]; then
    echo "comment-diff-guard: FAIL — range $range touched no scannable source (vacuous)" >&2
    exit 1
  fi
  if [ "$fail" -ne 0 ]; then
    echo "comment-diff-guard: FAIL — $range" >&2
    exit 1
  fi
  echo "comment-diff-guard: OK — $scanned files token-equal in $range"
}

case "${1:-}" in
  --selftest)
    selftest
    ;;
  --extract-comments)
    if [ -z "${2:-}" ] || [ ! -f "$2" ]; then
      echo "comment-diff-guard: FAIL — --extract-comments needs a file" >&2
      exit 1
    fi
    lang=$(python3 -c "
from pathlib import Path
import sys
p=sys.argv[1]
name=Path(p).name
suf=Path(p).suffix
if name=='Cargo.toml' or suf=='.toml':
    print('hash')
elif suf=='.rs': print('rust')
elif suf in {'.ts','.tsx','.js','.mjs','.cjs'}: print('ts')
elif suf=='.lean': print('lean')
elif suf in {'.sh','.bash'}: print('hash')
else: print('')
" "$2")
    if [ -z "$lang" ]; then
      echo "comment-diff-guard: FAIL — unsupported file $2" >&2
      exit 1
    fi
    extract_file "$lang" "$2"
    ;;
  ""|-h|--help)
    echo "usage: $0 <git-range> | --selftest | --extract-comments <file>" >&2
    exit 2
    ;;
  *)
    range_guard "$1"
    ;;
esac
