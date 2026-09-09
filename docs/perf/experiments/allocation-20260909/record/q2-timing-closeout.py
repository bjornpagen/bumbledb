#!/usr/bin/env python3
"""Verify the frozen Q2 ordinary panel and remaining consumer coverage obligations."""
import argparse
import shutil
import subprocess
from diagnostics import Phase, ROUND, SOURCE, digest, load
from q2_experiment import TREE, fingerprint, prior_identities
from q2_time_experiment import TREE as COMPARISON, CASES, dependencies

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
phase = Phase(f'q2-timing-closeout-{args.attempt}')
shutil.copy2(__file__, phase.path/'closeout.py')
gate = load(ROUND/'q2-gates-2/STATE.json')
timing = load(ROUND/'q2-timing-1/STATE.json')
review = load(ROUND/'q2-timing-review-1/STATE.json')
try:
    identity = fingerprint()
    assert identity == gate['source_fingerprint'] == timing['source_fingerprint'] == review['source_fingerprint']
    assert prior_identities() == gate['prior_identities'] == timing['prior_identities'] == review['prior_identities']
    assert timing['status'] == 'ORDINARY-Q2-TIMING-COMPLETE-REVIEW-REQUIRED'
    assert review['status'] == 'ALL-ORDINARY-Q2-DISTRIBUTIONS-VERIFIED-INTERPRETATION-REQUIRED'
    assert timing['processes_completed'] == 72 and review['distributions'] == 432
    assert review['clock_brackets'] == 216 and review['total_raw_values'] == 50688
    assert timing['cases'] == [list(c) for c in CASES]
    assert all(step['status'] == 'PASS' for step in timing['steps'])
    assert dependencies() == timing['dependencies'] == review['dependencies']
    assert not subprocess.check_output(['git', '-C', COMPARISON, 'status', '--porcelain'])
    assert subprocess.check_output(['git', '-C', COMPARISON, 'rev-parse', 'HEAD']).decode().strip() == SOURCE
    builds = {}
    for letter, variant in [('A', 'q1'), ('B', 'q2')]:
        path = ROUND/f'q2-time-build-{variant}-1'
        state = load(path/'STATE.json')
        assert state['status'] == 'ORDINARY-TIMING-BINARY-FROZEN'
        assert state['dependencies'] == timing['dependencies']
        assert all(s['status'] == 'PASS' for s in state['steps'])
        assert not state['engine_artifact']['fresh'] and not state['binary_artifact']['fresh']
        assert state['engine_artifact']['features'] == ['collision-probe']
        assert state['engine_artifact']['profile']['opt_level'] == '3'
        assert not state['engine_artifact']['profile']['debug_assertions']
        assert digest(path/'q1-timing') == state['binary_sha256'] == timing['executables'][letter]['sha256']
        assert all(digest(path/'source'/name) == sha for name, sha in state['source_files'].items())
        builds[variant] = dict(source=state['ordinary_source'], binary_sha256=state['binary_sha256'])
    assert builds['q1']['binary_sha256'] != builds['q2']['binary_sha256']
    assert all(digest(ROUND/'q2-timing-1'/name) == sha for name, sha in timing['input_hashes'].items())
    assert all(digest(ROUND/'q2-timing-1'/f'{name}.log') == sha for name, sha in review['log_hashes'].items())
    assert all(digest(ROUND/'q2-timing-review-1'/name) == sha for name, sha in review['output_hashes'].items())
    phase.run('format', ['cargo', 'fmt', '--manifest-path', TREE/'Cargo.toml', '--all', '--check'])
    phase.run('diff-check', ['git', '-C', TREE, 'diff', '--check'])
    assert fingerprint() == identity
    docs = ['Q2-ORDINARY-TIMING-REVIEW.md', 'Q2-CONSUMER-AUDIT.md', 'TRACE-EXHAUSTION.md']
    phase.state.update(source_fingerprint=identity, prior_identities=prior_identities(),
        builds=builds, comparison_source='clean at '+SOURCE,
        timing_sha256=digest(ROUND/'q2-timing-1/STATE.json'),
        review_sha256=digest(ROUND/'q2-timing-review-1/STATE.json'),
        documents={n: digest(ROUND/n) for n in docs},
        acceptance='Completed fixed numeric ordinary panel, not generic candidate acceptance. Text-output memo is a production consumer not covered by this numeric timing/allocation panel. Interpret adverse cases, preserve all samples, no full trace or exhaustion declaration.')
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('Q2-NUMERIC-ORDINARY-TIMING-CHECKPOINT-VERIFIED')
