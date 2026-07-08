#!/usr/bin/env python3
"""Benchmark visuals: bench run dirs -> the README charts.

Takes N run directories (each holding a report.json from `bumbledb-bench
bench`), computes the min-of-N p50 per family (the suite's merge rule),
and renders three charts into assets/:

  bench-vs-sqlite.svg   ours vs SQLite p50 per read family (log scale)
  bench-campaign.svg    p50 across the four campaign epochs, normalized
  bench-writes.svg      the honest chart: writes + cold, where fsync physics rules

Usage: python3 scripts/bench_viz.py bench-out/run1 bench-out/run2 ...
Needs: matplotlib (`python3 -m pip install matplotlib`).
"""

import json
import sys
from pathlib import Path

import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter

# ---------------------------------------------------------------- data

READ_ORDER = [
    "point", "string", "balance", "fk_walk", "skew",
    "range", "chain", "stats", "spread", "triangle",
]
WRITE_ORDER = ["commit_single", "commit_batch", "cold_fk_walk", "bulk"]

# The campaign ledger (p50 microseconds), from the pinned tables:
# docs/perf/baseline.md -> docs/silicon/final.md (baseline col = perf end)
# -> docs/silicon/final.md (final col) -> docs/silicon2/final2.md.
EPOCHS = ["first build", "docs/perf", "docs/silicon", "docs/silicon2"]
CAMPAIGN = {
    "point":    [1.1, 1.0, 0.4, 0.4],
    "string":   [1.8, 1.5, 0.8, 0.7],
    "balance":  [12.3, 1.4, 0.7, 0.7],
    "fk_walk":  [12.8, 6.8, 2.9, 6.0],
    "skew":     [59.5, 39.7, 35.8, 52.2],
    "range":    [59.1, 28.5, 28.5, 20.6],
    "chain":    [210.0, 134.4, 104.0, 100.9],
    "stats":    [4130.9, 1886.0, 1872.5, 1203.5],
    "spread":   [13415.1, 11281.6, 10725.8, 10269.9],
    "triangle": [17480.9, 15064.0, 11742.5, 9445.5],
}

OURS, THEIRS, FG, DIM, GRID, BG = (
    "#f0b429", "#8b949e", "#e6edf3", "#9da7b3", "#2d333b", "#0d1117",
)


def load(dirs):
    """Min-of-N p50 (ns) per family for ours and sqlite, reads + writes."""
    reads, writes = {}, {}
    for d in dirs:
        r = json.loads((Path(d) / "report.json").read_text())
        for table, out in ((r["reads"], reads), (r["writes"], writes)):
            for fam in table:
                slot = out.setdefault(fam["name"], {"ours": [], "theirs": []})
                slot["ours"].append(fam["ours"]["p50"])
                if fam.get("theirs"):
                    slot["theirs"].append(fam["theirs"]["p50"])
    return (
        {k: {s: min(v) for s, v in vv.items() if v} for k, vv in reads.items()},
        {k: {s: min(v) for s, v in vv.items() if v} for k, vv in writes.items()},
    )


def fmt_us(ns, _pos=None):
    us = ns / 1000
    if us < 10:
        return f"{us:.1f}µs"
    if us < 1000:
        return f"{us:.0f}µs"
    if us < 1_000_000:
        return f"{us / 1000:.0f}ms"
    return f"{us / 1e6:.1f}s"


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
    ours = [table[n]["ours"] for n in names]
    theirs = [table[n].get("theirs") for n in names]
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


def spread_labels(points, min_gap):
    """De-overlap 1-D label positions (log10 space), preserving order."""
    order = sorted(range(len(points)), key=lambda i: points[i], reverse=True)
    placed = []
    out = [0.0] * len(points)
    for i in order:
        y = points[i]
        if placed and placed[-1] - y < min_gap:
            y = placed[-1] - min_gap
        placed.append(y)
        out[i] = y
    return out


def chart_campaign(out):
    import math
    fig, ax = plt.subplots(figsize=(9.6, 5.4), facecolor=BG)
    dark(ax)
    cmap = plt.get_cmap("tab10")
    names = list(CAMPAIGN)
    rels = {n: [v / CAMPAIGN[n][0] for v in CAMPAIGN[n]] for n in names}
    label_y = spread_labels([math.log10(rels[n][-1]) for n in names], 0.062)
    for i, name in enumerate(names):
        rel = rels[name]
        color = OURS if name == "triangle" else cmap(i % 10)
        lw = 2.6 if name in ("triangle", "stats") else 1.6
        ax.plot(EPOCHS, rel, marker="o", markersize=4, linewidth=lw,
                color=color, alpha=0.95)
        ax.annotate(f"{name} {rel[-1] * 100:.0f}%", (3, 10 ** label_y[i]),
                    xytext=(10, 0), textcoords="offset points",
                    fontsize=8.5, color=color, family="monospace", va="center")
    ax.set_yscale("log")
    ax.set_yticks([1.0, 0.5, 0.25, 0.1, 0.05],
                  ["100%", "50%", "25%", "10%", "5%"])
    ax.set_xlim(-0.15, 3.9)
    ax.grid(axis="y", color=GRID, linewidth=0.6, zorder=0)
    ax.set_title("four campaigns of measured PRDs · read-family p50 relative to the first build",
                 fontsize=12, loc="left", pad=14, family="monospace")
    fig.text(0.01, 0.005, "every step min-of-3 under a measurement lock, gated by a 2,468-case differential oracle",
             fontsize=8, color=DIM, family="monospace")
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


def main():
    dirs = sys.argv[1:]
    if not dirs:
        sys.exit("usage: bench_viz.py <run-dir> [<run-dir> ...]")
    reads, writes = load(dirs)
    Path("assets").mkdir(exist_ok=True)
    chart_vs_sqlite(reads, "assets/bench-vs-sqlite.svg")
    chart_campaign("assets/bench-campaign.svg")
    chart_writes(writes, "assets/bench-writes.svg")
    for name in READ_ORDER:
        if name in reads and "theirs" in reads[name]:
            r = reads[name]
            print(f"{name:10} ours {fmt_us(r['ours']):>8}  sqlite {fmt_us(r['theirs']):>8}  "
                  f"{r['theirs'] / r['ours']:5.1f}x")


if __name__ == "__main__":
    main()
