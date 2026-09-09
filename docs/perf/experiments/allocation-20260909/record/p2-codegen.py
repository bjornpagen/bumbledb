#!/usr/bin/env python3
"""Inspect frozen P2 code generation serially; no process is profiled or timed."""
import argparse
import re
import shutil
from diagnostics import Phase, ROUND, digest, load

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('gate', type=int)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
gate = ROUND / f'p2-gates-{args.gate}'
state = load(gate / 'STATE.json')
assert state['status'] == 'GATES-COMPLETE-PERFORMANCE-REVIEW-REQUIRED'
binary = gate / 'bumbledb-bench'
assert digest(binary) == state['binary_sha256']
phase = Phase(f'p2-codegen-{args.attempt}')
shutil.copy2(__file__, phase.path / 'codegen.py')
phase.state.update(binary=str(binary), binary_sha256=state['binary_sha256'],
                   production_source_fingerprint=state['source_fingerprint'],
                   interpretation='Static release code generation, not sampled attribution or timings.')
phase.save()
try:
    phase.run('symbols', ['nm', '-n', binary])
    symbols = []
    for line in (phase.path / 'symbols.log').read_text().splitlines():
        match = re.fullmatch(r'([0-9a-fA-F]+) [tT] (.*)', line)
        if match:
            symbols.append((int(match[1], 16), match[2]))
    selected = {}
    for label, suffix, optional in [('shape', '19refresh_shape_cache', True),
                                    ('rebuild', '19rebuild_shape_cache', True),
                                    ('scan', '4Sink10begin_scan', False),
                                    ('emit', '4Sink10emit_batch', False),
                                    ('rows', '15fold_batch_rows', False)]:
        matches = [(addr, name) for addr, name in symbols
                   if '13AggregateSink' in name and name.endswith(suffix)]
        if optional and not matches:
            selected[label] = dict(out_of_line_symbol=False,
                                   note='May be absent or inlined; inspect callers.')
            continue
        assert len(matches) == 1, (label, matches)
        address, symbol = matches[0]
        end = min(addr for addr, _ in symbols if addr > address)
        selected[label] = dict(symbol=symbol, start=hex(address), end=hex(end))
        phase.run(label, ['xcrun', 'llvm-objdump', '--disassemble', '--demangle',
                         f'--start-address={hex(address)}', f'--stop-address={hex(end)}', binary])
    assert any('start' in selected[label] for label in ['shape', 'rebuild']), \
        'Neither cache helper has a symbol; inspect manually before changing the utility.'
    phase.state['symbols'] = selected
    phase.save()
    assert digest(binary) == state['binary_sha256']
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('STATIC-INSPECTION-COMPLETE')
