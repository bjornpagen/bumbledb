#!/usr/bin/env python3
"""DELTA tables: baseline-2026-07-25-post vs baseline-2026-07-25.

Recomputes every suite geomean from both runs' JSON with identical
pairing (common cells only; DNF/capped/twinless excluded-and-counted
identically both sides), exactly as the baseline's SUMMARY.md did vs the
campaign. ratio = ours/sqlite (lower is better); vs-baseline =
post geomean / baseline geomean (<1 = the fixes cashed). Also emits
per-cell p50 movers so the attribution shift has named targets.
Prints markdown to stdout.
"""

import json
import math
import sys

B = "/Users/bjorn/Documents/bumbledb/bench-out/baseline-2026-07-25"
P = "/Users/bjorn/Documents/bumbledb/bench-out/baseline-2026-07-25-post"


def geomean(xs):
    xs = [x for x in xs if x and x > 0]
    if not xs:
        return None
    return math.exp(sum(math.log(x) for x in xs) / len(xs))


def load(path):
    with open(path) as f:
        return json.load(f)


def rep_cells(report):
    """(name -> (ratio_p50, ours_p50, theirs_p50)) for reads + twinned writes."""
    cells = {}
    for r in report["reads"]:
        if r.get("ratio_p50"):
            cells["read/" + r["name"]] = (
                r["ratio_p50"], r["ours"]["p50"], r["theirs"]["p50"])
    for w in report["writes"]:
        ours, theirs = w.get("ours"), w.get("theirs")
        if ours and theirs and theirs.get("p50"):
            cells["write/" + w["name"]] = (
                ours["p50"] / theirs["p50"], ours["p50"], theirs["p50"])
    return cells


def pair_table(name, base_cells, post_cells, mover_gate=0.15, unit="ns"):
    common = sorted(set(base_cells) & set(post_cells))
    only_base = sorted(set(base_cells) - set(post_cells))
    only_post = sorted(set(post_cells) - set(base_cells))
    b_geo = geomean([base_cells[c][0] for c in common])
    p_geo = geomean([post_cells[c][0] for c in common])
    print(f"\n### {name} — {len(common)} common cells"
          + (f" (base-only: {only_base})" if only_base else "")
          + (f" (post-only: {only_post})" if only_post else ""))
    print(f"geomean(ratio_p50): post {p_geo:.4f} vs baseline {b_geo:.4f}"
          f" → **{p_geo / b_geo:.3f}**")
    movers = []
    for c in common:
        br, bo, _ = base_cells[c]
        pr, po, _ = post_cells[c]
        rr = pr / br
        if abs(rr - 1.0) >= mover_gate:
            movers.append((rr, c, bo, po, br, pr))
    movers.sort(key=lambda m: m[0])
    if movers:
        print(f"\n| cell | ours p50 base → post ({unit}) | ratio base → post | Δratio |")
        print("|---|---:|---:|---:|")
        for rr, c, bo, po, br, pr in movers:
            print(f"| {c} | {bo} → {po} | {br:.4f} → {pr:.4f} | {rr:.2f} |")
    return b_geo, p_geo


def scen_cells(path):
    cells = {}
    dnf = []
    for q in load(path)["queries"]:
        key = q["scenario"] + "/" + q["name"]
        sq = [l for l in q["lanes"] if l["lane"] == "sqlite"]
        if not sq:
            continue  # hand-lane-only query (t5_pack_key)
        lane = sq[0]
        if lane["outcome"] != "timed" or not lane.get("ratio_p50"):
            dnf.append(key)
            continue
        cells[key] = (lane["ratio_p50"], q["ours"]["p50"],
                      lane["stats"]["p50"])
    return cells, dnf


def crud_cells(path):
    cells = {}
    for lane in load(path)["lanes"]:
        for row in lane["rows"]:
            ours, theirs = row.get("ours"), row.get("theirs")
            if ours and theirs and theirs.get("p50"):
                cells[lane["lane"] + "/" + row["family"]] = (
                    ours["p50"] / theirs["p50"], ours["p50"], theirs["p50"])
    return cells


def lawful_cells(path):
    cells = {}
    for row in load(path)["lanes"]:
        ours, theirs = row.get("ours"), row.get("theirs")
        if ours and theirs and theirs.get("p50"):
            cells[row["lane"] + "/" + row["family"]] = (
                ours["p50"] / theirs["p50"], ours["p50"], theirs["p50"])
    return cells


def writes_cells(path):
    cells = {}
    for lane in load(path)["lanes"]:
        rows = lane.get("rows") or lane.get("families") or []
        for row in rows:
            ours, theirs = row.get("ours"), row.get("theirs")
            name = row.get("family") or row.get("name")
            if ours and theirs and theirs.get("p50"):
                cells[lane["lane"] + "/" + name] = (
                    ours["p50"] / theirs["p50"], ours["p50"], theirs["p50"])
    return cells


def main():
    print("# DELTA — baseline-2026-07-25-post vs baseline-2026-07-25")

    for rep in ["bench-durable-r1", "bench-durable-r2", "bench-durable-r3",
                "bench-ephemeral-r1", "bench-ephemeral-r2",
                "bench-ephemeral-r3"]:
        pair_table(rep, rep_cells(load(f"{B}/{rep}/report.json")),
                   rep_cells(load(f"{P}/{rep}/report.json")))

    b, bdnf = scen_cells(f"{B}/scenarios/scenarios.json")
    p, pdnf = scen_cells(f"{P}/scenarios/scenarios.json")
    print(f"\nscenario DNFs: baseline {bdnf} / post {pdnf}")
    pair_table("scenarios", b, p, mover_gate=0.10, unit="ns")

    pair_table("crud", crud_cells(f"{B}/crud/crud.json"),
               crud_cells(f"{P}/crud/crud.json"))
    pair_table("lawful", lawful_cells(f"{B}/lawful/lawful.json"),
               lawful_cells(f"{P}/lawful/lawful.json"))
    pair_table("writes", writes_cells(f"{B}/writes/writes-report.json"),
               writes_cells(f"{P}/writes/writes-report.json"))


if __name__ == "__main__":
    sys.exit(main())
