"""Retain standalone Rust normal-form verification with exact source hashes."""
from pathlib import Path
import argparse, hashlib, json, os, subprocess
from prepare import LAB
ap = argparse.ArgumentParser()
ap.add_argument('--essential-kernel', choices=['words','scalar','derived'], default='words')
ap.add_argument('--essential-layout', choices=['enum','slab'], default='enum')
ap.add_argument('--output', default='essential-rust-check.json')
args = ap.parse_args()
folder = LAB/'essential-prototype'
files = [folder/'Cargo.toml', folder/'Cargo.lock', folder/'src/lib.rs', LAB/'src/essential_raw.rs', LAB/'src/essential_words.rs', LAB/'src/essential_store.rs']
def digests():
    return {str(p.relative_to(LAB)): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
before = digests()
command = ['cargo', 'test', '--offline', '--release']
env = dict(os.environ, EVENT_LAB_ESSENTIAL_KERNEL=args.essential_kernel, EVENT_LAB_ESSENTIAL_LAYOUT=args.essential_layout)
run = subprocess.run(command, cwd=folder, capture_output=True, text=True, env=env)
assert digests() == before, 'Sources changed during compilation or checking'
log = Path(args.output).stem+'.log'
(LAB/'results'/log).write_text(run.stdout + '\n' + run.stderr)
result = {'passed': run.returncode == 0, 'command': command, 'cwd': str(folder.relative_to(LAB)),
          'scope': 'Standalone raw Boolean canonical form; not a scoped Event carrier or a performance comparison.',
          'profile': 'standalone release; no native engine, no timing measurement',
          'essential_kernel': args.essential_kernel,
          'essential_layout': args.essential_layout,
          'rustc': subprocess.check_output(['rustc', '--version'], text=True).strip(),
          'source_hashes': before, 'log': log}
(LAB/'results'/args.output).write_text(json.dumps(result, indent=2) + '\n')
print(run.stdout)
print(json.dumps(result))
raise SystemExit(run.returncode)
