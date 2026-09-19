#!/usr/bin/env python3
"""Check the native Event crate's denotational contracts; never Rust refinement."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lean", type=Path, required=True, help="Installed Lean 4.32.0 binary")
    parser.add_argument("--output", type=Path, required=True, help="New evidence directory")
    args = parser.parse_args()
    binary = args.lean.resolve(strict=True)
    version = subprocess.check_output([str(binary), "--version"], text=True).strip()
    if not version.startswith("Lean (version 4.32.0,"):
        raise SystemExit(f"Expected pinned Lean 4.32.0, found {version}")
    root = Path(__file__).resolve().parent.parent
    sources = sorted((root / "crates/bumbledb-event/semantics").glob("*.lean"))
    if not sources:
        raise SystemExit("No Event semantic modules found")
    args.output.mkdir(parents=True, exist_ok=False)
    shutil.copy2(__file__, args.output / "check.py")
    result = {
        "scope": "Native Event denotational contracts; not Rust, codec or allocator refinement",
        "lean_version": version,
        "lean_binary_sha256": digest(binary),
        "checker_sha256": digest(Path(__file__)),
        "files": [],
    }
    for source in sources:
        before = digest(source)
        shutil.copy2(source, args.output / source.name)
        expected = re.findall(r"^#print axioms ([\w.]+)$", source.read_text(), re.M)
        run = subprocess.run([str(binary), str(source)], text=True, capture_output=True, timeout=60)
        log = run.stdout + run.stderr
        (args.output / f"{source.stem}.log").write_text(log)
        reports = {}
        for name in expected:
            if f"'{name}' does not depend on any axioms" in log:
                reports[name] = []
            else:
                match = re.search(re.escape(f"'{name}' depends on axioms: [") + r"([^\]]*)\]", log)
                if match:
                    reports[name] = [value.strip() for value in match[1].split(",")]
        passed = (
            bool(expected)
            and run.returncode == 0
            and "warning:" not in log
            and len(reports) == len(expected)
            and all(set(axioms) <= {"propext", "Quot.sound", "Classical.choice"} for axioms in reports.values())
            and before == digest(source)
        )
        result["files"].append({"source": str(source.relative_to(root)), "sha256": before,
                                "exit_code": run.returncode, "reports": reports, "passed": passed})
        print(source.name, "passed" if passed else "FAILED", len(reports), "reports")
        if not passed:
            print(log)
    result["passed"] = all(file["passed"] for file in result["files"])
    (args.output / "check.json").write_text(json.dumps(result, indent=2) + "\n")
    raise SystemExit(0 if result["passed"] else 1)


if __name__ == "__main__":
    main()
