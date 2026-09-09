#!/usr/bin/env python3
"""Verify frozen Q3 correctness and static-code checkpoint, not performance."""
import argparse
import shutil
import subprocess
from diagnostics import Phase, ROUND, SOURCE, digest, load
from q3_experiment import TREE, fingerprint, prior_identities

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
phase = Phase(f'q3-checkpoint-{args.attempt}')
shutil.copy2(__file__, phase.path/'checkpoint.py')
try:
    gate = load(ROUND/'q3-gates-2/STATE.json')
    code = load(ROUND/'q3-codegen-1/STATE.json')
    identity = fingerprint()
    prior = prior_identities()
    assert gate['status'] == 'Q3-CORRECTNESS-GATES-COMPLETE-REVIEW-REQUIRED'
    assert code['status'] == 'Q3-STATIC-ORDINARY-CODE-EXPORTED-REVIEW-REQUIRED'
    assert identity == gate['source_fingerprint'] == code['source_fingerprint']
    assert prior == gate['prior_identities'] == code['prior_identities']
    assert all(s['status'] == 'PASS' for s in gate['steps'] + code['steps'])
    for variant in ['q2', 'q3']:
        built = load(ROUND/f'{variant}-gates-2/STATE.json')
        assert digest(ROUND/f'{variant}-gates-2/bumbledb-bench') == built['binary_sha256'] == code['selected'][variant]['binary_sha256']
        assert all(digest(ROUND/f'{variant}-gates-2/source'/n) == sha for n, sha in built['source_files'].items())
        assert all(digest(ROUND/'q3-codegen-1'/f['log']) == f['sha256'] for f in code['selected'][variant]['functions'])
    comparison = ROUND/'q1-control-baseline-source'
    assert not subprocess.check_output(['git', '-C', comparison, 'status', '--porcelain'])
    assert subprocess.check_output(['git', '-C', comparison, 'rev-parse', 'HEAD']).decode().strip() == SOURCE
    phase.run('format', ['cargo', 'fmt', '--manifest-path', TREE/'Cargo.toml', '--all', '--check'])
    phase.run('diff-check', ['git', '-C', TREE, 'diff', '--check'])
    assert fingerprint() == identity and prior_identities() == prior
    phase.state.update(source_fingerprint=identity, prior_identities=prior,
        gate_sha256=digest(ROUND/'q3-gates-2/STATE.json'),
        code_sha256=digest(ROUND/'q3-codegen-1/STATE.json'),
        documents={name:digest(ROUND/name) for name in ['Q3-DENSE-PUBLICATION.md',
            'Q2-ORDINARY-TIMING-REVIEW.md', 'Q2-INSERTION-CODE-REVIEW.md',
            'Q2-CONSUMER-AUDIT.md', 'TRACE-EXHAUSTION.md']},
        acceptance='Correctness/static deletion verified. No Q3 owner/timing comparison or generic text consumer acceptance. No full-trace authorization or completion claim.')
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('Q3-CORRECTNESS-AND-STATIC-CHECKPOINT-VERIFIED')
