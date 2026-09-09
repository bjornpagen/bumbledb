#!/usr/bin/env python3
"""Bounded ordinary M1 construction/probe control, reusing the warmth lane."""
import argparse
import math
import shutil
from diagnostics import Phase, ROUND, digest, load
from m1_experiment import fingerprint

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
gates = {'A': ROUND/'p3-gates-baseline-1', 'B': ROUND/'m1-gates-duplicate-1'}
states = {key: load(path/'STATE.json') for key, path in gates.items()}
assert all(s['status'] == 'GATES-COMPLETE-REVIEW-REQUIRED' for s in states.values())
assert states['A']['variant'] == 'baseline' and states['B']['variant'] == 'duplicate-only'
source = fingerprint()
assert source == states['B']['source_fingerprint']
families = ['triangle', 'point']
order = ['A0', 'B0', 'B1', 'A1']
phase = Phase(f'm1-comparison-{args.attempt}')
shutil.copy2(__file__, phase.path/'compare.py')
for key, gate in gates.items():
    shutil.copy2(gate/'STATE.json', phase.path/f'{key}-gate-STATE.json')
phase.state.update(diagnostic_only=False, source_fingerprint=source, order=order,
    executables={key: dict(path=str(path/'bumbledb-bench'), sha256=states[key]['binary_sha256'],
                          role=states[key]['variant']) for key, path in gates.items()},
    protocol='ABBA, scale S, seed 1. Existing triangle and point curves, 8 warmups/32 samples; '
             'warmth: 2 discarded reopen rounds +16 cold/warm measured rounds, then '
             '8 warmups/64 memoized samples. Open/prepare excluded; OS page cache warm. '
             'Independent value-multiset gate before timing. No instrumented executable.',
    rationale='triangle exercises general cold join-map construction and warm probes; '
              'point is a small lookup control. This is not a duplicate-boundary frequency census.',
    limitations='Shared macOS host with QoS, not hard P-core pinning. Warmth uses one '
                'non-retrying clock bracket for all engines/rounds; curve brackets may retry once. '
                'Parameter draws are pooled; all distributions/flags retained, no frequency '
                'normalization or significance/Pi/RSS claim. Runtime git provenance names cwd; '
                'the frozen gate/source and executable hashes identify variants.',
    expected_families=families)
phase.save()
reports = {}
try:
    for label in order:
        key = label[0]
        assert fingerprint() == source
        binary = gates[key]/'bumbledb-bench'
        assert digest(binary) == states[key]['binary_sha256']
        out = phase.path/label
        artifact = out/'curves-report.json'
        phase.run(label+'-warmth', [binary, 'curves', '--scales', 'S', '--seed', '1',
            '--families', ','.join(families), '--samples', '32', '--warmth',
            '--dir', gates[key]/'data', '--out', out], artifact)
        report = load(artifact)
        assert report['seed'] == 1 and report['samples'] == 32
        assert [r['name'] for r in report['families']] == families
        for family in report['families']:
            assert len(family['rows']) == 1 and family['rows'][0]['scale'] == 'S'
            assert family['rows'][0]['cap'] is None and family['warmth'] is not None
            assert family['rows'][0]['ours'] is not None
            assert family['rows'][0]['theirs'] is not None
            assert not family['warmth']['ghz']['retried']
        reports[label] = {row['name']: row for row in report['families']}
    for name in families:
        assert len({r[name]['rows'][0]['facts'] for r in reports.values()}) == 1
        assert len({r[name]['rows'][0]['answers'] for r in reports.values()}) == 1
        for kind in ['curve', 'ours_cold', 'ours_warm', 'ours_memoized']:
            for stat in ['p50', 'mean_ns', 'p90', 'p95', 'p99', 'max']:
                def value(label):
                    family = reports[label][name]
                    return (family['rows'][0]['ours'] if kind == 'curve'
                            else family['warmth'][kind])[stat]
                centers = {key: math.sqrt(value(key+'0')*value(key+'1')) for key in gates}
                print('DESCRIPTIVE', name, kind, stat, 'B/A', centers['B']/centers['A'], flush=True)
    assert fingerprint() == source
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('MEASUREMENTS-COMPLETE-REVIEW-REQUIRED')
