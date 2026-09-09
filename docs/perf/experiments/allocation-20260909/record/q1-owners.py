#!/usr/bin/env python3
"""Observe current saved-corpus output and survivor owners, no new CPU trace."""
import argparse
import hashlib
import json
import shutil
import subprocess
from diagnostics import ENV, Phase, REPO, ROUND, SOURCE, digest, load

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
parser.add_argument('--prepare-only', action='store_true')
args = parser.parse_args()
tree = ROUND/'q1-source'
phase = Phase(f'q1-{"prepare" if args.prepare_only else "owners"}-{args.attempt}')
mounts = {
    'crates/bumbledb/src/api/prepared/tests.rs': ('q1_prepare.rs' if args.prepare_only else 'q1_saved.rs', 'mod q1_saved;'),
    'crates/bumbledb/src/exec/colt.rs': ('q1_colt_observer.rs', 'mod q1_observer;'),
    'crates/bumbledb/src/exec/sink.rs': ('q1_sink_observer.rs', 'mod q1_observer;'),
    'crates/bumbledb/src/exec/wordmap.rs': ('q1_wordmap_observer.rs', 'mod q1_observer;'),
    'crates/bumbledb/src/image/view.rs': ('q1_view_observer.rs', 'pub(crate) mod q1_observer;'),
}

def git(*args):
    return subprocess.check_output(['git', '-C', str(tree), *args])

def source():
    assert git('rev-parse', 'HEAD').decode().strip() == SOURCE
    assert not git('ls-files', '--others', '--exclude-standard')
    names = git('diff', '--name-only', 'HEAD').decode().splitlines()
    assert set(names) == set(mounts), names
    for name, (external, declaration) in mounts.items():
        hook = (f'\n#[path = "{ROUND/external}"]\n{declaration}\n').encode()
        if not name.endswith('/tests.rs'):
            hook = b'\n#[cfg(test)]' + hook
        actual = (tree/name).read_bytes()
        assert actual.count(hook) == 1, name
        assert actual.replace(hook, b'') == git('show', 'HEAD:'+name), name
    return hashlib.sha256(git('diff', '--binary', 'HEAD')).hexdigest()

original = ROUND/'m1-gates-duplicate-1/data/fa73e680324f9b26'
baseline = load(ROUND/'m1-saved-baseline-1/STATE.json')
dependencies = [ROUND/pair[0] for pair in mounts.values()]
if args.prepare_only:
    dependencies.append(ROUND/'q1_saved.rs')
dependencies.append(tree/'crates/bumbledb-bench/src/schema.rs')
hashes = {str(path): digest(path) for path in dependencies}
db_hash = digest(original/'db/data.mdb')
oracle_hash = digest(original/'oracle.sqlite')
assert db_hash == baseline['database_sha256'] and oracle_hash == baseline['oracle_sha256']
ENV.update(CARGO_BUILD_JOBS='1', CARGO_TARGET_DIR=str(REPO/'target'))
try:
    source_hash = source()
    phase.state.update(source_tree=str(tree), hooks_sha256=source_hash,
        ordinary_source=SOURCE, dependencies=hashes, database_sha256=db_hash,
        oracle_sha256=oracle_hash, protocol=('Two saved preparations, no execution; counts sampled before owner/estimate observation' if args.prepare_only else '32 exact saved SQL windows; allocation counts before observation; seed replay outside counters; no timing or full trace'))
    phase.save()
    shutil.copy2(__file__, phase.path/'owners.py')
    for path in dependencies:
        shutil.copy2(path, phase.path/path.name)
    phase.run('source-patch', ['git', '-C', tree, 'diff', '--binary', 'HEAD'])
    shutil.copytree(original/'db', phase.path/'db')
    assert digest(phase.path/'db/data.mdb') == db_hash
    phase.run('oracle', ['sqlite3', '-readonly', '-csv', original/'oracle.sqlite',
        "WITH draws(i,lo,hi) AS (VALUES (0,1,6),(1,167,172),(2,333,338),(3,500,500)) "
        "SELECT DISTINCT 'triangle', d.i,a.account FROM draws d JOIN Posting a "
        "ON a.account>=d.lo AND a.account<d.hi JOIN Posting b ON a.instrument=b.instrument "
        "JOIN Posting c ON b.entry=c.entry AND a.account=c.account ORDER BY d.i,a.account; "
        "WITH draws(i) AS (VALUES (0),(1),(2),(3)) "
        "SELECT 'range', d.i,p.id,p.amount FROM draws d JOIN Posting p "
        "ON p.at>=1700000000000000+5000000*(2*d.i+1)/16 "
        "AND p.at<1700000000000000+5000000*(2*d.i+1)/16+100000 ORDER BY d.i,p.id,p.amount;"])
    phase.run('build', ['cargo', 'test', '--manifest-path', tree/'Cargo.toml', '--locked',
        '-p', 'bumbledb', '--features', 'alloc-counter', '--lib', '--no-run', '--message-format=json'])
    binaries = [r['executable'] for line in (phase.path/'build.log').read_text().splitlines()
        if line.startswith('{') and (r := json.loads(line)).get('reason') == 'compiler-artifact'
        and r.get('executable') and r.get('target', {}).get('name') == 'bumbledb']
    assert len(binaries) == 1
    binary = phase.path/'saved-test'
    shutil.copy2(binaries[0], binary)
    phase.state['binary_sha256'] = digest(binary)
    phase.save()
    phase.run('ownership', ['env', 'BUMBLEDB_Q1_DB='+str(phase.path/'db'),
        'BUMBLEDB_Q1_ORACLE='+str(phase.path/'oracle.log'), binary,
        'q1_saved::'+('saved_preparation' if args.prepare_only else 'saved_output_and_survivors'), '--ignored', '--nocapture', '--test-threads=1'])
    expected = 'PASS saved preparation; two queries before any execution' if args.prepare_only else 'PASS saved output and survivor ownership; 32 exact SQL windows'
    assert expected in (phase.path/'ownership.log').read_text()
    assert source() == source_hash
    assert all(digest(path) == sha for path, sha in hashes.items())
    assert digest(original/'db/data.mdb') == db_hash == digest(phase.path/'db/data.mdb')
    assert digest(original/'oracle.sqlite') == oracle_hash
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('OWNERSHIP-COMPLETE-REVIEW-REQUIRED')
