"""Compile/check the borrowed occupancy prototype; retain exact source evidence."""
from pathlib import Path
import argparse, hashlib, json, os, subprocess
from prepare import LAB, SCRATCH

ap = argparse.ArgumentParser()
ap.add_argument('--essential-layout', choices=['enum','slab'], default='enum')
ap.add_argument('--occupancy-kernel', choices=['scalar','words','scratch'], default='scalar')
ap.add_argument('--output', default='classify-check.json')
args = ap.parse_args()
folder = LAB/'classify-prototype'
files = [folder/'Cargo.toml', folder/'src/lib.rs', LAB/'src/essential_raw.rs', LAB/'src/essential_words.rs', LAB/'src/occupancy.rs', LAB/'src/essential_store.rs', LAB/'src/occupancy_words.rs', LAB/'src/occupancy_scratch.rs', LAB/'src/view_product.rs']
def digests():
    return {str(p.relative_to(LAB)):hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
before = digests()
env = dict(os.environ, CARGO_TARGET_DIR=str(SCRATCH/'classify-target'), EVENT_LAB_ESSENTIAL_KERNEL='derived', EVENT_LAB_ESSENTIAL_LAYOUT=args.essential_layout, EVENT_LAB_OCCUPANCY_KERNEL=args.occupancy_kernel)
command = ['cargo','test','--release','--offline','--','--test-threads=1']
run = subprocess.run(command,cwd=folder,env=env,text=True,capture_output=True)
assert before == digests(), 'Sources changed during check'
log = Path(args.output).stem+'.log'
(LAB/'results'/log).write_text(run.stdout+'\n'+run.stderr)
result = dict(passed=run.returncode==0, command=command, profile='standalone release; correctness only, no native timing',
              source_hashes=before, essential_kernel='derived', essential_layout=args.essential_layout, occupancy_kernel=args.occupancy_kernel, log=log,
              scope='Read-only raw finite-binary classifier; Matched scalar/allocated-word/bounded-scratch classification, all small supported predicates and symmetries, scattered coordinates, pointwise axis insertion and pinned/broadcast views up to twelve local axes. Not a scoped/public API or native filter.')
if (folder/'Cargo.lock').exists():
    result['cargo_lock_sha256'] = hashlib.sha256((folder/'Cargo.lock').read_bytes()).hexdigest()
(LAB/'results'/args.output).write_text(json.dumps(result,indent=2)+'\n')
print(run.stdout[-4000:]);print(run.stderr[-2000:]);print(json.dumps(result))
raise SystemExit(run.returncode)
