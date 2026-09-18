"""Compare retained graph/work counts; this is not a Free Join benchmark."""
from pathlib import Path
import hashlib, json, random, subprocess
from prepare import LAB

folder=LAB/'difference-prototype'
meta=json.loads(subprocess.check_output(['cargo','metadata','--offline','--no-deps','--format-version=1'],cwd=folder,text=True))
binary=Path(meta['target_directory'])/'release/event-difference-prototype'
assert binary.is_file(), 'Compile the structural probe before running it'
sources=[folder/'Cargo.toml',folder/'Cargo.lock',*sorted((folder/'src').glob('*.rs'))]
hashes={str(p.relative_to(LAB)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}
jobs=[(shape,basis,layout) for shape in ['composition','coup']
      for basis in ['shannon','positive','negative','mixed'] for layout in ['face-major','bit-major']]
random.Random(20260918).shuffle(jobs)
logs=LAB/'results/difference-shapes-logs';logs.mkdir(exist_ok=False)
result=dict(passed=False,binary=str(binary),binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
    source_hashes=hashes,checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),runs=[],
    boundary='Operation/graph census in a standalone raw manager. Coup explicitly replays the clover binding formula; no native Free Join, normalized wire transport, law contraction or timings are claimed. Bytes estimate retained capacities and exclude allocator/temporary/oracle memory.')
for i,job in enumerate(jobs):
    logfile=f'difference-shapes-logs/{i:03d}-'+ '-'.join(job)+'.log'
    try:
        p=subprocess.run([str(binary),*job],capture_output=True,text=True,timeout=120)
        output=p.stdout+'\n'+p.stderr
        rows=[json.loads(s.split('DIFFERENCE_SHAPE ',1)[1]) for s in p.stdout.splitlines() if 'DIFFERENCE_SHAPE {' in s]
        status='passed' if p.returncode==0 and len(rows)==1 and rows[0]['passed'] else 'failed'
    except subprocess.TimeoutExpired as e:
        output=str(e);rows=[];status='resource_cap'
    (LAB/'results'/logfile).write_text(output)
    result['runs'].append(dict(job=job,status=status,rows=rows,log=logfile))
    (LAB/'results/difference-shapes.json').write_text(json.dumps(result,indent=2)+'\n')
    print(i,*job,status,flush=True)
assert hashes=={str(p.relative_to(LAB)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}
answers={}
for run in result['runs']:
    if run['status']!='passed': continue
    r=run['rows'][0];key=r['shape']
    assert answers.setdefault(key,r['checksum'])==r['checksum']
result['passed']=all(r['status']=='passed' for r in result['runs'])
(LAB/'results/difference-shapes.json').write_text(json.dumps(result,indent=2)+'\n')
