#!/usr/bin/env python3
"""Resume P1's frozen evaluation after its binary-bound stamp refusal.

The original attempt is immutable evidence. Successful timings are adopted by
hash, not repeated; the candidate gets a genuinely reverified private corpus.
No source build, profiler, stamp rewriting, or verification bypass is allowed.
"""

import argparse
import importlib.util
import os
import shutil
import signal
import subprocess
import sys
import time

from diagnostics import Phase, REPO, ROUND, digest, load

ORIGINAL = ROUND / 'p1-evaluation-2'
BASELINE = ROUND / 'release/bumbledb-bench'
CANDIDATE = ORIGINAL / 'candidate-bumbledb-bench'
LABELS = ['A0', 'A1', 'B0', 'A2', 'B1', 'A3']
SPEC = importlib.util.spec_from_file_location('original_p1', ORIGINAL / 'evaluate.py')
EVALUATION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(EVALUATION)


def checked_original():
    state = load(ORIGINAL / 'STATE.json')
    assert EVALUATION.source_fingerprint() == state['source_fingerprint']
    assert digest(BASELINE) == state['baseline_sha256']
    assert digest(CANDIDATE) == state['candidate_sha256']
    assert digest(ORIGINAL / 'evaluate.py') == state['evaluation_driver_sha256']
    steps = {step['name']: step for step in state['steps']}
    for name in ['library-tests', 'allocation-tests', 'clippy', 'build-candidate',
                 'verify-candidate', 'verify-baseline']:
        assert steps[name]['status'] == 'PASS', name
    return state, steps


def check_report(path, kind, binary):
    report = load(path)
    if kind == 'scenarios':
        assert report['samples'] == 24
        assert [row['name'] for row in report['queries']] == [
            row['name'] for row in load(ROUND / 'full/scenarios/scenarios.json')['queries']]
    else:
        assert report['config']['samples'] == 96
        assert report['corpus_digest'] == load(ROUND / 'full/reads/report.json')['corpus_digest']
        assert {row['name'] for row in report['reads']} == set(EVALUATION.READS)
        assert all(row['batch'] == 1 for row in report['reads'])
        assert not report['verify_stamp'].startswith('UNVERIFIED')
        verified = ORIGINAL / ('verify-baseline.log' if binary == BASELINE else 'verify-candidate.log')
        expected = verified.read_text().rsplit('stamp ', 1)[1].splitlines()[0].strip()
        assert report['verify_stamp'].split()[0] == expected


def run():
    state, steps = checked_original()
    assert state['status'] == 'INCOMPLETE', 'original attempt must have stopped normally'
    failed = [step for step in steps.values() if step['status'] != 'PASS']
    assert len(failed) == 1 and failed[0]['name'] == 'B0-reads'
    assert failed[0]['status'] == 'FAIL'
    assert 'no fresh verify stamp for this corpus' in (ORIGINAL / 'B0-reads.log').read_text()
    assert not (ORIGINAL / 'B0/reads/report.json').exists()
    # Recheck the last child: no second timing process may be overlapped.
    for step in failed:
        try:
            os.kill(step['pid'], 0)
        except ProcessLookupError:
            pass
        else:
            raise RuntimeError('previous child PID still exists; inspect before continuing')

    phase = Phase('p1-evaluation-2-resume-1')
    shutil.copy2(__file__, phase.path / 'resume.py')
    phase.state.update(
        diagnostic_only=False,
        original_state=str(ORIGINAL / 'STATE.json'),
        original_state_sha256=digest(ORIGINAL / 'STATE.json'),
        source_fingerprint=state['source_fingerprint'],
        candidate_sha256=digest(CANDIDATE), baseline_sha256=digest(BASELINE),
        resume_driver_sha256=digest(__file__),
        protocol=state['protocol'], limitations=state['limitations'],
        repair='Separate binary-specific verified corpora; reuse successful original receipts. '
               'Original candidate corpus was replaced by baseline verification, so reverify '
               'candidate once in a fresh directory. No fabricated/restored verification stamp.',
        measurements=[], adopted_gates=[steps[name] for name in [
            'library-tests', 'allocation-tests', 'clippy', 'build-candidate',
            'verify-candidate', 'verify-baseline']],
    )
    phase.save()
    try:
        candidate_data = phase.path / 'candidate-data'
        phase.run('verify-candidate-private', [CANDIDATE, 'verify', '--dir', candidate_data])
        for label in LABELS:
            binary = BASELINE if label.startswith('A') else CANDIDATE
            data = ORIGINAL / 'data' if binary == BASELINE else candidate_data
            for kind in ['scenarios', 'reads']:
                assert EVALUATION.source_fingerprint() == state['source_fingerprint']
                name = label + '-' + kind
                old = steps.get(name)
                if old is not None and old['status'] == 'PASS':
                    artifact = old['artifact']
                    assert digest(artifact) == old['sha256']
                    check_report(artifact, kind, binary)
                    receipt = dict(old, adopted_from=str(ORIGINAL / 'STATE.json'))
                else:
                    assert old is None or name == 'B0-reads'
                    out = phase.path / label
                    if kind == 'scenarios':
                        artifact = out / 'scenarios/scenarios.json'
                        command = [binary, kind, '--samples', '24', '--dir', out / 'corpus',
                                   '--out', out / 'scenarios']
                    else:
                        artifact = out / 'reads/report.json'
                        command = [binary, 'bench', '--samples', '96', '--read-batch', '1',
                                   '--families', ','.join(EVALUATION.READS), '--dir', data,
                                   '--out', out / 'reads']
                    phase.run(name, command, artifact)
                    check_report(artifact, kind, binary)
                    receipt = dict(phase.state['steps'][-1])
                phase.state['measurements'].append(receipt)
                phase.save()
        checked_original()
    except BaseException:
        phase.finish('INCOMPLETE')
        raise
    else:
        phase.finish('MEASUREMENTS-COMPLETE-REVIEW-REQUIRED')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['check', 'wait', 'run'])
    args = parser.parse_args()
    if args.mode == 'check':
        state, steps = checked_original()
        for step in steps.values():
            if step['status'] == 'PASS' and 'artifact' in step:
                assert digest(step['artifact']) == step['sha256']
                check_report(step['artifact'], step['name'].split('-')[1], BASELINE)
        print('Frozen binaries, passed gates, and completed report receipts validated.')
    elif args.mode == 'wait':
        # A monitor only: no benchmark or corpus writes until the original
        # driver finishes and measure.sh grants the serial measurement lock.
        while load(ORIGINAL / 'STATE.json')['status'] == 'RUNNING':
            time.sleep(20)
        command = ['bash', str(REPO / 'scripts/measure.sh'), sys.executable, __file__, 'run']
        os.execvp(command[0], command)
    else:
        run()


if __name__ == '__main__':
    def stop(_signum, _frame):
        raise KeyboardInterrupt

    signal.signal(signal.SIGTERM, stop)
    main()
