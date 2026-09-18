"""Serial same-binary comparison of dependency-directed projection gates."""
from pathlib import Path
import argparse
import hashlib
import json
import random
from run import LAB, RESULTS, execute, check_engine_sources

parser = argparse.ArgumentParser()
parser.add_argument('--output', required=True)
parser.add_argument('--trials', type=int, default=3)
parser.add_argument('--seed', type=int, default=20260922)
parser.add_argument('--timeout', type=int, default=180)
parser.add_argument('--candidate', nargs='+', choices=[
    'prefix64', 'prefix512', 'retraction64', 'retraction512',
    'essential64', 'essential512', 'packed512', 'dense-dispatched'],
    default=['prefix512', 'retraction512', 'essential512', 'packed512', 'dense-dispatched'])
parser.add_argument('--storage', nargs='+', choices=['enum', 'slab'], default=['slab'])
parser.add_argument('--layout', nargs='+', choices=['bit-major', 'face-major', 'pair-major'], default=['bit-major', 'face-major'])
parser.add_argument('--domain', nargs='+', choices=['full', 'below', 'holes', 'fibred'], default=['full', 'below', 'holes', 'fibred'])
parser.add_argument('--family', nargs='+', choices=['suffix-1', 'suffix-half', 'whole', 'non-suffix'], default=['suffix-1', 'suffix-half', 'whole', 'non-suffix'])
parser.add_argument('--faces', nargs='+', choices=['x', 'y', 'xy'], default=['x', 'xy'])
parser.add_argument('--factor', nargs='+', choices=['joint', 'faces'], default=['faces'])
parser.add_argument('--normalize', nargs='+', choices=['staged', 'equal', 'ite'], default=['staged', 'ite'])
parser.add_argument('--gates',nargs='+',choices=['joint','active','witness'],default=['joint','active','witness'])
args = parser.parse_args()
assert args.trials > 0
check_engine_sources()
build = json.loads((RESULTS/'build.json').read_text())
binary = Path(build['binary'])
assert hashlib.sha256(binary.read_bytes()).hexdigest() == build['binary_sha256']
for name, digest in build['lab_sources'].items():
    assert hashlib.sha256((LAB/name).read_bytes()).hexdigest() == digest, name
folder = RESULTS/(Path(args.output).stem+'-logs')
folder.mkdir(exist_ok=False)
(folder/'build.json').write_text(json.dumps(build, indent=2)+'\n')
common = dict(EVENT_LAB_RETRACTION_COUNT='words', EVENT_LAB_RETRACTION_FACTOR='faces',
              EVENT_LAB_ESSENTIAL_KERNEL='derived', EVENT_LAB_OCCUPANCY_KERNEL='words',
              EVENT_LAB_PACKED_MAP='local', EVENT_LAB_TRANSFER_IMPORT='words',
              EVENT_LAB_IDENTITY='native', EVENT_LAB_PRODUCT='materialized')
runs = []


def save():
    record = dict(
        experiment='Same-binary joint/active/witness projection gates and conditional normalization; actual three-input Free Join and grouped union, followed by original/Possible/Guaranteed/Ambiguous Events; 64 outputs',
        build_metadata=build, trials=args.trials, seed=args.seed, timeout_s=args.timeout,
        runs=runs, checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
    (RESULTS/args.output).write_text(json.dumps(record, indent=2)+'\n')


for storage in args.storage:
    for mode in args.normalize:
        job = dict(common, verify=True, EVENT_LAB_ESSENTIAL_LAYOUT=storage, EVENT_LAB_RETRACTION_NORMALIZE=mode, EVENT_LAB_RETRACTION_PROJECT=args.gates[-1])
        result = execute(str(binary), job, len(runs), args.trials, args.timeout, folder)
        runs.append(result)
        save()
        if result['status'] != 'passed':
            raise SystemExit('Completion verification failed: '+result['log'])

jobs = []
for candidate in args.candidate:
    structured = candidate.startswith(('prefix', 'retraction', 'essential'))
    for storage in args.storage if structured else ['enum']:
        for layout in args.layout:
            for domain in args.domain:
                for family in args.family:
                    for faces in args.faces:
                        for factor in args.factor if candidate.startswith(('prefix', 'retraction')) else ['joint']:
                            for mode in args.normalize if candidate.startswith(('prefix', 'retraction')) else ['staged']:
                                for gates in args.gates if candidate.startswith(('prefix','retraction')) else ['joint']:
                                    jobs.append(dict(common, EVENT_LAB_LANE='readouts', EVENT_LAB_CANDIDATE=candidate,
                                                 EVENT_LAB_ESSENTIAL_LAYOUT=storage, EVENT_LAB_LAYOUT=layout,
                                                 EVENT_LAB_LEGAL_DOMAIN=domain, EVENT_LAB_READOUT_FAMILY=family,
                                                 EVENT_LAB_READOUT_FACES=faces, EVENT_LAB_RETRACTION_FACTOR=factor,
                                                 EVENT_LAB_RETRACTION_NORMALIZE=mode, EVENT_LAB_RETRACTION_PROJECT=gates))
random.Random(args.seed).shuffle(jobs)
for job in jobs:
    result = execute(str(binary), job, len(runs), args.trials, args.timeout, folder)
    runs.append(result)
    save()
    if result['status'] == 'failed':
        raise SystemExit('Readout query failed: '+result['log'])
print('Saved '+str(RESULTS/args.output), flush=True)
