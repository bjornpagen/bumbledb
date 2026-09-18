"""Matched same-binary canonical carriers and essential-table kernels, serially."""
from pathlib import Path
import argparse, hashlib, json, random
from run import LAB, RESULTS, CANDIDATES, execute, check_engine_sources

ap = argparse.ArgumentParser()
ap.add_argument('--output', required=True)
ap.add_argument('--trials', type=int, default=5)
ap.add_argument('--timeout', type=int, default=120)
ap.add_argument('--seed', type=int, default=20260923)
ap.add_argument('--candidate', nargs='+', choices=CANDIDATES,
                default=['essential64','essential512','packed64','packed512','block64','dense-dispatched'])
ap.add_argument('--lane', nargs='+', choices=['owned','join','laws','symbolic-relations','relations'],
                default=['owned','join','laws','symbolic-relations'])
ap.add_argument('--scenario', nargs='+', default=['coup_4290'],
                choices=['coup_4290','coup_product_65536','structured_65536','sparse_65536','runs_65536'])
ap.add_argument('--essential-kernel', nargs='+', choices=['words','scalar','derived'], default=['words','scalar'])
args = ap.parse_args()
check_engine_sources()
metadata = json.loads((RESULTS/'build.json').read_text())
binary = Path(metadata['binary'])
assert hashlib.sha256(binary.read_bytes()).hexdigest() == metadata['binary_sha256']
for name, digest in metadata['lab_sources'].items():
    assert hashlib.sha256((LAB/name).read_bytes()).hexdigest() == digest, name
folder = RESULTS/(Path(args.output).stem+'-logs')
folder.mkdir(exist_ok=False)
(folder/'build.json').write_text(json.dumps(metadata, indent=2)+'\n')
records = []
def save():
    (RESULTS/args.output).write_text(json.dumps({
        'experiment':'Essential-coordinate tables; same-binary algorithm paths and canonical-carrier controls',
        'essential_kernel_modes':args.essential_kernel,
        'seed':args.seed,'trials':args.trials,'timeout_s':args.timeout,
        'build_metadata':metadata,'runs':records,
    },indent=2)+'\n')
common = {'EVENT_LAB_PACKED_MAP':'local','EVENT_LAB_IDENTITY':'native',
          'EVENT_LAB_TRANSFER_IMPORT':'words','EVENT_LAB_LAYOUT':'bit-major'}
for kernel in args.essential_kernel:
    record = execute(str(binary),dict(common,verify=True,EVENT_LAB_ESSENTIAL_KERNEL=kernel),
                     len(records),args.trials,args.timeout,folder)
    records.append(record); save()
    if record['status'] != 'passed': raise SystemExit('Verification failed: '+record['log'])
jobs = []
for candidate in args.candidate:
    for lane in args.lane:
        if lane == 'symbolic-relations' and candidate in ['dense','dense-dispatched','sparse','roaring','runs']: continue
        for kernel in args.essential_kernel if candidate.startswith('essential') else ['words']:
            for scenario in args.scenario if lane == 'join' else ['']:
                job = dict(common,EVENT_LAB_CANDIDATE=candidate,EVENT_LAB_LANE=lane,EVENT_LAB_ESSENTIAL_KERNEL=kernel)
                if scenario: job['EVENT_LAB_SCENARIO']=scenario
                jobs.append(job)
random.Random(args.seed).shuffle(jobs)
for job in jobs:
    record = execute(str(binary),job,len(records),args.trials,args.timeout,folder)
    records.append(record); save()
    if record['status'] == 'failed': raise SystemExit('Correctness failed: '+record['log'])
print('Saved '+str(RESULTS/args.output),flush=True)
