#!/usr/bin/env python3
"""Benchmark visuals: bench run dirs -> the README charts.

Takes N run directories (each holding a report.json from `bumbledb-bench
bench`), computes the min-of-N p50 per family (the suite's merge rule),
and renders three charts into assets/:

  bench-vs-sqlite.svg   ours vs SQLite p50 per read family (log scale)
  bench-speedup.svg     the same data as multipliers, big and readable
  bench-tails.svg       p50 -> p99 per family: tail behavior, both engines
  bench-writes.svg      the honest chart: writes + cold, where fsync physics rules
  bench-scenarios.svg   the non-ledger worlds (joins/graph/olap/points), per query

Usage: python3 scripts/bench_viz.py <run-dir> ... [--scenarios <scenarios.md>]
Needs: matplotlib (`python3 -m pip install matplotlib`).
"""

import json
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


def main():
    args = sys.argv[1:]
    scenarios_md = None
    if "--scenarios" in args:
        i = args.index("--scenarios")
        scenarios_md = args[i + 1]
        args = args[:i] + args[i + 2:]
    if not args:
        sys.exit("usage: bench_viz.py <run-dir> ... [--scenarios <scenarios.md>]")
    reads, writes = load(args)
    Path("assets").mkdir(exist_ok=True)
    chart_vs_sqlite(reads, "assets/bench-vs-sqlite.svg")
    chart_speedup(reads, "assets/bench-speedup.svg")
    chart_tails(reads, "assets/bench-tails.svg")
    chart_writes(writes, "assets/bench-writes.svg")
    if scenarios_md:
        chart_scenarios(load_scenarios(scenarios_md), "assets/bench-scenarios.svg")
    for name in READ_ORDER:
        if name in reads and "theirs" in reads[name]:
            r = reads[name]
            print(f"{name:10} ours {fmt_us(r['ours']['p50']):>8}  sqlite {fmt_us(r['theirs']['p50']):>8}  "
                  f"{r['theirs']['p50'] / r['ours']['p50']:5.1f}x")


if __name__ == "__main__":
    main()
