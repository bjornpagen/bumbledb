#!/usr/bin/env python3
"""Verify restored G2 gate source and preservation of every other candidate."""
import argparse
import runpy
import shutil
from diagnostics import Phase, ROUND, load, digest
from g2_experiment import TREE, fingerprint as g2, git
from g1_experiment import fingerprint as g1
from e1_experiment import fingerprint as e1
from m1_experiment import fingerprint as m1
from overlap_experiment import fingerprint as p3

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
phase = Phase(f'g2-closeout-{args.attempt}')
shutil.copy2(__file__, phase.path/'closeout.py')
expected = dict(load(ROUND/'g1-closeout-2/STATE.json')['identities'])
expected['G2'] = load(ROUND/'g2-gates-3/STATE.json')['source_fingerprint']
def identities():
    return {'E1':e1(),'M1':m1(),'P3':p3(),'G1':g1(),'G2':g2(),
        'P2':runpy.run_path(str(ROUND/'p2-gate.py'))['fingerprint']()}
try:
    phase.state['identities'] = identities()
    assert phase.state['identities'] == expected
    force = 'crates/bumbledb/src/exec/colt/force.rs'
    assert (TREE/force).read_bytes() == git('show','HEAD:'+force)
    assert digest(TREE/'crates/bumbledb/src/exec/colt/grow.rs') == digest(ROUND/'g2-ordinary-grow.rs')
    phase.save()
    phase.run('diff-check',['git','-C',TREE,'diff','--check'])
    phase.run('format',['cargo','fmt','--manifest-path',TREE/'Cargo.toml','--all','--check'])
    assert identities() == expected
except BaseException:
    phase.finish('INCOMPLETE')
    raise
else:
    phase.finish('IDENTITIES-AND-UNMOUNTED-SOURCE-VERIFIED')
