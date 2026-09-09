#!/usr/bin/env python3
"""Freeze an ordinary release binary for matched per-draw G2 timing."""
import argparse
import json
import shutil
from diagnostics import ENV, Phase, REPO, ROUND, digest, load
import g2_experiment
from g2_experiment import TREE, fingerprint, freeze, git
g2_experiment.ALLOWED.add("crates/bumbledb-bench/Cargo.toml")

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('variant', choices=['baseline', 'candidate'])
parser.add_argument('attempt', type=int)
args = parser.parse_args()
gate = load(ROUND/'g2-gates-3/STATE.json')
assert gate['status'] == 'CHECKS-COMPLETE-REVIEW-REQUIRED'
import hashlib
force = 'crates/bumbledb/src/exec/colt/force.rs'
production = {force: hashlib.sha256(git('show','HEAD:'+force)).hexdigest(),
    'crates/bumbledb/src/exec/colt/grow.rs': digest(ROUND/'g2-ordinary-grow.rs')}
if args.variant == 'baseline':
    assert not git('diff', 'HEAD', '--', *production)
else:
    assert all(digest(TREE/name) == sha for name, sha in production.items())
source = fingerprint()
harness = ROUND/'e1_timing.rs'
harness_sha = digest(harness)
phase = Phase(f'g2-time-build-{args.variant}-{args.attempt}')
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
    phase.run('clippy', cargo('clippy', '--locked', '-p', 'bumbledb-bench', '--bin', 'g2-timing',
        '--', '-D', 'warnings'))
    phase.run('build', cargo('build', '--locked', '--release', '-p', 'bumbledb-bench',
        '--bin', 'g2-timing', '--message-format=json'))
    artifacts = [json.loads(line) for line in (phase.path/'build.log').read_text().splitlines()
        if line.startswith('{') and json.loads(line).get('reason') == 'compiler-artifact']
    engine = [r for r in artifacts if r['target']['name'] == 'bumbledb']
    assert len(engine) == 1 and engine[0]['features'] == ['collision-probe']
    assert engine[0]['profile']['opt_level'] == '3'
    assert not engine[0]['profile']['test'] and not engine[0]['profile']['debug_assertions']
    phase.state['engine_features'] = engine[0]['features']
    binaries = [r['executable'] for r in artifacts if r['target']['name'] == 'g2-timing' and r['executable']]
    assert len(binaries) == 1
    binary = phase.path/'g2-timing'
    shutil.copy2(binaries[0], binary)
    phase.state['binary_sha256'] = digest(binary)
    phase.save()
    assert fingerprint() == source and digest(harness) == harness_sha
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('ORDINARY-TIMING-BINARY-FROZEN')
