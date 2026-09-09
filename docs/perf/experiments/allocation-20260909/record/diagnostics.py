#!/usr/bin/env python3
"""Serial, source-frozen diagnostics; invoke under scripts/measure.sh.

The timing authority is full/, never any of these instrumented runs.
Fresh phase paths are mandatory; failed output is preserved, not overwritten.
"""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys

REPO = Path('/Users/bjorn/Documents/bumbledb')
ROUND = Path(__file__).resolve().parent
SOURCE = 'b02a641e087364ec09c161a97c67c87cf626e6b2'
PROFILE = ROUND / 'profiling/bumbledb-bench'
SAMPLY = '/tmp/bumbledb-profiler-tools/bin/samply'
ENV = dict(os.environ, BUMBLEDB_PROFILE_UNDER_LOCK='1',
           BUMBLEDB_BENCH_BOOST='1', BUMBLEDB_SAMPLY=SAMPLY)
sys.path.insert(0, str(REPO / 'scripts'))
from bench_night import lanes


def digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def load(path):
    return json.loads(Path(path).read_text())


def now():
    return datetime.now(timezone.utc).isoformat()


def roster():
    reads = load(ROUND / 'full/reads/report.json')['reads']
    scenarios = load(ROUND / 'full/scenarios/scenarios.json')['queries']
    names = [row['name'] for row in [*reads, *scenarios]]
    assert len(reads) == 32 and len(scenarios) == 34
    assert len(set(names)) == len(names)
    return names


class Phase:
    def __init__(self, name):
        self.path = ROUND / name
        self.path.mkdir()
        self.state = dict(source=SOURCE, phase=name, started=now(),
                          status='RUNNING', diagnostic_only=True, steps=[],
                          driver_sha256=digest(__file__))
        self.save()

    def save(self):
        temp = self.path / 'STATE.json.tmp'
        temp.write_text(json.dumps(self.state, indent=2) + '\n')
        temp.replace(self.path / 'STATE.json')

    def run(self, name, args, artifact=None):
        args = [str(arg) for arg in args]
        record = dict(name=name, command=args, started=now(), status='RUNNING')
        self.state['steps'].append(record)
        with (self.path / f'{name}.log').open('x') as log:
            child = subprocess.Popen(args, cwd=REPO, env=ENV,
                                     stdout=log, stderr=subprocess.STDOUT,
                                     start_new_session=True)
            record['pid'] = child.pid
            self.save()
            print(f'{now()} START {name} pid={child.pid}', flush=True)
            try:
                rc = child.wait()
            except BaseException:
                if child.poll() is None:
                    os.killpg(child.pid, signal.SIGTERM)
                    try:
                        child.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.wait()
                record.update(finished=now(), status='INTERRUPTED')
                self.save()
                raise
        record.update(finished=now(), exit_code=rc,
                      status='PASS' if rc == 0 else 'FAIL')
        if rc == 0 and artifact is not None:
            try:
                payload = load(artifact)
                assert isinstance(payload, dict) and payload, artifact
                record.update(artifact=str(artifact), sha256=digest(artifact))
            except BaseException as error:
                record.update(status='FAIL', error=str(error))
                self.save()
                raise
        self.save()
        print(f'{now()} {record["status"]} {name}', flush=True)
        if rc:
            raise RuntimeError(f'{name} failed: {self.path / (name + ".log")}')

    def finish(self, status):
        self.state.update(status=status, finished=now())
        self.save()


def source_check():
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip()
    dirty = subprocess.check_output(['git', 'status', '--porcelain'], cwd=REPO, text=True)
    assert revision == SOURCE and not dirty, 'source changed; do not relabel frozen captures'
    manifest = load(ROUND / 'full/MANIFEST.json')
    assert manifest['status'] == 'LOCAL-LANES-COMPLETE', 'ordinary suite must complete first'
    assert manifest['source_revision'] == SOURCE
    assert manifest['binary_sha256'] == digest(ROUND / 'release/bumbledb-bench')


def trace_suite(phase):
    data = phase.path / 'data'
    phase.run('verify', [PROFILE, 'verify', '--dir', data / 'reads'])
    for job in lanes(PROFILE, data, phase.path / 'workloads', True):
        capture = phase.path / 'captures' / job.name
        phase.run('capture-' + job.name,
                  ['bash', REPO / 'scripts/profile.sh', capture, *job.command], job.artifact)
        # This deliberately includes setup and both engine controls. Selected
        # per-query windows below are the attribution authority for reads.
        phase.run('export-' + job.name,
                  [sys.executable, REPO / 'scripts/flame.py', 'native',
                   capture / 'profile.json.gz', capture / 'cpu', 'bumbledb_bench::main'],
                  capture / 'cpu.summary.json')
        phase.run('analyze-' + job.name,
                  [sys.executable, REPO / 'scripts/flame.py', 'analyze',
                   capture / 'cpu.summary.json', capture / 'analyzed'],
                  capture / 'analyzed.summary.json')
        if job.name != 'hash-probe':
            # The full capture includes SQLite setup, controls and helper
            # threads. Keep that view plus an engine-name substring view from
            # the SAME capture. Generic arguments can match this substring:
            # analyze's anchored nearest-engine owners, not the view's whole
            # CPU total, distinguish engine callers from those false matches.
            # Setup is included; only bound read windows isolate execution.
            phase.run('export-engine-' + job.name,
                      [sys.executable, REPO / 'scripts/flame.py', 'native',
                       capture / 'profile.json.gz', capture / 'engine', 'bumbledb::'],
                      capture / 'engine.summary.json')
            phase.run('analyze-engine-' + job.name,
                      [sys.executable, REPO / 'scripts/flame.py', 'analyze',
                       capture / 'engine.summary.json', capture / 'engine-analyzed'],
                      capture / 'engine-analyzed.summary.json')
            engine_view = load(capture / 'engine-analyzed.summary.json')
            phase.state.setdefault('suite_engine_views', {})[job.name] = {
                'filter': 'function substring bumbledb::; not an exact module filter',
                'owner_cpu_ns': sum(row['cpu_ns'] for row in engine_view['self_owners']
                                    if row['owner'].startswith(('bumbledb::', '<bumbledb::'))),
                'outside_cpu_ns': sum(row['cpu_ns'] for row in engine_view['self_owners']
                                      if not row['owner'].startswith(('bumbledb::', '<bumbledb::'))),
                'interpretation': 'Engine setup included. LTO aliases and missing symbols remain limits.'}
            phase.save()


def trace_reads(phase):
    names = roster()
    phase.state['expected_families'] = names
    phase.save()
    data = phase.path / 'data'
    phase.run('verify', [PROFILE, 'verify', '--dir', data])
    summaries = []
    for family in names:
        capture = phase.path / family
        workload = capture / 'workload/workload.json'
        phase.run('capture-' + family,
                  ['bash', REPO / 'scripts/profile.sh', capture, PROFILE,
                   'profile', '--family', family, '--seconds', '10',
                   '--dir', data, '--out', capture / 'workload'], workload)
        phase.run('export-' + family,
                  [sys.executable, REPO / 'scripts/flame.py', 'native',
                   capture / 'profile.json.gz', capture / 'cpu', '--workload', workload],
                  capture / 'cpu.summary.json')
        summary = capture / 'analyzed.summary.json'
        phase.run('analyze-' + family,
                  [sys.executable, REPO / 'scripts/flame.py', 'analyze',
                   capture / 'cpu.summary.json', capture / 'analyzed'], summary)
        summaries.append(summary)
    phase.run('survey', [sys.executable, REPO / 'scripts/flame.py', 'survey',
                        phase.path / 'survey', '--expect', ','.join(names), *summaries],
              phase.path / 'survey.survey.json')


def allocations(phase):
    phase.run('build', ['cargo', 'build', '--locked', '--release', '-p',
                        'bumbledb-bench', '--features', 'alloc-counter'])
    binary = phase.path / 'bumbledb-bench'
    shutil.copy2(REPO / 'target/release/bumbledb-bench', binary)
    phase.state['binary_sha256'] = digest(binary)
    phase.save()
    data = phase.path / 'data'
    phase.run('verify', [binary, 'verify', '--dir', data])
    phase.run('reads', [binary, 'bench', '--alloc', '--read-batch', '1',
                       '--dir', data, '--out', phase.path / 'reads'],
              phase.path / 'reads/report.json')
    phase.run('scenarios', [binary, 'scenarios', '--alloc', '--dir', data / 'scenarios',
                           '--out', phase.path / 'scenarios'],
              phase.path / 'scenarios/scenarios.json')
    report = load(phase.path / 'reads/report.json')
    assert [r['name'] for r in report['reads']] == roster()[:32]
    assert all(r['alloc'] is not None for r in report['reads'])
    scenarios = load(phase.path / 'scenarios/scenarios.json')
    assert [r['name'] for r in scenarios['queries']] == roster()[32:]
    assert all(r.get('alloc') is not None for r in scenarios['queries'])
    # The report's config.samples is NOT the denominator for displaced reads.
    # With no --samples override, frozen-source displaced::PROTO uses 12;
    # ordinary and closure reads inherit config.samples (256 by default).
    displaced = {'disp_probe', 'disp_probe_d24', 'disp_probe_d96',
                 'disp_stream', 'disp_stream_d24', 'disp_stream_d96'}
    assert {r['name'] for r in report['reads'] if r['name'].startswith('disp_')} == displaced
    samples_by_family = {r['name']: 12 if r['name'] in displaced
                         else report['config']['samples'] for r in report['reads']}
    phase.state['allocation_windows'] = {
        'reads_samples': samples_by_family,
        'reads_batch': {r['name']: r['batch'] for r in report['reads']},
        'reads_operations': {r['name']: samples_by_family[r['name']] * r['batch']
                             for r in report['reads']},
        'reads_sample_vec_requested_bytes': {name: 8 * count
                                             for name, count in samples_by_family.items()},
        'reads_protocol_source': (
            'Frozen source: displaced.rs PROTO.samples=12, driver/bench.rs passes '
            'args.samples=None; other families inherit report.config.samples.'),
        'scenarios_samples': scenarios['samples'],
        'scenarios_batch': 1,
        'interpretation': (
            'Each alloc record totals one measured window, not one operation. '
            'Use reads_operations per family, never the global config.samples '
            'for displaced families. '
            'It includes the harness sample Vec allocated after counter reset '
            '(one allocation of 8 * samples requested bytes, with proxy-per-rep off). '
            'Parameter construction and WorkContext creation are inside read draws. '
            'Counters record Rust requests, including reallocations; not RSS, '
            'mapped pages, C allocation, or allocator physical traffic. '
            'No harness subtraction has been applied to the retained raw reports.')}


def census(phase):
    # Release has no source debug information. Retain release optimization,
    # but build this call-site instrument with packed symbols before running it.
    phase.run('build', ['cargo', 'test', '--locked', '--profile', 'profiling', '-p',
                       'bumbledb', '--test', 'alloc_census', '--no-run',
                       '--message-format=json'])
    artifacts = []
    for line in (phase.path / 'build.log').read_text().splitlines():
        if not line.startswith('{'):
            continue
        row = json.loads(line)
        if (row.get('reason') == 'compiler-artifact'
                and row.get('target', {}).get('name') == 'alloc_census'
                and row.get('profile', {}).get('test') and row.get('executable')):
            artifacts.append(Path(row['executable']))
    assert len(artifacts) == 1, artifacts
    built = artifacts[0]
    binary = phase.path / built.name
    shutil.copy2(built, binary)
    phase.state['binary_sha256'] = digest(binary)
    if sys.platform == 'darwin':
        symbols = Path(str(built) + '.dSYM')
        frozen_symbols = Path(str(binary) + '.dSYM')
        assert symbols.is_dir(), f'missing packed census symbols: {symbols}'
        shutil.copytree(symbols, frozen_symbols, symlinks=False)
        phase.run('symbols', ['dwarfdump', '--uuid', binary, frozen_symbols])
        uuids = re.findall(r'UUID: ([0-9A-Fa-f-]+)',
                           (phase.path / 'symbols.log').read_text())
        assert len(uuids) == 2 and uuids[0] == uuids[1], uuids
        phase.state['symbol_uuid'] = uuids[0]
    phase.save()
    phase.run('allocation-sites', [binary, '--ignored', '--exact',
              'allocation_deep_census', '--nocapture', '--test-threads=1'])
    output = (phase.path / 'allocation-sites.log').read_text()
    assert 'test result: ok. 1 passed;' in output, 'census did not complete'
    assert re.search(r'SITE .*crates/bumbledb/src/[^\n]+:\d+', output), (
        'census has no engine source-line attribution; inspect symbols')
    phase.state['dropped_attribution_events'] = sum(
        int(value) for value in re.findall(r'event cap hit: (\d+) events untraced', output))
    phase.state['interpretation'] = (
        'Rust allocation requests, not RSS or C allocations. Backtrace-attributed '
        'windows include recorder overhead; use unattributed count windows for '
        'totals and SITE events for locations. Cold and warm windows are not '
        'interchangeable count controls. Insert fixtures construct/clone rows '
        'inside some windows: those are caller costs, not engine allocation. '
        'Check dropped event counts. '
        'This diagnostic is not the removed long churn benchmark.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['trace-suite', 'trace-reads', 'allocations', 'census'])
    args = parser.parse_args()
    if args.phase.startswith('trace-'):
        raise SystemExit(
            'Full tracing is deferred by the latest user instruction. Mine the '
            'saved traces and implement/test their supported improvements first. '
            'Revisit this guard only after an evidence-backed exhaustion audit.')
    source_check()
    phase = Phase(args.phase)
    try:
        globals()[args.phase.replace('-', '_')](phase)
        source_check()
    except BaseException:
        phase.finish('INCOMPLETE')
        raise
    else:
        phase.finish('COMPLETE')


if __name__ == '__main__':
    def stop(_signum, _frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, stop)
    main()
