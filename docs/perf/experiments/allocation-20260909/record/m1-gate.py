#!/usr/bin/env python3
"""Correctness/build gates for isolated M1 source; no timings or profiling."""
import argparse
import shutil
from diagnostics import ENV, Phase, REPO, ROUND, digest
from m1_experiment import TREE, fingerprint, freeze

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
assert 'mod audit;' not in (TREE/'crates/bumbledb/src/exec/colt/tests/construction.rs').read_text()
source = fingerprint()
phase = Phase(f'm1-gates-duplicate-{args.attempt}')
shutil.copy2(__file__, phase.path/'gate.py')
phase.state.update(source_fingerprint=source, source_tree=str(TREE),
    variant='duplicate-only', acceptance='Correctness/build only. Synthetic ownership is separate '
    'from ordinary timing. No P1/P2/P3 changes, compaction or full traces.')
phase.save()
ENV.update(CARGO_BUILD_JOBS='1', CARGO_TARGET_DIR=str(REPO/'target'))
def cargo(action, *args):
    return ['cargo', action, '--manifest-path', TREE/'Cargo.toml', *args]
try:
    freeze(phase)
    for name, command in [
        ('format', cargo('fmt', '--all', '--check')),
        ('clippy', cargo('clippy', '--locked', '-p', 'bumbledb', '-p', 'bumbledb-bench',
                         '--all-targets', '--', '-D', 'warnings')),
        ('library-tests', cargo('test', '--locked', '-p', 'bumbledb', '--lib',
                               '--', '--quiet', '--test-threads=1')),
        ('allocation-tests', cargo('test', '--locked', '-p', 'bumbledb', '--features',
                                  'alloc-counter', '--lib', '--', '--quiet', '--test-threads=1')),
        ('layout-model-tests', cargo('test', '--locked', '-p', 'bumbledb-bench', 'displaced::',
                                    '--', '--quiet', '--test-threads=1')),
        ('release-build', cargo('build', '--locked', '--release', '-p', 'bumbledb-bench')),
    ]:
        assert fingerprint() == source
        phase.run(name, command)
    binary = phase.path/'bumbledb-bench'
    shutil.copy2(REPO/'target/release/bumbledb-bench', binary)
    phase.state['binary_sha256'] = digest(binary)
    phase.save()
    phase.run('verify', [binary, 'verify', '--dir', phase.path/'data'])
    assert fingerprint() == source
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('GATES-COMPLETE-REVIEW-REQUIRED')
