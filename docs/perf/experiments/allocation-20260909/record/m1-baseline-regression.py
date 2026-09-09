#!/usr/bin/env python3
"""Preserve the intended old-policy failure without relabeling it as a pass."""
import shutil
from diagnostics import Phase, ROUND, digest, load
from m1_experiment import fingerprint

baseline = ROUND/'m1-audit-baseline-2'
state = load(baseline/'STATE.json')
assert state['status'] == 'OWNERSHIP-COMPLETE-REVIEW-REQUIRED'
assert fingerprint() == state['source_fingerprint']
binary = baseline/'m1-audit-test'
assert digest(binary) == state['binary_sha256']
phase = Phase('m1-baseline-regression-1')
shutil.copy2(__file__, phase.path/'regression.py')
phase.state.update(source_fingerprint=state['source_fingerprint'],
                   binary_sha256=state['binary_sha256'], expected_exit_code=101)
phase.save()
try:
    phase.run('regression', [binary, 'construction::duplicates_do_not_grow_maps_but_new_keys_do',
                            '--nocapture', '--test-threads=1'])
except RuntimeError:
    output = (phase.path/'regression.log').read_text()
    assert phase.state['steps'][-1]['exit_code'] == 101
    assert all(part in output for part in ['width=1 distinct=25 grouped=false generation=0',
                                          'left: 16', 'right: 8', '0 passed; 1 failed;'])
    assert fingerprint() == state['source_fingerprint']
    phase.finish('EXPECTED-BASELINE-GROWTH-FAILURE-CONFIRMED')
else:
    phase.finish('UNEXPECTED-PASS')
    raise AssertionError('baseline did not reproduce the intended needless growth')
