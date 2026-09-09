#!/usr/bin/env python3
"""Inspect ordinary Q3 gate code; no new capture or execution timing."""
import argparse
import re
import shutil
from diagnostics import Phase, ROUND, digest, load
from q3_experiment import fingerprint, prior_identities

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
identity = fingerprint()
prior = prior_identities()
phase = Phase(f'q3-codegen-{args.attempt}')
shutil.copy2(__file__, phase.path/'codegen.py')
try:
    selected = {}
    for variant in ['q2', 'q3']:
        build = ROUND/f'{variant}-gates-2'
        state = load(build/'STATE.json')
        assert state['status'] == f'{variant.upper()}-CORRECTNESS-GATES-COMPLETE-REVIEW-REQUIRED'
        assert all(step['status'] == 'PASS' for step in state['steps'])
        assert state['engine_artifact']['features'] == ['collision-probe']
        assert not state['engine_artifact']['fresh'] and not state['binary_artifact']['fresh']
        assert all(digest(build/'source'/n) == sha for n, sha in state['source_files'].items())
        binary = build/'bumbledb-bench'
        assert digest(binary) == state['binary_sha256']
        phase.run(variant+'-symbols', ['nm', '-n', binary])
        symbols = []
        for line in (phase.path/f'{variant}-symbols.log').read_text().splitlines():
            match = re.fullmatch(r'([0-9a-fA-F]+) [tT] (.*)', line)
            if match:
                symbols.append((int(match[1], 16), match[2]))
        addresses = sorted({address for address, _ in symbols})
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
            row = dict(log=label+'.log', symbol=name, start=address, end=end, size=end-address,
                       sha256=digest(phase.path/(label+'.log')))
            exported.append(row)
            print(variant, row, flush=True)
        assert exported, variant
        selected[variant] = dict(binary_sha256=state['binary_sha256'], functions=exported,
                                source_fingerprint=state['source_fingerprint'])
    assert selected['q3']['source_fingerprint'] == identity
    assert fingerprint() == identity and prior_identities() == prior
    phase.state.update(source_fingerprint=identity, prior_identities=prior, selected=selected,
        interpretation='Static ordinary gate executables, not the earlier timing-driver executable. Separate source paths and code layouts: inspect concrete checks/stores, do not infer timing or causal speedup from sizes. No new trace or timed workload.')
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('Q3-STATIC-ORDINARY-CODE-EXPORTED-REVIEW-REQUIRED')
