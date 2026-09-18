"""Same-binary relationship classification in retained owners through Free Join."""
from pathlib import Path
import argparse, hashlib, json, random
from run import LAB, RESULTS, CANDIDATES, DIRECT_SIGNATURE, execute, check_engine_sources

ap = argparse.ArgumentParser()
ap.add_argument('--output',required=True)
ap.add_argument('--trials',type=int,default=5)
ap.add_argument('--timeout',type=int,default=120)
ap.add_argument('--seed',type=int,default=20260925)
ap.add_argument('--candidate',nargs='+',choices=CANDIDATES,
                default=['essential64','essential512','packed64','packed512','dense-dispatched','bdd-anchored'])
ap.add_argument('--scenario',nargs='+',default=['coup_4290','coup_product_65536'],
                choices=['coup_4290','coup_product_65536','structured_65536'])
ap.add_argument('--classifier',nargs='+',choices=['cells','direct'],default=['cells','direct'])
args = ap.parse_args()
check_engine_sources()
metadata = json.loads((RESULTS/'build.json').read_text())
binary = Path(metadata['binary'])
assert hashlib.sha256(binary.read_bytes()).hexdigest() == metadata['binary_sha256']
for name,digest in metadata['lab_sources'].items():
    assert hashlib.sha256((LAB/name).read_bytes()).hexdigest() == digest,name
folder = RESULTS/(Path(args.output).stem+'-logs')
folder.mkdir(exist_ok=False)
(folder/'build.json').write_text(json.dumps(metadata,indent=2)+'\n')
records = []
def save():
    (RESULTS/args.output).write_text(json.dumps(dict(
        experiment='Scoped direct classification versus four Venn cells; same native binary, serial jobs',
        seed=args.seed,trials=args.trials,timeout_s=args.timeout,build_metadata=metadata,
        direct_capabilities=DIRECT_SIGNATURE,runs=records),indent=2)+'\n')
common = dict(EVENT_LAB_ESSENTIAL_KERNEL='derived',EVENT_LAB_PACKED_MAP='local',
              EVENT_LAB_TRANSFER_IMPORT='words',EVENT_LAB_IDENTITY='native')
record = execute(str(binary),dict(common,verify=True),0,args.trials,args.timeout,folder)
records.append(record); save()
if record['status'] != 'passed': raise SystemExit('Native verification failed: '+record['log'])
jobs = [dict(common,EVENT_LAB_LANE='classify',EVENT_LAB_CANDIDATE=candidate,
             EVENT_LAB_SCENARIO=scenario,EVENT_LAB_CLASSIFY=mode)
        for candidate in args.candidate for scenario in args.scenario for mode in args.classifier
        if mode != 'direct' or candidate in DIRECT_SIGNATURE]
random.Random(args.seed).shuffle(jobs)
for job in jobs:
    record = execute(str(binary),job,len(records),args.trials,args.timeout,folder)
    records.append(record); save()
    if record['status']=='failed': raise SystemExit('Correctness failure: '+record['log'])
print('Saved '+str(RESULTS/args.output),flush=True)
