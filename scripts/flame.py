#!/usr/bin/env python3
"""Flamegraph rendering: folded stacks -> a self-contained SVG flamegraph,
plus the differential (two folded profiles -> a red/blue diff SVG). Hand-rolled,
stdlib-only, no network and no external flamegraph.pl — the same dependency
quarantine the rest of the estate keeps.

The input is the folded-stack format the engine already emits beside every
Chrome trace (crates/bumbledb-bench/src/trace_out/fold.rs — `<stem>.folded`,
lines `frameA;frameB <self_ns>`): the engine span tree collapsed by trace_out's
containment sweep, self time charged to each stack's terminal frame. This tool
never captures or folds anything; it re-represents an existing folded profile.
The Chrome JSON keeps loading in speedscope/chrome exactly as before — this is a
second, orthogonal view of the same capture.

Subcommands (scripts/flame.sh and scripts/flamediff.sh drive `render`/`diff`):

  render <folded|-> <out-dir> <name>   write <name>.folded (the source, copied)
                                       + <name>.svg, print the top-10 self table
  diff   <A.folded> <B.folded> <out-dir> <name>
                                       write <name>.diff.folded + <name>.diff.svg
                                       (A = before, B = after)
  svg    <folded|-> [title]            print an SVG for one folded profile
  top    <folded|-> [n]                print the top-N self-time table
  difffolded <A.folded> <B.folded>     print `stack before after` lines
  diffsvg    <diff.folded|-> [title]   print the red/blue diff SVG
  selftest                             golden folded -> SVG, folded pair -> diff
"""

import html
import os
import sys

def parse_folded(text):
    """Folded text -> list of (frames, weight). The weight is the last
    whitespace token; the stack (which never contains a space) is the rest."""
    out = []
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        head, _, weight = line.rpartition(" ")
        out.append((head.split(";"), int(weight)))
    return out

class Node:
    __slots__ = ("name", "children", "before", "after")

    def __init__(self, name):
        self.name = name
        self.children = {}
        self.before = 0   
        self.after = 0    

    def child(self, name):
        node = self.children.get(name)
        if node is None:
            node = Node(name)
            self.children[name] = node
        return node

def build_tree(stacks, slot):
    """Folds parsed folded lines into a tree, charging each line's weight to
    its terminal frame's `slot` attribute (`before` or `after`)."""
    root = Node("all")
    for frames, weight in stacks:
        node = root
        for frame in frames:
            node = node.child(frame)
        setattr(node, slot, getattr(node, slot) + weight)
    return root

def totals(node):
    """(before_total, after_total, depth) over the subtree; `depth` counts the
    synthetic root level, so the deepest drawn frame sits at `depth - 1`."""
    b, a = node.before, node.after
    depth = 0
    for kid in node.children.values():
        kb, ka, kd = totals(kid)
        b += kb
        a += ka
        depth = max(depth, kd + 1)
    return b, a, depth

WIDTH = 1200
MARGIN = 10
FRAME_H = 16
HEADER_H = 34
FONT = 12
CHAR_W = 7.0  

def _hot_color(name):
    """A deterministic warm color per frame name (the flamegraph.pl 'hot'
    palette, seeded by a stable hash instead of rand so snapshots hold)."""
    h = 2166136261
    for ch in name:
        h = ((h ^ ord(ch)) * 16777619) & 0xFFFFFFFF
    v1 = (h & 0xFF) / 255.0
    v2 = ((h >> 8) & 0xFF) / 255.0
    v3 = ((h >> 16) & 0xFF) / 255.0
    r = 205 + int(50 * v1)
    g = int(230 * v2)
    b = int(55 * v3)
    return "rgb(%d,%d,%d)" % (r, g, b)

def _diff_color(before, after, scale):
    """Red = grew (after > before, a regression), blue = shrank; intensity is
    |delta| / scale. White at no change."""
    delta = after - before
    if scale <= 0 or delta == 0:
        return "rgb(238,238,238)"
    frac = min(1.0, abs(delta) / scale)
    fade = 255 - int(210 * frac)
    if delta > 0:
        return "rgb(255,%d,%d)" % (fade, fade)
    return "rgb(%d,%d,255)" % (fade, fade)

def _rect(x, w, y, label, color, tip):
    x, w, y = round(x, 2), round(w, 2), round(y, 2)
    text = ""
    room = int((w - 6) / CHAR_W)
    if room >= 3:
        shown = label if len(label) <= room else label[:room - 2] + ".."
        text = ('<text x="%s" y="%s" font-family="monospace" '
                'font-size="%d">%s</text>'
                % (x + 3, y + FRAME_H - 4, FONT, html.escape(shown)))
    return ('<g><title>%s</title><rect x="%s" y="%s" width="%s" height="%d" '
            'fill="%s" stroke="white" stroke-width="0.5"/>%s</g>'
            % (html.escape(tip), x, y, w, FRAME_H, color, text))

def _svg_document(rects, title, max_depth):
    height = HEADER_H + (max_depth + 1) * FRAME_H + MARGIN
    parts = [
        '<?xml version="1.0" encoding="UTF-8" standalone="no"?>',
        '<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" '
        'viewBox="0 0 %d %d">' % (WIDTH, height, WIDTH, height),
        '<rect width="100%" height="100%" fill="rgb(245,245,245)"/>',
        '<text x="%d" y="20" font-family="monospace" font-size="15" '
        'font-weight="bold">%s</text>' % (MARGIN, html.escape(title)),
    ]
    parts.extend(rects)
    parts.append('</svg>')
    return "\n".join(parts) + "\n"

def _layout(node, x, avail, base_total, depth, max_depth, rects, weight_of,
            color_of, tip_of):
    """Places `node`'s children left to right (alphabetical), each width
    proportional to its weight, and recurses. Depth 0 sits at the bottom."""
    if base_total <= 0:
        return
    y = HEADER_H + (max_depth - depth) * FRAME_H
    cursor = x
    for name in sorted(node.children):
        kid = node.children[name]
        w = weight_of(kid)
        px = avail * (w / base_total)
        if px >= 0.2:
            rects.append(_rect(cursor, px, y, name, color_of(kid),
                               tip_of(name, kid, base_total)))
            _layout(kid, cursor, px, w, depth + 1, max_depth, rects,
                    weight_of, color_of, tip_of)
        cursor += px

def render_svg(folded_text, title):
    root = build_tree(parse_folded(folded_text), "before")
    total, _, depth = totals(root)
    if total <= 0:
        return _svg_document([], title + " (empty)", 0)
    max_depth = depth - 1  
    rects = []
    avail = WIDTH - 2 * MARGIN

    def weight_of(node):
        b, _, _ = totals(node)
        return b

    def tip_of(name, node, base):

        b, _, _ = totals(node)
        return "%s  %.3f us  %.1f%%" % (name, b / 1000.0, 100.0 * b / total)

    _layout(root, MARGIN, avail, total, 0, max_depth, rects,
            weight_of, lambda node: _hot_color(node.name), tip_of)
    return _svg_document(rects, title, max_depth)

def render_diff_svg(diff_text, title):
    """Widths come from profile B (the 'after' run); color encodes B-vs-A per
    frame. Frames present only in A do not appear — the diff is drawn on the
    after-profile, the flamegraph.pl differential convention."""
    root = Node("all")
    for line in diff_text.splitlines():
        line = line.strip()
        if not line:
            continue
        parts = line.split()
        before, after = int(parts[-2]), int(parts[-1])
        frames = " ".join(parts[:-2]).split(";")
        node = root
        for frame in frames:
            node = node.child(frame)
        node.before += before
        node.after += after

    _, total_after, depth = totals(root)
    if total_after <= 0:
        return _svg_document([], title + " (empty)", 0)
    max_depth = depth - 1  

    scale = 0
    stack = [root]
    while stack:
        n = stack.pop()
        b, a, _ = totals(n)
        scale = max(scale, abs(a - b))
        stack.extend(n.children.values())

    rects = []
    avail = WIDTH - 2 * MARGIN

    def weight_of(node):
        _, a, _ = totals(node)
        return a

    def color_of(node):
        b, a, _ = totals(node)
        return _diff_color(b, a, scale)

    def tip_of(name, node, base):
        b, a, _ = totals(node)
        return ("%s  %.3f -> %.3f us  (%+.3f us)"
                % (name, b / 1000.0, a / 1000.0, (a - b) / 1000.0))

    _layout(root, MARGIN, avail, total_after, 0, max_depth, rects,
            weight_of, color_of, tip_of)
    return _svg_document(rects, title, max_depth)

def top_table(folded_text, n):
    self_by_name = {}
    stacks_by_name = {}
    total = 0
    for frames, weight in parse_folded(folded_text):
        leaf = frames[-1]
        self_by_name[leaf] = self_by_name.get(leaf, 0) + weight
        stacks_by_name[leaf] = stacks_by_name.get(leaf, 0) + 1
        total += weight

    rows = sorted(self_by_name.items(), key=lambda kv: (-kv[1], kv[0]))
    out = ["%-24s %12s %8s %7s" % ("span", "self_us", "pct", "stacks")]
    for name, self_ns in rows[:n]:
        pct = 100.0 * self_ns / total if total else 0.0
        out.append("%-24s %12.3f %7.1f%% %7d"
                   % (name, self_ns / 1000.0, pct, stacks_by_name[name]))
    out.append("total self %.3f us" % (total / 1000.0))
    return "\n".join(out) + "\n"

def diff_folded(a_text, b_text):
    """`stack before after` lines over the union of both profiles' stacks,
    sorted by stack (difffolded.pl's output, with the leading stack kept
    space-free so a plain rsplit recovers the two counts)."""
    a = {tuple(f): w for f, w in parse_folded(a_text)}
    b = {tuple(f): w for f, w in parse_folded(b_text)}
    keys = sorted(set(a) | set(b))
    return "".join("%s %d %d\n" % (";".join(k), a.get(k, 0), b.get(k, 0))
                   for k in keys)

def _read(path):
    if path == "-":
        return sys.stdin.read()
    with open(path, "r", encoding="utf-8") as fh:
        return fh.read()

def cmd_render(folded_src, out_dir, name):
    folded = _read(folded_src)
    os.makedirs(out_dir, exist_ok=True)
    folded_path = os.path.join(out_dir, name + ".folded")
    svg_path = os.path.join(out_dir, name + ".svg")
    with open(folded_path, "w", encoding="utf-8") as fh:
        fh.write(folded)
    with open(svg_path, "w", encoding="utf-8") as fh:
        fh.write(render_svg(folded, name))
    sys.stdout.write(top_table(folded, 10))
    sys.stderr.write("flame: %s\nflame: %s\n" % (folded_path, svg_path))

def cmd_diff(a_folded, b_folded, out_dir, name):
    os.makedirs(out_dir, exist_ok=True)
    diff = diff_folded(_read(a_folded), _read(b_folded))
    diff_path = os.path.join(out_dir, name + ".diff.folded")
    svg_path = os.path.join(out_dir, name + ".diff.svg")
    with open(diff_path, "w", encoding="utf-8") as fh:
        fh.write(diff)
    with open(svg_path, "w", encoding="utf-8") as fh:
        fh.write(render_diff_svg(diff, name + " (red=grew, blue=shrank)"))
    sys.stderr.write("flame: %s\nflame: %s\n" % (diff_path, svg_path))

def _selftest():
    here = os.path.dirname(os.path.abspath(__file__))
    fx = os.path.join(here, "flame-fixtures")

    got_svg = render_svg(_read(os.path.join(fx, "mini.folded")), "mini")
    if got_svg != _read(os.path.join(fx, "mini.svg")):
        sys.stderr.write("SELFTEST FAIL: svg mismatch\n")
        return 1

    got_diff = diff_folded(_read(os.path.join(fx, "before.folded")),
                           _read(os.path.join(fx, "after.folded")))
    if got_diff != _read(os.path.join(fx, "diff.folded")):
        sys.stderr.write("SELFTEST FAIL: diff folded mismatch\n--- got ---\n%s"
                         % got_diff)
        return 1

    got_diff_svg = render_diff_svg(got_diff, "diff (red=grew, blue=shrank)")
    if got_diff_svg != _read(os.path.join(fx, "diff.svg")):
        sys.stderr.write("SELFTEST FAIL: diff svg mismatch\n")
        return 1

    sys.stderr.write("flame selftest: OK (svg, diff folded, diff svg)\n")
    return 0

def main(argv):
    if len(argv) < 2:
        sys.stderr.write(__doc__)
        return 2
    cmd = argv[1]
    if cmd == "render" and len(argv) == 5:
        cmd_render(argv[2], argv[3], argv[4])
    elif cmd == "diff" and len(argv) == 6:
        cmd_diff(argv[2], argv[3], argv[4], argv[5])
    elif cmd == "svg" and len(argv) in (3, 4):
        title = argv[3] if len(argv) == 4 else "flame"
        sys.stdout.write(render_svg(_read(argv[2]), title))
    elif cmd == "top" and len(argv) in (3, 4):
        n = int(argv[3]) if len(argv) == 4 else 10
        sys.stdout.write(top_table(_read(argv[2]), n))
    elif cmd == "difffolded" and len(argv) == 4:
        sys.stdout.write(diff_folded(_read(argv[2]), _read(argv[3])))
    elif cmd == "diffsvg" and len(argv) in (3, 4):
        title = argv[3] if len(argv) == 4 else "diff (red=grew, blue=shrank)"
        sys.stdout.write(render_diff_svg(_read(argv[2]), title))
    elif cmd == "selftest" and len(argv) == 2:
        return _selftest()
    else:
        sys.stderr.write(__doc__)
        return 2
    return 0

if __name__ == "__main__":
    sys.exit(main(sys.argv))
