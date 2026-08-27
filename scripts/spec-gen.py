#!/usr/bin/env python3
"""The spec generator: the independent third mind of the v:3 corpus.

Assembles the ok-golden bytes of every byte grammar from the corpus
metadata alone — inventory.json, the per-case .json sidecars, and
schemas.json under crates/bumbledb-log/conformance/v3/ — spelling the
written field rosters directly: version byte, u64le, length-delimited
vectors, raw digests. It never calls bumbledb-log's encode paths; a
reader agreeing with itself proves nothing, so the goldens are produced
by construction and the reader is checked against them.

Families spelled: batch (header + tagged ops over the schema roster),
manifest / checkpoint / sidecar documents, counter (canonical decimal
ASCII), lease (the LEASE/1 line body), scratch (version byte + digest).
Chain goldens are batch bytes whose ops are not spelled in their
sidecars — the batch decode/re-encode fixpoint lane owns them.

Modes:
  --check      reassemble every covered ok golden and diff against the
               checked-in .bin bytes; nonzero exit on any drift.
  --emit DIR   write the reassembled goldens under DIR (corpus-relative
               paths) plus truncations/index.json — the mechanical
               refusal family: every strict prefix of one representative
               ok body per family, with the expected outcome spelled.
"""

import argparse
import json
import re
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CORPUS = ROOT / "crates" / "bumbledb-log" / "conformance" / "v3"

MAGIC = b"BDBL"
WIRE_VERSION = 3
DOC_VERSION = 3
U64_MAX = 2**64 - 1
I64_MIN = -(2**63)
I64_MAX = 2**63 - 1

TAG_BOOL = 0x00
TAG_U64 = 0x01
TAG_I64 = 0x02
TAG_STRING = 0x03
TAG_FIXED_BYTES = 0x04
TAG_INTERVAL = 0x05
TAG_FIXED_INTERVAL = 0x06

OP_KINDS = {"insert": 0x01, "delete": 0x02}


def refuse(label, why):
    raise SystemExit(f"spec-gen: {label}: {why}")


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def u64(label, text):
    if not isinstance(text, str) or not re.fullmatch(r"0|[1-9][0-9]*", text):
        refuse(label, f"a u64 is a canonical decimal string, got {text!r}")
    value = int(text)
    if value > U64_MAX:
        refuse(label, f"{text} is past u64::MAX")
    return value


def i64(label, text):
    if not isinstance(text, str) or not re.fullmatch(r"0|-?[1-9][0-9]*", text):
        refuse(label, f"an i64 is a canonical decimal string, got {text!r}")
    value = int(text)
    if not I64_MIN <= value <= I64_MAX:
        refuse(label, f"{text} is outside i64")
    return value


def digest32(label, text):
    if not isinstance(text, str) or not re.fullmatch(r"[0-9a-f]{64}", text):
        refuse(label, f"a digest is 64 lowercase hex, got {text!r}")
    return bytes.fromhex(text)


def braid_raw(label, text):
    if not isinstance(text, str) or not re.fullmatch(r"c[0-9a-f]{8}", text):
        refuse(label, f"a braid is 'c' + 8 hex, got {text!r}")
    return int(text[1:], 16)


def le64u(value):
    return struct.pack("<Q", value)


def le64i(value):
    return struct.pack("<q", value)


def le32(value):
    return struct.pack("<I", value)


# ---- schema rosters ---------------------------------------------------


def field_types(schemas, name):
    """The per-relation field-type rosters of one named fixture schema."""
    relations = schemas["schemas"][name]["relations"]
    return [[field["type"] for field in rel["fields"]] for rel in relations]


def encode_value(label, ftype, value):
    """One tagged wire value: the tag speaks shape, the schema speaks sign."""
    if not isinstance(value, dict) or len(value) != 1:
        refuse(label, f"a sidecar value is a one-key object, got {value!r}")
    (key, payload), = value.items()
    if ftype == "bool":
        if key != "bool" or not isinstance(payload, bool):
            refuse(label, f"the roster says bool, the sidecar says {key}")
        return bytes([TAG_BOOL, 1 if payload else 0])
    if ftype == "u64":
        if key != "u64":
            refuse(label, f"the roster says u64, the sidecar says {key}")
        return bytes([TAG_U64]) + le64u(u64(label, payload))
    if ftype == "i64":
        if key != "i64":
            refuse(label, f"the roster says i64, the sidecar says {key}")
        return bytes([TAG_I64]) + le64i(i64(label, payload))
    if ftype == "string":
        if key != "string" or not isinstance(payload, str):
            refuse(label, f"the roster says string, the sidecar says {key}")
        raw = payload.encode("utf-8")
        return bytes([TAG_STRING]) + le32(len(raw)) + raw
    if isinstance(ftype, dict) and "fixedBytes" in ftype:
        width = ftype["fixedBytes"]
        if key != "fixedBytes" or not re.fullmatch(r"([0-9a-f]{2})*", payload):
            refuse(label, f"the roster says fixedBytes, the sidecar says {key}")
        raw = bytes.fromhex(payload)
        if len(raw) != width:
            refuse(label, f"fixedBytes width {width}, got {len(raw)} bytes")
        return bytes([TAG_FIXED_BYTES]) + raw
    if isinstance(ftype, dict) and "interval" in ftype:
        element = ftype["interval"]
        want = "intervalU64" if element == "u64" else "intervalI64"
        if key != want:
            refuse(label, f"the roster says {want}, the sidecar says {key}")
        parse = u64 if element == "u64" else i64
        pack = le64u if element == "u64" else le64i
        lo, hi = payload
        return bytes([TAG_INTERVAL]) + pack(parse(label, lo)) + pack(parse(label, hi))
    if isinstance(ftype, dict) and "fixedInterval" in ftype:
        element = ftype["fixedInterval"]["element"]
        width = u64(f"{label}.width", ftype["fixedInterval"]["width"])
        want = "intervalU64" if element == "u64" else "intervalI64"
        if key != want:
            refuse(label, f"the roster says {want}, the sidecar says {key}")
        parse = u64 if element == "u64" else i64
        pack = le64u if element == "u64" else le64i
        lo, hi = (parse(label, bound) for bound in payload)
        if hi - lo != width:
            refuse(label, f"a fixed interval spans its width {width}, got [{lo}, {hi})")
        return bytes([TAG_FIXED_INTERVAL]) + pack(lo)
    refuse(label, f"unknown roster type {ftype!r}")


# ---- family assemblers ------------------------------------------------


def assemble_batch(label, sidecar, schemas):
    """Magic, u16le version, u16le zero flags, fingerprint, braid u32le,
    gen u64le, prev digest, writer u64le, timestamp u64le, u32le op
    count, then ops: kind u8, relation u32le, row count u32le, tagged
    values in roster order."""
    rosters = field_types(schemas, sidecar["schema"])
    header = sidecar["header"]
    out = bytearray()
    out += MAGIC
    out += struct.pack("<H", WIRE_VERSION)
    out += struct.pack("<H", 0)
    out += digest32(f"{label}.fingerprint", sidecar["fingerprint"])
    out += le32(braid_raw(f"{label}.braid", header["braid"]))
    out += le64u(u64(f"{label}.braidGen", header["braidGen"]))
    out += digest32(f"{label}.prev", header["prev"])
    out += le64u(u64(f"{label}.writer", header["writer"]))
    out += le64u(u64(f"{label}.timestamp", header["timestamp"]))
    ops = sidecar["ops"]
    out += le32(len(ops))
    for index, op in enumerate(ops):
        where = f"{label}.ops[{index}]"
        if op["kind"] not in OP_KINDS:
            refuse(where, f"unknown op kind {op['kind']!r}")
        relation = op["relation"]
        if not isinstance(relation, int) or not 0 <= relation < len(rosters):
            refuse(where, f"relation {relation!r} is outside the roster")
        roster = rosters[relation]
        out.append(OP_KINDS[op["kind"]])
        out += le32(relation)
        out += le32(len(op["rows"]))
        for row in op["rows"]:
            if len(row) != len(roster):
                refuse(where, f"a row carries {len(roster)} fields, got {len(row)}")
            for ftype, value in zip(roster, row):
                out += encode_value(where, ftype, value)
    return bytes(out)


def optional_digest(label, out, text):
    if text is None:
        out.append(0x00)
    else:
        out.append(0x01)
        out += digest32(label, text)


def assemble_manifest(label, sidecar):
    """Version byte, fingerprint digest, optional checkpoint digest."""
    value = sidecar["value"]
    out = bytearray([DOC_VERSION])
    out += digest32(f"{label}.fingerprint", value["fingerprint"])
    optional_digest(f"{label}.checkpoint", out, value["checkpoint"])
    return bytes(out)


def braid_entries(label, mapping):
    """The braid map as an ascending raw-braid roster."""
    entries = sorted(
        (braid_raw(f"{label}.{name}", name), name, head)
        for name, head in mapping.items()
    )
    return entries


def assemble_checkpoint(label, sidecar):
    """Version byte, u32le braid count, ascending (braid u32le, g u64le,
    hash digest, ts u64le) entries, catalog digest, writer u64le,
    optional prev digest."""
    value = sidecar["value"]
    out = bytearray([DOC_VERSION])
    entries = braid_entries(label, value["braids"])
    out += le32(len(entries))
    for raw, name, head in entries:
        out += le32(raw)
        out += le64u(u64(f"{label}.{name}.g", head["g"]))
        out += digest32(f"{label}.{name}.hash", head["hash"])
        out += le64u(u64(f"{label}.{name}.ts", head["ts"]))
    out += digest32(f"{label}.catalog", value["catalog"])
    out += le64u(u64(f"{label}.writer", value["writer"]))
    optional_digest(f"{label}.prev", out, value["prev"])
    return bytes(out)


def assemble_sidecar_doc(label, sidecar):
    """Version byte, u32le chain count, ascending (braid u32le, g u64le,
    prev digest, ts u64le) entries, then the pending arm: absent, or
    braid u32le, gen u64le, u32le length, the held batch bytes."""
    value = sidecar["value"]
    out = bytearray([DOC_VERSION])
    entries = braid_entries(label, value["chain"])
    out += le32(len(entries))
    for raw, name, entry in entries:
        out += le32(raw)
        out += le64u(u64(f"{label}.{name}.g", entry["g"]))
        out += digest32(f"{label}.{name}.prev", entry["prev"])
        out += le64u(u64(f"{label}.{name}.ts", entry["ts"]))
    pending = value["pending"]
    if pending is None:
        out.append(0x00)
    else:
        raw = pending["bytes"]
        if not re.fullmatch(r"([0-9a-f]{2})+", raw):
            refuse(f"{label}.pending.bytes", "lowercase hex batch bytes")
        held = bytes.fromhex(raw)
        out.append(0x01)
        out += le32(braid_raw(f"{label}.pending.braid", pending["braid"]))
        out += le64u(u64(f"{label}.pending.gen", pending["gen"]))
        out += le32(len(held))
        out += held
    return bytes(out)


def assemble_counter(label, sidecar):
    """The canonical decimal ASCII u64, nothing else."""
    return str(u64(f"{label}.value", sidecar["value"])).encode("ascii")


def assemble_lease(label, sidecar):
    """The LEASE/1 body: magic line, holder, token, expires, each
    newline-terminated."""
    value = sidecar["value"]
    holder = u64(f"{label}.holder", value["holder"])
    token = u64(f"{label}.token", value["token"])
    expires = u64(f"{label}.expires", value["expires"])
    return f"LEASE/1\n{holder}\n{token}\n{expires}\n".encode("ascii")


def assemble_scratch(label, sidecar):
    """Version byte plus the 32-byte checkpoint digest: 33 bytes exactly."""
    return bytes([DOC_VERSION]) + digest32(f"{label}.value", sidecar["value"])


# ---- the corpus walk --------------------------------------------------


def ok_stems(inventory):
    """(family, corpus-relative stem) for every ok byte golden the
    metadata spells. Chain stems are excluded: their sidecars name the
    verify verdict, not the ops."""
    rows = []
    for stem in inventory["batch_ok"]:
        rows.append(("batch", f"batch/{stem}"))
    for stem in inventory["documents"]:
        if stem.rsplit("/", 1)[1].startswith("ok_"):
            rows.append((stem.split("/")[1], stem))
    for family in ("counter", "lease", "scratch"):
        for stem in inventory[family]:
            if stem.startswith("ok_"):
                rows.append((family, f"{family}/{stem}"))
    return rows


ASSEMBLERS = {
    "batch": lambda label, sidecar, schemas: assemble_batch(label, sidecar, schemas),
    "manifest": lambda label, sidecar, schemas: assemble_manifest(label, sidecar),
    "checkpoint": lambda label, sidecar, schemas: assemble_checkpoint(label, sidecar),
    "sidecar": lambda label, sidecar, schemas: assemble_sidecar_doc(label, sidecar),
    "counter": lambda label, sidecar, schemas: assemble_counter(label, sidecar),
    "lease": lambda label, sidecar, schemas: assemble_lease(label, sidecar),
    "scratch": lambda label, sidecar, schemas: assemble_scratch(label, sidecar),
}


def assemble_all():
    """Every spelled ok golden: {stem: (family, sidecar, bytes)}."""
    inventory = load(CORPUS / "inventory.json")
    schemas = load(CORPUS / "schemas.json")
    assembled = {}
    for family, stem in ok_stems(inventory):
        sidecar = load(CORPUS / f"{stem}.json")
        if sidecar["expect"] != "ok":
            refuse(stem, "an ok stem carries expect: ok")
        body = ASSEMBLERS[family](stem, sidecar, schemas)
        hex_pin = sidecar.get("hex")
        if hex_pin is not None and bytes.fromhex(hex_pin) != body:
            refuse(stem, "the sidecar hex disagrees with the assembled bytes")
        assembled[stem] = (family, sidecar, body)
    return assembled


def counter_prefix_values(body):
    """The counter law applied to every strict prefix: a nonempty
    all-digit prefix with no leading zero is its own smaller value;
    anything else parses to nothing."""
    values = []
    for length in range(len(body)):
        prefix = body[:length]
        if (
            prefix.isdigit()
            and not (prefix.startswith(b"0") and prefix != b"0")
            and int(prefix) <= U64_MAX
        ):
            values.append(str(int(prefix)))
        else:
            values.append(None)
    return values


def truncation_index(assembled):
    """One representative ok body per family — the longest, ties broken
    by stem — and the outcome the grammar owes every strict prefix:
    batch prefixes are Truncated, document prefixes are Malformed,
    scratch prefixes are silence, counter prefixes are the counter law
    applied per prefix, lease prefixes are total (the line grammar does
    not spell whether a cut body parses, only that parsing returns)."""
    representative = {}
    for stem, (family, _sidecar, body) in sorted(assembled.items()):
        best = representative.get(family)
        if best is None or len(body) > len(best[1]):
            representative[family] = (stem, body)
    families = []
    for family in sorted(representative):
        stem, body = representative[family]
        _family, sidecar, _body = assembled[stem]
        entry = {"family": family, "of": stem, "body": body.hex()}
        if family == "batch":
            entry["schema"] = sidecar["schema"]
            entry["mode"] = "identity"
            entry["refusal"] = "Truncated"
        elif family in ("manifest", "checkpoint", "sidecar"):
            if "schema" in sidecar:
                entry["schema"] = sidecar["schema"]
            entry["mode"] = "identity"
            entry["refusal"] = "Malformed"
        elif family == "scratch":
            entry["mode"] = "silence"
        elif family == "counter":
            entry["mode"] = "counter"
            entry["prefixes"] = counter_prefix_values(body)
        else:
            entry["mode"] = "total"
        families.append(entry)
    return {"families": families}


def run_check():
    assembled = assemble_all()
    drift = []
    for stem, (_family, _sidecar, body) in sorted(assembled.items()):
        golden = (CORPUS / f"{stem}.bin").read_bytes()
        if golden != body:
            drift.append(stem)
            print(f"spec-gen: MISMATCH {stem}: assembled {len(body)} bytes, golden {len(golden)}", file=sys.stderr)
    if drift:
        print(f"spec-gen: FAIL — {len(drift)} golden(s) disagree with the spelled spec", file=sys.stderr)
        return 1
    index = truncation_index(assembled)
    print(
        f"spec-gen: OK — {len(assembled)} ok goldens byte-identical, "
        f"{len(index['families'])} truncation families"
    )
    return 0


def run_emit(out_dir):
    assembled = assemble_all()
    out = Path(out_dir)
    for stem, (_family, _sidecar, body) in sorted(assembled.items()):
        target = out / f"{stem}.bin"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(body)
    index_path = out / "truncations" / "index.json"
    index_path.parent.mkdir(parents=True, exist_ok=True)
    index_path.write_text(
        json.dumps(truncation_index(assembled), indent=2) + "\n", encoding="utf-8"
    )
    print(f"spec-gen: emitted {len(assembled)} goldens and the truncation index under {out}")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", action="store_true", help="diff the assembly against the corpus")
    group.add_argument("--emit", metavar="DIR", help="write the assembly under DIR")
    args = parser.parse_args()
    if args.check:
        return run_check()
    return run_emit(args.emit)


if __name__ == "__main__":
    sys.exit(main())
