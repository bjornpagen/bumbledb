#!/usr/bin/env python3
"""Untimed route observer on an identical private copy of the measured store."""
import shutil
from diagnostics import Phase, REPO, ROUND, digest

phase = Phase('p1-route-1')
try:
    source = ROUND / 'p1-evaluation-2-resume-1/A3/reads/scratch/displaced/db'
    target = phase.path / 'db'
    original_hash = digest(source / 'data.mdb')
    shutil.copytree(source, target)
    assert digest(target / 'data.mdb') == original_hash
    phase.state.update(store_sha256=original_hash, original=str(source),
                       diagnostic_source_sha256=digest(ROUND / 'probe_route.rs'))
    phase.save()
    phase.run('route', ['env', 'BUMBLEDB_ROUTE_DB=' + str(target), 'cargo', 'test',
                       '--locked', '--release', '-p', 'bumbledb', '--lib',
                       'probe_route_diagnostic::saved_displaced_route', '--',
                       '--ignored', '--nocapture', '--test-threads=1'])
    assert digest(source / 'data.mdb') == original_hash
    assert digest(target / 'data.mdb') == original_hash
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('COMPLETE')
