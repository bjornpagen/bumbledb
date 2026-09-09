#!/usr/bin/env python3
"""Untimed saved-query demand census + exact replay, debug build, no profiler."""
import argparse
import json
import shutil
from diagnostics import ENV, Phase, REPO, ROUND, digest, load
from overlap_experiment import TREE, fingerprint, git

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
source = fingerprint(demand=True)
reference = load(ROUND/'p3-gates-compact-2/STATE.json')
dependencies = {ROUND/name: digest(ROUND/name) for name in [
    'p3-demand-cache.rs', 'p3-demand-exec.rs', 'p3-demand-query.rs', 'p3-flat-kernels.rs']}
phase = Phase(f'p3-demand-{args.attempt}')
shutil.copy2(__file__, phase.path/'demand.py')
shutil.copy2(ROUND/'overlap_experiment.py', phase.path/'overlap_experiment.py')
for path in dependencies:
    shutil.copy2(path, phase.path/path.name)
original = ROUND/'data/scenarios/scenarios/temporal'
store_hash, oracle_hash = digest(original/'db/data.mdb'), digest(original/'oracle.sqlite')
phase.state.update(source_tree=str(TREE), source_with_hooks=source,
    reference_source=reference['source_fingerprint'], test_sources={str(k): v for k, v in dependencies.items()},
    store_sha256=store_hash, oracle_sha256=oracle_hash,
    protocol='Debug test-only demand observation after actual indexed queries. Cold/warm schedules, '
             'exact ordered replay, real output capacities, flat-input export. No timing/counter claims.',
    limitations='Not a full trace, CPU sampling, allocation census, RSS or production speed measurement. '
                'Capture Vec is allocated outside the operation; no growth permitted inside recording. '
                'Replay growth events describe only the output Vec, not whole-query allocations.')
phase.save()
ENV.update(CARGO_BUILD_JOBS='1', CARGO_TARGET_DIR=str(REPO/'target'))
try:
    shutil.copytree(original/'db', phase.path/'db')
    assert digest(phase.path/'db/data.mdb') == store_hash
    phase.run('oracle', ['sqlite3', '-readonly', original/'oracle.sqlite',
        'SELECT count(*) FROM "Span" a JOIN "Span" b ON a."key"=b."key" '
        'AND a.id < b.id AND a.span_start < b.span_end AND b.span_start < a.span_end;'])
    expected = int((phase.path/'oracle.log').read_text().strip())
    phase.run('source-patch', ['git', '-C', TREE, 'diff', '--binary', 'HEAD'])
    for name in filter(None, git('ls-files', '--others', '--exclude-standard', '-z').split(b'\0')):
        dest = phase.path/'untracked'/name.decode()
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(TREE/name.decode(), dest)
    phase.run('build', ['cargo', 'test', '--manifest-path', TREE/'Cargo.toml', '--locked',
        '-p', 'bumbledb', '--lib', '--no-run', '--message-format=json'])
    binaries = [record['executable'] for line in (phase.path/'build.log').read_text().splitlines()
        if line.startswith('{') and (record := json.loads(line)).get('reason') == 'compiler-artifact'
        and record.get('executable') and record.get('target', {}).get('name') == 'bumbledb']
    assert len(binaries) == 1, binaries
    binary = phase.path/'bumbledb-demand-test'
    shutil.copy2(binaries[0], binary)
    phase.state.update(binary_sha256=digest(binary), expected_answer=expected)
    phase.save()
    phase.run('demand', ['env', 'BUMBLEDB_DEMAND_DB='+str(phase.path/'db'),
        'BUMBLEDB_DEMAND_OUT='+str(phase.path/'flat-input.bin'), 'BUMBLEDB_DEMAND_ANSWER='+str(expected),
        binary, 'p3_demand_query::saved_flat_filter_demand', '--ignored', '--nocapture', '--test-threads=1'])
    output = (phase.path/'demand.log').read_text()
    assert all('PASS '+label in output for label in ['cold', 'warm'])
    assert 'test result: ok. 1 passed;' in output
    phase.state['input_sha256'] = digest(phase.path/'flat-input.bin')
    assert fingerprint(demand=True) == source
    assert all(digest(path) == sha for path, sha in dependencies.items())
    assert digest(original/'db/data.mdb') == store_hash == digest(phase.path/'db/data.mdb')
    assert digest(original/'oracle.sqlite') == oracle_hash
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('DEMAND-COMPLETE-REVIEW-REQUIRED')
