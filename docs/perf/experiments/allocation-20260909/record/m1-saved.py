#!/usr/bin/env python3
"""Untimed active/parked COLT ownership on the saved triangle corpus."""
import argparse
import hashlib
import json
import shutil
from diagnostics import ENV, Phase, REPO, ROUND, digest
from m1_experiment import TREE, fingerprint, freeze, git

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('variant', choices=['baseline', 'duplicate'])
parser.add_argument('attempt', type=int)
args = parser.parse_args()
force = (TREE/'crates/bumbledb/src/exec/colt/force.rs').read_bytes()
hook = b'        #[cfg(test)]\n        super::m1_observer::record_clone(other);\n'
assert force.count(hook) == 1
unhooked_force = force.replace(hook, b'')
if args.variant == 'baseline':
    assert unhooked_force == git('show', 'HEAD:crates/bumbledb/src/exec/colt/force.rs')
else:
    assert hashlib.sha256(unhooked_force).hexdigest() == 'd478afedabc0474d85b217140c63186431d978bc9766b8675cc8108f573f34b3'
source = fingerprint(observer=True)
phase = Phase(f'm1-saved-{args.variant}-{args.attempt}')
dependencies = [ROUND/'m1_saved.rs', ROUND/'m1_saved_observer.rs',
                TREE/'crates/bumbledb-bench/src/schema.rs']
hashes = {str(path): digest(path) for path in dependencies}
for path in dependencies:
    shutil.copy2(path, phase.path/path.name)
shutil.copy2(__file__, phase.path/'saved.py')
original = ROUND/'m1-gates-duplicate-1/data/fa73e680324f9b26'
db_hash = digest(original/'db/data.mdb')
oracle_hash = digest(original/'oracle.sqlite')
phase.state.update(source_fingerprint_with_test_hooks=source, variant=args.variant,
    unhooked_force_sha256=hashlib.sha256(unhooked_force).hexdigest(),
    dependencies=hashes, source_tree=str(TREE), database_sha256=db_hash,
    oracle_sha256=oracle_hash,
    protocol='Debug counting-allocator library; saved scale S/seed 1 triangle query, '
             'four exact saved parameter draws. Fresh prepared DB per draw cold/warm/repeat; '
             'then two four-draw rotations. Exact SQLite account sets in all 20 windows. '
             'Snapshot every active/parked occurrence; nonoverlapping live-map ranges checked. '
             'No timing, profiler, full trace or new corpus generation.',
    limitations='Payload Vec lengths/capacities and Rust requested layouts, not RSS, mmap, '
                'allocator usable size, peak query ownership or Pi qualification. Clone counter '
                'counts calls/three-arena source bytes/retired source bytes; not allocator traffic. '
                'Snapshot/report allocations occur outside execute windows; no snapshot in cloning.')
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
    phase.run('build', ['cargo', 'test', '--manifest-path', TREE/'Cargo.toml', '--locked',
        '-p', 'bumbledb', '--features', 'alloc-counter', '--lib', '--no-run', '--message-format=json'])
    binaries = [r['executable'] for line in (phase.path/'build.log').read_text().splitlines()
        if line.startswith('{') and (r := json.loads(line)).get('reason') == 'compiler-artifact'
        and r.get('executable') and r.get('target', {}).get('name') == 'bumbledb']
    assert len(binaries) == 1
    binary = phase.path/'m1-saved-test'
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
