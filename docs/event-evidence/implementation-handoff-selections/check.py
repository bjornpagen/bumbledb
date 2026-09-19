#!/usr/bin/env python3
"""Audit the current Event proposal and proof evidence, without asserting Rust refinement.

Unlike the historical revision-0.8 auditor, this permits native implementation
changes and preserves its old isolation record. No build or benchmark is run.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import shutil

ROOT = Path(__file__).resolve().parent.parent
PROPOSAL = ROOT / "proposal"
PERMITTED = {"propext", "Quot.sound", "Classical.choice"}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_text())


def verify_reports(source, log, reports):
    expected = re.findall(r"^#print axioms ([\w.]+)$", source.read_text(), re.M)
    assert expected and len(expected) == len(set(expected)), source
    # Older modules print local names inside a namespace; Lean reports the
    # qualified declaration. Resolve uniquely rather than weakening coverage.
    resolved = []
    for name in expected:
        matches = [full for full in reports if full == name or full.endswith("." + name)]
        assert len(matches) == 1, (source, name, matches)
        resolved.extend(matches)
    assert len(resolved) == len(set(resolved)) and set(resolved) == set(reports), source
    assert "error:" not in log and "warning:" not in log and "sorryAx" not in log, source
    for name, axioms in reports.items():
        assert set(axioms) <= PERMITTED, (name, axioms)
        if not axioms:
            assert f"'{name}' does not depend on any axioms" in log, name
        else:
            match = re.search(re.escape(f"'{name}' depends on axioms: [") + r"([^\]]*)\]", log)
            assert match and [s.strip() for s in match[1].split(",")] == axioms, name


def proposal_proof(folder):
    result = read(folder / "check.json")
    assert result["passed"] and result["scope"] == "central-plus-pack-plus-public-contract"
    assert digest(Path(result["lean_binary"])) == result["lean_binary_sha256"]
    assert digest(PROPOSAL / "semantics/check.py") == digest(folder / "check.py") == result["checker_sha256"]
    for name, sha in result["source_sha256"].items():
        assert digest(PROPOSAL / name) == digest(folder / "sources" / name) == sha, name
    for row in result["files"]:
        assert row["passed"] and row["exit_code"] == 0
        verify_reports(PROPOSAL / row["source"], (folder / row["log"]).read_text(), row["axioms"])
    reports = sum(len(row["axioms"]) for row in result["files"])
    assert reports == result["reports"] == 246
    return {"files": len(result["files"]), "reports": reports, "manifest_sha256": digest(folder / "check.json")}


def native_proof(folder):
    result = read(folder / "check.json")
    assert result["passed"]
    assert digest(ROOT / "scripts/check-event-semantics.py") == digest(folder / "check.py") == result["checker_sha256"]
    expected = sorted(path.relative_to(ROOT).as_posix() for path in
                      (ROOT / "crates/bumbledb-event/semantics").glob("*.lean"))
    assert sorted(row["source"] for row in result["files"]) == expected
    for row in result["files"]:
        source = ROOT / row["source"]
        assert row["passed"] and row["exit_code"] == 0
        assert digest(source) == digest(folder / source.name) == row["sha256"], source
        verify_reports(source, (folder / f"{source.stem}.log").read_text(), row["reports"])
    return {"files": len(result["files"]),
            "reports": sum(len(row["reports"]) for row in result["files"]),
            "manifest_sha256": digest(folder / "check.json")}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--proposal-proof", type=Path, required=True)
    parser.add_argument("--native-proof", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True, help="New evidence directory")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    shutil.copy2(__file__, args.output / "check.py")
    # Only read-only subchecks; never invoke the old native-isolation main().
    spec = importlib.util.spec_from_file_location("event_baseline", PROPOSAL / "semantics/verify_ready.py")
    baseline = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(baseline)
    documents = baseline.check_documents()
    references = baseline.check_references()
    archive = PROPOSAL / "archive/implementation-baseline-0.8"
    historical = read(PROPOSAL / "semantics/results/implementation-ready.json")
    assert read(archive / "implementation-ready.json") == historical
    assert read(archive / "manifest.json")["files"] == historical["documents_sha256"]
    for name, sha in historical["documents_sha256"].items():
        assert digest(archive / name) == sha, name
    for name in documents:
        target = args.output / "sources/proposal" / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(PROPOSAL / name, target)
    assert read(args.native_proof / "check.json")["lean_binary_sha256"] == \
        read(args.proposal_proof / "check.json")["lean_binary_sha256"]
    native_documents = {}
    for source in (ROOT / "docs/event-implementation.md", ROOT / "docs/event-value-format.md",
                   ROOT / "docs/event-projections.md", ROOT / "docs/event-maps.md",
                   ROOT / "docs/event-relations.md", ROOT / "docs/event-inspection.md",
                   ROOT / "docs/event-information.md", ROOT / "docs/event-fixed-points.md",
                   ROOT / "docs/event-partitions.md", ROOT / "docs/event-actions.md",
                   ROOT / "docs/event-queries.md", ROOT / "docs/event-descriptors.md",
                   ROOT / "docs/event-sources.md", ROOT / "docs/event-functions.md",
                   ROOT / "docs/event-revisions.md", ROOT / "docs/event-source-descriptors.md",
                   ROOT / "docs/event-sdk.md", ROOT / "docs/event-selections.md"):
        text = source.read_text()
        assert sum(line.startswith("```") for line in text.splitlines()) % 2 == 0, source
        for link in re.findall(r"\]\(([^)]+)\)", text):
            if not link.startswith(("http:", "https:", "#")):
                target = source.parent / link.split("#")[0]
                # This manifest is written after validating its referring ledger.
                assert target.exists() or target.resolve() == (args.output / "check.json").resolve(), target
        name = str(source.relative_to(ROOT))
        native_documents[name] = digest(source)
        target = args.output / "sources" / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
    result = {
        "revision": "0.9", "passed": True,
        "scope": "Current proposal, exact Lean evidence and retained semantic references; no Rust or performance qualification",
        "proposal_proof": proposal_proof(args.proposal_proof),
        "native_proof": native_proof(args.native_proof),
        "references": references,
        "documents_sha256": documents,
        "native_documents_sha256": native_documents,
        "baseline_archive_verified": True,
        "historical_production_isolation_reasserted": False,
        "implementation_complete": False,
        "checker_sha256": digest(Path(__file__)),
    }
    (args.output / "check.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({key: value for key, value in result.items() if key != "documents_sha256"}, indent=2))


if __name__ == "__main__":
    main()
