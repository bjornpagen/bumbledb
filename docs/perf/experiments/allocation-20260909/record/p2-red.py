#!/usr/bin/env python3
"""Prove the no-staging discriminator rejects the unchanged aggregate kernel."""
import shutil
import subprocess
from diagnostics import ENV, Phase, REPO, ROUND, digest, load

assert load(ROUND / 'p1-variants-1/STATE.json')['status'] == \
    'MEASUREMENTS-COMPLETE-REVIEW-REQUIRED'
assert not subprocess.check_output(
    ['git', 'diff', 'HEAD', '--', 'crates/bumbledb/src/exec/run/probe_pass.rs',
     'crates/bumbledb/src/exec/sink/aggregate'], cwd=REPO)
ENV['CARGO_BUILD_JOBS'] = '1'
phase = Phase('p2-regression-baseline-1')
test = REPO / 'crates/bumbledb/src/exec/sink/tests/borrowed_rows.rs'
shutil.copy2(test, phase.path / 'borrowed_rows.rs')
phase.state.update(test_sha256=digest(test), purpose='Expected structural failure on baseline; '
                   'union-key and typed-refusal controls must pass.')
phase.save()
try:
    phase.run('baseline-test', ['cargo', 'test', '--locked', '-p', 'bumbledb', '--lib',
                              'exec::sink::tests::borrowed_rows', '--', '--test-threads=1'])
except RuntimeError:
    log = (phase.path / 'baseline-test.log').read_text()
    expected = (phase.state['steps'][-1].get('exit_code') == 101
                and 'test result: FAILED. 2 passed; 1 failed;' in log
                and 'witnessed folds must not stage unused binding words' in log)
    phase.finish('EXPECTED-STRUCTURAL-FAILURE' if expected else 'INCOMPLETE')
    if not expected:
        raise
else:
    phase.finish('UNEXPECTED-PASS-REVIEW-REQUIRED')
    raise RuntimeError('Discriminator did not reject baseline')
