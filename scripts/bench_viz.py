#!/usr/bin/env python3
"""Benchmark visuals: committed report artifacts -> the README charts.

Takes N run directories (each holding a report.json from `bumbledb-bench
bench`), computes the min-of-N p50 per family (the suite's merge rule),
and renders the legacy charts; the three metric-lane report flags render
one chart (or two) each from their committed report JSONs:

  bench-vs-sqlite.svg     ours vs SQLite p50 per read family (log scale)
  bench-speedup.svg       the same data as multipliers, big and readable
  bench-tails.svg         p50 -> p99 per family: tail behavior, both engines
  bench-writes.svg        the honest chart: writes + cold, where fsync physics rules
  bench-scenarios.svg     the non-ledger worlds (joins/graph/olap/points), per query
  bench-storage.svg       bytes per fact per scale/world (+ churn checkpoints)
  bench-writes-rates.svg  rows/sec per (family, batch), one panel per durability lane
  bench-curves.svg        log-log scale curves per family, fitted exponents, DNF caps
  bench-warmth.svg        cold/warm/memoized, both engines, per warmth-carrying family

Usage: python3 scripts/bench_viz.py [<run-dir> ...]
           [--scenarios <scenarios.md>] [--out-dir <dir>]
           [--storage-report <storage-report.json>]
           [--writes-report <writes-report.json>]
           [--curves-report <curves-report.json>]

`--out-dir` defaults to assets/ (the owner's ceremony path); every other
invocation should point it elsewhere. Charts render ONLY from committed
report pins — never from live runs.
Needs: matplotlib (`python3 -m pip install matplotlib`).
"""

import json
import math
import os
import sys
from pathlib import Path

import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter

# ---------------------------------------------------------------- data

READ_ORDER = [
    "point", "mandate_at_instant", "string", "entries_for_account_set",
    "balance", "containment_walk", "postings_without_tag", "mandate_overlap",
    "skew", "range", "chain", "stats", "latest_posting_per_account",
    "spread", "triangle",
    # The calendar family set (the second theory), in registry order;
    # rsvp_union_off is the elision-delta sub-measurement.
    "busy_scan", "meets_chain", "rsvp_union", "rsvp_union_off",
    "conflict_pairs", "conflict_free", "free_busy", "claim_hours",
]
WRITE_ORDER = ["commit_single", "commit_witnessed", "commit_batch",
               "cold_containment_walk", "bulk"]

OURS, THEIRS, FG, DIM, GRID, BG = (
    "#f0b429", "#8b949e", "#e6edf3", "#9da7b3", "#2d333b", "#0d1117",
)


def load(dirs):
    """Min-of-N stats per family for ours and sqlite, reads + writes.

    Values are dicts of percentile -> min-across-runs (the merge rule,
    applied per percentile)."""
    reads, writes = {}, {}
    for d in dirs:
        r = json.loads((Path(d) / "report.json").read_text())
        for table, out in ((r["reads"], reads), (r["writes"], writes)):
            for fam in table:
                slot = out.setdefault(fam["name"], {"ours": [], "theirs": []})
                slot["ours"].append(fam["ours"])
                if fam.get("theirs"):
                    slot["theirs"].append(fam["theirs"])
    def merge(rows):
        return {
            k: {
                side: {p: min(s[p] for s in samples) for p in ("p50", "p95", "p99")}
                for side, samples in vv.items() if samples
            }
            for k, vv in rows.items()
        }
    return merge(reads), merge(writes)


def load_scenarios(path):
    """Parse scenarios.md: [(scenario, query, ours_us, sqlite_us, ratio)]."""
    rows, scenario = [], None
    for line in Path(path).read_text().splitlines():
        if line.startswith("## "):
            scenario = line[3:].split(" (")[0]
        elif line.startswith("|") and scenario and "---" not in line and "query" not in line:
            cells = [c.strip() for c in line.strip("|").split("|")]
            if len(cells) >= 5:
                rows.append((scenario, cells[0], float(cells[2]), float(cells[3]),
                             float(cells[4])))
    return rows


def load_report(path):
    """One committed lane report JSON, whole."""
    return json.loads(Path(path).read_text())


def fmt_us(ns, _pos=None):
    us = ns / 1000
    if us < 10:
        return f"{us:.1f}µs"
    if us < 1000:
        return f"{us:.0f}µs"
    if us < 1_000_000:
        return f"{us / 1000:.0f}ms"
    return f"{us / 1e6:.1f}s"


def fmt_bytes(n, _pos=None):
    """Absolute store bytes: B / KiB / MiB / GiB, monospace-friendly."""
    n = float(n)
    for unit in ("B", "KiB", "MiB", "GiB"):
        if n < 1024 or unit == "GiB":
            return f"{n:.0f}{unit}" if unit == "B" else f"{n:.1f}{unit}"
        n /= 1024
    return f"{n:.1f}GiB"


def fmt_rate(v, _pos=None):
    """Rows (or commits) per second across the decades."""
    if v >= 1e6:
        return f"{v / 1e6:.1f}M/s"
    if v >= 1e3:
        return f"{v / 1e3:.1f}k/s"
    return f"{v:.0f}/s"


def fit_exponent(facts, p50s):
    """Least-squares slope of log10(p50) against log10(facts) — the
    fitted scaling exponent, over the points that HAVE stats."""
    if len(facts) < 2:
        return None
    lx = [math.log10(x) for x in facts]
    ly = [math.log10(y) for y in p50s]
    mx, my = sum(lx) / len(lx), sum(ly) / len(ly)
    den = sum((a - mx) ** 2 for a in lx)
    if den == 0:
        return None
    return sum((a - mx) * (b - my) for a, b in zip(lx, ly)) / den


def dark(ax):
    ax.set_facecolor(BG)
    for spine in ax.spines.values():
        spine.set_color(GRID)
    ax.tick_params(colors=DIM, labelsize=9)
    ax.xaxis.label.set_color(DIM)
    ax.yaxis.label.set_color(DIM)
    ax.title.set_color(FG)


def paired_bars(ax, names, table, note_ratio=True):
    ys = range(len(names))
    ours = [table[n]["ours"]["p50"] for n in names]
    theirs = [table[n]["theirs"]["p50"] if "theirs" in table[n] else None for n in names]
    ax.barh([y + 0.19 for y in ys], [t or 0 for t in theirs], height=0.34,
            color=THEIRS, label="SQLite", zorder=3)
    ax.barh([y - 0.19 for y in ys], ours, height=0.34,
            color=OURS, label="bumbledb", zorder=3)
    for y, (o, t) in enumerate(zip(ours, theirs)):
        label = fmt_us(o)
        if t and note_ratio:
            label += f"   {t / o:.0f}×" if t / o >= 10 else f"   {t / o:.1f}×"
        ax.text(o * 1.15, y - 0.19, label, va="center", fontsize=9,
                color=OURS, fontweight="bold", family="monospace")
        if t:
            ax.text(t * 1.15, y + 0.19, fmt_us(t), va="center", fontsize=8,
                    color=DIM, family="monospace")
    ax.set_yticks(list(ys), names, fontsize=10, family="monospace", color=FG)
    ax.invert_yaxis()
    ax.set_xscale("log")
    ax.xaxis.set_major_formatter(FuncFormatter(fmt_us))
    ax.grid(axis="x", color=GRID, linewidth=0.6, zorder=0)
    ax.legend(loc="lower right", facecolor=BG, edgecolor=GRID,
              labelcolor=FG, fontsize=9)


def chart_vs_sqlite(reads, out):
    names = [n for n in READ_ORDER if n in reads]
    fig, ax = plt.subplots(figsize=(9.6, 6.2), facecolor=BG)
    dark(ax)
    paired_bars(ax, names, reads)
    ax.set_xlim(2e2, 2e9)
    ax.set_title("read families · p50, min-of-3 · same corpus, oracle-verified identical results",
                 fontsize=12, loc="left", pad=14, family="monospace")
    fig.text(0.01, 0.005, "log scale — shorter is faster · S-scale ledger corpus · Apple M2 Max",
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_speedup(reads, out):
    names = [n for n in READ_ORDER if n in reads and "theirs" in reads[n]]
    ratios = [reads[n]["theirs"]["p50"] / reads[n]["ours"]["p50"] for n in names]
    fig, ax = plt.subplots(figsize=(9.6, 5.2), facecolor=BG)
    dark(ax)
    ax.barh(range(len(names)), ratios, height=0.62, color=OURS, zorder=3)
    for y, r in enumerate(ratios):
        ax.text(r * 1.04, y, f"{r:.0f}×" if r >= 10 else f"{r:.1f}×",
                va="center", fontsize=13, color=OURS, fontweight="bold",
                family="monospace")
    ax.axvline(1.0, color=DIM, linewidth=1, linestyle="--")
    ax.text(1.0, -0.8, "parity", fontsize=9, color=DIM, ha="center", family="monospace")
    ax.set_yticks(range(len(names)), names, fontsize=11, family="monospace", color=FG)
    ax.invert_yaxis()
    ax.set_xscale("log")
    ax.set_xlim(0.5, 700)
    ax.set_xticks([1, 2, 5, 10, 20, 50, 100, 200, 500],
                  ["1×", "2×", "5×", "10×", "20×", "50×", "100×", "200×", "500×"])
    ax.grid(axis="x", color=GRID, linewidth=0.6, zorder=0)
    ax.set_title("speedup over SQLite · read-family p50 multiples · min-of-3 both sides",
                 fontsize=12, loc="left", pad=14, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_tails(reads, out):
    names = [n for n in READ_ORDER if n in reads and "theirs" in reads[n]]
    fig, ax = plt.subplots(figsize=(9.6, 6.2), facecolor=BG)
    dark(ax)
    for y, n in enumerate(names):
        for side, color, dy in (("theirs", THEIRS, 0.18), ("ours", OURS, -0.18)):
            st = reads[n][side]
            ax.plot([st["p50"], st["p99"]], [y + dy, y + dy], color=color,
                    linewidth=2.2, solid_capstyle="round", zorder=3, alpha=0.85)
            ax.plot(st["p50"], y + dy, "o", ms=6, color=color, zorder=4)
            ax.plot(st["p95"], y + dy, "d", ms=4.5, color=color, zorder=4)
            ax.plot(st["p99"], y + dy, "s", ms=3.5, color=color, zorder=4)
    ax.set_yticks(range(len(names)), names, fontsize=10, family="monospace", color=FG)
    ax.invert_yaxis()
    ax.set_xscale("log")
    ax.xaxis.set_major_formatter(FuncFormatter(fmt_us))
    ax.grid(axis="x", color=GRID, linewidth=0.6, zorder=0)
    from matplotlib.lines import Line2D
    ax.legend(handles=[
        Line2D([], [], color=OURS, marker="o", label="bumbledb  p50 ● p95 ◆ p99 ■"),
        Line2D([], [], color=THEIRS, marker="o", label="SQLite"),
    ], loc="lower right", facecolor=BG, edgecolor=GRID, labelcolor=FG, fontsize=9)
    ax.set_title("tail behavior · p50 → p95 → p99 per read family, both engines",
                 fontsize=12, loc="left", pad=14, family="monospace")
    # The p50 dots for slot_booking_overlap and postings_without_tag are
    # rotation-boundary tail-maxima: their two fastest param populations
    # fill ranks 0-127 of the 256-sample rotation exactly, so nearest-rank
    # p50 = sorted[127] = the max of the fast mass — a per-process tail
    # draw (0.34-2.01 pair ratios on identical binaries), not an engine
    # mode. Mechanism + falsification evidence: the family doc comments
    # (crates/bumbledb-bench/src/{calendar/families.rs,families/read.rs}).
    fig.text(0.01, 0.005, "bimodal families (containment_walk, balance, skew, chain) show their true tails — gated on p95, published anyway",
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_scenarios(rows, out):
    fig, ax = plt.subplots(figsize=(9.6, 0.34 * len(rows) + 1.6), facecolor=BG)
    dark(ax)
    y, yticks, ylabels, seen = 0, [], [], None
    for scenario, query, ours_us, sqlite_us, _ratio in rows:
        if scenario != seen:
            seen = scenario
            ax.text(0.55, y - 0.15, scenario, fontsize=11, color=FG,
                    fontweight="bold", family="monospace")
            y += 1
        # From the raw p50 columns — the markdown's ratio rounds to 2
        # decimals, which floors the >100x queries to 0.00.
        speed = sqlite_us / ours_us if ours_us > 0 else 0
        color = OURS if speed >= 1 else "#f85149"
        ax.barh(y, speed, height=0.6, color=color, zorder=3)
        label = f"{speed:.0f}×" if speed >= 10 else f"{speed:.1f}×"
        ax.text(max(speed * 1.06, 1.15), y, label, va="center", fontsize=9,
                color=color, fontweight="bold", family="monospace")
        yticks.append(y)
        ylabels.append(query)
        y += 1
    ax.axvline(1.0, color=DIM, linewidth=1, linestyle="--")
    ax.set_yticks(yticks, ylabels, fontsize=9, family="monospace", color=FG)
    ax.set_ylim(y - 0.3, -0.7)
    ax.set_xscale("log")
    ax.set_xlim(0.4, 2500)
    ax.set_xticks([1, 3, 10, 30, 100, 300, 1000],
                  ["1×", "3×", "10×", "30×", "100×", "300×", "1000×"])
    ax.grid(axis="x", color=GRID, linewidth=0.6, zorder=0)
    ax.set_title("scenario worlds · speedup over SQLite per query · oracle-gated, non-ledger corpora",
                 fontsize=12, loc="left", pad=14, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_writes(writes, out):
    names = [n for n in WRITE_ORDER if n in writes]
    fig, ax = plt.subplots(figsize=(9.6, 3.4), facecolor=BG)
    dark(ax)
    paired_bars(ax, names, writes, note_ratio=False)
    ax.set_xlim(5e5, 8e9)
    ax.set_title("write + cold families · p50 · where fsync physics rules, honesty does too",
                 fontsize=12, loc="left", pad=14, family="monospace")
    fig.text(0.01, 0.005, "durable commits are an fsync-latency product on both engines; shown as measured",
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


# -------------------------------------------------- the metric lanes


def chart_storage(report, out):
    """bench-storage.svg: bytes per fact per scale, one panel per world
    (engine compacted vs sqlite indexed vs sqlite table-only), absolute
    store bytes annotated; churn checkpoints, when the report carries
    them, as an extra panel of absolute post-state bytes."""
    scales = report["scales"]
    churn = report.get("churn") or []
    worlds = [w["world"] for w in scales[0]["worlds"]] if scales else []
    panels = len(worlds) + (1 if churn else 0)
    fig, axes = plt.subplots(panels, 1, facecolor=BG,
                             figsize=(9.6, 1.1 * max(len(scales), 2) * max(len(worlds), 1)
                                      + (1.4 if churn else 0) + 2.2))
    axes = [axes] if panels == 1 else list(axes)

    lanes = (
        ("engine (compacted)", "engine_bytes_per_fact", "engine_compacted_bytes",
         OURS, None, 1.0, -0.26),
        ("sqlite indexed", "sqlite_indexed_bytes_per_fact", "sqlite_indexed_bytes",
         THEIRS, None, 1.0, 0.0),
        ("sqlite table-only", "sqlite_tableonly_bytes_per_fact", "sqlite_tableonly_bytes",
         THEIRS, "///", 0.55, 0.26),
    )
    for ax, world in zip(axes, worlds):
        dark(ax)
        rows = [(s["scale"], next(w for w in s["worlds"] if w["world"] == world))
                for s in scales]
        ys = range(len(rows))
        peak = max(w[k] for _, w in rows
                   for k in ("engine_bytes_per_fact", "sqlite_indexed_bytes_per_fact",
                             "sqlite_tableonly_bytes_per_fact"))
        for label, per_key, abs_key, color, hatch, alpha, dy in lanes:
            vals = [w[per_key] for _, w in rows]
            ax.barh([y + dy for y in ys], vals, height=0.22, color=color,
                    hatch=hatch, alpha=alpha, label=label, zorder=3)
            for y, (_, w) in enumerate(rows):
                ax.text(w[per_key] + peak * 0.015, y + dy, fmt_bytes(w[abs_key]),
                        va="center", fontsize=8, family="monospace",
                        color=OURS if color == OURS else DIM,
                        fontweight="bold" if color == OURS else "normal")
        ax.set_yticks(list(ys), [scale for scale, _ in rows],
                      fontsize=10, family="monospace", color=FG)
        ax.invert_yaxis()
        ax.set_xlim(0, peak * 1.22)
        ax.set_xlabel("bytes per fact", fontsize=9, family="monospace")
        ax.grid(axis="x", color=GRID, linewidth=0.6, zorder=0)
        ax.set_title(f"{world} · bytes per fact per scale · absolute store bytes annotated",
                     fontsize=11, loc="left", pad=10, family="monospace")
        if ax is axes[0]:
            ax.legend(loc="lower right", facecolor=BG, edgecolor=GRID,
                      labelcolor=FG, fontsize=8)

    if churn:
        ax = axes[-1]
        dark(ax)
        ys = range(len(churn))
        for row_index, row in enumerate(churn):
            engine, sqlite = row.get("engine_bytes"), row.get("sqlite_bytes")
            wal = row.get("sqlite_wal_bytes")
            if sqlite is not None:
                ax.barh(row_index + 0.19, sqlite, height=0.34, color=THEIRS,
                        label="sqlite" if row_index == 0 else None, zorder=3)
                note = fmt_bytes(sqlite)
                if wal:
                    note += f"  (wal {fmt_bytes(wal)})"
                ax.text(sqlite * 1.02, row_index + 0.19, note, va="center",
                        fontsize=8, color=DIM, family="monospace")
            if engine is not None:
                ax.barh(row_index - 0.19, engine, height=0.34, color=OURS,
                        label="engine" if row_index == 0 else None, zorder=3)
                ax.text(engine * 1.02, row_index - 0.19, fmt_bytes(engine),
                        va="center", fontsize=8, color=OURS,
                        fontweight="bold", family="monospace")
        ax.set_yticks(list(ys), [row["name"] for row in churn],
                      fontsize=9, family="monospace", color=FG)
        ax.invert_yaxis()
        ax.xaxis.set_major_formatter(FuncFormatter(fmt_bytes))
        ax.grid(axis="x", color=GRID, linewidth=0.6, zorder=0)
        ax.set_title("churn checkpoints · absolute store bytes (wal reported — an "
                     "uncheckpointed emission is visible)",
                     fontsize=11, loc="left", pad=10, family="monospace")
        ax.legend(loc="lower right", facecolor=BG, edgecolor=GRID,
                  labelcolor=FG, fontsize=8)

    fig.text(0.01, 0.005,
             "storage lane · report-class · every byte behind a count cross-check "
             "against the generator stream",
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_writes_rates(report, out):
    """bench-writes-rates.svg: rows/sec per (family, batch) row, ours vs
    theirs paired, one panel per durability lane — the lane + sqlite_sync
    labels ride in the panel title, so the number never appears without
    its durability context."""
    lanes = report["lanes"]
    heights = [0.5 * len(lane["rows"]) + 1.2 for lane in lanes]
    fig, axes = plt.subplots(len(lanes), 1, facecolor=BG,
                             figsize=(9.6, sum(heights) + 0.6),
                             gridspec_kw={"height_ratios": heights})
    axes = [axes] if len(lanes) == 1 else list(axes)
    for ax, lane in zip(axes, lanes):
        dark(ax)
        rows = lane["rows"]
        names = [r["name"] for r in rows]
        ys = range(len(rows))
        ours = [r["rows_per_sec_ours"] for r in rows]
        theirs = [r["rows_per_sec_theirs"] for r in rows]
        ax.barh([y + 0.19 for y in ys], theirs, height=0.34, color=THEIRS,
                label="SQLite", zorder=3)
        ax.barh([y - 0.19 for y in ys], ours, height=0.34, color=OURS,
                label="bumbledb", zorder=3)
        for y, (o, t) in enumerate(zip(ours, theirs)):
            ax.text(o * 1.12, y - 0.19, fmt_rate(o), va="center", fontsize=9,
                    color=OURS, fontweight="bold", family="monospace")
            ax.text(t * 1.12, y + 0.19, fmt_rate(t), va="center", fontsize=8,
                    color=DIM, family="monospace")
        ax.set_yticks(list(ys), names, fontsize=10, family="monospace", color=FG)
        ax.invert_yaxis()
        ax.set_xscale("log")
        ax.set_xlim(min(ours + theirs) * 0.5, max(ours + theirs) * 12)
        ax.xaxis.set_major_formatter(FuncFormatter(fmt_rate))
        ax.grid(axis="x", color=GRID, linewidth=0.6, zorder=0)
        ax.set_title(f"lane {lane['lane']} · sqlite {lane['sqlite_sync']} · "
                     "rows/sec — longer is more throughput",
                     fontsize=11, loc="left", pad=10, family="monospace")
        ax.legend(loc="lower right", facecolor=BG, edgecolor=GRID,
                  labelcolor=FG, fontsize=8)
    fig.text(0.01, 0.005,
             "writes lane · report-class · post-state value-verified (count arithmetic "
             "+ body multisets, ids projected out) · log scale",
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_curves(report, out):
    """bench-curves.svg: log-log p50-vs-facts lines, one panel per
    family — ours solid, sqlite canonical dashed, sqlite hand-tuned
    dotted; fitted exponents annotated; capped points drawn as open
    markers pinned at the cap ceiling and counted in the footer."""
    families = report["families"]
    cap_ns = report["cap_ms"] * 1e6
    cols = 2
    rows_n = (len(families) + cols - 1) // cols
    fig, axes = plt.subplots(rows_n, cols, facecolor=BG,
                             figsize=(9.6, 3.4 * rows_n + 0.6))
    flat = [axes] if rows_n * cols == 1 else list(axes.flat)
    capped_total = 0

    def line(ax, pts, color, style, label):
        if not pts:
            return
        xs, ys = zip(*pts)
        ax.plot(xs, ys, style, color=color, label=label, linewidth=2,
                marker="o", ms=4, zorder=3)
        slope = fit_exponent(xs, ys)
        if slope is not None:
            ax.annotate(f"~n^{slope:.2f}", (xs[-1], ys[-1]),
                        textcoords="offset points", xytext=(4, 4),
                        fontsize=8, color=color, family="monospace")

    for ax, family in zip(flat, families):
        dark(ax)
        pts = family["rows"]
        line(ax, [(p["facts"], p["ours"]["p50"]) for p in pts if p.get("ours")],
             OURS, "-", "bumbledb")
        line(ax, [(p["facts"], p["theirs"]["p50"]) for p in pts if p.get("theirs")],
             THEIRS, "--", "sqlite")
        line(ax, [(p["facts"], p["theirs_hand"]["p50"])
                  for p in pts if p.get("theirs_hand")],
             THEIRS, ":", "sqlite (hand-tuned)")
        for p in pts:
            if p.get("cap") and not p.get("theirs"):
                capped_total += 1
                ax.plot(p["facts"], cap_ns, "o", ms=8, mfc="none", mec=THEIRS,
                        mew=1.6, zorder=4)
                ax.annotate("DNF ≥ cap", (p["facts"], cap_ns),
                            textcoords="offset points", xytext=(-8, -14),
                            fontsize=8, color=THEIRS, family="monospace")
        ax.set_xscale("log")
        ax.set_yscale("log")
        ax.yaxis.set_major_formatter(FuncFormatter(fmt_us))
        ax.set_xlabel("facts", fontsize=9, family="monospace")
        ax.grid(color=GRID, linewidth=0.6, zorder=0)
        ax.set_title(f"{family['name']} · {family['world']}", fontsize=11,
                     loc="left", pad=8, family="monospace")
        ax.legend(loc="upper left", facecolor=BG, edgecolor=GRID,
                  labelcolor=FG, fontsize=8)
    for ax in flat[len(families):]:
        ax.set_visible(False)
    footer = ("curves lane · report-class · every point oracle-gated (value-identical "
              "multisets) before either engine is timed")
    if capped_total:
        footer += (f" · {capped_total} SQLite point"
                   + ("s" if capped_total != 1 else "")
                   + " exceeded the cap — excluded and counted")
    fig.text(0.01, 0.005, footer, fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_warmth(report, out):
    """bench-warmth.svg: cold/warm/memoized p50 per warmth-carrying
    family, ours vs sqlite paired per group — the memo effect made an
    explicit chart instead of an implicit flatterer."""
    families = [f for f in report["families"] if f.get("warmth")]
    phases = ("cold", "warm", "memoized")
    fig, ax = plt.subplots(figsize=(9.6, 4.2), facecolor=BG)
    dark(ax)
    values = [family["warmth"][f"{side}_{phase}"]["p50"]
              for family in families for side in ("ours", "theirs")
              for phase in phases]
    xticks, xlabels = [], []
    for fi, family in enumerate(families):
        w = family["warmth"]
        for pi, phase in enumerate(phases):
            x = fi * (len(phases) + 1) + pi
            o, t = w[f"ours_{phase}"]["p50"], w[f"theirs_{phase}"]["p50"]
            ax.bar(x - 0.2, o, width=0.4, color=OURS, zorder=3,
                   label="bumbledb" if fi == 0 and pi == 0 else None)
            ax.bar(x + 0.2, t, width=0.4, color=THEIRS, zorder=3,
                   label="SQLite" if fi == 0 and pi == 0 else None)
            ax.text(x - 0.2, o * 1.12, fmt_us(o), ha="center", fontsize=8,
                    color=OURS, fontweight="bold", family="monospace")
            ax.text(x + 0.2, t * 1.12, fmt_us(t), ha="center", fontsize=8,
                    color=DIM, family="monospace")
            xticks.append(x)
            xlabels.append(f"{family['name']}\n{phase}" if pi == 1 else phase)
    ax.set_yscale("log")
    if values:
        ax.set_ylim(min(values) * 0.3, max(values) * 4)
    ax.yaxis.set_major_formatter(FuncFormatter(fmt_us))
    ax.set_xticks(xticks, xlabels, fontsize=9, family="monospace", color=FG)
    ax.grid(axis="y", color=GRID, linewidth=0.6, zorder=0)
    ax.set_title("warmth · cold (process-fresh reopen, OS-warm) → warm → memoized · "
                 "p50, both engines",
                 fontsize=12, loc="left", pad=14, family="monospace")
    ax.legend(loc="upper right", facecolor=BG, edgecolor=GRID,
              labelcolor=FG, fontsize=9)
    fig.text(0.01, 0.005,
             "what it prices: the (relation, generation) image cache and the "
             "resolved-filter view slots — the memo effect explicit",
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


# --------------------------------------------------------------- main


def pop_flag(args, name):
    """Pops `name <value>` out of args, returning the value or None."""
    if name not in args:
        return None
    i = args.index(name)
    value = args[i + 1]
    del args[i:i + 2]
    return value


def main():
    args = sys.argv[1:]
    scenarios_md = pop_flag(args, "--scenarios")
    out_dir = pop_flag(args, "--out-dir") or "assets"
    storage_json = pop_flag(args, "--storage-report")
    writes_json = pop_flag(args, "--writes-report")
    curves_json = pop_flag(args, "--curves-report")
    if not args and not (storage_json or writes_json or curves_json):
        sys.exit("usage: bench_viz.py [<run-dir> ...] [--scenarios <scenarios.md>]"
                 " [--out-dir <dir>] [--storage-report <json>]"
                 " [--writes-report <json>] [--curves-report <json>]")
    Path(out_dir).mkdir(parents=True, exist_ok=True)
    if args:
        reads, writes = load(args)
        chart_vs_sqlite(reads, os.path.join(out_dir, "bench-vs-sqlite.svg"))
        chart_speedup(reads, os.path.join(out_dir, "bench-speedup.svg"))
        chart_tails(reads, os.path.join(out_dir, "bench-tails.svg"))
        chart_writes(writes, os.path.join(out_dir, "bench-writes.svg"))
        if scenarios_md:
            chart_scenarios(load_scenarios(scenarios_md),
                            os.path.join(out_dir, "bench-scenarios.svg"))
        for name in READ_ORDER:
            if name in reads and "theirs" in reads[name]:
                r = reads[name]
                print(f"{name:10} ours {fmt_us(r['ours']['p50']):>8}  sqlite {fmt_us(r['theirs']['p50']):>8}  "
                      f"{r['theirs']['p50'] / r['ours']['p50']:5.1f}x")
    if storage_json:
        chart_storage(load_report(storage_json),
                      os.path.join(out_dir, "bench-storage.svg"))
    if writes_json:
        chart_writes_rates(load_report(writes_json),
                           os.path.join(out_dir, "bench-writes-rates.svg"))
    if curves_json:
        report = load_report(curves_json)
        chart_curves(report, os.path.join(out_dir, "bench-curves.svg"))
        chart_warmth(report, os.path.join(out_dir, "bench-warmth.svg"))


if __name__ == "__main__":
    main()
