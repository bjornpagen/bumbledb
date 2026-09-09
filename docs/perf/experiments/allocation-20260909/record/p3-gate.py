#!/usr/bin/env python3
"""Gate independent P3 source and freeze its ordinary executable; no timings."""
import argparse
import shutil
from diagnostics import ENV, Phase, REPO, ROUND, digest
from overlap_experiment import TREE, assert_baseline_production, fingerprint, git

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('variant', choices=['baseline', 'compact', 'block8'])
parser.add_argument('attempt', type=int)
args = parser.parse_args()
if args.variant == 'baseline':
    assert_baseline_production()
source_hash = fingerprint()
assert 'p3_cache_observer' not in (TREE/'crates/bumbledb/src/interval/overlap.rs').read_text()
ENV.update(CARGO_BUILD_JOBS='1', CARGO_TARGET_DIR=str(REPO/'target'))
phase = Phase(f'p3-gates-{args.variant}-{args.attempt}')
shutil.copy2(__file__, phase.path / 'gate.py')
shutil.copy2(ROUND/'overlap_experiment.py', phase.path/'overlap_experiment.py')
phase.state.update(source_fingerprint=source_hash, source_tree=str(TREE),
    variant=args.variant, acceptance='Correctness only, not memory or timing acceptance. '
    'Published baseline production for baseline; no P1/P2 production changes in this worktree.')
phase.save()
manifest = str(TREE/'Cargo.toml')
def cargo(action, *args):
    return ['cargo', action, '--manifest-path', manifest, *args]
try:
    phase.run('source-patch', ['git', '-C', TREE, 'diff', '--binary', 'HEAD'])
    for name in filter(None, git('ls-files', '--others', '--exclude-standard', '-z').split(b'\0')):
        dest = phase.path/'untracked'/name.decode()
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(TREE/name.decode(), dest)
    for label, command in [
        ('format', cargo('fmt', '--all', '--check')),
        ('clippy', cargo('clippy', '--locked', '-p', 'bumbledb', '--all-targets', '--', '-D', 'warnings')),
        ('library-tests', cargo('test', '--locked', '-p', 'bumbledb', '--lib', '--', '--quiet', '--test-threads=1')),
        ('allocation-tests', cargo('test', '--locked', '-p', 'bumbledb', '--features', 'alloc-counter',
                                    '--lib', '--', '--quiet', '--test-threads=1')),
        ('release-build', cargo('build', '--locked', '--release', '-p', 'bumbledb-bench')),
    ]:
        assert fingerprint() == source_hash
        phase.run(label, command)
    binary = phase.path/'bumbledb-bench'
    shutil.copy2(REPO/'target/release/bumbledb-bench', binary)
    phase.state['binary_sha256'] = digest(binary)
    phase.save()
    phase.run('verify', [binary, 'verify', '--dir', phase.path/'data'])
    assert fingerprint() == source_hash
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('GATES-COMPLETE-REVIEW-REQUIRED')
