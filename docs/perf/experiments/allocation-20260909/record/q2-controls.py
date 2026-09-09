#!/usr/bin/env python3
"""Matched requested/retained allocation controls for packed WordMap vs Q1."""
import argparse
import json
import re
import shutil
import sys
from diagnostics import ENV, Phase, ROUND, digest
from q2_control_experiment import TREE, GATES, source, dependencies
from q2_experiment import prior_identities

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('variant', choices=['q1', 'q2'])
parser.add_argument('attempt', type=int)
args = parser.parse_args()
assert GATES['q2']['status'] == 'Q2-CORRECTNESS-GATES-COMPLETE-REVIEW-REQUIRED'
files, deps = source(args.variant), dependencies(args.variant)
phase = Phase(f'q2-controls-{args.variant}-{args.attempt}')
original = ROUND/'m1-gates-duplicate-1/data/fa73e680324f9b26'
inputs = {name: digest(original/name) for name in ['db/data.mdb', 'oracle.sqlite']}
phase.state.update(variant=args.variant, source_tree=str(TREE), source_files=files,
    ordinary_source=GATES[args.variant]['source_fingerprint'], dependencies=deps,
    input_hashes=inputs, prior_identities=prior_identities(),
    owner_schema=['ctrl bytes', 'key words', 'values', 'generation stamp bytes',
                  'Q1 dense sparse-slot indices' if args.variant == 'q1' else 'Q2 sparse dense-row ordinals'],
    protocol='Unchanged 41-case saved Q1 corpus, all 328 request windows and 488 map owners; independent SQLite answers; no timing/full trace. Same-path fresh target AND build dirs. Exact owner schema transition explicit. Common-field regression must fail for Q1 and pass for Q2.')
phase.save()
shutil.copy2(__file__, phase.path/'controls.py')
shutil.copy2(ROUND/'q2_control_experiment.py', phase.path/'experiment.py')
ENV.update(CARGO_BUILD_JOBS='1', CARGO_TARGET_DIR=str(phase.path/'cargo-target'),
           CARGO_BUILD_BUILD_DIR=str(phase.path/'cargo-build'))
try:
    for n, path in enumerate(deps):
        shutil.copy2(path, phase.path/f'dependency-{n}-{path.rsplit("/", 1)[-1]}')
    for name in files:
        dest = phase.path/'source'/name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(TREE/name, dest)
    phase.run('source-patch', ['git', '-C', TREE, 'diff', '--binary', 'HEAD'])
    shutil.copytree(original/'db', phase.path/'db')
    shutil.copy2(original/'oracle.sqlite', phase.path/'oracle.sqlite')
    phase.run('build', ['cargo', 'test', '--manifest-path', TREE/'Cargo.toml', '--locked',
        '-p', 'bumbledb', '--features', 'alloc-counter', '--lib', '--no-run', '--message-format=json'])
    artifacts = [r for line in (phase.path/'build.log').read_text().splitlines()
        if line.startswith('{') and (r := json.loads(line)).get('reason') == 'compiler-artifact'
        and r.get('executable') and r.get('target', {}).get('name') == 'bumbledb']
    assert len(artifacts) == 1 and not artifacts[0]['fresh']
    assert artifacts[0]['features'] == ['alloc-counter']
    assert artifacts[0]['package_id'].startswith('path+file://'+str(TREE)+'/crates/bumbledb#')
    assert artifacts[0]['executable'].startswith(str(phase.path)+'/')
    binary = phase.path/'controls-test'
    shutil.copy2(artifacts[0]['executable'], binary)
    phase.state.update(engine_artifact=artifacts[0], binary_sha256=digest(binary))
    phase.save()
    regression = [str(binary), 'q2_payload_regression::packed_payload_lengths_discriminator',
                  '--ignored', '--nocapture', '--test-threads=1']
    if args.variant == 'q1':
        # Keep the failing test output and require precisely the expected
        # assertion, not just any nonzero process exit or a compiler failure.
        check = ('import subprocess,sys; p=subprocess.run(sys.argv[1:],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True); '
                 'print(p.stdout); assert p.returncode == 101; '
                 'assert "values must be packed" in p.stdout; '
                 'assert "0 passed; 1 failed" in p.stdout; print("EXPECTED-Q1-REGRESSION-FAILURE")')
        phase.run('regression', [sys.executable, '-c', check, *regression])
    else:
        phase.run('regression', regression)
    phase.run('controls', ['env', 'BUMBLEDB_Q1_DB='+str(phase.path/'db'),
        'BUMBLEDB_Q1_ORACLE='+str(phase.path/'oracle.sqlite'), binary,
        'q1_controls::saved_controls', '--ignored', '--nocapture', '--test-threads=1'])
    raw = (phase.path/'controls.log').read_text()
    assert 'PASS controls cases=41 windows=328' in raw
    maps = re.findall(r'^MAP (\S+) role=(\S+) rows=(\d+) arity=(\d+) owners=(\[.*?\]) bytes=(\d+)$', raw, re.M)
    assert len(maps) == 488
    assert all(int(size) == 0 for label, role, rows, arity, owners, size in maps if ':prepare/' in label)
    assert any(label == 'union:100000:cold/main' and int(rows) == 100000 for label, role, rows, arity, owners, size in maps)
    assert source(args.variant) == files and dependencies(args.variant) == deps
    assert prior_identities() == phase.state['prior_identities']
    assert all(digest(original/n) == sha == digest(phase.path/n) for n, sha in inputs.items())
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('Q2-ALLOCATION-CONTROLS-COMPLETE-REVIEW-REQUIRED')
