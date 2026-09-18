"""Matched raw structural screens; never a native Free Join timing claim."""
from pathlib import Path
import argparse,hashlib,itertools,json,random,subprocess
from prepare import LAB

ap=argparse.ArgumentParser();ap.add_argument('--output',default='unary-block-shapes.json');args=ap.parse_args()
folder=LAB/'unary-prototype'
files=[folder/'Cargo.toml',folder/'Cargo.lock',*sorted((folder/'src').rglob('*.rs'))]
def hashes():return {str(p.relative_to(LAB)):hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
before=hashes();command=['cargo','build','--offline','--release']
p=subprocess.run(command,cwd=folder,text=True,capture_output=True)
buildlog=Path(args.output).stem+'-build.log';(LAB/'results'/buildlog).write_text(p.stdout+'\n'+p.stderr)
assert p.returncode==0,p.stderr
meta=json.loads(subprocess.check_output(['cargo','metadata','--offline','--no-deps','--format-version=1'],cwd=folder,text=True))
binary=Path(meta['target_directory'])/'release/event-unary-prototype'
jobs=list(itertools.product(['composition','coup','bilinear'],['plain','chains'],['face-major','bit-major'],['keep','compact'],['steps','blocks']))
random.Random(20260918).shuffle(jobs)
logfolder=Path(args.output).stem+'-logs';(LAB/'results'/logfolder).mkdir(exist_ok=False)
result=dict(passed=False,source_hashes=before,binary=str(binary),binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
    checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),build_command=command,build_log=buildlog,runs=[],
    boundary='Matched structural counts on standalone raw managers. Input keep/compact controls and final copied-live-graph census pay label bytes; no timings, native Free Join, concurrent GC, common wire transport or designated-law contraction.')
for i,job in enumerate(jobs):
    logfile=f'{logfolder}/{i:03d}-'+'-'.join(job)+'.log'
    try:
        p=subprocess.run([str(binary),*job],capture_output=True,text=True,timeout=45)
        output=p.stdout+'\n'+p.stderr
        rows=[json.loads(s.split('UNARY_SHAPE ',1)[1]) for s in p.stdout.splitlines() if s.startswith('UNARY_SHAPE {')]
        status='passed' if p.returncode==0 and len(rows)==1 and rows[0]['passed'] else 'resource_cap' if 'RESOURCE_CAP:' in output else 'failed'
    except subprocess.TimeoutExpired as e:
        output=(e.stdout or b'').decode()+'\n'+(e.stderr or b'').decode()+'\nRESOURCE_CAP: 45-second screen limit\n'
        rows=[];status='resource_cap'
    phases=[json.loads(s.split('UNARY_PHASE ',1)[1]) for s in output.splitlines() if s.startswith('UNARY_PHASE {')]
    (LAB/'results'/logfile).write_text(output)
    result['runs'].append(dict(job=list(job),status=status,rows=rows,phases=phases,log=logfile))
    (LAB/'results'/args.output).write_text(json.dumps(result,indent=2)+'\n')
    print(i,*job,status,flush=True)
assert hashes()==before,'Sources changed during build/screen'
result['passed']=all(r['status']=='passed' for r in result['runs'])
result['semantic_failures']=sum(r['status']=='failed' for r in result['runs'])
(LAB/'results'/args.output).write_text(json.dumps(result,indent=2)+'\n')
assert not result['semantic_failures']
