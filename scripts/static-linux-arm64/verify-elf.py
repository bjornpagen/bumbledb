#!/usr/bin/env python3
"""Fail closed unless each artifact is a loader-free AArch64 ELF executable."""

import argparse
import hashlib
import json
from pathlib import Path
import struct


def inspect_elf(data):
    if len(data) < 64 or data[:7] != b"\x7fELF\x02\x01\x01":
        raise ValueError("expected a complete little-endian ELF64 header")
    header = struct.unpack_from("<16sHHIQQQIHHHHHH", data)
    _, kind, machine, version, entry, phoff, _, _, ehsize, phsize, phnum, _, _, _ = header
    if machine != 183 or kind not in (2, 3) or version != 1 or ehsize != 64:
        raise ValueError("expected an AArch64 executable or static PIE")
    if phsize != 56 or phnum == 0 or phoff < 64 or phoff + phsize * phnum > len(data):
        raise ValueError("invalid or truncated ELF program headers")
    loads = 0
    executable_entry = False
    for index in range(phnum):
        ptype, flags, offset, vaddr, _, filesz, memsz, _ = struct.unpack_from("<IIQQQQQQ", data, phoff + phsize * index)
        if offset + filesz > len(data) or filesz > memsz:
            raise ValueError("invalid or truncated ELF segment")
        if ptype == 3:
            raise ValueError("PT_INTERP requires a runtime loader")
        if ptype == 1:
            loads += 1
            executable_entry |= bool(flags & 1) and vaddr <= entry < vaddr + memsz
        if ptype == 2:
            if not filesz or filesz % 16:
                raise ValueError("invalid dynamic table")
            terminated = False
            for position in range(offset, offset + filesz, 16):
                tag, _ = struct.unpack_from("<qQ", data, position)
                if tag == 1:
                    raise ValueError("DT_NEEDED requires a shared library")
                if tag == 0:
                    terminated = True
                    break
            if not terminated:
                raise ValueError("unterminated dynamic table")
    if not loads or not executable_entry:
        raise ValueError("no executable load segment contains the entry point")
    return {"machine": "AArch64", "elfType": "EXEC" if kind == 2 else "static PIE", "interpreter": None, "neededLibraries": [], "loadSegments": loads}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("files", type=Path, nargs="*")
    parser.add_argument("--cargo-json", type=Path)
    args = parser.parse_args()
    files = list(args.files)
    if args.cargo_json:
        artifacts = [json.loads(line) for line in args.cargo_json.read_text().splitlines()]
        files.extend(Path(a["executable"]) for a in artifacts if a.get("reason") == "compiler-artifact" and a.get("executable"))
    if not files:
        parser.error("at least one real executable is required")
    records = []
    for file in files:
        data = file.read_bytes()
        try:
            properties = inspect_elf(data)
        except ValueError as error:
            raise SystemExit(f"{file}: {error}") from error
        records.append({"file": file.name, "sha256": hashlib.sha256(data).hexdigest(), **properties})
    print(json.dumps(records, indent=2))


if __name__ == "__main__":
    main()
