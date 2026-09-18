"""Verify the combinator-parametric raw prototype; no native performance claims."""
from pathlib import Path
import argparse, hashlib, json, subprocess
from prepare import LAB

ap = argparse.ArgumentParser()
ap.add_argument('--output', default='difference-initial-check.json')
args = ap.parse_args()
folder = LAB/'difference-prototype'
files = [folder/'Cargo.toml', folder/'Cargo.lock', *sorted((folder/'src').rglob('*.rs'))]
def hashes(): return {str(p.relative_to(LAB)): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
before = hashes()
command = ['cargo', 'test', '--offline', '--release', '--', '--nocapture', '--test-threads=1']
p = subprocess.run(command, cwd=folder, text=True, capture_output=True)
assert hashes() == before, 'Rust sources changed during compilation or verification'
log = Path(args.output).stem+'.log'
(LAB/'results'/log).write_text(p.stdout+'\n'+p.stderr)
records = [json.loads(line.split('DIFFERENCE_CHECK ',1)[1]) for line in p.stdout.splitlines() if 'DIFFERENCE_CHECK {' in line]
count_records = [json.loads(line.split('DIFFERENCE_COUNT_CHECK ',1)[1]) for line in p.stdout.splitlines() if 'DIFFERENCE_COUNT_CHECK {' in line]
result = dict(passed=p.returncode == 0 and len(records)==1 and len(count_records)==1,
    count_records=count_records, source_hashes=before,
    command=command, records=records, log=log,
    checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    boundary='Raw fixed-basis diagrams and completed projection oracle; no production Event manager, common transport codec, source-law contraction or Free Join timing is established.')
(LAB/'results'/args.output).write_text(json.dumps(result,indent=2)+'\n')
print(p.stdout[-3000:]); print(p.stderr[-4000:]); print(json.dumps(result))
raise SystemExit(0 if result['passed'] else 1)
