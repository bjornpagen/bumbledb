#!/usr/bin/env python3
"""Untimed isolated construction ownership, with frozen test-only observer."""
import argparse
import json
import shutil
from diagnostics import ENV, Phase, REPO, ROUND, digest
from m1_experiment import TREE, fingerprint, freeze

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('variant', choices=['baseline', 'duplicate'])
parser.add_argument('attempt', type=int)
args = parser.parse_args()
if args.variant == 'baseline':
    from m1_experiment import git
    assert not git('diff', 'HEAD', '--', 'crates/bumbledb/src/exec/colt/force.rs')
source = fingerprint()
observer = ROUND/'m1-audit.rs'
observer_hash = digest(observer)
phase = Phase(f'm1-audit-{args.variant}-{args.attempt}')
shutil.copy2(__file__, phase.path/'audit.py')
shutil.copy2(observer, phase.path/observer.name)
phase.state.update(source_fingerprint=source, observer_sha256=observer_hash,
    variant=args.variant, source_tree=str(TREE),
    protocol='Debug allocation-counter build; 28 synthetic cells, fixed/dynamic/zero widths, '
             '25/26 distinct keys, grouped/interleaved real image order; exact model rows, cold force, '
             'same-shape clone, reset/reforce. No timings, CPU samples or full trace.',
    owner_fields=['nbuckets', 'distinct', 'ctrl_len', 'ctrl_capacity', 'bucket_len', 'bucket_capacity',
                  'dense_len', 'dense_capacity', 'three_arena_live_bytes', 'all_pool_retained_bytes'],
    allocation_fields=['requests', 'dealloc_calls', 'requested_bytes', 'relinquished_bytes'],
    limitations='Synthetic discriminators do not prove workload prevalence. Requested layouts and '
                'Vec payload capacities are not RSS, mmap, allocator physical traffic or Pi results.')
phase.save()
ENV.update(CARGO_BUILD_JOBS='1', CARGO_TARGET_DIR=str(REPO/'target'))
try:
    freeze(phase)
    phase.run('build', ['cargo', 'test', '--manifest-path', TREE/'Cargo.toml', '--locked',
        '-p', 'bumbledb', '--features', 'alloc-counter', '--lib', '--no-run', '--message-format=json'])
    binaries = [record['executable'] for line in (phase.path/'build.log').read_text().splitlines()
        if line.startswith('{') and (record := json.loads(line)).get('reason') == 'compiler-artifact'
        and record.get('executable') and record.get('target', {}).get('name') == 'bumbledb']
    assert len(binaries) == 1
    binary = phase.path/'m1-audit-test'
    shutil.copy2(binaries[0], binary)
    phase.state['binary_sha256'] = digest(binary)
    phase.save()
    phase.run('ownership', [binary, 'construction::audit::construction_ownership',
                          '--ignored', '--nocapture', '--test-threads=1'])
    output = (phase.path/'ownership.log').read_text()
    assert 'PASS construction ownership; 28 cells;' in output
    assert 'test result: ok. 1 passed;' in output
    assert fingerprint() == source and digest(observer) == observer_hash
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('OWNERSHIP-COMPLETE-REVIEW-REQUIRED')
