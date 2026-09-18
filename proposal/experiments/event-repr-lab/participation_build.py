"""Build a separately owned engine copy; preserve the frozen native laboratory."""
from pathlib import Path
import argparse,datetime,hashlib,json,os,platform,shutil,subprocess
from prepare import LAB,ROOT,SCRATCH
from run import check_engine_sources

ap=argparse.ArgumentParser();ap.add_argument('--output',default='build-participation.json');args=ap.parse_args()
check_engine_sources()
engine=SCRATCH/'participation-engine';engine.mkdir(exist_ok=True)
names=['bumbledb','bumbledb-theory','bumbledb-macros','bumbledb-query-macros']
for name in names:
    src=ROOT/'crates'/name;dst=engine/'crates'/name
    if not dst.exists():shutil.copytree(src,dst)
manifest=(ROOT/'Cargo.toml').read_text();start=manifest.index('members = [');end=manifest.index('\n]',start)+2
manifest=manifest[:start]+'members = ['+','.join('"crates/'+n+'"' for n in names)+']'+manifest[end:]
(engine/'Cargo.toml').write_text(manifest)
for name in ['Cargo.lock','rust-toolchain.toml']:shutil.copy2(ROOT/name,engine/name)
lib=engine/'crates/bumbledb/src/lib.rs'
lib.write_text((ROOT/'crates/bumbledb/src/lib.rs').read_text()+'\n#[cfg(test)]\n#[path = '+json.dumps(str(LAB/'participation-src/lib.rs'))+']\nmod event_repr_lab;\n')
cargo=engine/'crates/bumbledb/Cargo.toml'
cargo.write_text((ROOT/'crates/bumbledb/Cargo.toml').read_text().replace('[dev-dependencies]', '[dev-dependencies]\nroaring = "=0.10.12"\nrustc-hash = "=2.1.0"\nnum-bigint = "=0.4.8"\nnum-rational = "=0.4.2"\nnum-traits = "=0.2.19"'))
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def sources():return {str(p.relative_to(LAB)):digest(p) for p in sorted((LAB/'participation-src').glob('*.rs'))}
before=sources();engine_before={str(p.relative_to(engine)):digest(p) for p in sorted((engine/'crates').rglob('*')) if p.is_file()}
cmd=['cargo','test','--release','--offline','-p','bumbledb','--lib','--no-run','--message-format=json']
start=datetime.datetime.now(datetime.timezone.utc).isoformat()
print('Building isolated participation comparison.',flush=True)
p=subprocess.run(cmd,cwd=engine,text=True,capture_output=True)
log=Path(args.output).stem+'.log';(LAB/'results'/log).write_text(p.stderr+'\n'+p.stdout)
assert sources()==before,'Rust changed during native build'
assert engine_before=={str(f.relative_to(engine)):digest(f) for f in sorted((engine/'crates').rglob('*')) if f.is_file()},'Engine changed during native build'
check_engine_sources()
if p.returncode:
    print(p.stderr)
    for line in p.stdout.splitlines():
        try:r=json.loads(line)
        except json.JSONDecodeError:continue
        if r.get('message',{}).get('rendered'):print(r['message']['rendered'])
    raise SystemExit(p.returncode)
binary=None
for line in p.stdout.splitlines():
    try:r=json.loads(line)
    except json.JSONDecodeError:continue
    if r.get('executable') and r.get('target',{}).get('name')=='bumbledb':binary=r['executable']
assert binary
out=dict(binary=binary,binary_sha256=digest(Path(binary)),source_sha256=before,
    engine_source_sha256=engine_before,production_manifest=json.loads((LAB/'results/source-manifest.json').read_text()),
    rustc=subprocess.check_output(['rustc','--version','--verbose'],cwd=ROOT,text=True),
    command=cmd,started_utc=start,finished_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),
    machine=platform.machine(),platform=platform.platform(),log=log,builder_sha256=digest(Path(__file__)),
    boundary='Same-binary complete, staged and participation-index relational schedules. Immutable original carriers and source snapshots. Timing requires separate serial runs.')
(LAB/'results'/args.output).write_text(json.dumps(out,indent=2)+'\n')
print('Native pack comparison built.',flush=True)
