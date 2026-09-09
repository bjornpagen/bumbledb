#!/usr/bin/env python3
"""Same saved triangle and observer as M1, with G1 construction-tail reclaim."""
import argparse
import json
import shutil
from diagnostics import ENV, Phase, REPO, ROUND, digest, load
from g1_experiment import TREE, fingerprint, freeze, git

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
hook = b'        #[cfg(test)]\n        super::m1_observer::record_clone(other);\n'
force = (TREE/'crates/bumbledb/src/exec/colt/force.rs').read_bytes()
assert force.count(hook) == 1
import hashlib
gate = load(ROUND/'g1-gates-2/STATE.json')
assert gate['status'] == 'CHECKS-COMPLETE-REVIEW-REQUIRED'
assert hashlib.sha256(force.replace(hook, b'')).hexdigest() == \
    gate['ordinary_file_sha256']['crates/bumbledb/src/exec/colt/force.rs']
assert digest(TREE/'crates/bumbledb/src/exec/colt/grow.rs') == \
    gate['ordinary_file_sha256']['crates/bumbledb/src/exec/colt/grow.rs']
source = fingerprint(observer=True)
baseline = load(ROUND/'m1-saved-baseline-1/STATE.json')
assert baseline['status'] == 'OWNERSHIP-COMPLETE-REVIEW-REQUIRED'
phase = Phase(f'g1-saved-{args.attempt}')
dependencies = [ROUND/'m1_saved.rs', ROUND/'m1_saved_observer.rs',
                ROUND/'m1-source/crates/bumbledb-bench/src/schema.rs']
hashes = {str(path): digest(path) for path in dependencies}
assert hashes == baseline['dependencies']
for path in dependencies:
    shutil.copy2(path, phase.path/path.name)
shutil.copy2(__file__, phase.path/'saved.py')
original = ROUND/'m1-gates-duplicate-1/data/fa73e680324f9b26'
db_hash = digest(original/'db/data.mdb')
oracle_hash = digest(original/'oracle.sqlite')
assert db_hash == baseline['database_sha256']
assert oracle_hash == baseline['oracle_sha256']
phase.state.update(source_fingerprint_with_test_hooks=source, source_tree=str(TREE),
    baseline=str(ROUND/'m1-saved-baseline-1'), dependencies=hashes,
    database_sha256=db_hash, oracle_sha256=oracle_hash,
    protocol=baseline['protocol'], limitations=baseline['limitations'])
phase.save()
ENV.update(CARGO_BUILD_JOBS='1', CARGO_TARGET_DIR=str(REPO/'target'))
try:
    freeze(phase)
    shutil.copytree(original/'db', phase.path/'db')
    assert digest(phase.path/'db/data.mdb') == db_hash
    phase.run('oracle', ['sqlite3', '-readonly', '-csv', original/'oracle.sqlite',
        'WITH draws(lo,hi) AS (VALUES (1,6),(167,172),(333,338),(500,500)) '
        'SELECT DISTINCT d.lo,a.account FROM draws d JOIN Posting a '
        'ON a.account>=d.lo AND a.account<d.hi JOIN Posting b ON a.instrument=b.instrument '
        'JOIN Posting c ON b.entry=c.entry AND a.account=c.account ORDER BY d.lo,a.account;'])
    assert digest(phase.path/'oracle.log') == digest(ROUND/'m1-saved-baseline-1/oracle.log')
    phase.run('build', ['cargo', 'test', '--manifest-path', TREE/'Cargo.toml', '--locked',
        '-p', 'bumbledb', '--features', 'alloc-counter', '--lib', '--no-run', '--message-format=json'])
    binaries = [r['executable'] for line in (phase.path/'build.log').read_text().splitlines()
        if line.startswith('{') and (r := json.loads(line)).get('reason') == 'compiler-artifact'
        and r.get('executable') and r.get('target', {}).get('name') == 'bumbledb']
    assert len(binaries) == 1
    binary = phase.path/'g1-saved-test'
    shutil.copy2(binaries[0], binary)
    phase.state['binary_sha256'] = digest(binary)
    phase.save()
    phase.run('ownership', ['env', 'BUMBLEDB_M1_DB='+str(phase.path/'db'),
        'BUMBLEDB_M1_ORACLE='+str(phase.path/'oracle.log'), binary,
        'm1_saved::saved_triangle_ownership', '--ignored', '--nocapture', '--test-threads=1'])
    output = (phase.path/'ownership.log').read_text()
    assert 'test result: ok. 1 passed;' in output
    assert 'PASS saved triangle ownership; 20 exact SQL windows' in output
    assert fingerprint(observer=True) == source
    assert all(digest(path) == sha for path, sha in hashes.items())
    assert digest(original/'db/data.mdb') == db_hash == digest(phase.path/'db/data.mdb')
    assert digest(original/'oracle.sqlite') == oracle_hash
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('OWNERSHIP-COMPLETE-REVIEW-REQUIRED')
