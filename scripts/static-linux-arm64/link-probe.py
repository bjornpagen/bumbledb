#!/usr/bin/env python3
"""Build a private C driver against the actual self-contained core archive."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

TARGET = "aarch64-unknown-linux-musl"
ROOT = Path(__file__).resolve().parents[2]


def run(args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("artifacts", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    records = [json.loads(line) for line in args.artifacts.read_text().splitlines()]
    artifacts = [r for r in records if r.get("reason") == "compiler-artifact"]
    core = [r for r in artifacts if r["target"]["name"] == "bumbledb"]
    assert len(core) == 1, "expected exactly one core compilation unit"
    core_files = [Path(p) for p in core[0]["filenames"]]
    archive = next(p for p in core_files if p.suffix == ".a")
    rlib = next(p for p in core_files if p.suffix == ".rlib")
    # Use Cargo's actual dependency outputs, including nightly build-dir v2
    # and host proc-macros. Never guess a legacy target/.../deps layout.
    directories = sorted({str(Path(p).parent) for a in artifacts for p in a["filenames"]})
    native = Path(subprocess.check_output(
        ["rustc", "--print", "target-libdir", "--target", TARGET], text=True
    ).strip()) / "self-contained"
    inputs = [archive, native / "libunwind.a", native / "libc.a"]
    assert all(p.is_file() for p in inputs), "Rust's matching static musl/unwind archives are required"
    bundled = output / "libbumbledb.a"
    # MRI merges archive members instead of nesting .a files. The delivered
    # archive contains Rust, LMDB, musl libc and Rust's matching unwinder.
    mri = f'CREATE "{bundled}"\n' + "".join(f'ADDLIB "{p}"\n' for p in inputs) + "SAVE\nEND\n"
    run(["ar", "-M"], input=mri, text=True, capture_output=True)
    run(["ar", "s", str(bundled)])
    symbols = subprocess.check_output(["nm", "-g", "--defined-only", str(bundled)], text=True)
    for symbol in ["mdb_env_create", "mdb_txn_begin", "malloc", "pthread_create", "_Unwind_RaiseException"]:
        assert re.search(r"\b" + re.escape(symbol) + r"$", symbols, re.MULTILINE), f"archive must contain {symbol}"

    obj = output / "static-probe.o"
    compile_args = ["rustc", "--edition=2024", "--crate-name", "bumbledb_static_probe",
                    "--crate-type", "lib", "--emit=obj", "--target", TARGET,
                    "-C", "target-feature=+crt-static", "-C", "opt-level=3", "-C", "codegen-units=1"]
    for directory in directories:
        compile_args.extend(["-L", "dependency=" + directory])
    compile_args.extend(["--extern", "bumbledb=" + str(rlib), str(ROOT / "scripts/static-linux-arm64/smoke.rs"), "-o", str(obj)])
    run(compile_args)
    driver = str(ROOT / "scripts/static-linux-arm64/smoke.c")
    # Use the matching startup objects, not a second libc implicitly supplied
    # by the host C driver's defaults. Mixing the two musl builds duplicates
    # private TLS/allocator symbols even though both archives are static.
    crt_before = [str(native / name) for name in ["crt1.o", "crti.o", "crtbegin.o"]]
    crt_after = [str(native / name) for name in ["crtend.o", "crtn.o"]]
    flags = ["cc", "-static", "-nostdlib", "-no-pie", "-Wall", "-Wextra", "-Werror", "-Wl,--gc-sections", "-Wl,--no-undefined", "-Wl,--eh-frame-hdr"]
    run(flags + crt_before + [driver, str(obj), str(bundled)] + crt_after + ["-Wl,-Map=" + str(output / "static-link.map"), "-o", str(output / "bumbledb-static-smoke")])
    missing = subprocess.run(flags + crt_before + [driver, str(obj)] + crt_after + ["-o", str(output / "missing-library-negative-control")], capture_output=True, text=True)
    assert missing.returncode != 0 and "undefined reference" in missing.stderr, "probe must actually require the delivered core archive"
    report = {
        "target": TARGET,
        "archive": {"file": bundled.name, "sha256": sha256(bundled)},
        "bundledInputs": [{"file": p.name, "sha256": sha256(p)} for p in inputs],
        "startupObjects": [{"file": Path(p).name, "sha256": sha256(Path(p))} for p in crt_before + crt_after],
        "coreArchiveRequired": True,
        "nativeContents": ["LMDB", "musl libc", "Rust unwinder"],
        "scope": "static Rust core archive; C driver is a private link test, not a public C API",
    }
    (output / "static-library.json").write_text(json.dumps(report, indent=2) + "\n")
    print("static archive contains Rust, LMDB, musl libc and unwinder; C link and missing-archive negative control passed")


if __name__ == "__main__":
    main()
