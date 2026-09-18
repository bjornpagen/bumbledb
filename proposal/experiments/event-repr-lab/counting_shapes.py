"""Structural exact-count probe; builds separately and never reports latency."""
from pathlib import Path
import argparse, hashlib, json, random, subprocess
from prepare import LAB

ap=argparse.ArgumentParser()
ap.add_argument('--output',default='difference-counting-shapes.json')
args=ap.parse_args()
folder=LAB/'difference-prototype'
sources=[folder/'Cargo.toml',folder/'Cargo.lock',*sorted((folder/'src').rglob('*.rs'))]
def hashes():
    return {str(p.relative_to(LAB)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}
before=hashes()
command=['cargo','build','--offline','--release','--bin','constraint_counting']
build=subprocess.run(command,cwd=folder,capture_output=True,text=True)
buildlog=Path(args.output).stem+'-build.log'
(LAB/'results'/buildlog).write_text(build.stdout+'\n'+build.stderr)
assert build.returncode==0, build.stderr
metadata=json.loads(subprocess.check_output(['cargo','metadata','--offline','--no-deps','--format-version=1'],cwd=folder,text=True))
binary=Path(metadata['target_directory'])/'release/constraint_counting'
jobs=[[str(g),order,kernel] for g in [4,8,12,16,24] for order in ['forward','reverse'] for kernel in ['coefficients','cofactors']]
random.Random(20260918).shuffle(jobs)
jobs=[['small']]+jobs
logfolder=Path(args.output).stem+'-logs'
(LAB/'results'/logfolder).mkdir(exist_ok=False)
result=dict(source_hashes=before,checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    binary=str(binary),binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
    build_command=command,build_log=buildlog,runs=[],passed=False,
    boundary='Exact uniform counting on the full raw cube. This is not decoded support counting, arbitrary source-law contraction, a native Free Join run, or a latency comparison. Large cases use the proven constraint identity and independently evaluated four-input circuit as the scalar oracle.')
for i,job in enumerate(jobs):
    logfile=f'{logfolder}/{i:03d}-'+ '-'.join(job)+'.log'
    try:
        p=subprocess.run([str(binary),*job],capture_output=True,text=True,timeout=45)
        output=p.stdout+'\n'+p.stderr
        rows=[json.loads(s.split(' ',1)[1]) for s in p.stdout.splitlines() if s.startswith(('COUNTING_SMALL {','COUNTING_SHAPE {'))]
        status='passed' if p.returncode==0 and len(rows)==1 and rows[0]['passed'] else 'resource_cap' if 'RESOURCE_CAP:' in output else 'failed'
    except subprocess.TimeoutExpired as e:
        output=(e.stdout or b'').decode()+'\n'+(e.stderr or b'').decode()+'\nRESOURCE_CAP: 45-second screen limit\n'
        rows=[];status='resource_cap'
    phases=[json.loads(s.split(' ',1)[1]) for s in output.splitlines() if s.startswith('COUNTING_PHASE {')]
    (LAB/'results'/logfile).write_text(output)
    result['runs'].append(dict(job=job,status=status,rows=rows,phases=phases,log=logfile))
    (LAB/'results'/args.output).write_text(json.dumps(result,indent=2)+'\n')
    print(i,*job,status,flush=True)
assert hashes()==before, 'Source changed during build/screen'
result['passed']=all(r['status']=='passed' for r in result['runs'])
result['semantic_failures']=sum(r['status']=='failed' for r in result['runs'])
(LAB/'results'/args.output).write_text(json.dumps(result,indent=2)+'\n')
assert not result['semantic_failures']
