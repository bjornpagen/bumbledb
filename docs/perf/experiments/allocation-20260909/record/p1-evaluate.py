#!/usr/bin/env python3
"""Serial correctness/allocation gates, then ordinary A/A and A/B controls.

No profiler or full trace is invoked. This evaluates one frozen source patch;
it never accepts a candidate automatically from a single timing comparison.
"""

import argparse
import hashlib
import shutil
import signal
import subprocess

from diagnostics import Phase, REPO, ROUND, SOURCE, digest, load

TEST = 'crates/bumbledb/src/exec/run/tests/probe_runs.rs'
TRACKED = {
    'crates/bumbledb/src/exec/run/probe_pass.rs',
    'crates/bumbledb/src/exec/run/tests.rs',
    'crates/bumbledb/tests/alloc_census.rs',
}
READS = ['point', 'range', 'triangle', 'stats', 'conflict_pairs',
         'disp_probe', 'disp_probe_d24', 'disp_probe_d96', 'disp_stream_d24']


def source_fingerprint():
    def git(*args):
        return subprocess.check_output(['git', *args], cwd=REPO)

    assert git('rev-parse', 'HEAD').decode().strip() == SOURCE
    assert set(git('diff', '--name-only', 'HEAD').decode().splitlines()) == TRACKED
    assert git('ls-files', '--others', '--exclude-standard').decode().splitlines() == [TEST]
    assert load(ROUND / 'census-retry-1/STATE.json')['status'] == 'COMPLETE'
    patch = git('diff', '--binary', 'HEAD')
    return hashlib.sha256(patch + (REPO / TEST).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('attempt', type=int)
    args = parser.parse_args()
    assert args.attempt > 0
    source_hash = source_fingerprint()
    phase = Phase(f'p1-evaluation-{args.attempt}')
    shutil.copy2(__file__, phase.path / 'evaluate.py')
    phase.state.update(
        diagnostic_only=False,
        source_note='Base revision plus recorded P1 patch; not unchanged baseline source.',
        source_fingerprint=source_hash,
        evaluation_driver_sha256=digest(__file__),
        hypothesis='Adjacent equal tagged cursors reuse probe setup; no new warm allocations.',
        protocol='A0,A1 baseline controls, then B0,A2,B1,A3; 24 scenario samples, 96 read samples, batch 1.',
        limitations='Shared M2 Max host, macOS QoS steering is not hard P-core affinity. No Pi qualification.',
        prior_tests='Baseline sibling tests 4 passed; candidate library 1322 passed, 18 ignored (session 67198).',
    )
    phase.save()
    try:
        phase.run('source-patch', ['git', 'diff', '--binary', 'HEAD'])
        shutil.copy2(REPO / TEST, phase.path / 'probe_runs.rs')
        phase.run('library-tests', ['cargo', 'test', '--locked', '-p', 'bumbledb',
                                   '--lib', '--', '--quiet', '--test-threads=1'])
        phase.run('allocation-tests', ['cargo', 'test', '--locked', '-p', 'bumbledb',
                  '--features', 'alloc-counter', '--lib', 'exec::', '--', '--quiet', '--test-threads=1'])
        phase.run('clippy', ['cargo', 'clippy', '--locked', '-p', 'bumbledb',
                            '--all-targets', '--', '-D', 'warnings'])
        phase.run('build-candidate', ['cargo', 'build', '--locked', '--release',
                                     '-p', 'bumbledb-bench'])
        candidate = phase.path / 'candidate-bumbledb-bench'
        shutil.copy2(REPO / 'target/release/bumbledb-bench', candidate)
        baseline = ROUND / 'release/bumbledb-bench'
        manifest = load(ROUND / 'full/MANIFEST.json')
        assert manifest['status'] == 'LOCAL-LANES-COMPLETE'
        assert manifest['binary_sha256'] == digest(baseline)
        phase.state.update(candidate_sha256=digest(candidate), baseline_sha256=digest(baseline))
        phase.save()
        assert source_fingerprint() == source_hash
        data = phase.path / 'data'
        phase.run('verify-candidate', [candidate, 'verify', '--dir', data])
        phase.run('verify-baseline', [baseline, 'verify', '--dir', data])
        for label, binary in [('A0', baseline), ('A1', baseline), ('B0', candidate),
                              ('A2', baseline), ('B1', candidate), ('A3', baseline)]:
            assert source_fingerprint() == source_hash
            out = phase.path / label
            phase.run(label + '-scenarios', [binary, 'scenarios', '--samples', '24',
                      '--dir', out / 'corpus', '--out', out / 'scenarios'],
                      out / 'scenarios/scenarios.json')
            rows = load(out / 'scenarios/scenarios.json')['queries']
            assert [row['name'] for row in rows] == [
                row['name'] for row in load(ROUND / 'full/scenarios/scenarios.json')['queries']]
            phase.run(label + '-reads', [binary, 'bench', '--samples', '96', '--read-batch', '1',
                      '--families', ','.join(READS), '--dir', data, '--out', out / 'reads'],
                      out / 'reads/report.json')
            rows = load(out / 'reads/report.json')['reads']
            assert {row['name'] for row in rows} == set(READS)
            assert all(row['batch'] == 1 for row in rows)
        assert source_fingerprint() == source_hash
    except BaseException:
        phase.finish('INCOMPLETE')
        raise
    else:
        phase.finish('MEASUREMENTS-COMPLETE-REVIEW-REQUIRED')


if __name__ == '__main__':
    def stop(_signum, _frame):
        raise KeyboardInterrupt

    signal.signal(signal.SIGTERM, stop)
    main()
