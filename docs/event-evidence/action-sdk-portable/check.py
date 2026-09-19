#!/usr/bin/env python3
"""Qualify Event proposal proofs/readiness using only staged Git files.

An isolated checkout has no local research cache or build tree. Both proof
runners use the explicit installed Lean executable; no downloads or builds run.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tree():
    return subprocess.check_output(["git", "write-tree"], cwd=ROOT, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lean", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path, help="New evidence directory")
    args = parser.parse_args()
    lean = args.lean.resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    shutil.copy2(__file__, output / "check.py")
    source_tree = tree()
    result = {"passed": False, "source_tree": source_tree,
              "scope": "Staged-only Event proposal package, fresh Lean elaboration and readiness; no Rust refinement or performance claim",
              "checker_sha256": digest(Path(__file__)), "checks": []}
    with tempfile.TemporaryDirectory(prefix="bumbledb-event-checkout-") as temp:
        fresh = Path(temp)
        subprocess.run(["git", "checkout-index", "--all", "--prefix=" + temp + "/"],
                       cwd=ROOT, check=True, timeout=180)
        checks = [
            ("package", ["python3", "scripts/check-event-package.py"]),
            ("proposal", ["python3", "proposal/semantics/check.py", "--label", "fresh-checkout",
                          "--lean", str(lean)]),
            ("native", ["python3", "scripts/check-event-semantics.py", "--lean", str(lean),
                        "--output", "docs/event-evidence/fresh-checkout-native"]),
            ("readiness", ["python3", "scripts/check-event-readiness.py",
                           "--proposal-proof", "proposal/semantics/results/fresh-checkout",
                           "--native-proof", "docs/event-evidence/fresh-checkout-native",
                           "--output", "docs/event-evidence/fresh-checkout-readiness"]),
        ]
        for name, command in checks:
            log = output / (name + ".log")
            with log.open("w") as stream:
                completed = subprocess.run(command, cwd=fresh, stdout=stream,
                                           stderr=subprocess.STDOUT, timeout=600)
            result["checks"].append({"name": name, "command": command,
                                     "exit_code": completed.returncode,
                                     "log": log.name, "sha256": digest(log)})
            (output / "check.json").write_text(json.dumps(result, indent=2) + "\n")
            print(name, "passed" if completed.returncode == 0 else "FAILED", flush=True)
            if completed.returncode:
                raise SystemExit(completed.returncode)
        for name, folder in [
            ("proposal", fresh / "proposal/semantics/results/fresh-checkout"),
            ("native", fresh / "docs/event-evidence/fresh-checkout-native"),
            ("readiness", fresh / "docs/event-evidence/fresh-checkout-readiness"),
        ]:
            record = output / (name + "-check.json")
            shutil.copy2(folder / "check.json", record)
            result[name + "_manifest_sha256"] = digest(record)
            # Source hashes already refer to the staged tree; keep exact Lean
            # reports without duplicating the full proposal tree per audit.
            for log in sorted(folder.glob("*.log")):
                target = output / name / log.name
                target.parent.mkdir(exist_ok=True)
                shutil.copy2(log, target)
        result["package_manifest_sha256"] = digest(fresh / "proposal/package.json")
    result["index_unchanged"] = tree() == source_tree
    result["passed"] = result["index_unchanged"]
    (output / "check.json").write_text(json.dumps(result, indent=2) + "\n")
    raise SystemExit(0 if result["passed"] else 1)


if __name__ == "__main__":
    main()
