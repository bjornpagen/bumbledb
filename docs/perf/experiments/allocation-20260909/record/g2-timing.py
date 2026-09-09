#!/usr/bin/env python3
"""One fixed per-case ABBA panel on the saved corpus; no tracing or retries."""
import argparse
import json
import shutil
from diagnostics import Phase, ROUND, digest, load
from g2_experiment import fingerprint

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
parser.add_argument('--baseline-build', type=int, default=1)
parser.add_argument('--candidate-build', type=int, default=1)
args = parser.parse_args()
builds = {'A': ROUND/f'g2-time-build-baseline-{args.baseline_build}',
          'B': ROUND/f'g2-time-build-candidate-{args.candidate_build}'}
states = {key: load(path/'STATE.json') for key, path in builds.items()}
for key, state in states.items():
    assert state['status'] == 'ORDINARY-TIMING-BINARY-FROZEN'
    assert state['variant'] == {'A': 'baseline', 'B': 'candidate'}[key]
    assert all(step['status'] == 'PASS' for step in state['steps'])
    assert 'alloc-counter' not in state['engine_features']
assert states['A']['harness_sha256'] == states['B']['harness_sha256'] == digest(ROUND/'e1_timing.rs')
source = fingerprint()
assert source == load(ROUND/'g2-gates-3/STATE.json')['source_fingerprint']
phase = Phase(f'g2-timing-{args.attempt}')
shutil.copy2(__file__, phase.path/'timing.py')
for key, path in builds.items():
    shutil.copy2(path/'STATE.json', phase.path/f'{key}-build-STATE.json')
original = ROUND/'m1-gates-duplicate-1/data/fa73e680324f9b26'
hashes = {name: digest(original/name) for name in ['db/data.mdb', 'oracle.sqlite']}
cases = [(family, draw) for draw in range(4) for family in ['triangle', 'point']]
order = ['A0', 'B0', 'B1', 'A1']
phase.state.update(diagnostic_only=False, source_fingerprint=source, cases=cases, order=order,
    executables={key: dict(path=str(path/'g2-timing'), sha256=states[key]['binary_sha256'],
                          variant=states[key]['variant']) for key, path in builds.items()},
    input_hashes=hashes, harness_sha256=states['A']['harness_sha256'],
    protocol='Per-case ABBA. Four existing triangle draws and four point controls, S/seed 1. '
             'Cold: fresh DB reopen and prepare excluded, 2 discarded +16 first/second calls. '
             'Alternating-parameter warmed (rotating label): alternate draw (i+1)%4 outside timer, then target, 8 warmups +64 samples. '
             'Same-target warmed (memoized label): 8 warmups +64 timed batches of16 calls. All raw elapsed values '
             'preserved, no clock normalization/filtering/retry. Four hand-SQL value-multiset '
             'gates before each process times anything, checked answers after measured operations.',
    limitations='Shared macOS host; verified user-interactive QoS is steering, not hard P-core '
                'affinity. First/second share one non-retrying clock bracket; rotating and '
                'memoized each have one. Oracle checks outside timers can warm output memory. '
                'Cold args are constructed outside timers; rotating/memoized include the existing '
                'parameter-array helper. A batch validates its final reusable output after timing. '
                'Not the full suite, a new full trace, whole-app/RSS or Pi qualification.')
phase.save()
reports = {}
try:
    shutil.copytree(original/'db', phase.path/'db')
    shutil.copy2(original/'oracle.sqlite', phase.path/'oracle.sqlite')
    for name, sha in hashes.items():
        assert digest(phase.path/name) == sha
    for family, draw in cases:
        for label in order:
            key = label[0]
            assert fingerprint() == source
            binary = builds[key]/'g2-timing'
            assert digest(binary) == states[key]['binary_sha256']
            name = f'{family}-{draw}-{label}'
            phase.run(name, [binary, phase.path/'db', phase.path/'oracle.sqlite', family, str(draw)])
            lines = [json.loads(line) for line in (phase.path/f'{name}.log').read_text().splitlines()
                     if line.startswith('{')]
            assert len(lines) == 5
            meta, *blocks = lines
            assert meta['family'] == family and meta['draw'] == draw and meta['alternate'] == (draw+1)%4
            assert meta['oracle_draws_checked'] == 4
            if family == 'triangle':
                assert meta['parameters'] == [[1, 6], [167, 172], [333, 338], [500, 500]][draw]
                assert meta['answers'] == (0 if draw == 3 else 5)
            assert [b['kind'] for b in blocks] == ['cold', 'second', 'rotating', 'memoized']
            for block in blocks:
                assert len(block['raw_ns']) == (16 if block['kind'] in ['cold', 'second'] else 64)
                assert block['batch'] == (16 if block['kind'] == 'memoized' else 1)
                assert not block['ghz']['retried']
            reports[name] = lines
            for input_name, sha in hashes.items():
                assert digest(phase.path/input_name) == sha
    for name, sha in hashes.items():
        assert digest(original/name) == sha
    assert fingerprint() == source and digest(ROUND/'e1_timing.rs') == states['A']['harness_sha256']
    phase.state['processes_completed'] = len(reports)
    phase.state['distributions'] = sum(len(records)-1 for records in reports.values())
    phase.save()
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('PER-DRAW-MEASUREMENTS-COMPLETE-REVIEW-REQUIRED')
