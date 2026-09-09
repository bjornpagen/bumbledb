#!/usr/bin/env python3
"""Serial standalone mechanism on exported actual inputs plus controls; not engine timing."""
import argparse
import shutil
from diagnostics import ENV, Phase, REPO, ROUND, digest, load
from overlap_experiment import fingerprint

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
source = load(ROUND/'p3-gates-compact-2/STATE.json')['source_fingerprint']
assert fingerprint() == source
demand = ROUND/'p3-demand-1'
state = load(demand/'STATE.json')
assert state['status'] == 'DEMAND-COMPLETE-REVIEW-REQUIRED'
assert digest(demand/'flat-input.bin') == state['input_sha256']
paths = [ROUND/'p3-flat-mechanism.rs', ROUND/'p3-flat-kernels-v2.rs', ROUND/'p3-flat-kernels-v3.rs',
         REPO/'crates/bumbledb-bench/src/boost.rs', REPO/'crates/bumbledb-bench/src/clockproxy.rs']
hashes = {path: digest(path) for path in paths}
phase = Phase(f'p3-filter-mechanism-{args.attempt}')
shutil.copy2(__file__, phase.path/'mechanism.py')
for path in paths:
    shutil.copy2(path, phase.path/path.name)
phase.state.update(source_fingerprint=source, input_path=str(demand/'flat-input.bin'),
    roles={'A': 'same branchy baseline', 'B': '8-row start check with branchy ends',
           'C': '8-row start check with end mask and ordered set-bit emission'},
    input_sha256=state['input_sha256'], sources={str(path): sha for path, sha in hashes.items()},
    protocol='ABC/CBA matched non-inline slice kernels with equal length guards, opaque function pointer '
             'calls and equal 4096-entry warm output capacities. Actual 144953 flat query order plus nine '
             'synthetic controls, exact naive ordered answers checked outside timing. No dropped/retried windows.',
    limitations='Standalone native optimized mechanism, not executor/query speed, RSS, allocations or Pi qualification. '
                'Two blocks per arm, not significance. Clock boundary flags retained. No CPU trace or profiler.')
phase.save()
try:
    phase.run('compiler', ['rustc', '--version', '--verbose'])
    binary = phase.path/'flat-mechanism'
    phase.run('build', ['rustc', '--edition=2024', '-C', 'opt-level=3', '-C', 'codegen-units=1',
        '-C', 'debuginfo=1', '-C', 'target-cpu=native', ROUND/'p3-flat-mechanism.rs', '-o', binary])
    phase.state['binary_sha256'] = digest(binary)
    phase.save()
    phase.run('mechanism', [binary, demand/'flat-input.bin'])
    assert fingerprint() == source
    assert all(digest(path) == sha for path, sha in hashes.items())
    assert digest(demand/'flat-input.bin') == state['input_sha256']
    output = (phase.path/'mechanism.log').read_text()
    assert sum(line.startswith('MECH ') for line in output.splitlines()) == 60
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('MECHANISM-COMPLETE-REVIEW-REQUIRED')
