#!/usr/bin/env python3
"""Actual saved-query cache ownership, serial and untimed; temporary test hooks."""
import argparse
import json
import runpy
import shutil
import subprocess
from diagnostics import ENV, Phase, REPO, ROUND, digest, load

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
parser.add_argument('--worktree', action='store_true')
parser.add_argument('--production-fingerprint')
parser.add_argument('--mixed-paths', action='store_true')
args = parser.parse_args()
if args.worktree:
    from overlap_experiment import TREE, fingerprint as worktree_fingerprint
    assert args.production_fingerprint
    source_tree = TREE
    fingerprint = lambda: worktree_fingerprint(observer=True)
    unhooked = args.production_fingerprint
    source_note = 'Independent P3 worktree; no P1/P2 production changes.'
else:
    source_tree = REPO
    fingerprint = runpy.run_path(str(ROUND / 'p2-gate.py'))['fingerprint']
    unhooked = load(ROUND / 'p2-gates-6/STATE.json')['source_fingerprint']
    source_note = 'Gate 6 unaccepted P2; interval and executor production logic unchanged.'
source_hash = fingerprint()
phase = Phase(f'p3-owner-{args.attempt}')
shutil.copy2(__file__, phase.path / 'owner.py')
dependencies = {ROUND / name: digest(ROUND / name) for name in [
    'p3_cache_observer.rs', 'p3_exec_observer.rs', 'p3_owner.rs']}
for path in dependencies:
    shutil.copy2(path, phase.path / path.name)
original = ROUND / 'data/scenarios/scenarios/temporal'
store_hash = digest(original / 'db/data.mdb')
oracle_hash = digest(original / 'oracle.sqlite')
phase.state.update(source_fingerprint_with_test_hooks=source_hash,
    unhooked_production_fingerprint=unhooked, source_tree=str(source_tree),
    production_note=source_note,
    test_sources={str(path): sha for path, sha in dependencies.items()},
    store_sha256=store_hash, oracle_sha256=oracle_hash,
    protocol='Same saved query/schema on a byte-identical private LMDB copy; cold/warm/released passes; '
             'independent SQLite value oracle and exact per-key group counts; no timings or profiler.',
    limitations='Counting allocator + observer changes code generation. Cache-probe scoped requests '
                'include its feed closure; global query counts may include harness activity. '
                'Payload capacities are not allocator usable sizes, RSS, mmap or C allocations.')
phase.state['mixed_paths'] = args.mixed_paths
phase.save()
ENV['CARGO_BUILD_JOBS'] = '1'
ENV['CARGO_TARGET_DIR'] = str(REPO / 'target')
try:
    shutil.copytree(original / 'db', phase.path / 'db')
    assert digest(phase.path / 'db/data.mdb') == store_hash
    phase.run('oracle', ['sqlite3', '-readonly', original / 'oracle.sqlite',
        'SELECT count(*) FROM "Span" a JOIN "Span" b ON a."key"=b."key" '
        'AND a.id < b.id AND a.span_start < b.span_end AND b.span_start < a.span_end;'])
    expected = int((phase.path / 'oracle.log').read_text().strip())
    phase.run('groups', ['sqlite3', '-readonly', '-csv', original / 'oracle.sqlite',
        'SELECT "key", count(*) FROM "Span" GROUP BY "key" ORDER BY "key";'])
    if args.mixed_paths:
        phase.run('mixed-pairs', ['sqlite3', '-readonly', '-csv', original/'oracle.sqlite',
            'SELECT a."key",a.id,b.id FROM "Span" a JOIN "Span" b ON a."key"=b."key" '
            'WHERE a."key" IN (0,1,5,1000000) AND '
            '((a.span_start > b.span_start AND a.span_end < b.span_end) OR a.span_end=b.span_start) '
            'ORDER BY a."key",a.id,b.id;'])
        phase.state['mixed_pairs_sha256'] = digest(phase.path/'mixed-pairs.log')
        phase.save()
    phase.run('source-patch', ['git', '-C', source_tree, 'diff', '--binary', 'HEAD'])
    for name in filter(None, subprocess.check_output(
            ['git', '-C', source_tree, 'ls-files', '--others', '--exclude-standard', '-z']).split(b'\0')):
        dest = phase.path / 'untracked' / name.decode()
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source_tree / name.decode(), dest)
    phase.run('build', ['cargo', 'test', '--manifest-path', source_tree/'Cargo.toml',
        '--locked', '--release', '-p', 'bumbledb',
        '--features', 'alloc-counter', '--lib', '--no-run', '--message-format=json'])
    binaries = []
    for line in (phase.path / 'build.log').read_text().splitlines():
        if line.startswith('{'):
            record = json.loads(line)
            if record.get('reason') == 'compiler-artifact' and record.get('executable') \
                    and record.get('target', {}).get('name') == 'bumbledb':
                binaries.append(record['executable'])
    assert len(binaries) == 1, binaries
    binary = phase.path / 'bumbledb-owner-test'
    shutil.copy2(binaries[0], binary)
    phase.state.update(binary_sha256=digest(binary), expected_answer=expected)
    phase.save()
    assert fingerprint() == source_hash
    assert all(digest(path) == sha for path, sha in dependencies.items())
    phase.run('ownership', ['env', 'BUMBLEDB_OWNER_DB='+str(phase.path / 'db'),
        'BUMBLEDB_OWNER_GROUPS='+str(phase.path / 'groups.log'),
        'BUMBLEDB_OWNER_ANSWER='+str(expected), binary,
        'p3_owner_diagnostic::saved_temporal_cache_ownership', '--ignored', '--nocapture',
        '--test-threads=1'])
    output = (phase.path / 'ownership.log').read_text()
    assert 'test result: ok. 1 passed;' in output
    assert all('PASS '+label in output for label in ['cold', 'warm', 'released'])
    if args.mixed_paths:
        phase.run('mixed-paths', ['env', 'BUMBLEDB_OWNER_DB='+str(phase.path/'db'),
            'BUMBLEDB_MIXED_PAIRS='+str(phase.path/'mixed-pairs.log'), binary,
            'p3_owner_diagnostic::saved_mixed_mask_paths', '--ignored', '--nocapture',
            '--test-threads=1'])
        output = (phase.path/'mixed-paths.log').read_text()
        assert 'test result: ok. 1 passed;' in output
        assert 'PASS mixed exact-pairs and repeated branch counts' in output
    assert fingerprint() == source_hash
    assert all(digest(path) == sha for path, sha in dependencies.items())
    assert digest(original / 'db/data.mdb') == store_hash
    assert digest(phase.path / 'db/data.mdb') == store_hash
    assert digest(original / 'oracle.sqlite') == oracle_hash
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('OWNERSHIP-COMPLETE-REVIEW-REQUIRED')
