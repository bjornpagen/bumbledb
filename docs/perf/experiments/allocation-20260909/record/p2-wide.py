#!/usr/bin/env python3
"""Freeze/run the ordinary wide-row mechanism test, after engine timings end."""
import argparse
import json
import runpy
import shutil
from diagnostics import ENV, Phase, REPO, ROUND, digest, load

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('gate', type=int)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
assert load(ROUND / 'p2-comparison-1/STATE.json')['status'] == 'MEASUREMENTS-COMPLETE-REVIEW-REQUIRED'
assert load(ROUND / 'p2-comparison-2/STATE.json')['status'] == 'MEASUREMENTS-COMPLETE-REVIEW-REQUIRED'
gated = load(ROUND / f'p2-gates-{args.gate}/STATE.json')
assert gated['status'] == 'GATES-COMPLETE-PERFORMANCE-REVIEW-REQUIRED'
kernel = REPO / 'crates/bumbledb/src/exec/sink/aggregate/fold_row.rs'
assert 'mod wide_diagnostic;' in kernel.read_text(), 'Mount the recorded test-only hook first'
fingerprint = runpy.run_path(str(ROUND / 'p2-gate.py'))['fingerprint']
source_hash = fingerprint()
dependencies = {
    path: digest(path) for path in [
        ROUND / 'p2_wide.rs',
        REPO / 'crates/bumbledb-bench/src/boost.rs',
        REPO / 'crates/bumbledb-bench/src/clockproxy.rs',
    ]
}
ENV['CARGO_BUILD_JOBS'] = '1'
phase = Phase(f'p2-wide-{args.attempt}')
shutil.copy2(__file__, phase.path / 'wide.py')
shutil.copy2(ROUND / 'p2_wide.rs', phase.path / 'p2_wide.rs')
shutil.copy2(kernel, phase.path / 'mounted_fold_row.rs')
phase.state.update(
    source_fingerprint_with_test_hook=source_hash,
    production_source_fingerprint=gated['source_fingerprint'],
    production_gate=args.gate,
    diagnostic_source_sha256=digest(ROUND / 'p2_wide.rs'),
    boost_sha256=digest(REPO / 'crates/bumbledb-bench/src/boost.rs'),
    clock_sha256=digest(REPO / 'crates/bumbledb-bench/src/clockproxy.rs'),
    protocol='Staged/taken-cache/in-place/production ABCD/DCBA; nine key/group widths and chunks of 1/128, '
             '8 samples x 32 complete reset+128-row executions per sample; independent expected answers.',
    limitations='Mechanism ablation shares the new row-fold helper, NOT the published binary. '
                'Staged control has independent preallocated row/outer storage, no take/restore, '
                'and shares candidate shape-refresh work. Taken/in-place controls have matched '
                'dispatch; production arm separately includes emit_batch dispatch. '
                'No full benchmark, profiler or trace. '
                'Keep all clock-flagged observations. '
                'macOS QoS is not hard P-core affinity; no Pi qualification.',
)
phase.save()
try:
    phase.run('source-patch', ['git', 'diff', '--binary', 'HEAD'])
    phase.run('build', ['cargo', 'test', '--locked', '--release', '-p', 'bumbledb',
                        '--lib', '--no-run', '--message-format=json'])
    executables = []
    for line in (phase.path / 'build.log').read_text().splitlines():
        if line.startswith('{'):
            event = json.loads(line)
            if event.get('reason') == 'compiler-artifact' and event.get('executable') \
               and event.get('target', {}).get('name') == 'bumbledb':
                executables.append(event['executable'])
    assert len(executables) == 1, executables
    binary = phase.path / 'bumbledb-wide-test'
    shutil.copy2(executables[0], binary)
    phase.state['binary_sha256'] = digest(binary)
    phase.save()
    assert fingerprint() == source_hash
    assert all(digest(path) == sha for path, sha in dependencies.items())
    phase.run('measure', [binary, 'wide_diagnostic::wide_borrowed_input_discriminator',
                          '--ignored', '--nocapture', '--test-threads=1'])
    log = (phase.path / 'measure.log').read_text()
    assert sum(line.startswith('ROUND ') for line in log.splitlines()) == 144
    assert 'test result: ok. 1 passed;' in log
    assert fingerprint() == source_hash
    assert all(digest(path) == sha for path, sha in dependencies.items())
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('MEASUREMENTS-COMPLETE-REVIEW-REQUIRED')
