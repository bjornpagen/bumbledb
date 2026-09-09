#!/usr/bin/env python3
"""Serial correctness/lint/oracle gate for the isolated Q2 representation."""
import argparse
import json
import shutil
from diagnostics import ENV, Phase, ROUND, digest
from q2_experiment import TREE, Q1, fingerprint, freeze, prior_identities

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
identity = fingerprint()
phase = Phase(f'q2-gates-{args.attempt}')
shutil.copy2(__file__, phase.path/'check.py')
shutil.copy2(ROUND/'q2_experiment.py', phase.path/'q2_experiment.py')
phase.state.update(source_fingerprint=identity, source_tree=str(TREE),
    base_q1_fingerprint=Q1['source_fingerprint'], prior_identities=prior_identities(),
    acceptance='Hardening/correctness only. Dense payload and first-use tradeoff unmeasured; not speed/RSS/Pi/full-suite acceptance.')
phase.save()
ENV.update(CARGO_BUILD_JOBS='1', CARGO_TARGET_DIR=str(phase.path/'cargo-target'),
           CARGO_BUILD_BUILD_DIR=str(phase.path/'cargo-build'))
def cargo(action, *extra):
    return ['cargo', action, '--manifest-path', TREE/'Cargo.toml', *extra]
try:
    freeze(phase)
    for name, command in [
        ('format', cargo('fmt', '--all', '--check')),
        ('wordmap-tests', cargo('test', '--locked', '-p', 'bumbledb', '--lib', 'exec::wordmap::tests::', '--', '--nocapture', '--test-threads=1')),
        ('sink-tests', cargo('test', '--locked', '-p', 'bumbledb', '--lib', 'exec::sink::tests::', '--', '--quiet', '--test-threads=1')),
        ('clippy', cargo('clippy', '--locked', '-p', 'bumbledb', '-p', 'bumbledb-bench', '--all-targets', '--', '-D', 'warnings')),
        ('library-tests', cargo('test', '--locked', '-p', 'bumbledb', '--lib', '--', '--quiet', '--test-threads=1')),
        ('allocation-tests', cargo('test', '--locked', '-p', 'bumbledb', '--features', 'alloc-counter', '--lib', '--', '--quiet', '--test-threads=1')),
        ('release-build', cargo('build', '--locked', '--release', '-p', 'bumbledb-bench', '--message-format=json')),
    ]:
        assert fingerprint() == identity
        phase.run(name, command)
    artifacts = [json.loads(l) for l in (phase.path/'release-build.log').read_text().splitlines()
                 if l.startswith('{') and json.loads(l).get('reason') == 'compiler-artifact']
    engine = [a for a in artifacts if a['target']['name'] == 'bumbledb']
    assert len(engine) == 1 and not engine[0]['fresh']
    assert engine[0]['features'] == ['collision-probe']
    assert engine[0]['profile']['opt_level'] == '3' and not engine[0]['profile']['debug_assertions']
    assert engine[0]['package_id'].startswith('path+file://'+str(TREE)+'/crates/bumbledb#')
    executable = [a for a in artifacts if a['target']['name'] == 'bumbledb-bench' and a['executable']]
    assert len(executable) == 1 and not executable[0]['fresh']
    binary = phase.path/'bumbledb-bench'
    shutil.copy2(executable[0]['executable'], binary)
    phase.state.update(engine_artifact=engine[0], binary_artifact=executable[0], binary_sha256=digest(binary))
    phase.save()
    phase.run('verify', [binary, 'verify', '--dir', phase.path/'data'])
    assert fingerprint() == identity and prior_identities() == phase.state['prior_identities']
except BaseException:
    phase.finish('INCOMPLETE'); raise
else:
    phase.finish('Q2-CORRECTNESS-GATES-COMPLETE-REVIEW-REQUIRED')
