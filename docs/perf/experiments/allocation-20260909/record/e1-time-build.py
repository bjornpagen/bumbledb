#!/usr/bin/env python3
"""Freeze an ordinary release binary for matched per-draw E1 timing."""
import argparse
import json
import shutil
from diagnostics import ENV, Phase, REPO, ROUND, digest
from e1_experiment import TREE, fingerprint, freeze, git

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('variant', choices=['baseline', 'candidate'])
parser.add_argument('attempt', type=int)
args = parser.parse_args()
production = {
    'crates/bumbledb/src/api/prepared/run_join.rs': 'd957718e40ea4827a13649938f6aa6fc9d292413b9ea289ac79e8ce6c84d6cd4',
    'crates/bumbledb/src/exec/colt/new.rs': '2424d024e019d6566d29f80f7e97975c420c0cfe29914e07333b29c4293caa79',
}
if args.variant == 'baseline':
    assert not git('diff', 'HEAD', '--', *production)
else:
    assert all(digest(TREE/name) == sha for name, sha in production.items())
source = fingerprint(timing=True)
harness = ROUND/'e1_timing.rs'
harness_sha = digest(harness)
phase = Phase(f'e1-time-build-{args.variant}-{args.attempt}')
shutil.copy2(__file__, phase.path/'build.py')
shutil.copy2(harness, phase.path/harness.name)
phase.state.update(variant=args.variant, source_tree=str(TREE), source_fingerprint=source,
    harness_sha256=harness_sha, production_files={name: digest(TREE/name) for name in production},
    protocol='Same external per-draw driver and workspace fat-LTO release profile. '
             'No allocator features, observer hooks, profiler or timed compilation.')
phase.save()
ENV.update(CARGO_BUILD_JOBS='1', CARGO_TARGET_DIR=str(REPO/'target'))
def cargo(action, *extra):
    return ['cargo', action, '--manifest-path', TREE/'Cargo.toml', *extra]
try:
    freeze(phase)
    phase.run('format', cargo('fmt', '--all', '--check'))
    phase.run('clippy', cargo('clippy', '--locked', '-p', 'bumbledb-bench', '--bin', 'e1-timing',
        '--', '-D', 'warnings'))
    phase.run('build', cargo('build', '--locked', '--release', '-p', 'bumbledb-bench',
        '--bin', 'e1-timing', '--message-format=json'))
    artifacts = [json.loads(line) for line in (phase.path/'build.log').read_text().splitlines()
        if line.startswith('{') and json.loads(line).get('reason') == 'compiler-artifact']
    engine = [r for r in artifacts if r['target']['name'] == 'bumbledb']
    assert len(engine) == 1 and 'alloc-counter' not in engine[0]['features']
    phase.state['engine_features'] = engine[0]['features']
    binaries = [r['executable'] for r in artifacts if r['target']['name'] == 'e1-timing' and r['executable']]
    assert len(binaries) == 1
    binary = phase.path/'e1-timing'
    shutil.copy2(binaries[0], binary)
    phase.state['binary_sha256'] = digest(binary)
    phase.save()
    assert fingerprint(timing=True) == source and digest(harness) == harness_sha
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('ORDINARY-TIMING-BINARY-FROZEN')
