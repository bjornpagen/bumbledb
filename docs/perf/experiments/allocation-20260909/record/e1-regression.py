#!/usr/bin/env python3
"""Reproduce the baseline construction regression without changing production."""
import argparse
import shutil
from diagnostics import ENV, Phase, REPO
from e1_experiment import TREE, fingerprint, freeze, git

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
assert not git('diff', 'HEAD', '--', 'crates/bumbledb/src/exec/colt/new.rs',
               'crates/bumbledb/src/api/prepared/run_join.rs')
source = fingerprint()
phase = Phase(f'e1-baseline-regression-{args.attempt}')
shutil.copy2(__file__, phase.path/'regression.py')
phase.state.update(source_fingerprint=source, source_tree=str(TREE),
                   protocol='Expect a specific old-policy assertion failure; keep raw FAIL, not a passing test.')
phase.save()
ENV.update(CARGO_BUILD_JOBS='1', CARGO_TARGET_DIR=str(REPO/'target'))
try:
    freeze(phase)
    try:
        phase.run('regression', ['cargo', 'test', '--manifest-path', TREE/'Cargo.toml',
            '--locked', '-p', 'bumbledb', '--lib',
            'empty_inputs::empty_positive_inputs_leave_all_join_maps_unforced',
            '--', '--nocapture', '--test-threads=1'])
    except RuntimeError:
        output = (phase.path/'regression.log').read_text()
        assert 'known-empty positive input forced join maps:' in output
        assert 'test result: FAILED. 0 passed; 1 failed;' in output
        assert phase.state['steps'][-1]['exit_code'] == 101
    else:
        raise AssertionError('Fixture did not reproduce the expected baseline failure')
    assert fingerprint() == source
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('EXPECTED-BASELINE-CONSTRUCTION-FAILURE-CONFIRMED')
