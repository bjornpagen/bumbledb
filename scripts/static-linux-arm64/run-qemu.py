#!/usr/bin/env python3
"""Boot a real ARM64 Linux kernel; user-mode QEMU lacks robust futex syscalls."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("--qemu", default="qemu-system-aarch64")
    parser.add_argument("--timeout", type=int, default=180)
    args = parser.parse_args()
    output = args.output.resolve()
    kernel = output / "vmlinuz-virt"
    initramfs = output / "initramfs.cpio.gz"
    qemu = shutil.which(args.qemu)
    assert qemu and kernel.is_file() and initramfs.is_file(), "QEMU, kernel and initramfs are required"
    command = [qemu, "-machine", "virt,gic-version=2", "-cpu", "cortex-a53",
               "-accel", "tcg,thread=multi", "-smp", "2", "-m", "2048",
               "-nodefaults", "-display", "none", "-serial", "stdio", "-monitor", "none",
               "-nic", "none", "-no-reboot", "-kernel", str(kernel), "-initrd", str(initramfs),
               "-append", "console=ttyAMA0 rdinit=/init panic=-1 bumbledb.static-probe=1"]
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=args.timeout)
    except subprocess.TimeoutExpired as error:
        captured = error.stdout or b""
        if isinstance(captured, bytes):
            captured = captured.decode(errors="replace")
        (output / "qemu.log").write_text(captured)
        raise SystemExit("QEMU timed out; no static runtime success is claimed (see qemu.log)") from error
    log = result.stdout + result.stderr
    (output / "qemu.log").write_text(log)
    assert result.returncode == 0, f"QEMU exited {result.returncode}; see {output / 'qemu.log'}"
    assert "BUMBLEDB_STATIC_QEMU: PASS" in log and "BUMBLEDB_STATIC_QEMU: FAIL" not in log, f"guest verification failed; see {output / 'qemu.log'}"
    assert "static C/Rust/musl/LMDB link probe: OK" in log and "condition: Missing" in log
    report = {
        "mode": "full-system Linux, TCG",
        "cpu": "cortex-a53",
        "kernelSha256": hashlib.sha256(kernel.read_bytes()).hexdigest(),
        "initramfsSha256": hashlib.sha256(initramfs.read_bytes()).hexdigest(),
        "qemuVersion": subprocess.check_output([qemu, "--version"], text=True).splitlines()[0],
        "coreAndDutyPassed": True,
        "sharedLibrariesInGuest": False,
    }
    (output / "qemu-verification.json").write_text(json.dumps(report, indent=2) + "\n")
    print("PASS: full-system ARM64 QEMU booted Linux and ran the static core/C consumer and duty with no shared libraries")


if __name__ == "__main__":
    main()
