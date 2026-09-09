#!/usr/bin/env python3
"""Retry only the allocation census with an explicitly recorded fixture patch."""

import signal
import subprocess

from diagnostics import Phase, REPO, ROUND, SOURCE, census, digest, load

FIXTURE = 'crates/bumbledb/tests/alloc_census.rs'


def check_source():
    def git(*args):
        return subprocess.check_output(['git', *args], cwd=REPO, text=True).strip()

    assert git('rev-parse', 'HEAD') == SOURCE
    assert git('status', '--porcelain') == f'M {FIXTURE}', 'unexpected source change'
    assert git('diff', '--name-only', 'HEAD') == FIXTURE, 'engine must remain baseline'
    assert load(ROUND / 'allocations/STATE.json')['status'] == 'COMPLETE'
    manifest = load(ROUND / 'full/MANIFEST.json')
    assert manifest['status'] == 'LOCAL-LANES-COMPLETE'
    assert manifest['source_revision'] == SOURCE
    assert manifest['binary_sha256'] == digest(ROUND / 'release/bumbledb-bench')
    return digest(REPO / FIXTURE)


def main():
    fixture_hash = check_source()
    phase = Phase('census-retry-1')
    phase.state.update(
        engine_source=SOURCE,
        fixture_sha256=fixture_hash,
        fixture_patch='source-patch.log',
        retry_driver_sha256=digest(__file__),
        preceding_failed_attempt=str(ROUND / 'census'),
        source_note='Engine unchanged; test-only schema repair and non-ignored fixture smoke test.',
    )
    phase.save()
    try:
        phase.run('source-patch', ['git', 'diff', '--binary', 'HEAD', '--', FIXTURE])
        phase.run('fixture-smoke', ['cargo', 'test', '--locked', '-p', 'bumbledb',
                                   '--test', 'alloc_census', 'census_fixture_smoke',
                                   '--', '--exact', '--test-threads=1'])
        census(phase)
        assert check_source() == fixture_hash, 'fixture changed during measurement'
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
