#!/usr/bin/env python3
"""Record/check the curated Event proposal package, without building or fetching.

Only named source/report formats are eligible. Local downloads, native binaries,
build trees and Python caches remain untouched and are never package inputs.
The manifest is an editable inventory, not a replacement for frozen proof runs.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
PROPOSAL = ROOT / "proposal"
MANIFEST = PROPOSAL / "package.json"
FORMATS = {".md", ".rs", ".py", ".lean", ".json", ".log", ".s", ".toml", ".lock", ".ts", ".c"}
NAMES = {".gitignore", "lean-toolchain"}
EXCLUDED = {".scratch", "target", "__pycache__"}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sources():
    files = {}
    for path in sorted(PROPOSAL.rglob("*")):
        if EXCLUDED.intersection(path.relative_to(PROPOSAL).parts):
            continue
        if path == MANIFEST or not path.is_file():
            continue
        if path.suffix not in FORMATS and path.name not in NAMES:
            continue
        if path.is_symlink():
            raise ValueError(f"Package source may not be a symlink: {path}")
        data = path.read_bytes()
        data.decode("utf-8")
        if b"\0" in data:
            raise ValueError(f"Binary file in source/report roster: {path}")
        files[str(path.relative_to(ROOT))] = {
            "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()
        }
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record", action="store_true", help="Refresh the current source inventory")
    parser.add_argument("--index", action="store_true", help="Also require identical staged Git blobs")
    args = parser.parse_args()
    files = sources()
    if args.record:
        record = {
            "scope": "Event proposal, own research reports, reference programs and retained evidence; no binaries, downloads or build caches",
            "excluded_local_formats": [".pdf", ".txt", ".html", ".png"],
            "excluded_directories": sorted(EXCLUDED),
            "files": files,
        }
        MANIFEST.write_text(json.dumps(record, indent=2) + "\n")
    expected = json.loads(MANIFEST.read_text())["files"]
    if files != expected:
        changed = sorted(set(files) ^ set(expected) | {
            name for name in set(files) & set(expected) if files[name] != expected[name]
        })
        raise SystemExit("Proposal inventory changed; review and refresh: " + ", ".join(changed))
    if args.index:
        # One batch verifies exact content, not just whether a path was staged.
        with subprocess.Popen(["git", "cat-file", "--batch"], cwd=ROOT,
                              stdin=subprocess.PIPE, stdout=subprocess.PIPE) as process:
            for name in [*files, str(MANIFEST.relative_to(ROOT))]:
                process.stdin.write((":" + name + "\n").encode())
                process.stdin.flush()
                header = process.stdout.readline().split()
                if len(header) != 3 or header[1] != b"blob":
                    raise SystemExit("Package file missing from Git index: " + name)
                data = process.stdout.read(int(header[2]))
                assert process.stdout.read(1) == b"\n"
                if data != (ROOT / name).read_bytes():
                    raise SystemExit("Staged package content differs: " + name)
            process.stdin.close()
            assert process.wait() == 0
    print(json.dumps({"passed": True, "files": len(files),
                      "bytes": sum(row["bytes"] for row in files.values()),
                      "index_verified": args.index}, indent=2))


if __name__ == "__main__":
    main()
