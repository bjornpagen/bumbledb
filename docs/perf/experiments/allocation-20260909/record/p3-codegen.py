#!/usr/bin/env python3
"""Frozen ordinary/micro interval code generation; no workload execution."""
import argparse
import re
import shutil
from diagnostics import Phase, ROUND, digest, load
from overlap_experiment import fingerprint

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('gate', type=int)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
selected_gate = ROUND/f'p3-gates-block8-{args.gate}'
source = load(selected_gate/'STATE.json')
assert source['status'] == 'GATES-COMPLETE-REVIEW-REQUIRED'
assert fingerprint() == source['source_fingerprint']
phase = Phase(f'p3-codegen-{args.attempt}')
shutil.copy2(__file__, phase.path/'codegen.py')
phase.state.update(source_fingerprint=source['source_fingerprint'],
    interpretation='Static code generation only, not a trace, CPU attribution or timing result.')
phase.save()
try:
    for label, directory, filename, suffixes in [
        ('micro', ROUND/'p3-filter-mechanism-2', 'flat-mechanism', ['7branchy', '6block8', '5mask8']),
        ('compact', ROUND/'p3-gates-compact-2', 'bumbledb-bench', ['10query_into', '17overlap_enumerate']),
        ('block8', selected_gate, 'bumbledb-bench', ['10query_into', '17overlap_enumerate']),
    ]:
        binary = directory/filename
        sha = load(directory/'STATE.json')['binary_sha256']
        assert digest(binary) == sha
        phase.run(label+'-symbols', ['nm', '-n', binary])
        symbols = []
        for line in (phase.path/(label+'-symbols.log')).read_text().splitlines():
            match = re.fullmatch(r'([0-9a-fA-F]+) [tT] (.*)', line)
            if match: symbols.append((int(match[1], 16), match[2]))
        selected = {}
        for suffix in suffixes:
            matches = [(address, name) for address, name in symbols if name.endswith(suffix)]
            if not matches:
                selected[suffix] = {'out_of_line_symbol': False}
                continue
            assert len(matches) == 1, (label, suffix, matches)
            address, name = matches[0]
            end = min(addr for addr, _ in symbols if addr > address)
            selected[suffix] = dict(symbol=name, start=hex(address), end=hex(end))
            phase.run(label+'-'+suffix, ['xcrun', 'llvm-objdump', '--disassemble', '--demangle',
                '--start-address='+hex(address), '--stop-address='+hex(end), binary])
        phase.state.setdefault('binaries', {})[label] = dict(path=str(binary), sha256=sha, symbols=selected)
        phase.save()
        assert digest(binary) == sha
    assert fingerprint() == source['source_fingerprint']
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('STATIC-INSPECTION-COMPLETE')
