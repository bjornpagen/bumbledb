#!/usr/bin/env python3
"""Inspect frozen Q2/Q1 ordinary code after measured warm losses; no new build/trace."""
import argparse
import re
import shutil
from diagnostics import Phase, ROUND, digest, load
from q2_experiment import fingerprint, prior_identities

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
phase = Phase(f'q2-codegen-{args.attempt}')
shutil.copy2(__file__, phase.path/'codegen.py')
identity, prior = fingerprint(), prior_identities()
try:
    selected = {}
    for variant in ['q1', 'q2']:
        build = ROUND/f'q2-time-build-{variant}-1'
        state = load(build/'STATE.json')
        binary = build/'q1-timing'
        assert state['status'] == 'ORDINARY-TIMING-BINARY-FROZEN'
        assert digest(binary) == state['binary_sha256']
        phase.run(variant+'-symbols', ['nm', '-n', binary])
        symbols = []
        for line in (phase.path/f'{variant}-symbols.log').read_text().splitlines():
            m = re.fullmatch(r'([0-9a-fA-F]+) [tT] (.*)', line)
            if m:
                symbols.append((int(m[1], 16), m[2]))
        addresses = sorted({a for a, _ in symbols})
        exported = []
        for address, name in symbols:
            wanted = (('WordMap' in name and any(part in name for part in ['insert_rows', 'entry_dyn', 'grow']))
                      or ('SpillSet' in name and any(part in name for part in ['insert', 'append']))
                      or ('AggregateSink' in name and 'probe_group' in name)
                      or ('ResolveMemo' in name and 'resolve_checked' in name))
            if not wanted:
                continue
            end = next(a for a in addresses if a > address)
            label = f'{variant}-function-{len(exported)}'
            phase.run(label, ['xcrun', 'llvm-objdump', '--disassemble', '--demangle',
                '--no-show-raw-insn', f'--start-address={hex(address)}', f'--stop-address={hex(end)}', binary])
            row = dict(log=label+'.log', symbol=name, start=address, end=end, size=end-address)
            exported.append(row)
            print(variant, row, flush=True)
        assert exported, variant
        selected[variant] = dict(binary_sha256=state['binary_sha256'], functions=exported)
    assert fingerprint() == identity and prior_identities() == prior
    phase.state.update(source_fingerprint=identity, prior_identities=prior, selected=selected,
        interpretation='Static ordinary code, not cycle counts or sampled attribution. Large first-use wins do not explain unflagged warm losses. Shared ordinal/key payload insertion and duplicate probes need discrimination; no automatic instruction-count speed claim.')
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('Q2-STATIC-ORDINARY-CODE-EXPORTED-REVIEW-REQUIRED')
