#!/usr/bin/env python3
"""Ordinary ABC/CBA comparison on rings, joins and OLAP. No profiler."""
import math
import shutil

from diagnostics import Phase, ROUND, digest, load
from p1_resume_import import source_fingerprint

SCENARIOS = ['rings', 'joins', 'olap']
LABELS = ['A0', 'B0', 'C0', 'C1', 'B1', 'A1']


def main():
    source = load(ROUND / 'p1-iterator-1/STATE.json')
    old = load(ROUND / 'p1-evaluation-2/STATE.json')
    assert source_fingerprint() == source['source_fingerprint']
    assert source['status'] == 'GATES-COMPLETE-CODEGEN-AND-PERFORMANCE-REVIEW-REQUIRED'
    binaries = {
        'A': (ROUND / 'release/bumbledb-bench', old['baseline_sha256']),
        'B': (ROUND / 'p1-evaluation-2/candidate-bumbledb-bench', old['candidate_sha256']),
        'C': (ROUND / 'p1-iterator-1/bumbledb-bench', source['binary_sha256']),
    }
    for binary, expected in binaries.values():
        assert digest(binary) == expected
    names = [r['name'] for r in load(ROUND / 'full/scenarios/scenarios.json')['queries']
             if r['scenario'] in SCENARIOS]
    assert names and any(n.startswith('r4_') for n in names)
    phase = Phase('p1-variants-1')
    shutil.copy2(__file__, phase.path / 'variants.py')
    phase.state.update(
        diagnostic_only=False, source_fingerprint=source['source_fingerprint'],
        executables={label: dict(path=str(path), sha256=sha) for label, (path, sha) in binaries.items()},
        protocol='A baseline, B indexed cursor runs, C iterator cursor runs; A0 B0 C0 C1 B1 A1; '
                 '24 samples, 8 warmups, full registered parameter stream for rings/joins/olap.',
        limitations='Shared host; scenario suite has no per-family clock guard; '
                    'p99 is the maximum at 24 draws, not an estimated population tail. '
                    'This does not clear prior root-read signals or accept any candidate.',
        expected_queries=names,
    )
    phase.save()
    reports = {}
    try:
        phase.run('verify-iterator', [binaries['C'][0], 'verify', '--dir', phase.path / 'iterator-data'])
        for label in LABELS:
            assert source_fingerprint() == source['source_fingerprint']
            binary, expected = binaries[label[0]]
            assert digest(binary) == expected
            out = phase.path / label
            artifact = out / 'scenarios.json'
            phase.run(label, [binary, 'scenarios', '--only', ','.join(SCENARIOS),
                              '--samples', '24', '--dir', out / 'corpus', '--out', out], artifact)
            report = load(artifact)
            assert report['samples'] == 24 and report['warmups'] == 8
            assert [r['name'] for r in report['queries']] == names
            reports[label] = {r['name']: r for r in report['queries']}
            for name in names:
                assert len({r[name]['answers'] for r in reports.values()}) == 1, name
                print(label, name, reports[label][name]['ours'], flush=True)
        for name in names:
            for statistic in ['p50', 'mean_ns', 'p90', 'max']:
                values = {label: reports[label][name]['ours'][statistic] for label in LABELS}
                center = {kind: math.sqrt(values[kind+'0'] * values[kind+'1']) for kind in 'ABC'}
                print('DESCRIPTIVE', name, statistic, 'B/A', center['B']/center['A'],
                      'C/A', center['C']/center['A'], 'C/B', center['C']/center['B'], flush=True)
        assert source_fingerprint() == source['source_fingerprint']
    except BaseException:
        phase.finish('INCOMPLETE')
        raise
    else:
        phase.finish('MEASUREMENTS-COMPLETE-REVIEW-REQUIRED')


if __name__ == '__main__':
    main()
