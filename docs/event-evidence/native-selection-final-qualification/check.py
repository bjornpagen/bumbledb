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
    files.add(ROOT / "scripts/event-relation-wire.cjs")
    files.add(ROOT / "scripts/check-event-sdk.mjs")
    files.add(ROOT / "scripts/check-event-ingress.cjs")
    for folder in (ROOT / "ts/src", ROOT / "ts/test", ROOT / "ts/scripts"):
        files.update(path for path in folder.rglob("*") if path.is_file())
    files.update(ROOT / "ts" / name for name in ("package.json", "tsconfig.json", "tsconfig.build.json", "pnpm-lock.yaml", "pnpm-workspace.yaml"))
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
    parser.add_argument("--sdk", action="store_true", help="Include isolated Event SDK and related regression suites (requires --queries)")
    args = parser.parse_args()
    if args.sdk and not args.queries:
        parser.error("--sdk requires --queries")
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
                         "--test", "event_full", "--test", "event_storage", "--test", "event_sources"]),
        ("clippy", ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]),
        ("node-check", ["cargo", "check", "--manifest-path", "ts/crate/Cargo.toml", "--all-targets"]),
        ("node-clippy", ["cargo", "clippy", "--manifest-path", "ts/crate/Cargo.toml",
                         "--all-targets", "--", "-D", "warnings"]),
        ("node-format", ["cargo", "fmt", "--manifest-path", "ts/crate/Cargo.toml", "--check"]),
    ]
    if args.relations:
        checks[4][1].extend(["--test", "event_relations", "--test", "event_fixed_points",
                             "--test", "event_partitions", "--test", "event_actions"])
    if args.queries:
        checks[4][1].extend(["--test", "event_queries", "--test", "event_pack", "--test", "event_query_maps", "--test", "event_query_relations"])
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
        if args.sdk:
            checks.append(("schema-bindings-native", ["cargo", "test", "-p", "bumbledb-log", "--lib"]))
            checks.append(("event-ingress", ["node", "scripts/check-event-ingress.cjs", "ts/crate/target/debug/" + library]))
            checks.append(("sdk", ["node", "scripts/check-event-sdk.mjs", "ts/crate/target/debug/" + library]))
    result = {
        "sdk_scope": ("Owned Event field/carriers, worker-backed host algebra and supported Event/descriptor ingress, all seven BEDC structural constructors/descriptions/inspections, exact BERA scalars/BESC finite functions, kernels and explicit replayed revisions, weighted images, strict law designation and owned fixed-law observations with shared admission arithmetic, Event/Test/map/relation query builders/imports, contextual full projections and physical-only key lookup, exact Event schema selections with worker-owned input/output on managed core paths, complete operand diagnostics, generated bindings, native dependencies, persistence/reopen, literals/parameters, Pack and ownership/cancellation; parameter families, query observation/expectation heads, log schema-literal ingress, complete SDK and aggregate retained-memory policy remain open" if args.sdk else None),
        "query_scope": ("Complete-binding Event/Test heads, captured maps/faces/products, typed relation operators and Star, grouped Event Pack, per-occurrence context faults and raw Node descriptors; bound fixed-point query programs, full SDK and certified factoring remain open" if args.queries else None),
        "scope": ("Native finite Event maps, relations, diagram inspection, information, sealed fixed-point programs, indexed partitions, fully observed strategy witnesses and exact finite laws/BEVT v2, weighted images, conditional source closure, fixed-law revisions, signed expectations and BESC v1 checked source transport; belief-memory/source/full query integration and M0–M8 remain open; no performance qualification"
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
