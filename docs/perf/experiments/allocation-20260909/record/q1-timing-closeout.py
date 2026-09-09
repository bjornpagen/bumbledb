#!/usr/bin/env python3
"""Verify frozen ordinary Q1 evidence without rerunning any measured panel."""
import shutil
import subprocess
from diagnostics import Phase, ROUND, SOURCE, digest, load
from q1_experiment import TREE, fingerprint, prior_identities
from q1_time_experiment import TREE as COMPARISON, dependencies

phase = Phase('q1-timing-closeout-1')
shutil.copy2(__file__, phase.path/'closeout.py')
try:
    gate = load(ROUND/'q1-gates-2/STATE.json')
    timing = load(ROUND/'q1-timing-1/STATE.json')
    review = load(ROUND/'q1-timing-review-1/STATE.json')
    identity = fingerprint()
    identities = prior_identities()
    assert identity == gate['source_fingerprint'] == timing['source_fingerprint'] == review['source_fingerprint']
    assert identities == timing['prior_identities'] == review['prior_identities']
    assert timing['status'] == 'ORDINARY-Q1-TIMING-COMPLETE-REVIEW-REQUIRED'
    assert review['status'] == 'ALL-ORDINARY-Q1-DISTRIBUTIONS-VERIFIED-INTERPRETATION-REQUIRED'
    assert dependencies() == timing['dependencies'] == review['dependencies']
    assert review['distributions'] == 672 and review['clock_brackets'] == 336
    assert review['total_raw_values'] == 78848
    assert len(review['log_hashes']) == len(timing['steps']) == 112
    assert all(s['status'] == 'PASS' for s in timing['steps'])
    for name, sha in review['log_hashes'].items():
        assert digest(ROUND/'q1-timing-1'/f'{name}.log') == sha
    for name, sha in review['output_hashes'].items():
        assert digest(ROUND/'q1-timing-review-1'/name) == sha
    for name, sha in timing['input_hashes'].items():
        assert digest(ROUND/'q1-timing-1'/name) == sha
    binaries = {}
    for variant, attempt in [('baseline', 2), ('candidate', 1)]:
        path = ROUND/f'q1-time-build-{variant}-{attempt}'
        state = load(path/'STATE.json')
        assert state['status'] == 'ORDINARY-TIMING-BINARY-FROZEN'
        assert state['dependencies'] == dependencies()
        assert state['source_tree'] == str(COMPARISON)
        assert not state['engine_artifact']['fresh'] and not state['binary_artifact']['fresh']
        assert state['engine_artifact']['features'] == ['collision-probe']
        assert all(s['status'] == 'PASS' for s in state['steps'])
        assert digest(path/'q1-timing') == state['binary_sha256']
        binaries[variant] = state['binary_sha256']
    assert binaries['baseline'] != binaries['candidate']
    failed = load(ROUND/'q1-time-build-baseline-1/STATE.json')
    assert failed['status'] == 'INCOMPLETE' and failed['steps'][-1]['status'] == 'FAIL'
    assert subprocess.check_output(['git', '-C', COMPARISON, 'rev-parse', 'HEAD']).decode().strip() == SOURCE
    assert not subprocess.check_output(['git', '-C', COMPARISON, 'status', '--porcelain'])
    phase.state.update(source_fingerprint=identity, identities=identities,
        comparison_source=SOURCE, binaries=binaries,
        timing_state_sha256=digest(ROUND/'q1-timing-1/STATE.json'),
        review_state_sha256=digest(ROUND/'q1-timing-review-1/STATE.json'),
        documents={name:digest(ROUND/name) for name in
            ['Q1-ORDINARY-TIMING-REVIEW.md', 'Q1-DEMAND-GROWTH-REVIEW.md', 'Q1-BROADER-CONTROLS.md', 'TRACE-EXHAUSTION.md']},
        next='Read Q1-ORDINARY-TIMING-REVIEW.md for interpretation and the next saved-evidence discriminator. No identical timing/allocation rerun, full trace/full benchmark, commit/push or release.')
    phase.save()
    for label, tree in [('candidate', TREE), ('comparison', COMPARISON)]:
        phase.run(label+'-diff', ['git', '-C', tree, 'diff', '--check'])
        phase.run(label+'-format', ['cargo', 'fmt', '--manifest-path', tree/'Cargo.toml', '--all', '--check'])
    assert fingerprint() == identity and prior_identities() == identities
    assert not subprocess.check_output(['git', '-C', COMPARISON, 'status', '--porcelain'])
except BaseException:
    phase.finish('INCOMPLETE'); raise
else:
    phase.finish('ORDINARY-Q1-EVIDENCE-AND-UNMOUNTED-SOURCES-VERIFIED')
