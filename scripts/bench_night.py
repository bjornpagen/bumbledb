#!/usr/bin/env python3
"""Measure lanes serially by default; opt into concurrent load explicitly."""

from __future__ import annotations

import argparse
from collections import deque
from dataclasses import dataclass
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

from bench_scheduler import policy_for_host, worker_command

REPO = Path(__file__).resolve().parent.parent
PREREQUISITES = {
    "correspondence-oracles": "cargo test -p bumbledb-bench correspondence",
    "three-way-conformance": "cargo test -p bumbledb-bench three_way_conformance -- --ignored",
    "hosted-decision": "real S3/IAM required",
    "large-populated": ">40 GiB allocated blocks + cgroup memory.max required",
    "graviton-arm64": "real Graviton Linux ARM64 performance host required",
    "x86-node": "Linux x64 Node application timing required",
}


def job_count(value):
    if value == "auto":
        return None
    if not value.isdecimal() or int(value) < 1:
        raise argparse.ArgumentTypeError("jobs must be 'auto' or a positive integer")
    return int(value)


@dataclass(frozen=True)
class Job:
    name: str
    command: list
    artifact: Path | None = None


def lanes(binary, data, out, full):
    def job(name, command, artifact, *flags):
        return Job(name, [str(binary), command, "--dir", str(data / name),
                          *flags, "--out", str(out / name)], out / name / artifact)

    compact = [
        job("storage", "storage", "storage-report.json", "--scales", "S,M"),
        job("app-perf-warm", "app-perf", "app-perf.json", "--regimes", "warm"),
        job("app-perf-cold", "app-perf", "app-perf.json", "--regimes", "cold-open,post-write"),
        job("app-perf-large-result", "app-perf", "app-perf.json", "--regimes", "large-result"),
        job("app-perf-tenants", "app-perf", "app-perf.json", "--regimes", "tenant-churn"),
        Job("hash-probe", [str(binary), "hash-probe", "--out", str(out / "hash-probe")],
            out / "hash-probe/hash-probe.json"),
    ]
    if not full:
        return compact
    # Longest lanes first also keeps explicit concurrent-load runs occupied.
    return [
        job("curves", "curves", "curves-report.json", "--scales", "S,M,L", "--warmth"),
        job("reads", "bench", "report.json", "--read-batch", "1"),
        job("scenarios", "scenarios", "scenarios.json"),
        job("writes", "writes", "writes-report.json"),
        *compact,
        job("crud", "crud", "crud.json"),
        job("lawful", "lawful", "lawful.json"),
        job("heap", "heap", "heap-report.json"),
    ]


def now():
    return datetime.now(timezone.utc).isoformat()


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def run_jobs(jobs, workers, out, env, records, persist, cpu_ids=None):
    """Bound child processes, retain failures, and reap every child on interruption."""
    queue = deque(jobs)
    active = {}
    try:
        while queue or active:
            while queue and len(active) < workers:
                job = queue.popleft()
                log = (out / f"{job.name}.log").open("xb")
                try:
                    process = subprocess.Popen(worker_command(job.command, cpu_ids), cwd=REPO, env=env,
                                               stdout=log, stderr=subprocess.STDOUT,
                                               start_new_session=True)
                except BaseException:
                    log.close()
                    raise
                records[job.name] = {"status": "RUNNING", "command": job.command,
                                     "started": now(), "pid": process.pid}
                active[process] = (job, log)
                print(f"[{now()}] START {job.name} (pid {process.pid})", flush=True)
                persist()
            for process, (job, log) in list(active.items()):
                rc = process.poll()
                if rc is None:
                    continue
                log.close()
                del active[process]
                row = records[job.name]
                row.update(exit_code=rc, finished=now(), status="RUN-OK" if rc == 0 else "RUN-FAIL")
                if rc == 0 and job.artifact is not None:
                    try:
                        # Exit zero without a complete report is not successful measurement.
                        payload = json.loads(job.artifact.read_text())
                        if not isinstance(payload, dict) or not payload:
                            raise ValueError("report must be a nonempty JSON object")
                        row.update(artifact=str(job.artifact), sha256=sha256(job.artifact))
                    except (OSError, ValueError) as error:
                        row.update(status="RUN-FAIL", error=str(error))
                print(f"[{now()}] {row['status']} {job.name} (exit {rc}; {job.name}.log)", flush=True)
                persist()
            if active:
                time.sleep(0.1)
    finally:
        # Each child has its own session, so descendants cannot escape cleanup.
        for process in active:
            if process.poll() is None:
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
        for process, (job, log) in active.items():
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
            log.close()
            records[job.name].update(status="INTERRUPTED", finished=now())
        if active:
            persist()
    return all(records[job.name]["status"] == "RUN-OK" for job in jobs)


def parser():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("out", type=Path, help="fresh output directory; existing directories are refused")
    ap.add_argument("--plan", action="store_true", help="print the lane roster without running or writing")
    ap.add_argument("--full", action="store_true", help="include all ordinary benchmark lanes")
    ap.add_argument("--shared", action="store_true", help="boost scheduler QoS and record shared-machine provenance")
    ap.add_argument("--jobs", type=job_count, default=1, metavar="auto|N",
                    help="lane workers (default: 1 for latency comparisons); auto explicitly opts into P-core-count concurrent load")
    ap.add_argument("--cpus", help="Linux performance CPU IDs, e.g. 0-7,16-23; required on Linux")
    ap.add_argument("--allow-macos-qos", action="store_true",
                    help="accept macOS QoS steering, which cannot guarantee P-core-only placement")
    return ap


def main():
    args = parser().parse_args()
    scheduler = policy_for_host(args.cpus)
    workers = scheduler["worker_limit"] if args.jobs is None else args.jobs
    if workers > scheduler["worker_limit"]:
        raise ValueError(f"--jobs {workers} exceeds the selected performance CPU count {scheduler['worker_limit']}")
    out = args.out.resolve()
    data = Path(os.environ.get("BUMBLEDB_BENCH_DATA", REPO / "bench-data")).resolve()
    target = Path(os.environ.get("CARGO_TARGET_DIR", REPO / "target")).resolve()
    binary = Path(os.environ.get("BUMBLEDB_BENCH_BIN", target / "release/bumbledb-bench")).resolve()
    jobs = lanes(binary, data, out, args.full)
    setup = [Job("scorecard-plan", [str(binary), "app-perf", "--plan"]),
             Job("verify", [str(binary), "verify", "--dir", str(data / "reads")])]
    if args.plan:
        print(f"workers: {workers}; scheduler: {json.dumps(scheduler)}; full: {args.full}")
        for job in setup + jobs:
            print(f"{job.name}\tRUN\t{job.artifact or 'SETUP'}")
        for name, reason in PREREQUISITES.items():
            print(f"{name}\tNOTRUN-PREREQ\t{reason}")
        return 0
    if sys.platform == "darwin" and not args.allow_macos_qos:
        raise ValueError("macOS cannot guarantee P-core-only placement. No benchmarks started. "
                         "Use Linux CPU affinity for a hard constraint, or explicitly accept "
                         "QoS steering with --allow-macos-qos")
    if out.exists():
        raise ValueError(f"output already exists: {out}; use a fresh directory (no stale reports are reused)")
    lock = Path(os.environ.get("BUMBLEDB_MEASURE_LOCK", "/tmp/bumbledb.measure.lock"))
    if os.environ.get("BENCH_NIGHT_UNDER_LOCK") != "1":
        if lock.exists():
            raise ValueError(f"measurement lock held: {lock}; wait for the other measurement")
        env = dict(os.environ, BENCH_NIGHT_UNDER_LOCK="1")
        os.execve(str(REPO / "scripts/measure.sh"),
                  [str(REPO / "scripts/measure.sh"), sys.executable, str(Path(__file__).resolve()), *sys.argv[1:]], env)
    if not os.environ.get("BUMBLEDB_BENCH_BIN"):
        subprocess.run(["cargo", "build", "--release", "-p", "bumbledb-bench"], cwd=REPO, check=True)
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ValueError(f"missing executable: {binary}")
    out.mkdir(parents=True)
    # Both engines execute inside the same explicitly prioritized process.
    env = dict(os.environ, BUMBLEDB_BENCH_JOBS=str(workers), BUMBLEDB_BENCH_BOOST="1")
    records = {}
    manifest = {"format": 1, "started": now(), "binary": str(binary),
                "binary_sha256": sha256(binary), "source_revision": subprocess.check_output(
                    ["git", "rev-parse", "HEAD"], cwd=REPO, text=True).strip(),
                "runner_sha256": sha256(Path(__file__)), "workers": workers,
                "scheduler_sha256": sha256(REPO / "scripts/bench_scheduler.py"),
                "scheduler": scheduler, "shared_machine": args.shared,
                "full": args.full, "corpus": str(data), "status": "RUNNING",
                "lanes": records, "not_run_prerequisites": PREREQUISITES}

    def persist():
        temporary = out / "MANIFEST.json.tmp"
        temporary.write_text(json.dumps(manifest, indent=2) + "\n")
        temporary.replace(out / "MANIFEST.json")

    persist()
    try:
        for job in setup:
            if not run_jobs([job], 1, out, env, records, persist, scheduler["cpu_ids"]):
                manifest["status"] = "SETUP-FAILED"
                return 1
        good = run_jobs(jobs, workers, out, env, records, persist, scheduler["cpu_ids"])
        if good:
            note = f"{workers} lane workers; {scheduler['placement']}; {scheduler['priority']}; shared CPU/I/O"
            good = run_jobs([Job("charts", [sys.executable, str(REPO / "scripts/bench_viz.py"),
                "--night", str(out), "--out", str(out), "--note", note])],
                1, out, env, records, persist)
        manifest["status"] = "LOCAL-LANES-COMPLETE" if good else "INCOMPLETE"
        return 0 if good else 1
    finally:
        if manifest["status"] == "RUNNING":
            manifest["status"] = "INTERRUPTED"
        manifest["finished"] = now()
        persist()
        lines = [f"bumbledb bench night: {manifest['status']}",
                 f"workers: {workers}; scheduler: {json.dumps(scheduler)}",
                 f"source: {manifest['source_revision']}", f"binary sha256: {manifest['binary_sha256']}"]
        lines += [f"{name}\t{row['status']}\t{row.get('artifact', 'SETUP')}" for name, row in records.items()]
        lines += [f"{name}\tNOTRUN-PREREQ\t{reason}" for name, reason in PREREQUISITES.items()]
        (out / "MANIFEST.txt").write_text("\n".join(lines) + "\n")
        print("\n".join(lines), flush=True)


if __name__ == "__main__":
    def terminate(_signum, _frame):
        raise KeyboardInterrupt

    signal.signal(signal.SIGTERM, terminate)
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"bench-night: {error}", file=sys.stderr)
        sys.exit(2)
    except KeyboardInterrupt:
        sys.exit(130)
