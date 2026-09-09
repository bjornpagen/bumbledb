#!/usr/bin/env python3
"""Freeze the Q2 correctness/allocation checkpoint; no timing acceptance."""
import argparse
import shutil
import subprocess
from diagnostics import ENV, Phase, ROUND, SOURCE, digest, load
from q2_experiment import TREE, fingerprint, prior_identities

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
phase = Phase(f'q2-allocation-closeout-{args.attempt}')
shutil.copy2(__file__, phase.path/'closeout.py')
comparison = ROUND/'q1-control-baseline-source'
gate = load(ROUND/'q2-gates-2/STATE.json')
review = load(ROUND/'q2-controls-review-1/STATE.json')
identity = fingerprint()
try:
    assert identity == gate['source_fingerprint'] == review['sources']['q2']
    assert gate['status'] == 'Q2-CORRECTNESS-GATES-COMPLETE-REVIEW-REQUIRED'
    assert review['status'] == 'Q2-ALL-OWNERS-REQUESTS-AND-REPRESENTATION-RECONCILED'
    assert all(s['status'] == 'PASS' for s in gate['steps'])
    assert digest(ROUND/'q2-gates-2/bumbledb-bench') == gate['binary_sha256']
    assert 'verify OK: 2879 cases' in (ROUND/'q2-gates-2/verify.log').read_text()
    assert review['q1_saved_observations_reproduced']
    assert review['prior_identities'] == prior_identities() == gate['prior_identities']
    assert not subprocess.check_output(['git', '-C', comparison, 'status', '--porcelain'])
    assert subprocess.check_output(['git', '-C', comparison, 'rev-parse', 'HEAD']).decode().strip() == SOURCE
    controls = {}
    for variant in ['q1', 'q2']:
        path = ROUND/f'q2-controls-{variant}-1'
        s = load(path/'STATE.json')
        assert s['status'] == 'Q2-ALLOCATION-CONTROLS-COMPLETE-REVIEW-REQUIRED'
        assert all(step['status'] == 'PASS' for step in s['steps'])
        assert s['prior_identities'] == gate['prior_identities']
        assert not s['engine_artifact']['fresh']
        assert digest(path/'controls-test') == s['binary_sha256']
        assert digest(path/'controls.log') == review['log_hashes'][variant]
        assert all(digest(n) == sha for n, sha in s['dependencies'].items())
        assert all(digest(path/n) == sha for n, sha in s['input_hashes'].items())
        controls[variant] = dict(binary_sha256=s['binary_sha256'], source=s['ordinary_source'])
    phase.run('format', ['cargo', 'fmt', '--manifest-path', TREE/'Cargo.toml', '--all', '--check'])
    phase.run('diff-check', ['git', '-C', TREE, 'diff', '--check'])
    assert fingerprint() == identity
    docs = ['Q2-DENSE-WORDMAP-LEAD.md', 'Q2-ALLOCATION-REVIEW.md', 'TRACE-EXHAUSTION.md']
    scripts = ['q2_time_experiment.py', 'q2-time-build.py', 'q2-timing.py', 'q2-timing-review.py']
    phase.state.update(source_fingerprint=identity, prior_identities=prior_identities(),
        controls=controls, comparison_source='clean at '+SOURCE,
        gate_sha256=digest(ROUND/'q2-gates-2/STATE.json'),
        review_sha256=digest(ROUND/'q2-controls-review-1/STATE.json'),
        documents={n: digest(ROUND/n) for n in docs},
        unexecuted_timing_scripts={n: digest(ROUND/n) for n in scripts},
        acceptance='Correctness and allocation checkpoint only. Ordinary Q2-vs-Q1 timing remains required; no speed/RSS/Pi/full-suite acceptance or trace exhaustion.')
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('Q2-CORRECTNESS-ALLOCATION-CHECKPOINT-VERIFIED')
