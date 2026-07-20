#!/usr/bin/env python3
"""Benchmark visuals: committed report artifacts -> the README charts.

Charts are data: the CHARTS registry is an ordered list of specs
(filename, required inputs keys, render fn) and lane inputs are data —
one `inputs` dict keyed by lane id, filled by discovery or by flags. A
chart renders only when every key it requires is present and truthy;
otherwise it SKIPs by table lookup. "Chart rendered against an absent
lane" is unrepresentable: render is never called without its inputs.

The lane discriminant: a report.json whose top level carries a string
"lane" key is a lane payload, stored under inputs[payload["lane"]] (the
first occurrence of a lane wins; later duplicates print a note). A
report.json WITHOUT that key is a suite RunReport, classified by
config.store into the durable / ephemeral run pools. The preferred pool
(durable when non-empty, else ephemeral) merges min-of-N per percentile
(p50/p90/p95/p99) into inputs["reads"] / inputs["writes"]; the pool's
kind and size ride along as inputs["store_kind"] / inputs["rep_count"].

  bench-vs-sqlite.svg     ours vs SQLite p50 per read family (log scale)   [reads]
  bench-speedup.svg       the same data as multipliers, big and readable   [reads]
  bench-tails.svg         p50 -> p99 per family: tails, both engines       [reads]
  bench-writes.svg        the honest chart: writes + cold, fsync physics   [writes]
  bench-scenarios.svg     the non-ledger worlds, per query (or lane)       [scenarios]
  world-<world>.svg       one file per scenario world: paired p50 bars     [scenarios]
  ratio-waterfall.svg     every family + query as one sorted ratio bar     [reads (+scenarios)]
  tails-fan.svg           p50 -> p90 -> p99 fan per family, both engines   [reads]
  world-crud.svg          the OLTP home turf: speedup per (family, lane)   [crud_report]
  world-lawful.svg        the integrity home turf: same treatment          [lawful_report]
  bench-storage.svg       bytes per fact per scale/world (+ churn)         [storage_report]
  bench-writes-rates.svg  rows/sec per (family, batch), per lane           [writes_rates]
  bench-curves.svg        log-log scale curves, exponents, DNF caps        [curves_report]
  bench-warmth.svg        cold/warm/memoized, both engines                 [curves_report]
  write-throughput.svg    facts/sec per commit batch, per durability lane  [write_throughput]
  adversarial-dnf.svg     ours vs SQLite, capped twins drawn as capped     [adversarial]
  churn-latency-<run>.svg    probe p50 over cycles, every lane, per run    [churn_report]
  churn-size-<run>.svg       store size over cycles, every lane, per run   [churn_report]
  churn-throughput-<run>.svg commits/sec over cycles, every lane, per run  [churn_report]

Retired charts, reasons on the record (docs/architecture/61-bench-lanes.md
§ the chart inventory): storage-bytes-per-fact.svg and curves-loglog.svg
consumed lane payloads no emitter writes — their data is fully rendered by
bench-storage.svg / bench-curves.svg from the real flag-shaped reports;
cold-warm-memo.svg's contract had no theirs-memo slot, so it could not even
represent the real warmth measurement (curves-report.json times
theirs_memoized) that bench-warmth.svg draws whole. No phantom-input chart
survives.

Usage:
  python3 scripts/bench_viz.py <run-dir> ... [--scenarios <scenarios.md|scenarios.json>] [--out <dir>]
  python3 scripts/bench_viz.py --night <night-dir> [--out <dir>]

--scenarios dispatches on extension: `.json` is the scenario runner's
machine artifact (scenarios.json), whose lanes are a tagged union —
{"outcome":"timed", stats, ratio_p50} | {"outcome":"exceeded_cap",
cap_ms} — the true representation; the scenarios.md table is its
rendering. A DNF lane has no stats object, so it draws NO bar anywhere:
it becomes a right-edge annotation, excluded and counted in the title.
Anything else is a rendered scenarios.md table, parsed by its own
header row (the legacy 6-col pin and the lane-bearing 7-col format both
parse; a `DNF>cap` p50 cell parses to None and is skipped-and-counted
with the same annotation idiom). inputs["scenarios"] carries the parsed
tag ("json"|"md", path); the scenario, world, and waterfall charts all
consume that one tagged input.

Exactly one of {run dirs, --night} supplies run reports. --night scans a
night out-dir's one-level children: every <child>/report.json is ingested
through the lane discriminant, the first <child>/scenarios.json
(preferred) or <child>/scenarios.md becomes inputs["scenarios"], and the
real lane reports auto-ingest from their canonical night paths
(NIGHT_LANE_REPORTS: storage/storage-report.json -> storage_report,
writes/writes-report.json -> writes_rates, curves/curves-report.json ->
curves_report, crud/crud.json -> crud_report, lawful/lawful.json ->
lawful_report, churn/churn-report.json -> churn_report). The committed
lane-report flags work in either mode and OVERRIDE discovery, each
filling one inputs key: --storage-report -> storage_report,
--writes-report -> writes_rates, --curves-report -> curves_report.

A run dir carrying a CONTAMINATED.md marker is excluded and counted —
the contamination record is a file ON the pinned run, so "a contaminated
run leaked into the merge" is a missing file, not a forgotten footnote;
the excluded count rides the chart footers, and every chart whose
provenance says shared_machine carries the boosted-QoS caveat.

Two chart inputs are DERIVED from real artifacts (derive_lanes), each
passed through its lane-contract loader so the contract stays honest as
the adapter's output shape: adversarial from scenarios.json's
exceeded_cap lanes (the DNF data's one real home while the adversarial
subcommand remains unlanded), and write_throughput from
writes-report.json's commit/delete batch ladders (bulk_append is a
single point, fully drawn in bench-writes-rates.svg). A real lane
payload of the same name, once an emitter writes one, wins over the
derivation.

Lanes with a pinned contract validate at ingest: LANE_LOADERS maps a
lane name to its loader, so "chart fed a shapeless lane file" is
unrepresentable — the loader names its required keys and rejects
anything else with the file path in the error. The surviving contracts
are fixed as committed fixtures under scripts/viz-fixtures/:
fixture-write-throughput / fixture-adversarial (.report.json, the lane
payload shapes) and fixture-churn.report.json (the REAL churn_schema:1
runs -> lanes -> samples artifact). The adversarial lane carries its
DNF law in the shape: theirs_exceeded_cap => theirs is null, so a
capped SQLite time can never be drawn as a measurement — there are no
stats to draw, only the cap. The churn charts consume the runner's real
artifact directly — every run and every lane draws (steady's
sqlite-bare AND sqlite-maint side by side), one file per run per
metric, VACUUM/ANALYZE cycles marked from the maintenance_ns the run
recorded, so the marker can never drift from the measurement it
annotates.

`--out` (alias `--out-dir`) defaults to assets/ (the owner's ceremony
path); every other invocation should point it elsewhere. Charts render
ONLY from committed report pins — never from live runs.
Needs: matplotlib (`python3 -m pip install matplotlib`).
"""

import argparse
import json
import math
from dataclasses import dataclass
from pathlib import Path

import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter, MaxNLocator

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


def ordered(table, order):
    """Every family in the merged table, canonical names first in
    registry order, the rest in the report's own row order — a family
    present in the pin but absent from the chart is unrepresentable."""
    return [n for n in order if n in table] + [n for n in table if n not in order]

# The merged percentile set (p90 exists in every committed pin; the
# tail fan reads it).
PCTS = ("p50", "p90", "p95", "p99")

OURS, THEIRS, FG, DIM, GRID, BG = (
    "#f0b429", "#8b949e", "#e6edf3", "#9da7b3", "#2d333b", "#0d1117",
)


# -------------------------------------------------------------- inputs


def _stats_p50(container, key, where):
    """One optional stats slot: absent or null is fine (drawn as
    nothing); present means a dict carrying a numeric "p50"."""
    stats = container.get(key)
    if stats is None:
        return None
    if not isinstance(stats, dict) or not isinstance(stats.get("p50"), (int, float)):
        raise ValueError(f'{where}: "{key}" must be null or an object with a numeric "p50"')
    return stats


def load_write_throughput(payload):
    """The write_throughput lane contract: "lanes" is a non-empty list;
    every durability lane carries a "name" and a non-empty "batches"
    list; every batch row carries "batch" > 0 plus numeric
    ours_facts_per_sec / theirs_facts_per_sec."""
    lanes = payload.get("lanes")
    if not isinstance(lanes, list) or not lanes:
        raise ValueError('write_throughput lane: "lanes" must be a non-empty list')
    for lane in lanes:
        if not isinstance(lane, dict) or not isinstance(lane.get("name"), str) \
                or not lane["name"]:
            raise ValueError('write_throughput lane: every lane needs a "name" string')
        name = lane["name"]
        batches = lane.get("batches")
        if not isinstance(batches, list) or not batches:
            raise ValueError(
                f'write_throughput lane "{name}": "batches" must be a non-empty list')
        for row in batches:
            if not isinstance(row, dict) \
                    or not isinstance(row.get("batch"), (int, float)) or row["batch"] <= 0:
                raise ValueError(f'write_throughput lane "{name}": every batch row needs '
                                 '"batch" > 0')
            for key in ("ours_facts_per_sec", "theirs_facts_per_sec"):
                if not isinstance(row.get(key), (int, float)):
                    raise ValueError(f'write_throughput lane "{name}" batch '
                                     f'{row["batch"]}: "{key}" must be a number')
    return payload


def load_adversarial(payload):
    """The adversarial lane contract: "cap_ms" > 0 and a non-empty
    "queries" list; every query carries a "name", an "ours" stats
    object (numeric "p50") and a boolean "theirs_exceeded_cap". THE LAW
    carried by the shape: theirs_exceeded_cap=true => "theirs" is null
    — a payload claiming both a cap and a number is rejected, so a
    capped SQLite time can never be drawn as a measurement."""
    cap_ms = payload.get("cap_ms")
    if not isinstance(cap_ms, (int, float)) or cap_ms <= 0:
        raise ValueError('adversarial lane: "cap_ms" must be a number > 0')
    queries = payload.get("queries")
    if not isinstance(queries, list) or not queries:
        raise ValueError('adversarial lane: "queries" must be a non-empty list')
    for query in queries:
        if not isinstance(query, dict) or not isinstance(query.get("name"), str) \
                or not query["name"]:
            raise ValueError('adversarial lane: every query needs a "name" string')
        name = query["name"]
        if _stats_p50(query, "ours", f'adversarial lane query "{name}"') is None:
            raise ValueError(f'adversarial lane query "{name}": "ours" stats are required')
        capped = query.get("theirs_exceeded_cap")
        if not isinstance(capped, bool):
            raise ValueError(
                f'adversarial lane query "{name}": "theirs_exceeded_cap" must be a boolean')
        theirs = _stats_p50(query, "theirs", f'adversarial lane query "{name}"')
        if capped and theirs is not None:
            raise ValueError(f'adversarial lane query "{name}": theirs_exceeded_cap=true '
                             'yet "theirs" carries stats — a capped twin has no number')
    return payload


def load_churn_report(payload):
    """The churn runner's REAL artifact (churn_schema: 1, the frozen
    JSON face of crates/bumbledb-bench/src/churn/report.rs): runs ->
    lanes -> samples. Validated: "runs" non-empty; every run carries a
    "name", a "mix" object, and non-empty "lanes"; every lane carries a
    "lane" string, an "engine" in {bumbledb, sqlite}, and non-empty
    "samples"; every sample carries an integer "cycle", non-empty
    "probes" ({name, p50_ns} each), and numeric commits_per_sec /
    maintenance_ns / disk_bytes. The one-ours-one-theirs "lane":"churn"
    condensation is DELETED — it could not carry the steady run's two
    SQLite lanes (bare + maint) without hiding one, so no fixture-only
    path survives. Unknown extra keys are ignored (forward-compatible);
    the caller attaches the file path to errors."""
    if payload.get("churn_schema") != 1:
        raise ValueError('churn report: "churn_schema" must be 1')
    runs = payload.get("runs")
    if not isinstance(runs, list) or not runs:
        raise ValueError('churn report: "runs" must be a non-empty list')
    for run in runs:
        if not isinstance(run, dict) or not isinstance(run.get("name"), str) \
                or not run["name"]:
            raise ValueError('churn report: every run needs a "name" string')
        name = run["name"]
        if not isinstance(run.get("mix"), dict):
            raise ValueError(f'churn report run "{name}": "mix" must be an object')
        lanes = run.get("lanes")
        if not isinstance(lanes, list) or not lanes:
            raise ValueError(f'churn report run "{name}": "lanes" must be a non-empty list')
        for lane in lanes:
            if not isinstance(lane, dict) or not isinstance(lane.get("lane"), str) \
                    or not lane["lane"]:
                raise ValueError(f'churn report run "{name}": every lane needs a "lane" string')
            where = f'churn report run "{name}" lane "{lane["lane"]}"'
            if lane.get("engine") not in ("bumbledb", "sqlite"):
                raise ValueError(f'{where}: "engine" must be "bumbledb" or "sqlite"')
            samples = lane.get("samples")
            if not isinstance(samples, list) or not samples:
                raise ValueError(f'{where}: "samples" must be a non-empty list')
            for sample in samples:
                if not isinstance(sample, dict) or not isinstance(sample.get("cycle"), int):
                    raise ValueError(f'{where}: every sample needs an integer "cycle"')
                cycle = sample["cycle"]
                for key in ("commits_per_sec", "maintenance_ns", "disk_bytes"):
                    if not isinstance(sample.get(key), (int, float)):
                        raise ValueError(f'{where} cycle {cycle}: "{key}" must be a number')
                probes = sample.get("probes")
                if not isinstance(probes, list) or not probes:
                    raise ValueError(f'{where} cycle {cycle}: "probes" must be a non-empty list')
                for probe in probes:
                    if not isinstance(probe, dict) \
                            or not isinstance(probe.get("name"), str) or not probe["name"] \
                            or not isinstance(probe.get("p50_ns"), (int, float)):
                        raise ValueError(f'{where} cycle {cycle}: every probe needs a '
                                         '"name" string and a numeric "p50_ns"')
    return payload


def world_report_rows(report):
    """The two home-turf report shapes as ONE row stream of
    (lane_label, row): crud nests its rows under durability-lane
    objects; lawful's "lanes" entries ARE the rows (each carries its
    own "lane" label). Yields defensively — the contract loader is the
    validator; charts consume only validated payloads."""
    for entry in report.get("lanes", []):
        if not isinstance(entry, dict):
            continue
        if isinstance(entry.get("rows"), list):
            for row in entry["rows"]:
                if isinstance(row, dict):
                    yield entry.get("lane"), row
        else:
            yield entry.get("lane"), entry


def load_world_report(world):
    """The home-turf world contract (crud / lawful): "world" names the
    payload, "lanes" is non-empty, and every row carries a "family"
    string, a durability-lane label, and BOTH engines' stats with a
    numeric "p50" — these worlds have no DNF arm (a lane that cannot
    complete fails the whole run instead of rendering), so a missing
    twin number is a shape error, never a censoring."""
    def load(payload):
        if payload.get("world") != world:
            raise ValueError(f'{world} report: "world" must be "{world}"')
        lanes = payload.get("lanes")
        if not isinstance(lanes, list) or not lanes:
            raise ValueError(f'{world} report: "lanes" must be a non-empty list')
        count = 0
        for lane_label, row in world_report_rows(payload):
            if not isinstance(lane_label, str) or not lane_label \
                    or not isinstance(row.get("family"), str) or not row["family"]:
                raise ValueError(f'{world} report: every row needs "family" '
                                 'and lane label strings')
            where = f'{world} report row "{row["family"]}" [{lane_label}]'
            for side in ("ours", "theirs"):
                if _stats_p50(row, side, where) is None:
                    raise ValueError(f'{where}: "{side}" stats are required — '
                                     'this world has no DNF arm')
            count += 1
        if count == 0:
            raise ValueError(f'{world} report: "lanes" carries no rows')
        return payload
    return load


load_crud_report = load_world_report("crud")
load_lawful_report = load_world_report("lawful")


# Lane name -> contract loader; a lane without one is stored raw.
LANE_LOADERS = {
    "write_throughput": load_write_throughput,
    "adversarial": load_adversarial,
}


def ingest_report(inputs, path):
    """One report.json into the inputs dict, dispatched on the lane
    discriminant: a top-level "lane" string names a lane payload; its
    absence means a suite RunReport, classified by config.store."""
    payload = json.loads(Path(path).read_text())
    lane = payload.get("lane")
    if isinstance(lane, str):
        if lane in inputs:
            print(f"note: duplicate lane '{lane}' ({path}) — keeping the first")
            return
        loader = LANE_LOADERS.get(lane)
        if loader:
            try:
                payload = loader(payload)
            except ValueError as e:
                raise SystemExit(f"{path}: {e}")
        inputs[lane] = payload
        return
    config = payload.get("config")
    store = config.get("store") if isinstance(config, dict) else None
    if store in ("durable", "ephemeral"):
        inputs[f"{store}_runs"].append(payload)
    else:
        print(f"note: {path} is neither a lane payload nor a RunReport — skipped")


# The night dir's real lane reports, auto-ingested by discovery: the
# canonical child path, the inputs key EXACTLY as the matching flag sets
# it (flags override discovery), and the contract loader (None = the
# flag-shaped reports, consumed raw by their charts).
NIGHT_LANE_REPORTS = (
    ("storage/storage-report.json", "storage_report", None),
    ("writes/writes-report.json", "writes_rates", None),
    ("curves/curves-report.json", "curves_report", None),
    ("crud/crud.json", "crud_report", load_crud_report),
    ("lawful/lawful.json", "lawful_report", load_lawful_report),
    ("churn/churn-report.json", "churn_report", load_churn_report),
)


def contaminated(inputs, report_path):
    """The contamination record is a FILE on the pinned run dir
    (CONTAMINATED.md, the recorded ruling): a marked run is excluded
    from every merge and counted — it stays committed as the honest
    record, and it can never leak into a chart by someone forgetting a
    footnote."""
    marker = Path(report_path).parent / "CONTAMINATED.md"
    if not marker.exists():
        return False
    inputs["contaminated_runs"] += 1
    print(f"note: {Path(report_path).parent.name} excluded — {marker.name} "
          "(contaminated run, excluded and counted)")
    return True


def discover(night_dir):
    """A night out-dir -> the inputs dict: every one-level child's
    report.json through the lane discriminant (CONTAMINATED.md-marked
    runs excluded and counted), the scenario artifact one level down —
    the first scenarios.json when present (the tagged union is the true
    representation), else the first scenarios.md (its rendering) — and
    the real lane reports from their canonical night paths
    (NIGHT_LANE_REPORTS; the flags override)."""
    inputs = {"durable_runs": [], "ephemeral_runs": [], "contaminated_runs": 0}
    night = Path(night_dir)
    for report_path in sorted(night.glob("*/report.json")):
        if not contaminated(inputs, report_path):
            ingest_report(inputs, report_path)
    for pattern, kind in (("*/scenarios.json", "json"), ("*/scenarios.md", "md")):
        found = sorted(night.glob(pattern))
        if found:
            inputs["scenarios"] = (kind, str(found[0]))
            break
    for rel, key, loader in NIGHT_LANE_REPORTS:
        path = night / rel
        if not path.exists():
            continue
        payload = json.loads(path.read_text())
        if loader:
            try:
                payload = loader(payload)
            except ValueError as e:
                raise SystemExit(f"{path}: {e}")
        inputs[key] = payload
    return inputs


def gather(run_dirs):
    """Legacy mode -> the same inputs dict: each positional run dir's
    report.json, classified exactly like discovery (the contamination
    marker honored identically)."""
    inputs = {"durable_runs": [], "ephemeral_runs": [], "contaminated_runs": 0}
    for d in run_dirs:
        report_path = Path(d) / "report.json"
        if not contaminated(inputs, report_path):
            ingest_report(inputs, report_path)
    return inputs


def merge_runs(runs):
    """Min-of-N stats per family for ours and sqlite, reads + writes.

    Values are dicts of percentile -> min-across-runs (the suite's merge
    rule, applied per percentile)."""
    reads, writes = {}, {}
    for r in runs:
        for table, out in ((r["reads"], reads), (r["writes"], writes)):
            for fam in table:
                slot = out.setdefault(fam["name"], {"ours": [], "theirs": []})
                slot["ours"].append(fam["ours"])
                if fam.get("theirs"):
                    slot["theirs"].append(fam["theirs"])
    def merge(rows):
        return {
            k: {
                side: {p: min(s[p] for s in samples) for p in PCTS}
                for side, samples in vv.items() if samples
            }
            for k, vv in rows.items()
        }
    return merge(reads), merge(writes)


def derive_pools(inputs):
    """The merged reads/writes tables ride on the preferred pool —
    durable when non-empty, else ephemeral — with the pool's kind,
    size, corpus scale, host, and shared-machine flag recorded for
    captions (a boosted number never rides a chart without its flag)."""
    for kind in ("durable", "ephemeral"):
        pool = inputs[f"{kind}_runs"]
        if pool:
            inputs["reads"], inputs["writes"] = merge_runs(pool)
            inputs["store_kind"] = kind
            inputs["rep_count"] = len(pool)
            provenance = pool[0].get("provenance") or {}
            inputs["host"] = provenance.get("host", "unknown host")
            inputs["shared_machine"] = any(
                (r.get("provenance") or {}).get("shared_machine") for r in pool)
            inputs["scale"] = (pool[0].get("config") or {}).get("scale", "?")
            return


def prov_note(payload):
    """The shared-machine caveat from the payload's OWN provenance
    stamp — a lane measured on a loaded machine under boosted QoS says
    so on the chart itself (owner ruling 2026-07-20; the protocol lives
    in docs/architecture/61-bench-lanes.md)."""
    provenance = payload.get("provenance") if isinstance(payload, dict) else None
    if isinstance(provenance, dict) and provenance.get("shared_machine"):
        return " · shared-machine night, boosted QoS"
    return ""


def pool_note(inputs):
    """The merged-pool caption tail: min-of-N, store kind, exclusions,
    and the shared-machine caveat."""
    note = f"min-of-{inputs['rep_count']}, {inputs['store_kind']} store"
    if inputs.get("contaminated_runs"):
        n = inputs["contaminated_runs"]
        note += f" · {n} contaminated run{'s' if n != 1 else ''} excluded and counted"
    if inputs.get("shared_machine"):
        note += " · shared-machine night, boosted QoS"
    return note


def derive_write_throughput(inputs):
    """writes-report.json's commit/delete batch ladders -> the
    write_throughput lane payload, THROUGH the lane's contract loader
    (the contract is the adapter's output shape). One derived
    durability lane per (report lane × ladder): the batch ladder is the
    x-axis, rows/sec both engines the y. bulk_append is a single point,
    not a ladder — it stays fully drawn in bench-writes-rates.svg. A
    real write_throughput lane payload, once an emitter writes one,
    wins over this derivation."""
    if inputs.get("write_throughput") or not inputs.get("writes_rates"):
        return
    lanes = []
    for lane in inputs["writes_rates"].get("lanes", []):
        for prefix, label in (("commit_b", "commits"), ("delete_b", "deletes")):
            batches = [{"batch": row["batch"],
                        "ours_facts_per_sec": row["rows_per_sec_ours"],
                        "theirs_facts_per_sec": row["rows_per_sec_theirs"]}
                       for row in lane["rows"] if row["name"].startswith(prefix)]
            if batches:
                lanes.append({"name": f"{lane['lane']} {label}", "batches": batches})
    if lanes:
        payload = {"lane": "write_throughput", "lanes": lanes,
                   "provenance": inputs["writes_rates"].get("provenance")}
        inputs["write_throughput"] = load_write_throughput(payload)


def derive_adversarial(inputs):
    """scenarios.json's exceeded_cap lanes -> the adversarial lane
    payload, THROUGH the lane's contract loader — the DNF data's one
    real home while the adversarial subcommand remains unlanded
    (SKIP-UNAVAILABLE by design). Only capped lanes join: a timed lane
    already draws in the scenario and world charts. The cap => null law
    holds by construction (an exceeded_cap lane HAS no stats). A real
    adversarial lane payload wins over this derivation."""
    if inputs.get("adversarial") or not inputs.get("scenarios"):
        return
    kind, path = inputs["scenarios"]
    if kind != "json":
        return
    queries, caps = [], set()
    for scenario, query, lane_name, ours, lane in load_scenarios_json(path):
        if lane["outcome"] == "exceeded_cap":
            caps.add(lane["cap_ms"])
            queries.append({"name": f"{scenario}/{query}" + lane_suffix(lane_name),
                            "ours": ours, "theirs": None,
                            "theirs_exceeded_cap": True})
    if not queries:
        return
    if len(caps) != 1:
        raise SystemExit(f"{path}: exceeded_cap lanes carry mixed cap_ms values "
                         f"{sorted(caps)} — one adversarial payload cannot state them")
    payload = {"lane": "adversarial", "cap_ms": caps.pop(), "queries": queries}
    inputs["adversarial"] = load_adversarial(payload)


def derive_lanes(inputs):
    """Every derived chart input, in one place, after discovery and
    flags — real payloads always win over derivations."""
    derive_write_throughput(inputs)
    derive_adversarial(inputs)


def _md_p50(cell):
    """One p50 cell from the markdown table: a µs float, or None for the
    honest `DNF>cap` token — a capped lane has no number, so a number is
    never invented for it."""
    return None if cell.startswith("DNF") else float(cell)


def load_scenarios(path):
    """Parse scenarios.md: [(scenario, query, lane, ours_us, sqlite_us)].

    Column indices come from each table's own header row, so the legacy
    6-col pin and the lane-bearing 7-col format both parse — no
    positional indexing. A table without a `lane` column is the
    pre-lane pin: every row is the canonical "sqlite" lane. A `DNF>cap`
    p50 cell parses to None (consumers skip-and-count it under the
    annotation idiom); the rounded ratio column is never read — every
    ratio derives from the raw p50s."""
    rows, scenario, cols = [], None, None
    for line in Path(path).read_text().splitlines():
        if line.startswith("## "):
            scenario = line[3:].split(" (")[0]
        elif line.startswith("|") and scenario and "---" not in line:
            cells = [c.strip() for c in line.strip("|").split("|")]
            if cells[0] == "query":  # the header row names its columns
                cols = {name: index for index, name in enumerate(cells)}
                for needed in ("ours p50 (us)", "sqlite p50 (us)"):
                    if needed not in cols:
                        raise SystemExit(f'{path}: table header lacks "{needed}"')
            elif cols:
                lane = cells[cols["lane"]] if "lane" in cols else "sqlite"
                rows.append((scenario, cells[cols["query"]], lane,
                             _md_p50(cells[cols["ours p50 (us)"]]),
                             _md_p50(cells[cols["sqlite p50 (us)"]])))
    return rows


def load_scenarios_json(path):
    """Parse the runner's scenarios.json: [(scenario, query, lane,
    ours_stats, outcome_dict)] — one row per (query, lane); ours_stats
    is the query's own stats object (ns percentiles), the outcome dict
    verbatim from the emitter's tagged union (crates/bumbledb-bench/src/
    scenarios/json_out.rs): {"outcome": "timed", "stats": {...},
    "ratio_p50": f} or {"outcome": "exceeded_cap", "cap_ms": n}. Unknown
    extra keys are ignored (standard dict access, forward-compatible)."""
    doc = json.loads(Path(path).read_text())
    return [(q["scenario"], q["name"], lane["lane"], q["ours"], lane)
            for q in doc["queries"] for lane in q["lanes"]]


def lane_suffix(lane_name):
    """The ·tuned-style label idiom: the canonical "sqlite" lane rides
    unsuffixed; any other lane suffixes the query label."""
    return "" if lane_name == "sqlite" else "·" + lane_name.removeprefix("sqlite-")


def scenario_rows(scenarios):
    """The one normalized scenario row shape behind the world and
    waterfall charts, from the tagged scenarios input: [(scenario,
    label, ours_ns, sqlite_ns, dnf_note)]. The label carries the lane
    suffix; a DNF lane has sqlite_ns None plus its annotation text, so
    a capped time can never be drawn or ratioed — and every downstream
    ratio derives from these raw p50s (the md's rounded ratio column is
    never read)."""
    kind, path = scenarios
    if kind == "json":
        rows = []
        for scenario, query, lane_name, ours, lane in load_scenarios_json(path):
            label = query + lane_suffix(lane_name)
            if lane["outcome"] == "timed":
                rows.append((scenario, label, ours["p50"],
                             lane["stats"]["p50"], None))
            else:  # exceeded_cap: no stats — the annotation IS the datum
                rows.append((scenario, label, ours["p50"], None,
                             f"DNF > {lane['cap_ms']}ms"))
        return rows
    return [(scenario, query + lane_suffix(lane_name), ours_us * 1000.0,
             None if sqlite_us is None else sqlite_us * 1000.0,
             None if sqlite_us is not None else "DNF > cap")
            for scenario, query, lane_name, ours_us, sqlite_us in load_scenarios(path)]


def load_report(path):
    """One committed lane report JSON, whole."""
    return json.loads(Path(path).read_text())


# --------------------------------------------------------------- style


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


# -------------------------------------------------------------- charts


def chart_vs_sqlite(inputs, out):
    reads = inputs["reads"]
    names = ordered(reads, READ_ORDER)
    fig, ax = plt.subplots(figsize=(9.6, max(6.2, 0.30 * len(names) + 1.6)),
                           facecolor=BG)
    dark(ax)
    paired_bars(ax, names, reads)
    values = [slot[side]["p50"] for slot in reads.values()
              for side in ("ours", "theirs") if side in slot]
    ax.set_xlim(min(values) * 0.4, max(values) * 40)
    ax.set_title(f"read families · p50, min-of-{inputs['rep_count']} · "
                 "same corpus, oracle-verified identical results",
                 fontsize=12, loc="left", pad=14, family="monospace")
    fig.text(0.01, 0.005,
             f"log scale — shorter is faster · {inputs['scale']}-scale corpus · "
             f"{inputs['host']} · {pool_note(inputs)}",
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_speedup(inputs, out):
    reads = inputs["reads"]
    names = [n for n in ordered(reads, READ_ORDER) if "theirs" in reads[n]]
    ratios = [reads[n]["theirs"]["p50"] / reads[n]["ours"]["p50"] for n in names]
    fig, ax = plt.subplots(figsize=(9.6, max(5.2, 0.30 * len(names) + 1.6)),
                           facecolor=BG)
    dark(ax)
    ax.barh(range(len(names)), ratios, height=0.62,
            color=[OURS if r >= 1 else "#f85149" for r in ratios], zorder=3)
    for y, r in enumerate(ratios):
        ax.text(max(r, 1.0) * 1.06, y, f"{r:.0f}×" if r >= 10 else f"{r:.1f}×",
                va="center", fontsize=12, color=OURS if r >= 1 else "#f85149",
                fontweight="bold", family="monospace")
    ax.axvline(1.0, color=DIM, linewidth=1, linestyle="--")
    ax.text(1.0, -0.8, "parity", fontsize=9, color=DIM, ha="center", family="monospace")
    ax.set_yticks(range(len(names)), names, fontsize=11, family="monospace", color=FG)
    ax.invert_yaxis()
    ax.set_xscale("log")
    xlo, xhi = min(0.5, min(ratios) * 0.7), max(700, max(ratios) * 2)
    ax.set_xlim(xlo, xhi)
    ticks = [t for t in (1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000)
             if xlo < t < xhi]
    ax.set_xticks(ticks, [f"{t}×" for t in ticks])
    ax.grid(axis="x", color=GRID, linewidth=0.6, zorder=0)
    ax.set_title("speedup over SQLite · read-family p50 multiples · "
                 f"min-of-{inputs['rep_count']} both sides",
                 fontsize=12, loc="left", pad=14, family="monospace")
    fig.text(0.01, 0.005, pool_note(inputs), fontsize=8, color=DIM,
             family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_tails(inputs, out):
    reads = inputs["reads"]
    names = [n for n in ordered(reads, READ_ORDER) if "theirs" in reads[n]]
    fig, ax = plt.subplots(figsize=(9.6, max(6.2, 0.30 * len(names) + 1.6)),
                           facecolor=BG)
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
    fig.text(0.01, 0.005,
             "bimodal families (containment_walk, balance, skew, chain) show their "
             f"true tails — gated on p95, published anyway · {pool_note(inputs)}",
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_scenarios_lanes(rows, out):
    """The lane-aware bench-scenarios.svg, from scenarios.json rows:
    one bar per (query, lane) — speedup is 1/ratio_p50 (the json's
    ratio is ours/theirs), non-canonical lanes carry a ·suffix in the
    label (r2_temporal_ring·tuned). An exceeded_cap lane draws NO bar —
    a censored bar is unrepresentable because there is no stats object
    to draw — it renders as a right-edge `DNF > {cap_ms}ms` annotation
    beside the query label, excluded and counted in the title line."""
    speeds = [1.0 / lane["ratio_p50"]
              for _, _, _, _, lane in rows
              if lane["outcome"] == "timed" and lane["ratio_p50"] > 0]
    xhi = max(2500, max(speeds) * 3) if speeds else 2500
    fig, ax = plt.subplots(figsize=(9.6, 0.34 * len(rows) + 1.6), facecolor=BG)
    dark(ax)
    y, yticks, ylabels, seen, dnf = 0, [], [], None, 0
    for scenario, query, lane_name, _ours, lane in rows:
        if scenario != seen:
            seen = scenario
            ax.text(0.55, y - 0.15, scenario, fontsize=11, color=FG,
                    fontweight="bold", family="monospace")
            y += 1
        yticks.append(y)
        ylabels.append(query + lane_suffix(lane_name))
        if lane["outcome"] == "timed":
            speed = 1.0 / lane["ratio_p50"] if lane["ratio_p50"] > 0 else 0
            color = OURS if speed >= 1 else "#f85149"
            ax.barh(y, speed, height=0.6, color=color, zorder=3)
            label = f"{speed:.0f}×" if speed >= 10 else f"{speed:.1f}×"
            ax.text(max(speed * 1.06, 1.15), y, label, va="center", fontsize=9,
                    color=color, fontweight="bold", family="monospace")
        else:  # exceeded_cap: no stats, no bar — the annotation IS the datum
            dnf += 1
            ax.text(xhi * 0.85, y, f"DNF > {lane['cap_ms']}ms", va="center",
                    ha="right", fontsize=9, color=THEIRS, fontweight="bold",
                    family="monospace")
        y += 1
    ax.axvline(1.0, color=DIM, linewidth=1, linestyle="--")
    ax.set_yticks(yticks, ylabels, fontsize=9, family="monospace", color=FG)
    ax.set_ylim(y - 0.3, -0.7)
    ax.set_xscale("log")
    ax.set_xlim(0.4, xhi)
    ax.set_xticks([1, 3, 10, 30, 100, 300, 1000],
                  ["1×", "3×", "10×", "30×", "100×", "300×", "1000×"])
    ax.grid(axis="x", color=GRID, linewidth=0.6, zorder=0)
    title = ("scenario worlds · speedup over SQLite per (query, lane) · "
             "oracle-gated, non-ledger corpora")
    if dnf:
        title += f"\n{dnf} lane{'s' if dnf != 1 else ''} DNF > cap — excluded and counted"
    ax.set_title(title, fontsize=12, loc="left", pad=14, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_scenarios(inputs, out):
    kind, path = inputs["scenarios"]
    if kind == "json":
        chart_scenarios_lanes(load_scenarios_json(path), out)
        return
    rows = load_scenarios(path)
    fig, ax = plt.subplots(figsize=(9.6, 0.34 * len(rows) + 1.6), facecolor=BG)
    dark(ax)
    y, yticks, ylabels, seen, dnf = 0, [], [], None, 0
    for scenario, query, lane_name, ours_us, sqlite_us in rows:
        if scenario != seen:
            seen = scenario
            ax.text(0.55, y - 0.15, scenario, fontsize=11, color=FG,
                    fontweight="bold", family="monospace")
            y += 1
        yticks.append(y)
        ylabels.append(query + lane_suffix(lane_name))
        if sqlite_us is None:  # DNF > cap: no number, no bar — skipped, counted
            dnf += 1
            ax.text(2500 * 0.85, y, "DNF > cap", va="center", ha="right",
                    fontsize=9, color=THEIRS, fontweight="bold",
                    family="monospace")
        else:
            # From the raw p50 columns — the markdown's ratio rounds to 2
            # decimals, which floors the >100x queries to 0.00.
            speed = sqlite_us / ours_us if ours_us > 0 else 0
            color = OURS if speed >= 1 else "#f85149"
            ax.barh(y, speed, height=0.6, color=color, zorder=3)
            label = f"{speed:.0f}×" if speed >= 10 else f"{speed:.1f}×"
            ax.text(max(speed * 1.06, 1.15), y, label, va="center", fontsize=9,
                    color=color, fontweight="bold", family="monospace")
        y += 1
    ax.axvline(1.0, color=DIM, linewidth=1, linestyle="--")
    ax.set_yticks(yticks, ylabels, fontsize=9, family="monospace", color=FG)
    ax.set_ylim(y - 0.3, -0.7)
    ax.set_xscale("log")
    ax.set_xlim(0.4, 2500)
    ax.set_xticks([1, 3, 10, 30, 100, 300, 1000],
                  ["1×", "3×", "10×", "30×", "100×", "300×", "1000×"])
    ax.grid(axis="x", color=GRID, linewidth=0.6, zorder=0)
    title = ("scenario worlds · speedup over SQLite per query · "
             "oracle-gated, non-ledger corpora")
    if dnf:
        title += f"\n{dnf} lane{'s' if dnf != 1 else ''} DNF > cap — excluded and counted"
    ax.set_title(title, fontsize=12, loc="left", pad=14, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_worlds(inputs, out):
    """world-<world>.svg, one file per scenario world: horizontal
    paired log-scale p50 bars per (query, lane) label — SQLite grey
    above, ours amber below (the paired_bars idiom; p50s arrive as ns
    so fmt_us and the ratio labels apply unchanged). Consumes the one
    tagged scenarios input — json preferred, header-parsed md fallback.
    A DNF lane has no SQLite number, so it draws NO SQLite bar: a
    right-edge annotation marks it and the title counts it, excluded.
    One registry row emits the N files; render returns the written
    paths."""
    rows = scenario_rows(inputs["scenarios"])
    out_dir = Path(out).parent
    worlds = []
    by_world = {}
    for scenario, label, ours_ns, sqlite_ns, note in rows:
        if scenario not in by_world:
            worlds.append(scenario)
            by_world[scenario] = []
        by_world[scenario].append((label, ours_ns, sqlite_ns, note))
    written = []
    for world in worlds:
        entries = by_world[world]
        names = [label for label, _, _, _ in entries]
        table = {}
        for label, ours_ns, sqlite_ns, _note in entries:
            slot = {"ours": {"p50": ours_ns}}
            if sqlite_ns is not None:
                slot["theirs"] = {"p50": sqlite_ns}
            table[label] = slot
        fig, ax = plt.subplots(figsize=(9.6, 0.62 * len(names) + 1.8), facecolor=BG)
        dark(ax)
        paired_bars(ax, names, table)
        vals = [v for _, o, t, _ in entries for v in (o, t) if v is not None]
        xhi = max(vals) * 25
        ax.set_xlim(min(vals) * 0.4, xhi)
        dnf = 0
        for y, (_label, _ours, sqlite_ns, note) in enumerate(entries):
            if note:  # the capped lane: no bar to draw — the annotation IS the datum
                dnf += 1
                ax.text(xhi * 0.85, y + 0.19, note, va="center", ha="right",
                        fontsize=9, color=THEIRS, fontweight="bold",
                        family="monospace")
        title = f"{world} · ours vs SQLite p50 per query · oracle-gated, report-class"
        if dnf:
            title += (f"\n{dnf} lane{'s' if dnf != 1 else ''} DNF > cap — "
                      "excluded and counted")
        ax.set_title(title, fontsize=12, loc="left", pad=14, family="monospace")
        fig.text(0.01, 0.005, "log scale — shorter is faster",
                 fontsize=8, color=DIM, family="monospace")
        fig.tight_layout()
        outpath = out_dir / f"world-{world}.svg"
        fig.savefig(outpath, facecolor=BG, bbox_inches="tight")
        plt.close(fig)
        written.append(outpath)
    return written


def chart_ratio_waterfall(inputs, out):
    """ratio-waterfall.svg: every read family and every scenario
    (query, lane) as one bar of SQLite-p50 ÷ ours-p50, sorted
    descending — the composite honesty chart; a ratio below parity
    draws red. Scenario rows come through the one tagged input (json
    preferred, header-parsed md fallback); every ratio derives from the
    raw p50s, and a DNF lane — no SQLite number — joins no bar: it is
    excluded and counted in the title."""
    reads = inputs["reads"]
    rows = [(name, slot["theirs"]["p50"] / slot["ours"]["p50"])
            for name, slot in reads.items() if "theirs" in slot]
    srows = (scenario_rows(inputs["scenarios"])
             if inputs.get("scenarios") else [])
    dnf = 0
    for scenario, label, ours_ns, sqlite_ns, _note in srows:
        if sqlite_ns is None:  # DNF > cap: no number to ratio — skipped, counted
            dnf += 1
        elif ours_ns > 0:
            rows.append((f"{scenario}/{label}", sqlite_ns / ours_ns))
    rows.sort(key=lambda row: row[1], reverse=True)
    labels = [name for name, _ in rows]
    ratios = [r for _, r in rows]
    fig, ax = plt.subplots(figsize=(9.6, 0.28 * len(rows) + 1.8), facecolor=BG)
    dark(ax)
    for y, r in enumerate(ratios):
        color = OURS if r >= 1 else "#f85149"
        ax.barh(y, r, height=0.62, color=color, zorder=3)
        ax.text(r * 1.06, y, f"{r:.0f}×" if r >= 10 else f"{r:.1f}×",
                va="center", fontsize=8, color=color, fontweight="bold",
                family="monospace")
    ax.axvline(1.0, color=DIM, linewidth=1, linestyle="--")
    ax.text(1.0, -0.9, "parity", fontsize=9, color=DIM, ha="center",
            family="monospace")
    ax.set_yticks(range(len(labels)), labels, fontsize=8, family="monospace",
                  color=FG)
    ax.invert_yaxis()
    ax.set_xscale("log")
    xlo, xhi = min(0.4, min(ratios) * 0.7), max(ratios) * 3
    ax.set_xlim(xlo, xhi)
    ticks = [t for t in (1, 3, 10, 30, 100, 300, 1000) if xlo < t < xhi]
    ax.set_xticks(ticks, [f"{t}×" for t in ticks])
    ax.grid(axis="x", color=GRID, linewidth=0.6, zorder=0)
    title = ("every family and query · SQLite-p50 ÷ ours-p50, sorted · "
             "report-class composite")
    if dnf:
        title += f"\n{dnf} lane{'s' if dnf != 1 else ''} DNF > cap — excluded and counted"
    ax.set_title(title, fontsize=12, loc="left", pad=14, family="monospace")
    footer = f"ledger+calendar families ({pool_note(inputs)})"
    if srows:
        footer += " + scenario worlds"
    fig.text(0.01, 0.005, footer, fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_tails_fan(inputs, out):
    """tails-fan.svg: the p50 → p90 → p99 fan per read family, both
    engines — the legacy bench-tails.svg (p95) chart stays untouched."""
    reads = inputs["reads"]
    names = [n for n in ordered(reads, READ_ORDER) if "theirs" in reads[n]]
    fig, ax = plt.subplots(figsize=(9.6, max(6.2, 0.30 * len(names) + 1.6)),
                           facecolor=BG)
    dark(ax)
    for y, n in enumerate(names):
        for side, color, dy in (("theirs", THEIRS, 0.18), ("ours", OURS, -0.18)):
            st = reads[n][side]
            ax.plot([st["p50"], st["p99"]], [y + dy, y + dy], color=color,
                    linewidth=2.2, solid_capstyle="round", zorder=3, alpha=0.85)
            ax.plot(st["p50"], y + dy, "o", ms=6, color=color, zorder=4)
            ax.plot(st["p90"], y + dy, "d", ms=4.5, color=color, zorder=4)
            ax.plot(st["p99"], y + dy, "s", ms=3.5, color=color, zorder=4)
    ax.set_yticks(range(len(names)), names, fontsize=10, family="monospace", color=FG)
    ax.invert_yaxis()
    ax.set_xscale("log")
    ax.xaxis.set_major_formatter(FuncFormatter(fmt_us))
    ax.grid(axis="x", color=GRID, linewidth=0.6, zorder=0)
    from matplotlib.lines import Line2D
    ax.legend(handles=[
        Line2D([], [], color=OURS, marker="o", label="bumbledb  p50 ● p90 ◆ p99 ■"),
        Line2D([], [], color=THEIRS, marker="o", label="SQLite"),
    ], loc="lower right", facecolor=BG, edgecolor=GRID, labelcolor=FG, fontsize=9)
    ax.set_title("latency tail fan · p50 → p90 → p99 per read family, both engines",
                 fontsize=12, loc="left", pad=14, family="monospace")
    fig.text(0.01, 0.005,
             f"log scale · line spans p50 → p99 · {pool_note(inputs)}",
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_writes(inputs, out):
    writes = inputs["writes"]
    names = ordered(writes, WRITE_ORDER)
    fig, ax = plt.subplots(figsize=(9.6, max(3.4, 0.55 * len(names) + 1.6)),
                           facecolor=BG)
    dark(ax)
    paired_bars(ax, names, writes, note_ratio=False)
    values = [slot[side]["p50"] for slot in writes.values()
              for side in ("ours", "theirs") if side in slot]
    ax.set_xlim(min(values) * 0.4, max(values) * 40)
    ax.set_title("write + cold families · p50 · where fsync physics rules, honesty does too",
                 fontsize=12, loc="left", pad=14, family="monospace")
    fig.text(0.01, 0.005,
             "durable commits are an fsync-latency product on both engines; "
             f"shown as measured · {pool_note(inputs)}",
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


# -------------------------------------------------- the metric lanes


def chart_storage(inputs, out):
    """bench-storage.svg: bytes per fact per scale, one panel per world
    (engine compacted vs sqlite indexed vs sqlite table-only), absolute
    store bytes annotated; churn checkpoints, when the report carries
    them, as an extra panel of absolute post-state bytes."""
    report = inputs["storage_report"]
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
             f"against the generator stream{prov_note(report)}",
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_writes_rates(inputs, out):
    """bench-writes-rates.svg: rows/sec per (family, batch) row, ours vs
    theirs paired, one panel per durability lane — the lane + sqlite_sync
    labels ride in the panel title, so the number never appears without
    its durability context."""
    report = inputs["writes_rates"]
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
             f"+ body multisets, ids projected out) · log scale{prov_note(report)}",
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_curves(inputs, out):
    """bench-curves.svg: log-log p50-vs-facts lines, one panel per
    family — ours solid, sqlite canonical dashed, sqlite hand-tuned
    dotted; fitted exponents annotated; capped points drawn as open
    markers pinned at the cap ceiling and counted in the footer."""
    report = inputs["curves_report"]
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
    footer += prov_note(report)
    fig.text(0.01, 0.005, footer, fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_warmth(inputs, out):
    """bench-warmth.svg: cold/warm/memoized p50 per warmth-carrying
    family, ours vs sqlite paired per group — the memo effect made an
    explicit chart instead of an implicit flatterer."""
    report = inputs["curves_report"]
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
             f"resolved-filter view slots — the memo effect explicit{prov_note(report)}",
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_write_throughput(inputs, out):
    """write-throughput.svg: facts/sec across commit batch sizes, one
    line per (durability lane × engine) — engine by color, durability
    lane by linestyle, both axes log."""
    lanes = inputs["write_throughput"]["lanes"]
    styles = ("-", "--", ":", "-.")
    fig, ax = plt.subplots(figsize=(9.6, 5.4), facecolor=BG)
    dark(ax)
    batches = sorted({row["batch"] for lane in lanes for row in lane["batches"]})
    for index, lane in enumerate(lanes):
        style = styles[index % len(styles)]
        xs = [row["batch"] for row in lane["batches"]]
        for key, color, engine in (("ours_facts_per_sec", OURS, "bumbledb"),
                                   ("theirs_facts_per_sec", THEIRS, "SQLite")):
            ax.plot(xs, [row[key] for row in lane["batches"]], style,
                    color=color, marker="o", ms=4, linewidth=2, zorder=3,
                    label=f"{lane['name']} · {engine}")
    ax.set_xscale("log", base=2)
    ax.set_yscale("log")
    ax.set_xticks(batches, [str(b) for b in batches])
    ax.yaxis.set_major_formatter(FuncFormatter(fmt_rate))
    ax.set_xlabel("commit batch size (facts per commit)", fontsize=9,
                  family="monospace")
    ax.set_ylabel("facts/sec", fontsize=9, family="monospace")
    ax.grid(color=GRID, linewidth=0.6, zorder=0)
    ax.legend(loc="upper left", facecolor=BG, edgecolor=GRID,
              labelcolor=FG, fontsize=8)
    ax.set_title("write throughput · facts/sec across commit batch sizes, "
                 "per durability lane",
                 fontsize=12, loc="left", pad=14, family="monospace")
    fig.text(0.01, 0.005,
             "durable lanes are an fsync-latency product on both engines — "
             "shown as measured · report-class"
             + prov_note(inputs["write_throughput"]),
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def chart_adversarial_dnf(inputs, out):
    """adversarial-dnf.svg: paired horizontal log bars, ours vs SQLite
    p50 — a capped twin has NO stats to draw (the loader enforced the
    cap => null law), so its bar is the cap itself: hatched, hollow,
    drawn to cap_ms, labeled DNF in red, never given a ratio. The
    payload arrives from an adversarial lane report.json when one
    lands, else derived from scenarios.json's exceeded_cap lanes
    (derive_adversarial) — the same contract either way."""
    report = inputs["adversarial"]
    cap_ms = report["cap_ms"]
    cap_ns = cap_ms * 1e6
    queries = report["queries"]
    fig, ax = plt.subplots(figsize=(9.6, 0.9 * len(queries) + 1.9), facecolor=BG)
    dark(ax)
    peaks, capped_count = [cap_ns], 0
    for y, query in enumerate(queries):
        o = query["ours"]["p50"]
        peaks.append(o)
        if query["theirs_exceeded_cap"]:
            capped_count += 1
            ax.barh(y + 0.19, cap_ns, height=0.34, facecolor="none",
                    edgecolor=THEIRS, hatch="///", linewidth=1.0, zorder=3)
            ax.text(cap_ns * 1.15, y + 0.19, f"DNF > {cap_ms} ms cap",
                    va="center", fontsize=9, color="#f85149",
                    fontweight="bold", family="monospace")
            label = fmt_us(o)  # no ratio against a cap — there is no number
        elif query.get("theirs"):
            t = query["theirs"]["p50"]
            peaks.append(t)
            ax.barh(y + 0.19, t, height=0.34, color=THEIRS, zorder=3)
            ax.text(t * 1.15, y + 0.19, fmt_us(t), va="center", fontsize=8,
                    color=DIM, family="monospace")
            ratio = t / o
            label = fmt_us(o) + (f"   {ratio:.0f}×" if ratio >= 10
                                 else f"   {ratio:.1f}×")
        else:
            label = fmt_us(o)  # no twin at all: ours stands alone
        ax.barh(y - 0.19, o, height=0.34, color=OURS, zorder=3)
        ax.text(o * 1.15, y - 0.19, label, va="center", fontsize=9,
                color=OURS, fontweight="bold", family="monospace")
    ax.set_yticks(range(len(queries)), [q["name"] for q in queries],
                  fontsize=10, family="monospace", color=FG)
    ax.invert_yaxis()
    ax.set_xscale("log")
    ax.set_xlim(min(q["ours"]["p50"] for q in queries) * 0.5, max(peaks) * 60)
    ax.xaxis.set_major_formatter(FuncFormatter(fmt_us))
    ax.grid(axis="x", color=GRID, linewidth=0.6, zorder=0)
    from matplotlib.patches import Patch
    ax.legend(handles=[Patch(color=OURS, label="bumbledb"),
                       Patch(color=THEIRS, label="SQLite"),
                       Patch(facecolor="none", edgecolor=THEIRS, hatch="///",
                             label="SQLite DNF — drawn to the cap")],
              loc="lower right", facecolor=BG, edgecolor=GRID,
              labelcolor=FG, fontsize=8)
    ax.set_title("adversarial DNFs · ours vs SQLite p50 · capped twins shown "
                 "as capped, never as numbers",
                 fontsize=12, loc="left", pad=14, family="monospace")
    fig.text(0.01, 0.005,
             f"{capped_count} of {len(queries)} SQLite twins exceeded the "
             f"{cap_ms} ms per-sample cap — excluded from ratios, counted here",
             fontsize=8, color=DIM, family="monospace")
    fig.tight_layout()
    fig.savefig(out, facecolor=BG, bbox_inches="tight")
    plt.close(fig)


def home_turf_render(key, world, regime, oracle_note):
    """world-crud.svg / world-lawful.svg: the home-turf worlds where
    SQLite is expected to be strong, benched to lose honestly (the
    owner's standing order). One bar per (family, durability lane) —
    speedup = SQLite p50 ÷ ours p50 from the raw stats, grouped under a
    lane header carrying the lane's parity config in the row label. A
    below-parity bar (SQLite faster) draws red and the title COUNTS the
    losses — the caption never spins a loss into a footnote. No DNF arm
    exists in these worlds (the loader enforced both numbers), so every
    row draws a real bar."""
    def render(inputs, out):
        report = inputs[key]
        rows = [(lane, row["family"], row["ours"]["p50"], row["theirs"]["p50"])
                for lane, row in world_report_rows(report)]
        speeds = [t / o for _, _, o, t in rows if o > 0]
        losses = sum(1 for s in speeds if s < 1)
        fig, ax = plt.subplots(figsize=(9.6, 0.34 * (len(rows) + 2) + 1.9),
                               facecolor=BG)
        dark(ax)
        y, yticks, ylabels, seen = 0, [], [], None
        for lane, family, ours_ns, theirs_ns in rows:
            if lane != seen:
                seen = lane
                ax.text(0.28, y - 0.15, f"lane {lane}", fontsize=11, color=FG,
                        fontweight="bold", family="monospace")
                y += 1
            yticks.append(y)
            ylabels.append(family)
            speed = theirs_ns / ours_ns if ours_ns > 0 else 0
            color = OURS if speed >= 1 else "#f85149"
            ax.barh(y, speed, height=0.6, color=color, zorder=3)
            label = (f"{speed:.0f}×" if speed >= 10
                     else f"{speed:.3f}×" if speed < 0.01
                     else f"{speed:.2f}×" if speed < 1 else f"{speed:.1f}×")
            ax.text(max(speed * 1.08, 1.12), y, label, va="center", fontsize=9,
                    color=color, fontweight="bold", family="monospace")
            y += 1
        ax.axvline(1.0, color=DIM, linewidth=1, linestyle="--")
        ax.text(1.0, -0.45, "parity", fontsize=9, color=DIM, ha="center",
                family="monospace")
        ax.set_yticks(yticks, ylabels, fontsize=9, family="monospace", color=FG)
        ax.set_ylim(y - 0.3, -0.7)
        ax.set_xscale("log")
        xlo = min(0.25, min(speeds) * 0.6) if speeds else 0.25
        xhi = max(6, max(speeds) * 4) if speeds else 6
        ax.set_xlim(xlo, xhi)
        ticks = [t for t in (0.1, 0.3, 1, 3, 10, 30, 100) if xlo < t < xhi]
        ax.set_xticks(ticks, [f"{t:g}×" for t in ticks])
        ax.grid(axis="x", color=GRID, linewidth=0.6, zorder=0)
        title = (f"{world} — {regime} · speedup over SQLite "
                 "per (family, lane) · report-class")
        if losses:
            title += (f"\nSQLite wins {losses} of {len(rows)} rows — "
                      "drawn red, below parity, as measured")
        ax.set_title(title, fontsize=12, loc="left", pad=14, family="monospace")
        fig.text(0.01, 0.005,
                 f"seed {report.get('seed', '?')} · {oracle_note} · red = SQLite "
                 f"faster — benched to lose honestly{prov_note(report)}",
                 fontsize=8, color=DIM, family="monospace")
        fig.tight_layout()
        fig.savefig(out, facecolor=BG, bbox_inches="tight")
        plt.close(fig)
    return render


def chart_churn_series(inputs, out, stem, values_of, formatter, yscale, what):
    """The one churn time-series scaffold behind every churn chart: one
    FILE PER RUN (the world-*.svg multi-file idiom), one line per lane
    in that run — nothing condensed away, nothing hidden. Engine picks
    the color (ours amber, SQLite grey), the Nth lane of an engine
    picks the Nth linestyle. Every sample whose maintenance_ns > 0 (the
    sqlite-maint lane's VACUUM+ANALYZE window, recorded BY the run)
    draws a downward triangle on that lane's point — the marker is data
    on the sample, so it can never drift from the measurement it
    annotates. The title names the run, its cycle mix, and the working
    set; yscale "auto" goes log only past a 20x value spread."""
    report = inputs["churn_report"]
    config = report.get("config", {})
    out_dir = Path(out).parent
    written = []
    for run in report["runs"]:
        fig, ax = plt.subplots(figsize=(9.6, 4.8), facecolor=BG)
        dark(ax)
        values = churn_series(ax, run, values_of)
        if yscale == "auto":
            scale = "log" if max(values) / max(min(values), 1e-12) > 20 else "linear"
        else:
            scale = yscale
        ax.set_yscale(scale)
        ax.yaxis.set_major_formatter(FuncFormatter(formatter))
        # A series spanning less than a decade on a log axis auto-labels
        # minor ticks in scientific notation — keep the lane's voice there.
        if scale == "log" and max(values) / max(min(values), 1e-12) < 10:
            ax.yaxis.set_minor_formatter(FuncFormatter(formatter))
        ax.xaxis.set_major_locator(MaxNLocator(integer=True))
        ax.set_xlabel("cycle", fontsize=9, family="monospace")
        ax.grid(color=GRID, linewidth=0.6, zorder=0)
        ax.legend(loc="best", facecolor=BG, edgecolor=GRID, labelcolor=FG,
                  fontsize=8)
        mix = run.get("mix", {})
        mix_note = " ".join(f"{k}={v}" for k, v in mix.items())
        ax.set_title(f"churn · run {run['name']} ({mix_note}, working set "
                     f"{run.get('working_set', '?')}) · {what}",
                     fontsize=12, loc="left", pad=14, family="monospace")
        fig.text(0.01, 0.005, churn_footer(report, config), fontsize=8,
                 color=DIM, family="monospace")
        fig.tight_layout()
        outpath = out_dir / f"{stem}-{run['name']}.svg"
        fig.savefig(outpath, facecolor=BG, bbox_inches="tight")
        plt.close(fig)
        written.append(outpath)
    return written


CHURN_LINE_STYLES = ("-", "--", ":", "-.")


def churn_lane_styles(run):
    """lane name -> (color, linestyle): engine picks the color, the Nth
    lane of an engine picks the Nth linestyle — every lane in the run
    draws distinguishably, by construction."""
    counts, styles = {}, {}
    for lane in run["lanes"]:
        n = counts.get(lane["engine"], 0)
        counts[lane["engine"]] = n + 1
        color = OURS if lane["engine"] == "bumbledb" else THEIRS
        styles[lane["lane"]] = (color, CHURN_LINE_STYLES[n % len(CHURN_LINE_STYLES)])
    return styles


def churn_series(ax, run, values_of):
    """Every lane of one run onto one axes through the values_of
    accessor; maintenance samples (VACUUM/ANALYZE charged, from the
    data) get the triangle marker and the lane's legend label says so.
    Returns every plotted value (for the scale decision)."""
    styles = churn_lane_styles(run)
    values = []
    for lane in run["lanes"]:
        pts = [(s["cycle"], values_of(s)) for s in lane["samples"]]
        pts = [(x, y) for x, y in pts if y is not None]
        if not pts:
            continue
        color, style = styles[lane["lane"]]
        maint = [(s["cycle"], values_of(s)) for s in lane["samples"]
                 if s["maintenance_ns"] > 0 and values_of(s) is not None]
        label = lane["lane"] + (" (▼ VACUUM+ANALYZE)" if maint else "")
        ax.plot([x for x, _ in pts], [y for _, y in pts], style, color=color,
                marker="o", ms=3, linewidth=1.8, zorder=3, label=label)
        if maint:
            ax.plot([x for x, _ in maint], [y for _, y in maint], "v", ms=7,
                    color=color, zorder=4)
        values += [y for _, y in pts]
    return values


def churn_footer(report, config):
    """The one churn caption: the protocol strides from the report's own
    config, plus the provenance caveat."""
    return (f"churn lane · {config.get('cycles', '?')} cycles sampled every "
            f"{config.get('sample_every', '?')} · sqlite-maint VACUUM every "
            f"{config.get('vacuum_every', '?')} / ANALYZE every "
            f"{config.get('analyze_every', '?')}, charged as maintenance · "
            f"report-class{prov_note(report)}")


def churn_probe_value(probe_name):
    """A values_of accessor for one probe's p50 at a sample (None when
    the sample lacks the probe — drawn as nothing, never as zero)."""
    def value(sample):
        for probe in sample["probes"]:
            if probe["name"] == probe_name:
                return probe["p50_ns"]
        return None
    return value


def chart_churn_latency(inputs, out):
    """churn-latency-<run>.svg, one file per run: every probe family as
    its own panel, every lane as its own line, warm p50 over cycles —
    the degradation story with nothing condensed away."""
    report = inputs["churn_report"]
    config = report.get("config", {})
    out_dir = Path(out).parent
    written = []
    for run in report["runs"]:
        probes = []
        for lane in run["lanes"]:
            for probe in lane["samples"][0]["probes"]:
                if probe["name"] not in probes:
                    probes.append(probe["name"])
        fig, axes = plt.subplots(len(probes), 1, facecolor=BG,
                                 figsize=(9.6, 3.0 * len(probes) + 0.9))
        flat = [axes] if len(probes) == 1 else list(axes)
        for ax, probe_name in zip(flat, probes):
            dark(ax)
            churn_series(ax, run, churn_probe_value(probe_name))
            ax.set_yscale("log")
            ax.yaxis.set_major_formatter(FuncFormatter(fmt_us))
            ax.xaxis.set_major_locator(MaxNLocator(integer=True))
            ax.grid(color=GRID, linewidth=0.6, zorder=0)
            ax.set_title(f"{probe_name} · warm p50 over cycles", fontsize=10,
                         loc="left", pad=8, family="monospace")
            if ax is flat[0]:
                ax.legend(loc="best", facecolor=BG, edgecolor=GRID,
                          labelcolor=FG, fontsize=8)
            if ax is flat[-1]:
                ax.set_xlabel("cycle", fontsize=9, family="monospace")
        mix = run.get("mix", {})
        mix_note = " ".join(f"{k}={v}" for k, v in mix.items())
        fig.suptitle(f"churn · run {run['name']} ({mix_note}, working set "
                     f"{run.get('working_set', '?')}) · probe latency",
                     x=0.01, y=0.998, ha="left", fontsize=12, color=FG,
                     family="monospace")
        fig.text(0.01, 0.002, churn_footer(report, config), fontsize=8,
                 color=DIM, family="monospace")
        fig.tight_layout(rect=(0, 0.015, 1, 0.97))
        outpath = out_dir / f"churn-latency-{run['name']}.svg"
        fig.savefig(outpath, facecolor=BG, bbox_inches="tight")
        plt.close(fig)
        written.append(outpath)
    return written


def churn_metric_render(stem, values_of, formatter, yscale, what):
    """One churn metric -> a registry render fn over the per-run
    scaffold."""
    def render(inputs, out):
        return chart_churn_series(inputs, out, stem=stem, values_of=values_of,
                                  formatter=formatter, yscale=yscale, what=what)
    return render


# ---------------------------------------------------------- the registry


@dataclass(frozen=True)
class ChartSpec:
    """One chart as data: what it's called, what it needs, how it draws.

    A render fn may return the list of paths it wrote (a spec that
    emits one file per world); None means the single outpath."""
    filename: str
    requires: tuple  # inputs keys, all required present and truthy
    render: object   # fn(inputs, outpath) -> None | [written paths]


CHARTS = [
    ChartSpec("bench-vs-sqlite.svg", ("reads",), chart_vs_sqlite),
    ChartSpec("bench-speedup.svg", ("reads",), chart_speedup),
    ChartSpec("bench-tails.svg", ("reads",), chart_tails),
    ChartSpec("bench-writes.svg", ("writes",), chart_writes),
    ChartSpec("bench-scenarios.svg", ("scenarios",), chart_scenarios),
    ChartSpec("world-<world>.svg", ("scenarios",), chart_worlds),
    ChartSpec("ratio-waterfall.svg", ("reads",), chart_ratio_waterfall),
    ChartSpec("tails-fan.svg", ("reads",), chart_tails_fan),
    ChartSpec("bench-storage.svg", ("storage_report",), chart_storage),
    ChartSpec("bench-writes-rates.svg", ("writes_rates",), chart_writes_rates),
    ChartSpec("bench-curves.svg", ("curves_report",), chart_curves),
    ChartSpec("bench-warmth.svg", ("curves_report",), chart_warmth),
    ChartSpec("write-throughput.svg", ("write_throughput",), chart_write_throughput),
    ChartSpec("adversarial-dnf.svg", ("adversarial",), chart_adversarial_dnf),
    ChartSpec("world-crud.svg", ("crud_report",),
              home_turf_render("crud_report", "crud",
                               "the OLTP home turf, SQLite's strong regime",
                               "oracle-gated read query + post-state "
                               "value-verified, both relations, both lanes")),
    ChartSpec("world-lawful.svg", ("lawful_report",),
              home_turf_render("lawful_report", "lawful",
                               "the integrity home turf: judged-law admission "
                               "vs SQL constraints",
                               "post-state fold over all five relations + "
                               "naive verdict parity")),
    ChartSpec("churn-latency-<run>.svg", ("churn_report",), chart_churn_latency),
    ChartSpec("churn-size-<run>.svg", ("churn_report",),
              churn_metric_render("churn-size", lambda s: s["disk_bytes"],
                                  fmt_bytes, "auto", "store size over cycles")),
    ChartSpec("churn-throughput-<run>.svg", ("churn_report",),
              churn_metric_render("churn-throughput",
                                  lambda s: s["commits_per_sec"], fmt_rate,
                                  "auto", "write commits/sec over cycles")),
]


# --------------------------------------------------------------- main


def main():
    ap = argparse.ArgumentParser(
        description="Render the README benchmark charts from committed report pins.")
    ap.add_argument("run_dirs", nargs="*", metavar="run-dir",
                    help="suite run directories, each holding a report.json")
    ap.add_argument("--scenarios", metavar="PATH",
                    help="a committed scenarios.md (legacy per-query table) or "
                         "scenarios.json (the lane-aware machine artifact); "
                         "dispatched on the extension")
    ap.add_argument("--night", metavar="DIR",
                    help="a night out-dir to discover: */report.json through the "
                         "lane discriminant, plus */scenarios.md")
    ap.add_argument('--out', '--out-dir', dest="out", default="assets", metavar="DIR",
                    help="output directory (default: assets — the owner's ceremony path)")
    ap.add_argument("--storage-report", metavar="PATH",
                    help="a committed storage-report.json (fills the storage_report "
                         "artifact behind bench-storage.svg)")
    ap.add_argument("--writes-report", metavar="PATH",
                    help="a committed writes-report.json (fills the writes_rates lane)")
    ap.add_argument("--curves-report", metavar="PATH",
                    help="a committed curves-report.json (fills the curves_report "
                         "artifact behind bench-curves.svg / bench-warmth.svg)")
    args = ap.parse_args()

    if args.run_dirs and args.night:
        ap.error("pass run dirs or --night, not both")
    lane_flags = ((args.storage_report, "storage_report"),
                  (args.writes_report, "writes_rates"),
                  (args.curves_report, "curves_report"))
    if not (args.run_dirs or args.night or any(path for path, _ in lane_flags)):
        ap.error("nothing to render: pass run dirs, --night, or a lane-report flag")

    inputs = discover(args.night) if args.night else gather(args.run_dirs)
    if args.scenarios:
        kind = "json" if args.scenarios.endswith(".json") else "md"
        inputs["scenarios"] = (kind, args.scenarios)
    derive_pools(inputs)
    for path, key in lane_flags:
        if path:
            inputs[key] = load_report(path)
    derive_lanes(inputs)

    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    for spec in CHARTS:
        missing = [k for k in spec.requires if not inputs.get(k)]
        if missing:
            print(f"SKIP {spec.filename} (missing: {', '.join(missing)})")
            continue
        outpath = out_dir / spec.filename
        written = spec.render(inputs, outpath)
        for path in written if written is not None else [outpath]:
            print(f"wrote {path}")

    reads = inputs.get("reads") or {}
    for name in READ_ORDER:
        if name in reads and "theirs" in reads[name]:
            r = reads[name]
            print(f"{name:10} ours {fmt_us(r['ours']['p50']):>8}  sqlite {fmt_us(r['theirs']['p50']):>8}  "
                  f"{r['theirs']['p50'] / r['ours']['p50']:5.1f}x")


if __name__ == "__main__":
    main()
