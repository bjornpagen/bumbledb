#!/usr/bin/env python3
"""Qualify native Event field dependencies; not the complete M3 or M0–M8 gate.

Keeps logs and hashes of the exact native sources before/after the commands.
An existing output directory is never overwritten. Benchmarks are not run.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time


ROOT = Path(__file__).resolve().parent.parent


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def native_sources():
    files = {ROOT / name for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml")}
    files.add(Path(__file__).resolve())
    for crate in [*sorted((ROOT / "crates").iterdir()), ROOT / "ts/crate"]:
        for name in ("Cargo.toml", "Cargo.lock", "build.rs"):
            if (crate / name).is_file():
                files.add(crate / name)
        for name in ("src", "tests", "semantics"):
            folder = crate / name
            if folder.is_dir():
                files.update(path for path in folder.rglob("*") if path.is_file())
    return {str(path.relative_to(ROOT)): digest(path) for path in sorted(files)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True, help="New evidence directory")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    shutil.copy2(__file__, args.output / "check.py")
    before = native_sources()
    checks = [
        ("format", ["cargo", "fmt", "--all", "--check"]),
        ("check", ["cargo", "check", "--workspace", "--all-targets"]),
        ("core", ["cargo", "test", "-p", "bumbledb-event"]),
        ("engine", ["cargo", "test", "-p", "bumbledb", "--lib"]),
        ("integration", ["cargo", "test", "-p", "bumbledb", "--test", "event_dependencies",
                         "--test", "event_storage", "--test", "keyed_get", "--test", "api",
                         "--test", "schema_macro"]),
        ("reference", ["cargo", "test", "-p", "bumbledb-bench", "--lib", "correspondence"]),
        ("query-macro", ["cargo", "test", "-p", "bumbledb-query", "--test", "event"]),
        ("log", ["cargo", "test", "-p", "bumbledb-log", "--lib"]),
        ("clippy", ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]),
        ("node-check", ["cargo", "check", "--manifest-path", "ts/crate/Cargo.toml", "--all-targets"]),
        ("node", ["cargo", "test", "--manifest-path", "ts/crate/Cargo.toml", "--lib"]),
        ("node-clippy", ["cargo", "clippy", "--manifest-path", "ts/crate/Cargo.toml",
                         "--all-targets", "--", "-D", "warnings"]),
    ]
    result = {
        "scope": "Native unmeasured Event field dependencies; contextual constants, complete M3, M4–M8 and performance remain open",
        "base_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "rustc": subprocess.check_output(["rustc", "-Vv"], cwd=ROOT, text=True).strip(),
        "source_sha256": before,
        "checks": [],
    }
    for name, command in checks:
        log = args.output / f"{name}.log"
        started = time.monotonic()
        with log.open("w") as output:
            try:
                run = subprocess.run(command, cwd=ROOT, stdout=output, stderr=subprocess.STDOUT,
                                     timeout=600, check=False)
                code = run.returncode
            except subprocess.TimeoutExpired:
                code = -1
                output.write("\nQualification command exceeded its 600-second limit.\n")
        result["checks"].append({"name": name, "command": command, "exit_code": code,
                                 "elapsed_seconds": round(time.monotonic() - started, 3),
                                 "log": log.name, "sha256": digest(log)})
        result["sources_unchanged"] = native_sources() == before
        result["passed"] = (len(result["checks"]) == len(checks)
                            and all(row["exit_code"] == 0 for row in result["checks"])
                            and result["sources_unchanged"])
        (args.output / "check.json").write_text(json.dumps(result, indent=2) + "\n")
        print(name, "passed" if code == 0 else "FAILED", flush=True)
        if code != 0 or not result["sources_unchanged"]:
            raise SystemExit(1)
    raise SystemExit(0 if result["passed"] else 1)


if __name__ == "__main__":
    main()
