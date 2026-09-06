#!/usr/bin/env python3
"""Native CPU flamegraphs and before/after comparisons.

The `native` command exports an existing Samply capture, resolving each sampled
address from preserved debug files through an owned localhost Samply server.
It charges each thread CPU delta once and preserves attribution accounting.
Folded stacks (`frameA;frameB <cpu_ns>`) feed the self-contained SVG renderer
and differential views. These are views of one native capture, not a separate
instrumentation system. Python uses only the standard library; symbolication
requires Samply. No profile is uploaded.

Subcommands (scripts/flamediff.sh drives `diff`):

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
  native <profile.json.gz> <out-prefix> [root-function-substring]
         [--workload workload.json]     CPU-weighted native SVG + attribution
  compare <before.summary.json> <after.summary.json> <out-prefix>
                                       matched sampled CPU per completed draw
  check-workload <profile.json.gz> <workload.json>
                                       reject incomplete/wrong-process captures
  analyze <existing.summary.json> <fresh-out-prefix>
                                       caller context + sampling coverage from
                                       preserved address-specific symbolication
  survey <out-prefix> --expect <family,...> <analyzed.summary.json> ...
                                       explicit coverage and cross-query costs;
                                       never a suite speedup or release verdict
"""

from collections import Counter
import gzip
import html
import json
import math
import os
import selectors
import subprocess
import sys
import tempfile
import time
import urllib.parse
import urllib.request

def parse_folded(text):
    """Folded text -> list of (frames, CPU nanoseconds).

    The final space separates the weight; Rust symbols may contain spaces.
    """
    out = []
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        head, _, weight = line.rpartition(" ")
        nanos = int(weight)
        if nanos < 0 or not head or any(not frame for frame in head.split(";")):
            raise ValueError("invalid folded CPU stack: " + line)
        out.append((head.split(";"), nanos))
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
    out = ["%-24s %12s %8s %7s" % ("frame", "self_us", "pct", "stacks")]
    for name, self_ns in rows[:n]:
        pct = 100.0 * self_ns / total if total else 0.0
        out.append("%-24s %12.3f %7.1f%% %7d"
                   % (name, self_ns / 1000.0, pct, stacks_by_name[name]))
    out.append("total self %.3f us" % (total / 1000.0))
    return "\n".join(out) + "\n"

def diff_folded(a_text, b_text):
    """`stack before after` lines over the union of both profiles' stacks,
    sorted by stack. A right split recovers the two counts."""
    def counts(text):
        result = Counter()
        for frames, weight in parse_folded(text):
            result[tuple(frames)] += weight
        return result
    a, b = counts(a_text), counts(b_text)
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
    if any(os.path.lexists(path) for path in (folded_path, svg_path)):
        raise ValueError("refusing to overwrite rendered profile: " + name)
    svg = render_svg(folded, name)
    with open(folded_path, "x", encoding="utf-8") as fh:
        fh.write(folded)
    with open(svg_path, "x", encoding="utf-8") as fh:
        fh.write(svg)
    sys.stdout.write(top_table(folded, 10))
    sys.stderr.write("flame: %s\nflame: %s\n" % (folded_path, svg_path))

def cmd_diff(a_folded, b_folded, out_dir, name):
    os.makedirs(out_dir, exist_ok=True)
    diff = diff_folded(_read(a_folded), _read(b_folded))
    diff_path = os.path.join(out_dir, name + ".diff.folded")
    svg_path = os.path.join(out_dir, name + ".diff.svg")
    if any(os.path.lexists(path) for path in (diff_path, svg_path)):
        raise ValueError("refusing to overwrite differential profile: " + name)
    svg = render_diff_svg(diff, name + " (red=grew, blue=shrank)")
    with open(diff_path, "x", encoding="utf-8") as fh:
        fh.write(diff)
    with open(svg_path, "x", encoding="utf-8") as fh:
        fh.write(svg)
    sys.stderr.write("flame: %s\nflame: %s\n" % (diff_path, svg_path))

NATIVE_ROOT = "bumbledb_bench::driver::profile::profile_read_window::"

def zip_columns(*columns):
    # macOS's system Python is 3.9 (no zip(strict=True)).
    if len({len(column) for column in columns}) > 1:
        raise ValueError("profile table columns have different lengths")
    return zip(*columns)

def table_index(index, length, label):
    # Negative indices would silently attribute an invalid index to the LAST
    # row. Reject them, along with booleans masquerading as integers.
    if type(index) is not int or not 0 <= index < length:
        raise ValueError("invalid " + label + " index: " + repr(index))
    return index

def native_frame_keys(thread):
    """Map unsymbolicated physical frames to (library index, relative address)."""
    frames = thread["frameTable"]
    if any(depth not in (None, 0) for depth in frames["inlineDepth"]):
        raise ValueError("expected raw physical frames, not already-expanded inlines")
    result = []
    for function, address, _ in zip_columns(frames["func"], frames["address"], frames["inlineDepth"]):
        function = table_index(function, len(thread["funcTable"]["resource"]), "function")
        resource = thread["funcTable"]["resource"][function]
        lib = (thread["resourceTable"]["lib"][table_index(resource, len(thread["resourceTable"]["lib"]), "resource")]
               if resource is not None and resource != -1 else None)
        if type(address) is not int:
            raise ValueError("invalid relative instruction address")
        result.append((lib, address) if lib is not None and address >= 0 else None)
    return result

def native_request(profile):
    if profile["meta"]["symbolicated"]:
        raise ValueError("expected an unsymbolicated Samply capture")
    keys = sorted({key for thread in profile["threads"]
                   for key in native_frame_keys(thread) if key is not None})
    for lib, _ in keys:
        table_index(lib, len(profile["libs"]), "library")
    return keys, {
        "memoryMap": [[lib["debugName"], lib["breakpadId"]] for lib in profile["libs"]],
        "stacks": [[list(key) for key in keys]],
    }

def native_symbols(profile_path, request, log_path):
    """Resolve *addresses*, not functions. Never consult the lossy precog cache.

    A temporary alias avoids loading an adjacent .syms.json (Samply 0.13.1
    caches one inline stack per function there). The API looks up each address
    independently. No browser/upload; the owned localhost server always exits.
    """
    samply = os.environ.get("BUMBLEDB_SAMPLY", "samply")
    with tempfile.TemporaryDirectory(prefix="bumbledb-symbols-") as scratch:
        alias = os.path.join(scratch, "capture.json.gz")
        os.symlink(os.path.abspath(profile_path), alias)
        with open(log_path, "x", encoding="utf-8") as log:
            proc = subprocess.Popen(
                [samply, "load", "--no-open", "--address", "127.0.0.1",
                 "--port", "3000+", "--symbol-dir", os.path.dirname(os.path.abspath(profile_path)), alias],
                stdout=subprocess.PIPE, stderr=log, text=True,
            )
            try:
                with selectors.DefaultSelector() as ready:
                    ready.register(proc.stdout, selectors.EVENT_READ)
                    deadline = time.monotonic() + 30
                    endpoint = None
                    while endpoint is None:
                        if time.monotonic() >= deadline:
                            raise TimeoutError("Samply symbol server did not start within 30s")
                        if not ready.select(timeout=1):
                            if proc.poll() is not None:
                                raise RuntimeError("Samply symbol server failed; inspect " + log_path)
                            continue
                        line = proc.stdout.readline()
                        if not line:
                            raise RuntimeError("Samply symbol server closed; inspect " + log_path)
                        log.write(line)
                        log.flush()
                        query = urllib.parse.parse_qs(urllib.parse.urlparse(line.strip()).query)
                        endpoint = query.get("symbolServer", [None])[0]
                url = urllib.parse.urlparse(endpoint)
                if url.scheme != "http" or url.hostname != "127.0.0.1" or url.username:
                    raise ValueError("refusing a non-local symbol server")
                # Ignore proxy environment variables even for this local request.
                client = urllib.request.build_opener(urllib.request.ProxyHandler({}))
                req = urllib.request.Request(
                    endpoint + "/symbolicate/v5", json.dumps(request).encode(),
                    {"Content-Type": "application/json"}, method="POST",
                )
                with client.open(req, timeout=60) as response:
                    return json.load(response)
            finally:
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait()
                proc.stdout.close()

def native_symbol_map(profile, keys, response):
    result = response["results"][0]
    resolved = {}
    for key, frame in zip_columns(keys, result["stacks"][0]):
        lib, address = key
        if (int(frame["module_offset"], 16) != address
                or frame["module"] != profile["libs"][lib]["debugName"]):
            raise ValueError("symbol response does not match the sampled address")
        resolved[key] = frame
    return resolved

def native_fold(profile, symbols, root, details=False):
    """Charge each CPU delta once to its sampled stack, not weight * delta.

    CPU deltas are statistical endpoint attribution, not exact function timers.
    Zero-CPU/coalesced idle records stay in diagnostics, never gain CPU weight.
    Unknown frames stay visible. Unrooted/empty stacks are accounted separately.
    """
    if not root:
        raise ValueError("a nonempty sampling root is required")
    if profile["meta"]["sampleUnits"]["threadCPUDelta"] != "µs":
        raise ValueError("unsupported CPU delta unit (expected microseconds)")
    folded, counters, threads = Counter(), Counter(), []
    support, sites, site_support = Counter(), Counter(), Counter()
    selected_times = []
    for thread in profile["threads"]:
        keys = native_frame_keys(thread)
        names, leaves = [], []
        for index, key in enumerate(keys):
            frame = symbols.get(key, {})
            function = thread["frameTable"]["func"][index]
            string_index = thread["funcTable"]["name"][table_index(function, len(thread["funcTable"]["name"]), "function name")]
            raw = thread["stringArray"][table_index(string_index, len(thread["stringArray"]), "string")]
            unknown = "[unresolved:%s:%s]" % (key, raw)
            # The API's outer function precedes its inner-to-outer inline list.
            expanded = [frame.get("function") or unknown]
            expanded.extend(inline.get("function") or "[unresolved:inline]"
                            for inline in reversed(frame.get("inlines", [])))
            # Folded-stack delimiters must not corrupt Rust [T; N] symbols.
            names.append(tuple(name.replace(";", "；").replace("\n", " ").replace("\r", " ")
                               for name in expanded))
            # Inline locations are instruction-specific. The innermost inline
            # owns self CPU; never substitute the outer function's source line.
            leaf = frame["inlines"][0] if frame.get("inlines") else frame
            leaves.append((names[-1][-1], frame.get("module"), leaf.get("file"), leaf.get("line")))
        stacks = []
        for i, (prefix, frame) in enumerate(zip_columns(thread["stackTable"]["prefix"],
                                                        thread["stackTable"]["frame"])):
            if prefix is not None:
                table_index(prefix, i, "stack prefix")
            frame = table_index(frame, len(names), "frame")
            stacks.append((stacks[prefix] if prefix is not None else ()) + names[frame])
        samples = thread["samples"]
        if samples["weightType"] != "samples":
            raise ValueError("expected a CPU sampling profile, not allocation/tracing weights")
        local = Counter()
        previous_time = None
        observed = []
        for stack, micros, weight, timestamp in zip_columns(
                samples["stack"], samples["threadCPUDelta"], samples["weight"], samples["time"]):
            if any(type(value) not in (int, float) or not math.isfinite(value) or value < 0
                   for value in (micros, weight, timestamp)):
                raise ValueError("missing or invalid CPU delta/sample weight")
            if previous_time is not None and timestamp < previous_time:
                raise ValueError("sample timestamps are not ordered within a thread")
            previous_time = timestamp
            nanos = round(micros * 1000)
            local["sample_records"] += 1
            local["sample_weight"] += weight
            local["cpu_ns"] += nanos
            if not nanos:
                local["zero_cpu_records"] += 1
            if stack is not None:
                table_index(stack, len(stacks), "sample stack")
            path = stacks[stack] if stack is not None else ()
            roots = [i for i, name in enumerate(path) if root in name]
            if details:
                observed.append((timestamp, nanos, bool(roots), bool(path)))
            if not roots:
                local["unrooted_cpu_ns"] += nanos
                if not path:
                    local["empty_stack_cpu_ns"] += nanos
                continue
            path = path[roots[0]:]
            local["selected_records"] += 1
            local["selected_cpu_ns"] += nanos
            if nanos:
                local["selected_positive_cpu_records"] += 1
                folded[path] += nanos
                support[path] += 1
                selected_times.append(timestamp)
                leaf = leaves[thread["stackTable"]["frame"][stack]]
                sites[leaf] += nanos
                site_support[leaf] += 1
                if leaf[0].startswith("[unresolved:"):
                    local["selected_leaf_unresolved_cpu_ns"] += nanos
                if not leaf[2] or leaf[3] is None:
                    local["selected_leaf_no_source_cpu_ns"] += nanos
            if any(name.startswith("[unresolved:") for name in path):
                local["selected_unresolved_cpu_ns"] += nanos
        diagnostic = {}
        if details:
            rooted_times = [stamp for stamp, _, rooted, _ in observed if rooted]
            if rooted_times:
                first, last = min(rooted_times), max(rooted_times)
                interior = [(cpu, rooted, nonempty) for stamp, cpu, rooted, nonempty in observed
                            if first <= stamp <= last]
                diagnostic["observed_root_window"] = {
                    "first_sample_ms": first, "last_sample_ms": last,
                    "sample_records": len(interior),
                    "cpu_ns": sum(cpu for cpu, _, _ in interior),
                    "unrooted_cpu_ns": sum(cpu for cpu, rooted, _ in interior if not rooted),
                    "empty_stack_cpu_ns": sum(cpu for cpu, _, nonempty in interior if not nonempty),
                    "zero_cpu_records": sum(cpu == 0 for cpu, _, _ in interior),
                }
        threads.append({"name": thread["name"], "pid": thread.get("pid"), "tid": thread["tid"],
                        **local, **diagnostic})
        counters.update(local)
    total = counters["selected_cpu_ns"]
    if total <= 0:
        raise ValueError("no positive-CPU stacks matched sampling root: " + root)
    own, inclusive, own_support, inclusive_support = Counter(), Counter(), Counter(), Counter()
    for path, weight in folded.items():
        own[path[-1]] += weight
        own_support[path[-1]] += support[path]
        for name in set(path):  # Recursive/inlined repeats do not count twice.
            inclusive[name] += weight
            inclusive_support[name] += support[path]
    def ranked(counts, records):
        return [{"function": name, "cpu_ns": weight, "pct": 100 * weight / total,
                 "positive_sample_records": records[name]}
                for name, weight in counts.most_common()]
    report = {
        "kind": "sampled-cpu-attribution", "protocol": 2, "diagnostic_only": True,
        "root": root, "weighting": "threadCPUDelta microseconds converted to ns; not multiplied by sample weight",
        "counters": dict(counters), "threads": threads,
        "selected_sample_time_ms": [min(selected_times), max(selected_times)],
        "self": ranked(own, own_support), "inclusive": ranked(inclusive, inclusive_support),
        "self_sites": [{"function": site[0], "module": site[1], "file": site[2], "line": site[3],
                        "cpu_ns": weight, "pct": 100 * weight / total,
                        "positive_sample_records": site_support[site]}
                       for site, weight in sites.most_common()],
        "limitations": ["statistical CPU endpoint attribution, not per-call latency",
                        "sample records are correlated, not independent confidence intervals",
                        "missing source lines stay unknown; LTO aliases need calling context"],
    }
    if details:
        report["self_stacks"] = [
            {"stack": list(path), "cpu_ns": weight, "positive_sample_records": support[path]}
            for path, weight in sorted(folded.items(), key=lambda item: (-item[1], item[0]))
        ]
        report["limitations"].append(
            "observed root window spans first/last rooted samples, not exact operation boundaries; "
            "unrooted CPU inside it may indicate broken unwinding or another execution scope")
    text = "".join("%s %d\n" % (";".join(path), weight) for path, weight in sorted(folded.items()))
    return text, report

def native_top(report):
    lines = ["Sampled CPU attribution (not latency; inclusive rows overlap).",
             json.dumps(report["counters"], sort_keys=True)]
    for kind in ("self", "inclusive"):
        lines.append("\n%s CPU share:" % kind)
        for row in report[kind][:30]:
            lines.append("%6.2f%%  %10.3f ms  %6d records  %s" % (
                row["pct"], row["cpu_ns"] / 1e6, row["positive_sample_records"], row["function"]))
    lines.append("\nSelf CPU source sites (innermost instruction-specific inline):")
    for row in report["self_sites"][:30]:
        location = "%s:%s" % (row["file"], row["line"]) if row["file"] else "[source unavailable]"
        lines.append("%6.2f%%  %6d records  %s  %s" % (
            row["pct"], row["positive_sample_records"], location, row["function"]))
    if "workload" in report:
        workload = report["workload"]
        lines.append("\n%s: %d completed draws, %.3f sampled CPU us/draw (NOT a latency score)" % (
            workload["family"], workload["completed_draws"], workload["sampled_cpu_ns_per_draw"] / 1000))
    return "\n".join(lines) + "\n"

def validate_native_workload(profile, workload):
    if workload.get("kind") != "native-profile-workload" or workload.get("protocol") != 1:
        raise ValueError("per-draw attribution needs a completed protocol-1 native workload report")
    if workload.get("sampling_root") != "profile_read_window":
        raise ValueError("unknown native workload sampling root")
    executable = workload.get("executable")
    if not executable or not any(lib.get("path") and os.path.realpath(lib["path"]) == os.path.realpath(executable)
                                 for lib in profile["libs"]):
        raise ValueError("workload executable is not a module in this capture")
    process = workload.get("process_id")
    if (type(process) is not int or process <= 0
            or not any(str(thread.get("pid")) == str(process) for thread in profile["threads"])):
        raise ValueError("workload process is not in this capture")
    for field in ("cycles", "draws_per_cycle", "elapsed_ns"):
        if type(workload.get(field)) is not int or workload[field] <= 0:
            raise ValueError("invalid completed workload " + field)
    for field in ("rows_per_cycle", "displace_bytes_per_draw"):
        if type(workload.get(field)) is not int or workload[field] < 0:
            raise ValueError("invalid workload " + field)
    rows = workload.get("rows_per_draw")
    if (not isinstance(rows, list) or len(rows) != workload["draws_per_cycle"]
            or any(type(count) is not int or count < 0 for count in rows)
            or sum(rows) != workload["rows_per_cycle"]):
        raise ValueError("invalid per-draw output counts")
    for field in ("family", "scale", "input_digest", "answer_digest", "binary_fingerprint"):
        if not isinstance(workload.get(field), str) or not workload[field]:
            raise ValueError("missing workload " + field)
    if type(workload.get("seed")) is not int:
        raise ValueError("missing workload seed")
    return workload["cycles"] * workload["draws_per_cycle"]

def bind_native_workload(profile, report, workload):
    draws = validate_native_workload(profile, workload)
    if report["root"] != NATIVE_ROOT:
        raise ValueError("per-draw attribution requires the complete native read window root")
    selected = [thread for thread in report["threads"] if thread.get("selected_positive_cpu_records", 0)]
    if not selected or any(str(thread.get("pid")) != str(workload["process_id"]) for thread in selected):
        raise ValueError("selected stacks belong to another workload process")
    report["workload"] = {**workload, "completed_draws": draws,
                          "sampled_cpu_ns_per_draw": report["counters"]["selected_cpu_ns"] / draws}

def read_native_profile(profile_path):
    with (gzip.open if profile_path.endswith(".gz") else open)(profile_path, "rt") as file:
        return json.load(file)

def cmd_native(profile_path, prefix, root, workload_path=None):
    suffixes = (".symbols.log", ".symbols.json", ".summary.json", ".folded", ".svg", ".top.txt")
    if any(os.path.lexists(prefix + suffix) for suffix in suffixes):
        raise ValueError("refusing to overwrite native output prefix: " + prefix)
    profile = read_native_profile(profile_path)
    keys, request = native_request(profile)
    response = native_symbols(profile_path, request, prefix + ".symbols.log")
    # Preserve address mapping and original API response, including source lines
    # and missing-module errors, even when root validation subsequently fails.
    with open(prefix + ".symbols.json", "x", encoding="utf-8") as file:
        json.dump({"request": request, "response": response}, file)
    symbols = native_symbol_map(profile, keys, response)
    folded, report = native_fold(profile, symbols, root)
    report["profile"] = os.path.abspath(profile_path)
    report["module_errors"] = response["results"][0].get("module_errors", {})
    report["modules"] = profile["libs"]
    if workload_path is not None:
        bind_native_workload(profile, report, json.loads(_read(workload_path)))
    for suffix, content in (
            (".summary.json", json.dumps(report, indent=2) + "\n"),
            (".folded", folded), (".top.txt", native_top(report)),
            (".svg", render_svg(folded, os.path.basename(prefix) + " — sampled CPU; not latency"))):
        with open(prefix + suffix, "x", encoding="utf-8") as file:
            file.write(content)
    sys.stdout.write(native_top(report))

def compare_native(before, after):
    """Matched sampled CPU per completed draw, not raw fixed-duration width."""
    for report in (before, after):
        validate_attribution(report)
    if before["root"] != after["root"]:
        raise ValueError("different sampling roots")
    a, b = before["workload"], after["workload"]
    for field in ("protocol", "family", "scale", "seed", "draws_per_cycle", "rows_per_cycle", "rows_per_draw", "displace_bytes_per_draw", "input_digest", "answer_digest"):
        if a[field] != b[field]:
            raise ValueError("incomparable workload " + field)
    rows = {}
    for kind in ("self", "inclusive"):
        left = {r["function"]: r for r in before[kind]}
        right = {r["function"]: r for r in after[kind]}
        compared = []
        for function in left.keys() | right.keys():
            old, new = left.get(function, {}), right.get(function, {})
            old_ns = old.get("cpu_ns", 0) / a["completed_draws"]
            new_ns = new.get("cpu_ns", 0) / b["completed_draws"]
            compared.append({"function": function, "before_cpu_ns_per_draw": old_ns,
                             "after_cpu_ns_per_draw": new_ns, "delta_cpu_ns_per_draw": new_ns - old_ns,
                             "before_records": old.get("positive_sample_records", 0),
                             "after_records": new.get("positive_sample_records", 0)})
        rows[kind] = sorted(compared, key=lambda row: (-abs(row["delta_cpu_ns_per_draw"]), row["function"]))
    return {"kind": "sampled-cpu-per-draw-comparison", "diagnostic_only": True,
            "before": a, "after": b, **rows,
            "before_capture": before.get("profile"), "after_capture": after.get("profile"),
            "before_counters": before["counters"], "after_counters": after["counters"],
            "limitations": ["not latency; validate with unprofiled alternating controls",
                            "unobserved frames are not proven absent; inspect sample support",
                            "matched hardware/clock conditions remain a separate requirement",
                            "renamed functions and LTO aliases may split one mechanism across rows"]}

def cmd_compare_native(before_path, after_path, prefix):
    outputs = (prefix + ".comparison.json", prefix + ".comparison.txt")
    if any(os.path.lexists(path) for path in outputs):
        raise ValueError("refusing to overwrite native comparison: " + prefix)
    comparison = compare_native(json.loads(_read(before_path)), json.loads(_read(after_path)))
    lines = ["Matched sampled CPU per completed draw (NOT latency; inclusive rows overlap)."]
    for kind in ("self", "inclusive"):
        lines.append("\n%s: before -> after us/draw (delta); supporting sample records" % kind)
        for row in comparison[kind][:40]:
            lines.append("%9.3f -> %9.3f (%+9.3f)  %5d/%-5d  %s" % (
                row["before_cpu_ns_per_draw"] / 1000, row["after_cpu_ns_per_draw"] / 1000,
                row["delta_cpu_ns_per_draw"] / 1000, row["before_records"], row["after_records"], row["function"]))
    lines.extend("\n" + limitation for limitation in comparison["limitations"])
    rendered = "\n".join(lines) + "\n"
    for path, content in zip(outputs, (json.dumps(comparison, indent=2) + "\n", rendered)):
        with open(path, "x", encoding="utf-8") as file:
            file.write(content)
    sys.stdout.write(rendered)

def validate_attribution(report):
    """Validate the evidence we aggregate, not just a plausible-looking total."""
    if (report.get("kind") != "sampled-cpu-attribution" or report.get("protocol") != 2
            or report.get("root") != NATIVE_ROOT):
        raise ValueError("analysis requires protocol-2 attribution of the complete read window")
    workload = report.get("workload", {})
    draws = validate_native_workload({"libs": report["modules"], "threads": report["threads"]}, workload)
    if type(workload.get("completed_draws")) is not int or workload["completed_draws"] != draws:
        raise ValueError("invalid completed draw denominator")
    counts = report["counters"]
    for key, value in counts.items():
        if type(value) not in (int, float) or not math.isfinite(value) or value < 0:
            raise ValueError("invalid attribution counter: " + key)
    total = counts["selected_cpu_ns"]
    records = counts["selected_positive_cpu_records"]
    if (type(total) is not int or type(records) is not int or total <= 0 or records <= 0
            or total + counts.get("unrooted_cpu_ns", 0) != counts["cpu_ns"]):
        raise ValueError("CPU attribution is empty or does not conserve captured CPU")
    for field in ("selected_unresolved_cpu_ns", "selected_leaf_unresolved_cpu_ns", "selected_leaf_no_source_cpu_ns"):
        if counts.get(field, 0) > total:
            raise ValueError("unknown/source CPU exceeds selected CPU")
    def by_function(rows):
        result = {}
        for row in rows:
            name = row["function"]
            if not isinstance(name, str) or not name or name in result:
                raise ValueError("missing/duplicate attribution function")
            for field, bound in (("cpu_ns", total), ("positive_sample_records", records)):
                if type(row[field]) is not int or not 0 < row[field] <= bound:
                    raise ValueError("invalid function CPU/sample support")
            result[name] = (row["cpu_ns"], row["positive_sample_records"])
        return result
    own = by_function(report["self"])
    by_function(report["inclusive"])
    if (sum(cpu for cpu, _ in own.values()) != total
            or sum(count for _, count in own.values()) != records):
        raise ValueError("self CPU/sample support does not conserve selected totals")
    for field, expected in (("cpu_ns", total), ("positive_sample_records", records)):
        values = [row[field] for row in report["self_sites"]]
        if any(type(value) is not int or value <= 0 for value in values) or sum(values) != expected:
            raise ValueError("self source sites do not conserve selected CPU/sample support")
    if "self_stacks" in report:
        stack_cpu, stack_records, seen = Counter(), Counter(), set()
        for row in report["self_stacks"]:
            stack = row["stack"]
            if (not isinstance(stack, list) or not stack
                    or any(not isinstance(frame, str) or not frame for frame in stack)
                    or report["root"] not in stack[0] or tuple(stack) in seen):
                raise ValueError("invalid/duplicate rooted self stack")
            seen.add(tuple(stack))
            for field in ("cpu_ns", "positive_sample_records"):
                if type(row[field]) is not int or row[field] <= 0:
                    raise ValueError("invalid stack CPU/sample support")
            stack_cpu[stack[-1]] += row["cpu_ns"]
            stack_records[stack[-1]] += row["positive_sample_records"]
        if {name: (cpu, stack_records[name]) for name, cpu in stack_cpu.items()} != own:
            raise ValueError("caller stacks do not conserve self CPU/sample support")
    return draws

def native_context(report):
    """Attribute each self sample ONCE to its nearest bumbledb caller.

    This is a view of real native stacks, not a manually instrumented phase
    taxonomy. Keep the original stacks: LTO can alias an owner's display name.
    """
    cpu, records = Counter(), Counter()
    for row in report["self_stacks"]:
        # An external generic *containing* a bumbledb type is still external
        # code (e.g. Vec<Cell>::as_slice or heed::Database<OurComparator>::get).
        # Match the declaring path, never an arbitrary type-argument substring.
        owner = next((name for name in reversed(row["stack"])
                      if name.startswith(("bumbledb::", "<bumbledb::"))),
                     "[outside bumbledb: harness/system]")
        cpu[owner] += row["cpu_ns"]
        records[owner] += row["positive_sample_records"]
    total = report["counters"]["selected_cpu_ns"]
    draws = report["workload"]["completed_draws"]
    return [{"owner": name, "cpu_ns": weight, "pct": 100 * weight / total,
             "cpu_ns_per_draw": weight / draws, "positive_sample_records": records[name]}
            for name, weight in sorted(cpu.items(), key=lambda item: (-item[1], item[0]))]

def native_quality(report):
    counts = report["counters"]
    total = counts["selected_cpu_ns"]
    windows = [thread["observed_root_window"] for thread in report["threads"]
               if "observed_root_window" in thread]
    if not windows:
        raise ValueError("sampling-window diagnostics missing; run analyze first")
    for window in windows:
        for field in ("cpu_ns", "unrooted_cpu_ns", "empty_stack_cpu_ns", "sample_records", "zero_cpu_records"):
            if type(window[field]) is not int or window[field] < 0:
                raise ValueError("invalid sampling-window CPU/support")
        if (window["empty_stack_cpu_ns"] > window["unrooted_cpu_ns"]
                or window["unrooted_cpu_ns"] > window["cpu_ns"]
                or window["zero_cpu_records"] > window["sample_records"]):
            raise ValueError("sampling-window subset exceeds its total")
    window_cpu = sum(window["cpu_ns"] for window in windows)
    gap = sum(window["unrooted_cpu_ns"] for window in windows)
    if window_cpu != total + gap:
        raise ValueError("observed root-window CPU does not conserve selected/interior unrooted CPU")
    return {"selected_cpu_to_elapsed_ratio": total / report["workload"]["elapsed_ns"],
            "observed_window_cpu_ns": window_cpu, "interior_unrooted_cpu_ns": gap,
            "interior_unrooted_cpu_pct": 100 * gap / window_cpu,
            "unresolved_stack_cpu_pct": 100 * counts.get("selected_unresolved_cpu_ns", 0) / total,
            "unresolved_leaf_cpu_pct": 100 * counts.get("selected_leaf_unresolved_cpu_ns", 0) / total,
            "missing_leaf_source_cpu_pct": 100 * counts.get("selected_leaf_no_source_cpu_ns", 0) / total}

def cmd_analyze(summary_path, prefix):
    """Reanalyze preserved instruction-address symbols without sampling again."""
    suffixes = (".summary.json", ".folded", ".svg", ".top.txt")
    if any(os.path.lexists(prefix + suffix) for suffix in suffixes):
        raise ValueError("refusing to overwrite native analysis: " + prefix)
    original = json.loads(_read(summary_path))
    validate_attribution(original)
    if not summary_path.endswith(".summary.json"):
        raise ValueError("analysis input must be an exported .summary.json")
    symbols_path = original.get("symbolication", summary_path[:-len(".summary.json")] + ".symbols.json")
    profile = read_native_profile(original["profile"])
    saved = json.loads(_read(symbols_path))
    keys, request = native_request(profile)
    if saved["request"] != request:
        raise ValueError("preserved symbolication request does not match this capture")
    symbols = native_symbol_map(profile, keys, saved["response"])
    folded, report = native_fold(profile, symbols, original["root"], details=True)
    report.update({"profile": os.path.abspath(original["profile"]),
                   "symbolication": os.path.abspath(symbols_path),
                   "source_summary": os.path.abspath(summary_path),
                   "modules": profile["libs"],
                   "module_errors": saved["response"]["results"][0].get("module_errors", {})})
    bind_native_workload(profile, report, original["workload"])
    validate_attribution(report)
    if report["counters"] != original["counters"] or report["self"] != original["self"]:
        raise ValueError("reanalysis changed CPU accounting; inspect the original evidence")
    report["quality"] = native_quality(report)
    report["self_owners"] = native_context(report)
    text = native_top(report)
    text += "\nSampling diagnostics (not pass/fail thresholds):\n" + json.dumps(report["quality"], indent=2) + "\n"
    text += "\nExclusive CPU by nearest bumbledb caller (includes inlined/system helpers beneath it):\n"
    for row in report["self_owners"][:30]:
        text += "%6.2f%%  %6d records  %s\n" % (row["pct"], row["positive_sample_records"], row["owner"])
    text += "\nHottest self call paths (last six frames shown; complete stacks in JSON):\n"
    for row in report["self_stacks"][:30]:
        text += "%6.2f%%  %6d records  %s\n" % (
            100 * row["cpu_ns"] / report["counters"]["selected_cpu_ns"],
            row["positive_sample_records"], " -> ".join(row["stack"][-6:]))
    for suffix, content in ((".summary.json", json.dumps(report, indent=2) + "\n"),
                            (".folded", folded), (".top.txt", text),
                            (".svg", render_svg(folded, os.path.basename(prefix) + " — sampled CPU; not latency"))):
        with open(prefix + suffix, "x", encoding="utf-8") as file:
            file.write(content)
    print("native analysis: " + prefix + ".top.txt")

def survey_native(reports, expected):
    """One row per expected family. Never pool raw CPU across unequal jobs."""
    if not expected or any(not isinstance(name, str) or not name for name in expected) or len(set(expected)) != len(expected):
        raise ValueError("survey needs a nonempty, duplicate-free expected family roster")
    families, owners, identity = {}, {}, None
    for report in reports:
        draws = validate_attribution(report)
        if "self_stacks" not in report:
            raise ValueError("survey needs analyzed reports with exact caller/sample support")
        workload = report["workload"]
        name = workload["family"]
        if name in families or name not in expected:
            raise ValueError("duplicate or unexpected survey family: " + name)
        modules = {(lib["debugName"], lib["breakpadId"]) for lib in report["modules"]
                   if lib.get("path") and os.path.realpath(lib["path"]) == os.path.realpath(workload["executable"])}
        current = (workload["binary_fingerprint"], tuple(sorted(modules)), workload["scale"], workload["seed"], report["root"])
        if identity is not None and current != identity:
            raise ValueError("survey mixes different binaries, module identities, scales, seeds or roots")
        identity = current
        contexts = native_context(report)
        families[name] = {"family": name, "profile": report.get("profile"),
                          "completed_draws": draws, "cycles": workload["cycles"],
                          "draws_per_cycle": workload["draws_per_cycle"],
                          "rows_per_draw": workload["rows_per_draw"],
                          "displace_bytes_per_draw": workload["displace_bytes_per_draw"],
                          "input_digest": workload["input_digest"], "answer_digest": workload["answer_digest"],
                          "cpu_ns_per_draw": report["counters"]["selected_cpu_ns"] / draws,
                          "positive_sample_records": report["counters"]["selected_positive_cpu_records"],
                          "quality": native_quality(report), "self_owners": contexts}
        for row in contexts:
            owners.setdefault(row["owner"], []).append({"family": name, **row})
    ranked = [{"owner": owner, "families_observed": len(rows),
               "mean_family_cpu_pct": sum(row["pct"] for row in rows) / len(families),
               "members": sorted(rows, key=lambda row: (-row["pct"], row["family"]))}
              for owner, rows in owners.items()]
    missing = [name for name in expected if name not in families]
    return {"kind": "native-cpu-survey", "protocol": 1, "diagnostic_only": True,
            "expected": expected, "missing": missing, "coverage_complete": not missing,
            "identity": identity, "families": [families[name] for name in expected if name in families],
            "owners": sorted(ranked, key=lambda row: (-row["mean_family_cpu_pct"], row["owner"])),
            "limitations": ["coverage_complete applies ONLY to the supplied roster; not whole-suite or release qualification",
                            "each captured family has equal weight in mean_family_cpu_pct; not a production traffic mix or speedup",
                            "sampled CPU per completed draw is not latency; separate unprofiled controls are required",
                            "low CPU/elapsed can reflect descheduling, blocking or missing attribution, not an identified I/O cause",
                            "interior unrooted CPU tests unwinding coverage only between first/last observed rooted samples",
                            "sample support is correlated; zero observations do not prove absence",
                            "nearest engine caller includes helper self CPU once; original stacks are authoritative for LTO aliases"]}

def cmd_survey(prefix, expected, paths):
    outputs = (prefix + ".survey.json", prefix + ".survey.md")
    if any(os.path.lexists(path) for path in outputs):
        raise ValueError("refusing to overwrite native survey: " + prefix)
    report = survey_native([json.loads(_read(path)) for path in paths], expected)
    def cell(value):
        return str(value).replace("|", "\\|").replace("\n", " ").replace("\r", " ")
    lines = ["# Native CPU survey", "", "%d/%d expected families captured. **Diagnostic only, not a performance verdict.**" % (
             len(report["families"]), len(expected)), ""]
    if report["missing"]:
        lines.extend(["Missing: " + ", ".join(cell(name) for name in report["missing"]), ""])
    lines.extend(["## Per-family coverage and cost", "",
                  "CPU/draw includes the real read window and, where recorded, foreign-stream conditioning. CPU/elapsed is not an I/O diagnosis. Unrooted is CPU inside the observed first/last-rooted-sample bracket. Source missing includes resolved system code without lines.", "",
                  "| Family | Draws | CPU µs/draw | Positive records | CPU/elapsed | Interior unrooted % | Unknown leaf % | Missing source % |", "|---|---:|---:|---:|---:|---:|---:|---:|"])
    for row in report["families"]:
        quality = row["quality"]
        lines.append("| %s | %d | %.3f | %d | %.3f | %.3f | %.3f | %.2f |" % (
            cell(row["family"]), row["completed_draws"], row["cpu_ns_per_draw"] / 1000,
            row["positive_sample_records"], quality["selected_cpu_to_elapsed_ratio"],
            quality["interior_unrooted_cpu_pct"], quality["unresolved_leaf_cpu_pct"], quality["missing_leaf_source_cpu_pct"]))
    lines.extend(["", "## Shared caller costs", "",
                  "Each self sample belongs once to its nearest bumbledb caller; helpers are included. Mean share gives each captured family equal weight. This is an investigation ordering, not an application-wide score. All owners and per-family sample support are retained in JSON.", "",
                  "| Nearest engine caller | Mean family CPU % | Families observed | Largest family shares |", "|---|---:|---:|---|"])
    for row in report["owners"][:30]:
        members = "; ".join("%s %.1f%% (%d records)" % (member["family"], member["pct"], member["positive_sample_records"])
                            for member in row["members"][:3])
        lines.append("| %s | %.3f | %d | %s |" % (cell(row["owner"]), row["mean_family_cpu_pct"], row["families_observed"], cell(members)))
    lines.extend(["", "## Every family's next inspection", ""])
    for row in report["families"]:
        lines.extend(["### " + cell(row["family"]), ""])
        if row["displace_bytes_per_draw"]:
            lines.extend(["Includes %d bytes of foreign-stream conditioning per draw." % row["displace_bytes_per_draw"], ""])
        for owner in row["self_owners"][:5]:
            lines.append("- %.2f%%, %.3f CPU µs/draw, %d records: %s" % (
                owner["pct"], owner["cpu_ns_per_draw"] / 1000, owner["positive_sample_records"], cell(owner["owner"])))
        lines.append("")
    lines.extend(["## Interpretation limits", ""] + ["- " + limitation for limitation in report["limitations"]])
    for path, content in zip(outputs, (json.dumps(report, indent=2) + "\n", "\n".join(lines) + "\n")):
        with open(path, "x", encoding="utf-8") as file:
            file.write(content)
    print("native survey: %d/%d families; %s" % (len(report["families"]), len(expected), outputs[1]))
    return int(bool(report["missing"]))

def _native_selftest():
    # Two instruction addresses in one machine function have DIFFERENT inline
    # stacks. Recursion, coalesced idle weights and missing stacks must conserve
    # CPU attribution. Keep this tiny constructed table beside the parser.
    thread = {
        "name": "test", "pid": "123", "tid": 1,
        "frameTable": {"func": [0, 1, 2, 3], "address": [10, 20, 21, 30], "inlineDepth": [0] * 4},
        "funcTable": {"resource": [0] * 4, "name": [0, 1, 2, 3]},
        "resourceTable": {"lib": [0]}, "stringArray": ["root", "0x14", "0x15", "0x1e"],
        "stackTable": {"prefix": [None, 0, 0, 0, 1], "frame": [0, 1, 2, 3, 1]},
        "samples": {"weightType": "samples", "stack": [1, 2, 3, None, 0, 4],
                    "threadCPUDelta": [2, 3, 5, 7, 0, 11],
                    "weight": [100, 1, 1, 1, 50, 1], "time": [1, 2, 3, 4, 5, 6]},
    }
    profile = {"meta": {"symbolicated": False, "sampleUnits": {"threadCPUDelta": "µs"}},
               "libs": [{"debugName": "test", "breakpadId": "id", "path": "/test/binary"}], "threads": [thread]}
    keys, request = native_request(profile)
    assert request == {"memoryMap": [["test", "id"]], "stacks": [[[0, 10], [0, 20], [0, 21], [0, 30]]]}
    response = {"results": [{"stacks": [[
        {"module_offset": "0xa", "module": "test", "function": "root"},
        {"module_offset": "0x14", "module": "test", "function": "hot",
         "file": "outer.rs", "line": 100,
         "inlines": [{"function": "inner_a", "file": "actual.rs", "line": 7}, {"function": "outer"}]},
        {"module_offset": "0x15", "module": "test", "function": "hot",
         "inlines": [{"function": "inner_b<[u64; 4]>", "file": "actual.rs", "line": 8}]},
        {"module_offset": "0x1e", "module": "test"},
    ]]}]}
    symbols = native_symbol_map(profile, keys, response)
    folded, report = native_fold(profile, symbols, "root")
    report["modules"] = profile["libs"]
    assert "root;hot;outer;inner_a 2000\n" in folded
    assert "root;hot;inner_b<[u64； 4]> 3000\n" in folded
    assert "root;hot;outer;inner_a;hot;outer;inner_a 11000\n" in folded
    assert sum(weight for _, weight in parse_folded(folded)) == 21000
    counts = report["counters"]
    assert counts["cpu_ns"] == 28000
    assert counts["selected_cpu_ns"] + counts["unrooted_cpu_ns"] == counts["cpu_ns"]
    assert counts["empty_stack_cpu_ns"] == 7000
    assert counts["selected_unresolved_cpu_ns"] == 5000
    assert counts["selected_leaf_unresolved_cpu_ns"] == 5000
    assert counts["selected_leaf_no_source_cpu_ns"] == 5000
    assert counts["zero_cpu_records"] == 1
    assert counts["selected_positive_cpu_records"] == 4
    assert {row["function"]: row["cpu_ns"] for row in report["inclusive"]}["hot"] == 16000
    assert sum(row["cpu_ns"] for row in report["self"]) == 21000
    assert sum(row["cpu_ns"] for row in report["self_sites"]) == 21000
    assert sum(row["positive_sample_records"] for row in report["self"]) == 4
    site = next(row for row in report["self_sites"] if row["function"] == "inner_a")
    assert (site["file"], site["line"], site["cpu_ns"], site["positive_sample_records"]) == ("actual.rs", 7, 13000, 2)
    def refuses(call):
        try:
            call()
        except ValueError:
            return
        raise AssertionError("malformed/unsupported native input was accepted")
    refuses(lambda: native_fold(profile, symbols, "missing_root"))
    refuses(lambda: native_fold(profile, symbols, ""))
    refuses(lambda: zip_columns([1], []))
    # Invalid indices must never silently wrap around to another frame.
    for column in (thread["samples"]["stack"], thread["frameTable"]["func"],
                   thread["stackTable"]["frame"], thread["resourceTable"]["lib"]):
        original = column[0]
        column[0] = -1
        refuses(lambda: native_request(profile) and native_fold(profile, symbols, "root"))
        column[0] = original
    for field, value in (("threadCPUDelta", float("nan")), ("weight", float("inf")),
                         ("time", -1), ("time", 100)):
        original = thread["samples"][field][0]
        thread["samples"][field][0] = value
        refuses(lambda: native_fold(profile, symbols, "root"))
        thread["samples"][field][0] = original
    workload = {"kind": "native-profile-workload", "protocol": 1, "sampling_root": "profile_read_window",
                "executable": "/test/binary", "process_id": 123, "cycles": 5, "draws_per_cycle": 2,
                "elapsed_ns": 1000000, "rows_per_cycle": 3, "rows_per_draw": [1, 2], "displace_bytes_per_draw": 0,
                "family": "test", "scale": "S", "seed": 1,
                "input_digest": "input", "answer_digest": "answer", "binary_fingerprint": "binary"}
    report["root"] = NATIVE_ROOT
    bind_native_workload(profile, report, workload)
    assert report["workload"]["completed_draws"] == 10
    assert report["workload"]["sampled_cpu_ns_per_draw"] == 2100
    for field, value in (("process_id", 124), ("executable", "/other/binary"),
                         ("cycles", 0), ("protocol", 2), ("sampling_root", "run_join"),
                         ("rows_per_draw", [0, 1]), ("rows_per_draw", [3]), ("rows_per_draw", [True, 2])):
        refuses(lambda: bind_native_workload(profile, report, {**workload, field: value}))
    # A subtree can contain the window's spelling but represents only part
    # of the operation. It must not borrow the complete-draw denominator.
    refuses(lambda: bind_native_workload(profile, {**report, "root": NATIVE_ROOT + "one_subtree"}, workload))
    after = json.loads(json.dumps(report))
    # Equal captured CPU but twice as much completed work means HALF the CPU
    # per draw, not equal cost. Never normalize merely by profile duration.
    bind_native_workload(profile, after, {**workload, "cycles": 10})
    comparison = compare_native(report, after)
    assert all(row["after_cpu_ns_per_draw"] == row["before_cpu_ns_per_draw"] / 2 for row in comparison["self"])
    assert sum(row["delta_cpu_ns_per_draw"] for row in comparison["self"]) == -1050
    after["workload"]["answer_digest"] = "different answer"
    refuses(lambda: compare_native(report, after))
    after["workload"]["answer_digest"] = "answer"
    after["workload"]["input_digest"] = "different misses, same empty answers"
    refuses(lambda: compare_native(report, after))
    after["workload"]["input_digest"] = "input"
    after["workload"]["completed_draws"] = 0
    refuses(lambda: compare_native(report, after))
    # Caller ownership must absorb library helpers without double-charging
    # recursion. An empty/unrooted sample INSIDE the sampled window must remain
    # visible, even when every selected leaf but one is perfectly symbolicated.
    detailed_symbols = json.loads(json.dumps(response))
    detailed_symbols["results"][0]["stacks"][0][0]["function"] = NATIVE_ROOT + "test"
    for frame in detailed_symbols["results"][0]["stacks"][0][1:3]:
        frame["function"] = "bumbledb::exec::owner"
    detailed_symbols["results"][0]["stacks"][0][1]["inlines"][0]["function"] = "<alloc::vec::Vec<bumbledb::Cell>>::as_slice"
    _, detailed = native_fold(profile, native_symbol_map(profile, keys, detailed_symbols), NATIVE_ROOT, details=True)
    detailed["modules"] = profile["libs"]
    bind_native_workload(profile, detailed, workload)
    assert validate_attribution(detailed) == 10
    assert detailed["threads"][0]["observed_root_window"]["unrooted_cpu_ns"] == 7000
    assert detailed["threads"][0]["observed_root_window"]["empty_stack_cpu_ns"] == 7000
    assert native_quality(detailed)["interior_unrooted_cpu_pct"] == 25
    contexts = native_context(detailed)
    assert [(row["owner"], row["cpu_ns"], row["positive_sample_records"]) for row in contexts] == [
        ("bumbledb::exec::owner", 16000, 3), ("[outside bumbledb: harness/system]", 5000, 1)]
    second = json.loads(json.dumps(detailed))
    second["workload"]["family"] = "other"
    survey = survey_native([detailed, second], ["test", "other", "missing"])
    assert survey["missing"] == ["missing"] and not survey["coverage_complete"]
    assert survey["owners"][0]["families_observed"] == 2
    assert math.isclose(sum(row["mean_family_cpu_pct"] for row in survey["owners"]), 100)
    assert survey_native([detailed, second], ["test", "other"])["coverage_complete"]
    assert survey_native([], ["test"])["missing"] == ["test"]
    refuses(lambda: survey_native([detailed, detailed], ["test"]))
    refuses(lambda: survey_native([detailed], ["unrelated"]))
    refuses(lambda: survey_native([detailed], ["test", "test"]))
    for field, value in (("binary_fingerprint", "other-binary"), ("scale", "L"), ("seed", 2)):
        changed = {**second, "workload": {**second["workload"], field: value}}
        refuses(lambda: survey_native([detailed, changed], ["test", "other"]))
    changed = json.loads(json.dumps(detailed))
    changed["self_stacks"][0]["positive_sample_records"] += 1
    refuses(lambda: validate_attribution(changed))
    changed = json.loads(json.dumps(detailed))
    changed["self"][0]["cpu_ns"] += 1
    refuses(lambda: validate_attribution(changed))
    changed = json.loads(json.dumps(detailed))
    changed["workload"].update(cycles=1, draws_per_cycle=1, rows_per_draw=[3], completed_draws=True)
    refuses(lambda: validate_attribution(changed))
    changed = json.loads(json.dumps(detailed))
    changed["threads"][0]["observed_root_window"]["cpu_ns"] += 1
    refuses(lambda: native_quality(changed))
    changed = json.loads(json.dumps(second))
    changed["modules"][0]["breakpadId"] = "wrong-module"
    refuses(lambda: survey_native([detailed, changed], ["test", "other"]))
    # Exercise the real disk/export path without Samply or new test fixtures:
    # cached instruction-address symbols are sufficient, input evidence stays
    # intact, repeated output is refused, and the full survey is renderable.
    with tempfile.TemporaryDirectory(prefix="bumbledb-flame-test-") as scratch:
        profile_path = os.path.join(scratch, "profile.json")
        summary_path = os.path.join(scratch, "cpu.summary.json")
        prefix = os.path.join(scratch, "analyzed")
        detailed["profile"] = profile_path
        for path, value in ((profile_path, profile), (summary_path, detailed),
                            (os.path.join(scratch, "cpu.symbols.json"), {"request": request, "response": detailed_symbols})):
            with open(path, "x", encoding="utf-8") as file:
                json.dump(value, file)
        cmd_analyze(summary_path, prefix)
        exported = json.loads(_read(prefix + ".summary.json"))
        assert exported["self_stacks"] == detailed["self_stacks"]
        assert _read(prefix + ".folded")
        refuses(lambda: cmd_analyze(summary_path, prefix))
        assert cmd_survey(os.path.join(scratch, "full"), ["test"], [prefix + ".summary.json"]) == 0
        assert cmd_survey(os.path.join(scratch, "partial"), ["test", "missing"], [prefix + ".summary.json"]) == 1
        refuses(lambda: cmd_survey(os.path.join(scratch, "full"), ["test"], [prefix + ".summary.json"]))
        # A cached response for another instruction stream is never reused.
        wrong = {"request": {**request, "stacks": []}, "response": detailed_symbols}
        wrong_path = os.path.join(scratch, "wrong.symbols.json")
        wrong_summary = os.path.join(scratch, "wrong.summary.json")
        for path, value in ((wrong_path, wrong), (wrong_summary, {**detailed, "symbolication": wrong_path})):
            with open(path, "x", encoding="utf-8") as file:
                json.dump(value, file)
        refuses(lambda: cmd_analyze(wrong_summary, os.path.join(scratch, "refused")))
    profile["meta"]["sampleUnits"]["threadCPUDelta"] = "ms"
    refuses(lambda: native_fold(profile, symbols, "root"))
    profile["meta"]["sampleUnits"]["threadCPUDelta"] = "µs"
    thread["samples"]["threadCPUDelta"][0] = None
    refuses(lambda: native_fold(profile, symbols, "root"))
    thread["samples"]["threadCPUDelta"][0] = 2
    thread["stackTable"]["prefix"][0] = 0
    refuses(lambda: native_fold(profile, symbols, "root"))
    thread["stackTable"]["prefix"][0] = None
    response["results"][0]["stacks"][0][0]["module_offset"] = "0xb"
    refuses(lambda: native_symbol_map(profile, keys, response))
    thread["frameTable"]["inlineDepth"][0] = 1
    refuses(lambda: native_request(profile))

def _selftest():
    _native_selftest()
    assert parse_folded("fn with spaces;leaf 9\n") == [(["fn with spaces", "leaf"], 9)]
    assert diff_folded("a;b 2\na;b 3\n", "a;b 4\n") == "a;b 5 4\n"
    for invalid in ("a -1", ";a 1", "a; 1", "7", "a nope"):
        try:
            parse_folded(invalid)
        except ValueError:
            pass
        else:
            raise AssertionError("invalid folded stack was accepted: " + invalid)
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

    sys.stderr.write("flame selftest: OK (native address/inline/CPU accounting, svg, diff folded, diff svg)\n")
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
    elif cmd == "native":
        args = argv[2:]
        workload = None
        if len(args) >= 2 and args[-2] == "--workload":
            workload, args = args[-1], args[:-2]
        if len(args) not in (2, 3):
            raise ValueError("native needs profile, output prefix, optional root and --workload report")
        cmd_native(args[0], args[1], args[2] if len(args) == 3 else NATIVE_ROOT, workload)
    elif cmd == "compare" and len(argv) == 5:
        cmd_compare_native(argv[2], argv[3], argv[4])
    elif cmd == "check-workload" and len(argv) == 4:
        draws = validate_native_workload(read_native_profile(argv[2]), json.loads(_read(argv[3])))
        print("native workload completed: %d draws; captured process/module match" % draws)
    elif cmd == "analyze" and len(argv) == 4:
        cmd_analyze(argv[2], argv[3])
    elif cmd == "survey" and len(argv) >= 5 and argv[3] == "--expect":
        return cmd_survey(argv[2], argv[4].split(","), argv[5:])
    else:
        sys.stderr.write(__doc__)
        return 2
    return 0

if __name__ == "__main__":
    sys.exit(main(sys.argv))
