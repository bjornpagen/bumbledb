"""Optimized raw mapped-product checks, with exact source provenance."""
from pathlib import Path
import argparse, hashlib, json, os, subprocess
from prepare import LAB, SCRATCH

ap=argparse.ArgumentParser()
ap.add_argument('--layout',choices=['enum','slab'],default='enum')
ap.add_argument('--output',required=True)
ap.add_argument('--view-kernel',choices=['assignments','words'],default='words')
args=ap.parse_args()
folder=LAB/'essential-prototype'
files=[folder/'Cargo.toml',folder/'src/lib.rs']+[LAB/'src'/n for n in
    ['essential_raw.rs','essential_store.rs','essential_words.rs','view_product.rs']]
def hashes(): return {str(p.relative_to(LAB)):hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
before=hashes()
env=dict(os.environ,CARGO_TARGET_DIR=str(SCRATCH/'view-target'),
    EVENT_LAB_VIEW_KERNEL=args.view_kernel,EVENT_LAB_ESSENTIAL_KERNEL='derived',EVENT_LAB_ESSENTIAL_LAYOUT=args.layout)
command=['cargo','test','--release','--offline','--','--test-threads=1']
run=subprocess.run(command,cwd=folder,env=env,text=True,capture_output=True)
assert before==hashes(),'Sources changed during checking'
log=Path(args.output).stem+'.log'
(LAB/'results'/log).write_text(run.stdout+'\n'+run.stderr)
result=dict(passed=run.returncode==0,source_hashes=before,command=command,log=log,
    essential_layout=args.layout,essential_kernel='derived',view_kernel=args.view_kernel,
    cargo_lock_sha256=hashlib.sha256((folder/'Cargo.lock').read_bytes()).hexdigest(),
    checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    scope='Raw canonical mapped product: exhaustive two-axis maps/functions/eliminations, pending branch pins, 62-coordinate maps, multiword tables, complements, invalid maps and exact materialized identity. No native timings.')
(LAB/'results'/args.output).write_text(json.dumps(result,indent=2)+'\n')
print(run.stdout[-3500:]);print(run.stderr[-2500:])
raise SystemExit(run.returncode)
