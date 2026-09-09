#!/usr/bin/env python3
"""Frozen G1 checks, with explicit expected baseline retention failure."""
import argparse
import json
import shutil
from diagnostics import ENV, Phase, REPO, digest
from g1_experiment import TREE, fingerprint, freeze, git

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('mode',choices=['baseline','focused','gates'])
parser.add_argument('attempt',type=int)
args = parser.parse_args()
if args.mode == 'baseline':
    for name in ('force.rs','grow.rs'):
        path = 'crates/bumbledb/src/exec/colt/'+name
        assert (TREE/path).read_bytes() == git('show','HEAD:'+path)
source = fingerprint()
phase = Phase(f'g1-{args.mode}-{args.attempt}')
shutil.copy2(__file__,phase.path/'check.py')
phase.state.update(source_fingerprint=source,source_tree=str(TREE),
    ordinary_file_sha256={name:digest(TREE/name) for name in (
        'crates/bumbledb/src/exec/colt/force.rs',
        'crates/bumbledb/src/exec/colt/grow.rs')},
    acceptance='Correctness/retention discriminator only. No speed/RSS/full tracing claim.')
phase.save()
ENV.update(CARGO_BUILD_JOBS='1',CARGO_TARGET_DIR=str(REPO/'target'))
def cargo(action,*extra):
    return ['cargo',action,'--manifest-path',TREE/'Cargo.toml',*extra]
try:
    freeze(phase)
    phase.run('format',cargo('fmt','--all','--check'))
    if args.mode == 'baseline':
        phase.run('build',cargo('test','--locked','-p','bumbledb','--lib','--no-run','--message-format=json'))
        binaries = [r['executable'] for line in (phase.path/'build.log').read_text().splitlines()
            if line.startswith('{') and (r:=json.loads(line)).get('reason')=='compiler-artifact'
            and r.get('executable') and r.get('target',{}).get('name')=='bumbledb']
        assert len(binaries)==1
        binary = phase.path/'g1-test'
        shutil.copy2(binaries[0],binary)
        phase.state['binary_sha256']=digest(binary)
        phase.save()
        phase.run('controls',[binary,'construction_tails::construction_preserves_',
            '--nocapture','--test-threads=1'])
        try:
            phase.run('expected-retention-failure',[binary,
                'construction_tails::successful_construction_retains_only_live_table_lengths',
                '--nocapture','--test-threads=1'])
        except RuntimeError:
            log=(phase.path/'expected-retention-failure.log').read_text()
            assert 'completed construction leaves no retired table contents' in log
            assert 'test result: FAILED. 0 passed; 1 failed;' in log
            phase.state['expected_baseline_failure_verified']=True
            phase.save()
        else:
            raise AssertionError('The baseline must reproduce the retention discriminator')
    else:
        phase.run('focused',cargo('test','--locked','-p','bumbledb','--lib',
            'construction_tails::','--','--nocapture','--test-threads=1'))
        if args.mode=='gates':
            for name,command in [
                ('clippy',cargo('clippy','--locked','-p','bumbledb','-p','bumbledb-bench',
                    '--all-targets','--','-D','warnings')),
                ('library-tests',cargo('test','--locked','-p','bumbledb','--lib',
                    '--','--quiet','--test-threads=1')),
                ('allocation-tests',cargo('test','--locked','-p','bumbledb','--features',
                    'alloc-counter','--lib','--','--quiet','--test-threads=1')),
                ('release-build',cargo('build','--locked','--release','-p','bumbledb-bench')),
            ]:
                assert fingerprint()==source
                phase.run(name,command)
            binary=phase.path/'bumbledb-bench'
            shutil.copy2(REPO/'target/release/bumbledb-bench',binary)
            phase.state['binary_sha256']=digest(binary)
            phase.save()
            phase.run('verify',[binary,'verify','--dir',phase.path/'data'])
    assert fingerprint()==source
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('EXPECTED-BASELINE-FAILURE-VERIFIED' if args.mode=='baseline'
                 else 'CHECKS-COMPLETE-REVIEW-REQUIRED')
