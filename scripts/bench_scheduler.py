#!/usr/bin/env python3
"""Platform placement policy. CPU affinity and scheduler priority are distinct."""

from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys


def parse_cpu_ids(value):
    selected = set()
    for part in value.split(","):
        ends = part.split("-")
        if len(ends) not in (1, 2) or not all(end.isascii() and end.isdecimal() for end in ends):
            raise ValueError("--cpus must contain CPU IDs or inclusive ranges, e.g. 0-7,16-23")
        first, last = int(ends[0]), int(ends[-1])
        if first > last or last > 1_048_575:
            raise ValueError("invalid or unreasonably large CPU range")
        selected.update(range(first, last + 1))
    if not selected:
        raise ValueError("the CPU selection must not be empty")
    return sorted(selected)


def sysctl(name):
    return subprocess.check_output(["sysctl", "-n", name], text=True, stderr=subprocess.PIPE).strip()


def policy_for_host(cpu_selection=None):
    if sys.platform == "darwin":
        if cpu_selection is not None:
            raise ValueError("macOS does not expose Linux-style CPU ID affinity; --cpus is Linux-only")
        # Discover the performance level by name, never assume level 0 or CPU IDs.
        try:
            levels = int(sysctl("hw.nperflevels"))
            counts = [int(sysctl(f"hw.perflevel{level}.physicalcpu")) for level in range(levels)
                      if sysctl(f"hw.perflevel{level}.name").casefold() == "performance"]
        except (OSError, ValueError, subprocess.CalledProcessError) as error:
            raise ValueError("cannot identify this Mac's performance cores; refusing an all-core fallback") from error
        if len(counts) != 1 or counts[0] < 1:
            raise ValueError("expected one nonempty macOS performance-core class")
        return {"worker_limit": counts[0], "cpu_ids": None,
                "placement": "P-core-count cap; QoS steering, NOT CPU affinity",
                "priority": "qos-user-interactive (relative priority 0)",
                "hard_affinity": False}
    if sys.platform == "linux":
        if cpu_selection is None:
            raise ValueError("Linux requires --cpus with this host's performance CPU IDs; "
                             "CPU numbering and clock frequencies are not reliable core-type classifiers")
        selected = parse_cpu_ids(cpu_selection)
        allowed = os.sched_getaffinity(0)
        if not set(selected) <= allowed:
            raise ValueError(f"selected CPUs are outside the current affinity/cpuset: {sorted(set(selected) - allowed)}")
        return {"worker_limit": len(selected), "cpu_ids": selected,
                "placement": "verified affinity to user-selected performance CPUs",
                "priority": "nice -10 (absolute, read back; never silently downgraded)",
                "hard_affinity": True}
    raise ValueError(f"no performance-core scheduling policy for {sys.platform}")


def worker_command(command, cpu_ids):
    if cpu_ids is None:
        return command
    return [sys.executable, str(Path(__file__).resolve()),
            ",".join(str(cpu) for cpu in cpu_ids), *command]


def pin_worker(cpu_ids):
    """Called in a fresh single-threaded child before exec; descendants inherit."""
    selected = set(cpu_ids)
    if not selected or not selected <= os.sched_getaffinity(0):
        raise ValueError("worker CPU selection is empty or no longer allowed")
    os.sched_setaffinity(0, selected)
    actual = os.sched_getaffinity(0)
    if actual != selected:
        raise ValueError(f"CPU affinity readback differs: requested {sorted(selected)}, got {sorted(actual)}")


if __name__ == "__main__":
    try:
        if sys.platform != "linux" or len(sys.argv) < 3:
            raise ValueError("internal Linux worker: expected CPU IDs and an executable")
        pin_worker(parse_cpu_ids(sys.argv[1]))
        os.execv(sys.argv[2], sys.argv[2:])
    except (OSError, ValueError) as error:
        print(f"benchmark affinity refused: {error}", file=sys.stderr)
        sys.exit(2)
