#!/usr/bin/env python3
"""Read-only candidate identities and E1 unmounted-source checks."""
import runpy
import shutil
from diagnostics import Phase, ROUND, load
from e1_experiment import TREE, fingerprint as e1
from m1_experiment import fingerprint as m1
from overlap_experiment import fingerprint as p3

phase = Phase('e1-closeout-1')
shutil.copy2(__file__, phase.path/'closeout.py')
expected = {
    'E1': 'e44e04132d30992ab094d61c93d620207175e1390006046e53733c16fb52cd55',
    'M1': '11340858461482ca80cea8c55ce83661d8ae794e2f193438241fc66f47cd50fd',
    'P3': '3f89f14dfea19b2ae5a58bc3244e8022b0ea20131a39e5e7b87786902163f7a6',
    'P2': '3bc43f3f04a09fa8983abeaf183eb54152fc8159a4a5b26fd73cbceb9f26958c',
}
def identities():
    return {'E1': e1(), 'M1': m1(), 'P3': p3(),
            'P2': runpy.run_path(str(ROUND/'p2-gate.py'))['fingerprint']()}
try:
    phase.state['identities'] = identities()
    assert phase.state['identities'] == expected
    assert load(ROUND/'e1-timing-1/STATE.json')['status'] == \
        'PER-DRAW-MEASUREMENTS-COMPLETE-REVIEW-REQUIRED'
    phase.save()
    phase.run('diff-check', ['git', '-C', TREE, 'diff', '--check'])
    phase.run('format', ['cargo', 'fmt', '--manifest-path', TREE/'Cargo.toml', '--all', '--check'])
    assert identities() == expected
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('IDENTITIES-AND-UNMOUNTED-SOURCE-VERIFIED')
