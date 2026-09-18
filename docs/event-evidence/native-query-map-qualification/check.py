#!/usr/bin/env python3
"""Qualify finite Event maps and the optional relational/information/action layer.

Retains exact source hashes, bounded command logs and their exit status. This
does not benchmark kernels or assert that Lean verifies the Rust implementation.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parent.parent


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sources():
    files = {ROOT / name for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml")}
    files.add(Path(__file__).resolve())
    files.add(ROOT / "scripts/check-event-query-wire.cjs")
    for folder in (ROOT / "ts/src", ROOT / "ts/test"):
        files.update(path for path in folder.rglob("*") if path.is_file())
    files.update(ROOT / "ts" / name for name in ("package.json", "tsconfig.json"))
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
    parser.add_argument("--relations", action="store_true", help="Include relation, program, partition and strategy result integration")
    parser.add_argument("--queries", action="store_true", help="Include constructive Event/Test heads and raw Node query transport")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    shutil.copy2(__file__, args.output / "check.py")
    before = sources()
    checks = [
        ("format", ["cargo", "fmt", "--all", "--check"]),
        ("check", ["cargo", "check", "--workspace", "--all-targets"]),
        ("core", ["cargo", "test", "-p", "bumbledb-event"]),
        ("engine", ["cargo", "test", "-p", "bumbledb", "--lib"]),
        ("integration", ["cargo", "test", "-p", "bumbledb", "--test", "event_maps",
                         "--test", "event_core", "--test", "event_dependencies",
                         "--test", "event_full", "--test", "event_storage"]),
        ("clippy", ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]),
        ("node-check", ["cargo", "check", "--manifest-path", "ts/crate/Cargo.toml", "--all-targets"]),
        ("node-clippy", ["cargo", "clippy", "--manifest-path", "ts/crate/Cargo.toml",
                         "--all-targets", "--", "-D", "warnings"]),
    ]
    if args.relations:
        checks[4][1].extend(["--test", "event_relations", "--test", "event_fixed_points",
                             "--test", "event_partitions", "--test", "event_actions"])
    if args.queries:
        checks[4][1].extend(["--test", "event_queries", "--test", "event_pack", "--test", "event_query_maps"])
        library = "libbumbledb_node.dylib" if sys.platform == "darwin" else "libbumbledb_node.so"
        checks.extend([
            ("query-macros", ["cargo", "test", "-p", "bumbledb-query", "-p", "bumbledb-query-macros"]),
            ("node-tests", ["cargo", "test", "--manifest-path", "ts/crate/Cargo.toml", "--lib"]),
            ("node-build", ["cargo", "build", "--manifest-path", "ts/crate/Cargo.toml"]),
            ("query-wire", ["node", "scripts/check-event-query-wire.cjs", "ts/crate/target/debug/" + library]),
            ("typescript", [str(ROOT / "ts/node_modules/.bin/tsc"), "--noEmit", "-p", "ts/tsconfig.json"]),
            ("query-parser", ["node", "--conditions=bumbledb-src", "--test",
                              "ts/test/parse-query-ir.test.ts", "ts/test/wire-tags.test.ts"]),
        ])
    result = {
        "query_scope": ("Complete-binding Event/Test heads, captured readout imports, grouped Event Pack, typed per-occurrence context faults and raw Node descriptors; relation faces/programs, full SDK and certified factoring remain open" if args.queries else None),
        "scope": ("Native finite Event maps, relations, diagram inspection, information, sealed fixed-point programs, indexed partitions and fully observed strategy witnesses; belief-memory/source/full query integration and M0–M8 remain open; no performance qualification"
                  if args.relations else
                  "Native checked finite Event maps; complete M4 and M0–M8 remain open; no performance qualification"),
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
        result["sources_unchanged"] = sources() == before
        result["passed"] = (len(result["checks"]) == len(checks)
                            and all(row["exit_code"] == 0 for row in result["checks"])
                            and result["sources_unchanged"])
        (args.output / "check.json").write_text(json.dumps(result, indent=2) + "\n")
        print(name, "passed" if code == 0 else "FAILED", flush=True)
        if code != 0 or not result["sources_unchanged"]:
            raise SystemExit(1)


if __name__ == "__main__":
    main()
