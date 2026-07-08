#!/usr/bin/env python3
"""Benchmark visuals: bench run dirs -> the README charts.

Takes N run directories (each holding a report.json from `bumbledb-bench
bench`), computes the min-of-N p50 per family (the suite's merge rule),
and renders three charts into assets/:

  bench-vs-sqlite.svg   ours vs SQLite p50 per read family (log scale)
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
    chart_writes(writes, "assets/bench-writes.svg")
    for name in READ_ORDER:
        if name in reads and "theirs" in reads[name]:
            r = reads[name]
            print(f"{name:10} ours {fmt_us(r['ours']):>8}  sqlite {fmt_us(r['theirs']):>8}  "
                  f"{r['theirs'] / r['ours']:5.1f}x")


if __name__ == "__main__":
    main()
