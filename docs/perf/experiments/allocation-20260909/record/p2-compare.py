#!/usr/bin/env python3
"""Bounded ordinary P2 ABBA or ABC/CBA, private verified corpora; no profiler."""
import argparse
import math
import runpy
import shutil
from diagnostics import Phase, ROUND, digest, load

SCENARIOS = ['joins', 'olap']
READS = ['point', 'range', 'stats', 'triangle', 'disp_probe']
ORDER = ['A0', 'B0', 'B1', 'A1']
fingerprint = runpy.run_path(str(ROUND / 'p2-gate.py'))['fingerprint']


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('gate', type=int)
    parser.add_argument('attempt', type=int)
    parser.add_argument('--reference-gate', type=int)
    parser.add_argument('--scenarios', default=','.join(SCENARIOS))
    parser.add_argument('--reads', default=','.join(READS))
    args = parser.parse_args()
    scenarios = args.scenarios.split(',')
    reads = args.reads.split(',')
    assert len(set(scenarios)) == len(scenarios) and set(scenarios) <= set(SCENARIOS)
    assert len(set(reads)) == len(reads) and set(reads) <= set(READS)
    gates = ROUND / f'p2-gates-{args.gate}'
    source = load(gates / 'STATE.json')
    assert source['status'] == 'GATES-COMPLETE-PERFORMANCE-REVIEW-REQUIRED'
    assert fingerprint() == source['source_fingerprint']
    baseline = ROUND / 'release/bumbledb-bench'
    candidate = gates / 'bumbledb-bench'
    hashes = {'A': load(ROUND / 'full/MANIFEST.json')['binary_sha256'],
              'B': source['binary_sha256']}
    binaries = {'A': baseline, 'B': candidate}
    data = {'A': ROUND / 'p1-evaluation-2/data', 'B': gates / 'data'}
    order = ORDER
    labels = 'AB'
    protocol = 'ABBA'
    roles = {'A': 'published baseline', 'B': f'candidate gate {args.gate}'}
    if args.reference_gate is not None:
        reference = ROUND / f'p2-gates-{args.reference_gate}'
        reference_state = load(reference / 'STATE.json')
        assert reference_state['status'] == 'GATES-COMPLETE-PERFORMANCE-REVIEW-REQUIRED'
        assert args.reference_gate != args.gate
        hashes['C'] = hashes['B']
        binaries['C'] = binaries['B']
        data['C'] = data['B']
        hashes['B'] = reference_state['binary_sha256']
        binaries['B'] = reference / 'bumbledb-bench'
        data['B'] = reference / 'data'
        roles = {'A': 'published baseline', 'B': f'prior P2 gate {args.reference_gate}',
                 'C': f'current P2 gate {args.gate}'}
        order = ['A0', 'B0', 'C0', 'C1', 'B1', 'A1']
        labels = 'ABC'
        protocol = 'ABC/CBA'
    expected = [r['name'] for r in load(ROUND / 'full/scenarios/scenarios.json')['queries']
                if r['scenario'] in scenarios]
    assert expected
    phase = Phase(f'p2-comparison-{args.attempt}')
    shutil.copy2(__file__, phase.path / 'compare.py')
    phase.state.update(diagnostic_only=False, source_fingerprint=source['source_fingerprint'],
        executables={k: dict(path=str(v), sha256=hashes[k], role=roles[k]) for k, v in binaries.items()},
        protocol=protocol+'; 24 scenario samples and 8 warmups on '+','.join(scenarios)+'; '
                 '32 read samples, batch 1, on '+','.join(reads)+'. '
                 'Independent binary-bound verified read corpora; new scenario fixtures each round.',
        limitations='Shared host. Scenario suite has no per-family clock guard. '
                    'Retain flagged read windows and baseline pauses; no Pi qualification.',
        expected_scenarios=expected, expected_reads=reads)
    phase.save()
    reports = {}
    try:
        for label in order:
            assert fingerprint() == source['source_fingerprint']
            binary = binaries[label[0]]
            assert digest(binary) == hashes[label[0]]
            out = phase.path / label
            scenario_path = out / 'scenarios/scenarios.json'
            phase.run(label+'-scenarios', [binary, 'scenarios', '--only', ','.join(scenarios),
                      '--samples', '24', '--dir', out / 'corpus', '--out', out / 'scenarios'], scenario_path)
            scenario_report = load(scenario_path)
            assert scenario_report['samples'] == 24 and scenario_report['warmups'] == 8
            assert [r['name'] for r in scenario_report['queries']] == expected
            read_path = out / 'reads/report.json'
            phase.run(label+'-reads', [binary, 'bench', '--samples', '32', '--read-batch', '1',
                      '--families', ','.join(reads), '--dir', data[label[0]], '--out', out / 'reads'], read_path)
            read_report = load(read_path)
            assert read_report['config'] == dict(scale='S', seed=1, samples=32, store='durable')
            assert [r['name'] for r in read_report['reads']] == reads
            assert not read_report['verify_stamp'].startswith('UNVERIFIED')
            assert read_report['corpus_digest'] == load(ROUND / 'full/reads/report.json')['corpus_digest']
            assert all(r['batch'] == 1 for r in read_report['reads'])
            reports[label] = {r['name']: r for r in scenario_report['queries'] + read_report['reads']}
            for name, row in reports[label].items():
                if name in expected:
                    assert len({r[name]['answers'] for r in reports.values()}) == 1, name
                print(label, name, row['ours'], row.get('ghz_ours'), flush=True)
        for name in expected + reads:
            for stat in ['p50', 'mean_ns', 'p90', 'max']:
                center = {kind: math.sqrt(reports[kind+'0'][name]['ours'][stat] *
                                         reports[kind+'1'][name]['ours'][stat]) for kind in labels}
                for kind in labels[1:]:
                    print('DESCRIPTIVE', name, stat, kind+'/A', center[kind]/center['A'], flush=True)
                if 'C' in labels:
                    print('DESCRIPTIVE', name, stat, 'C/B', center['C']/center['B'], flush=True)
        assert fingerprint() == source['source_fingerprint']
    except BaseException:
        phase.finish('INCOMPLETE')
        raise
    else:
        phase.finish('MEASUREMENTS-COMPLETE-REVIEW-REQUIRED')


if __name__ == '__main__':
    main()
