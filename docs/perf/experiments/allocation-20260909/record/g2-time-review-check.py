#!/usr/bin/env python3
"""Preserve the full ordinary G2 raw-sample and clock review."""
import argparse
import shutil
import sys
from diagnostics import Phase, ROUND, digest

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
phase = Phase(f'g2-timing-review-{args.attempt}')
reviewer = ROUND/'g2-timing-review.py'
shutil.copy2(reviewer, phase.path/'review.py')
shutil.copy2(__file__, phase.path/'check.py')
phase.state['reviewer_sha256'] = digest(reviewer)
phase.save()
try:
    phase.run('review', [sys.executable, reviewer, str(args.attempt)])
    log = (phase.path/'review.log').read_text()
    assert 'PASS: all 128 distributions recomputed from raw samples; 96 clock brackets reviewed; no samples dropped.' in log
    assert digest(reviewer) == phase.state['reviewer_sha256']
    phase.state['review_log_sha256'] = digest(phase.path/'review.log')
    phase.save()
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('ALL-DISTRIBUTIONS-AND-CONTROLS-RECOMPUTED')
