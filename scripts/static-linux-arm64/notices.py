#!/usr/bin/env python3
"""Preserve source-package notices alongside static CI qualification artifacts."""

import json
from pathlib import Path
import shutil
import subprocess
import sys


def main():
    output = Path(sys.argv[1]).resolve()
    notices = output / "notices"
    notices.mkdir(parents=True, exist_ok=True)
    shutil.copy2("LICENSE", notices / "BUMBLEDB-LICENSE")
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--format-version=1",
         "--filter-platform", "aarch64-unknown-linux-musl"], text=True
    ))
    inventory = []
    for package in metadata["packages"]:
        if package["source"] is None:
            continue
        source = Path(package["manifest_path"]).parent
        destination = notices / "crates" / (package["name"] + "-" + package["version"])
        files = []
        for path in sorted(source.rglob("*")):
            if not path.is_file() or not path.name.upper().startswith(("LICENSE", "COPYING", "NOTICE", "COPYRIGHT")):
                continue
            relative = path.relative_to(source)
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target)
            files.append(str(relative))
        inventory.append({"name": package["name"], "version": package["version"],
                          "source": package["source"], "license": package["license"],
                          "repository": package["repository"], "noticeFiles": files})
    sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip())
    rust_notices = sysroot / "share/doc/rust"
    # Rust's distribution includes musl/compiler-rt/unwind attribution.
    for name in ["COPYRIGHT.html", "COPYRIGHT-library.html", "licenses"]:
        source = rust_notices / name
        if source.is_dir():
            shutil.copytree(source, notices / "rust" / name, dirs_exist_ok=True)
        else:
            (notices / "rust").mkdir(exist_ok=True)
            shutil.copy2(source, notices / "rust" / name)
    (notices / "inventory.json").write_text(json.dumps({
        "scope": "Conservative workspace dependency/source notice inventory, including build/test dependencies; not a linked-code SBOM or release license approval.",
        "packages": inventory,
    }, indent=2) + "\n")
    shutil.make_archive(str(output / "notices"), "gztar", output, "notices")
    print("preserved workspace dependency and Rust/musl/unwind notices")


if __name__ == "__main__":
    main()
