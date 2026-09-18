"""Check inline unary chains, including post-collection operation closure."""
from pathlib import Path
import argparse,hashlib,json,subprocess
from prepare import LAB

ap=argparse.ArgumentParser();ap.add_argument('--output',default='unary-block-check.json');args=ap.parse_args()
folder=LAB/'unary-prototype'
files=[folder/'Cargo.toml',folder/'Cargo.lock',*sorted((folder/'src').rglob('*.rs'))]
def hashes():return {str(p.relative_to(LAB)):hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
before=hashes();command=['cargo','test','--offline','--release','--','--nocapture','--test-threads=1']
p=subprocess.run(command,cwd=folder,text=True,capture_output=True)
assert hashes()==before,'Sources changed during correctness checking'
log=Path(args.output).stem+'.log';(LAB/'results'/log).write_text(p.stdout+'\n'+p.stderr)
records=[json.loads(s.split('UNARY_CHECK ',1)[1]) for s in p.stdout.splitlines() if 'UNARY_CHECK {' in s]
chains=[json.loads(s.split('UNARY_CHAIN_CHECK ',1)[1]) for s in p.stdout.splitlines() if 'UNARY_CHAIN_CHECK {' in s]
long_shapes=[json.loads(s.split('UNARY_LONG_SHAPE ',1)[1]) for s in p.stdout.splitlines() if 'UNARY_LONG_SHAPE {' in s]
multi=[json.loads(s.split('UNARY_MULTI_CHECK ',1)[1]) for s in p.stdout.splitlines() if 'UNARY_MULTI_CHECK {' in s]
result=dict(passed=p.returncode==0 and len(records)==1 and len(chains)==1 and len(long_shapes)==8 and len(multi)==1,
    multi_records=multi,
    long_shapes=long_shapes,
    source_hashes=before,command=command,records=records,chain_records=chains,log=log,
    checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    boundary='Raw canonical managers, copied-live-graph compaction, full-cube uniform counting. No Event ownership/common packet/designated law or native Free Join claim.')
(LAB/'results'/args.output).write_text(json.dumps(result,indent=2)+'\n')
print(p.stdout[-2500:]);print(p.stderr[-2500:]);print(json.dumps(result))
raise SystemExit(0 if result['passed'] else 1)
