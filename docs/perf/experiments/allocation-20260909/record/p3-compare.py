#!/usr/bin/env python3
"""Bounded ordinary standalone P3 ABBA; frozen binaries, no profiler."""
import argparse
import math
import shutil
from diagnostics import Phase, ROUND, digest, load
from overlap_experiment import fingerprint

READS = ['point', 'range', 'stats', 'mandate_overlap', 'conflict_pairs',
         'conflict_free', 'slot_booking_overlap']
ORDER = ['A0', 'B0', 'B1', 'A1']

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('gate', type=int)
parser.add_argument('attempt', type=int)
parser.add_argument('--variant', choices=['compact', 'block8'], default='compact')
parser.add_argument('--reference-compact', type=int)
args = parser.parse_args()
gates = {'A': ROUND/'p3-gates-baseline-1', 'B': ROUND/f'p3-gates-{args.variant}-{args.gate}'}
order = ORDER
protocol = 'ABBA'
candidate_key = 'B'
if args.reference_compact is not None:
    assert args.variant == 'block8'
    gates['C'] = gates['B']
    gates['B'] = ROUND/f'p3-gates-compact-{args.reference_compact}'
    candidate_key = 'C'
    order = ['A0', 'B0', 'C0', 'C1', 'B1', 'A1']
    protocol = 'ABC/CBA'
states = {key: load(path/'STATE.json') for key, path in gates.items()}
assert all(state['status'] == 'GATES-COMPLETE-REVIEW-REQUIRED' for state in states.values())
source_hash = states[candidate_key]['source_fingerprint']
assert fingerprint() == source_hash
expected = [row['name'] for row in load(ROUND/'full/scenarios/scenarios.json')['queries']
            if row['scenario'] == 'temporal']
phase = Phase(f'p3-comparison-{args.attempt}')
shutil.copy2(__file__, phase.path/'compare.py')
phase.state.update(diagnostic_only=False, source_fingerprint=source_hash,
    executables={key: dict(path=str(path/'bumbledb-bench'),
                          sha256=states[key]['binary_sha256'],
                          role='standalone '+states[key]['variant']) for key, path in gates.items()},
    protocol=protocol+'; 24 samples/8 warmups over all five temporal queries; '
             '32 samples, batch 1, over selected ledger/calendar read controls. '
             'Independent binary-bound oracle-verified read corpora; new scenario corpus each round.',
    limitations='Shared macOS host with QoS, not hard P-core affinity; no per-query scenario '
                'clock stamps. Keep every read boundary flag and round, no frequency normalization. '
                'Descriptive comparison, not independent sample-level significance or Pi qualification.',
    expected_scenarios=expected, expected_reads=READS, order=order)
phase.save()
reports = {}
try:
    for label in order:
        assert fingerprint() == source_hash
        key = label[0]
        binary = gates[key]/'bumbledb-bench'
        assert digest(binary) == states[key]['binary_sha256']
        out = phase.path/label
        scenario_path = out/'scenarios/scenarios.json'
        phase.run(label+'-scenarios', [binary, 'scenarios', '--only', 'temporal',
                  '--samples', '24', '--dir', out/'corpus', '--out', out/'scenarios'], scenario_path)
        scenario_report = load(scenario_path)
        assert scenario_report['samples'] == 24 and scenario_report['warmups'] == 8
        assert [row['name'] for row in scenario_report['queries']] == expected
        read_path = out/'reads/report.json'
        phase.run(label+'-reads', [binary, 'bench', '--samples', '32', '--read-batch', '1',
                  '--families', ','.join(READS), '--dir', gates[key]/'data', '--out', out/'reads'], read_path)
        read_report = load(read_path)
        assert read_report['config'] == dict(scale='S', seed=1, samples=32, store='durable')
        assert [row['name'] for row in read_report['reads']] == READS
        assert not read_report['verify_stamp'].startswith('UNVERIFIED')
        assert read_report['corpus_digest'] == load(ROUND/'full/reads/report.json')['corpus_digest']
        assert all(row['batch'] == 1 for row in read_report['reads'])
        reports[label] = {row['name']: row for row in scenario_report['queries'] + read_report['reads']}
        for name, row in reports[label].items():
            if name in expected:
                assert len({report[name]['answers'] for report in reports.values()}) == 1, name
            print(label, name, row['ours'], row.get('ghz_ours'), flush=True)
    for name in expected + READS:
        for stat in ['p50', 'mean_ns', 'p90', 'max']:
            centers = {key: math.sqrt(reports[key+'0'][name]['ours'][stat] *
                                     reports[key+'1'][name]['ours'][stat]) for key in gates}
            print('DESCRIPTIVE', name, stat, 'B/A', centers['B']/centers['A'], flush=True)
            if 'C' in centers:
                print('DESCRIPTIVE', name, stat, 'C/A', centers['C']/centers['A'], flush=True)
                print('DESCRIPTIVE', name, stat, 'C/B', centers['C']/centers['B'], flush=True)
    assert fingerprint() == source_hash
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('MEASUREMENTS-COMPLETE-REVIEW-REQUIRED')
