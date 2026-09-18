"""Fast shared semantic suite; native timing still requires the full engine build."""
from pathlib import Path
import argparse,hashlib,json,os,subprocess,time
from prepare import LAB,SCRATCH

ap=argparse.ArgumentParser();ap.add_argument('--release',action='store_true');ap.add_argument('--product',choices=['materialized','views','views-inputs'],default='materialized');ap.add_argument('--essential-kernel',choices=['words','scalar','derived'],default='words');ap.add_argument('--occupancy-kernel',choices=['scalar','words','scratch'],default='scalar');ap.add_argument('--essential-layout',choices=['enum','slab'],default='enum');ap.add_argument('--output',default='fast-check.json');ap.add_argument('--packed-map',choices=['local','recursive'],default='local');ap.add_argument('--transfer-import',choices=['words','direct','recursive'],default='words');ap.add_argument('--view-kernel',choices=['assignments','words'],default='assignments');ap.add_argument('--view-reuse',choices=['off','bounded'],default='off');ap.add_argument('--view-normalize',choices=['deferred','source'],default='deferred');ap.add_argument('--retraction-normalize',choices=['staged','equal'],default='staged');args=ap.parse_args()
folder=SCRATCH/'checker';(folder/'src').mkdir(parents=True,exist_ok=True)
(folder/'Cargo.toml').write_text('''[package]
name = "event-repr-checker"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
rustc-hash = "=2.1.0"
roaring = "=0.10.12"
num-bigint = "=0.4.8"
num-rational = "=0.4.2"
num-traits = "=0.2.19"
''')
names=['carrier','essential','essential_raw','diagram','diagonal_checks','finite','packed','relational_core','legal','legal_checks','semantic_checks','observation','observation_checks','dependencies','transfer','transfer_checks','occupancy','signature']
names += ['contraction', 'retraction', 'retraction_checks', 'factor_checks', 'readout']
if (LAB/'src/blocks.rs').exists(): names.append('blocks')
code='#![allow(dead_code)]\n'+''.join('#[path = '+json.dumps(str(LAB/'src'/(name+'.rs')))+']\nmod '+name+';\n' for name in names)+'\n#[test]\nfn shared_semantics() { semantic_checks::all(); }\n'
(folder/'src/lib.rs').write_text(code)
digests={str(p.relative_to(LAB)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((LAB/'src').glob('*.rs'))}
env=os.environ.copy();env['EVENT_LAB_PACKED_MAP']=args.packed_map
env['EVENT_LAB_TRANSFER_IMPORT']=args.transfer_import
env['EVENT_LAB_ESSENTIAL_KERNEL']=args.essential_kernel
env['EVENT_LAB_ESSENTIAL_LAYOUT']=args.essential_layout
env['EVENT_LAB_OCCUPANCY_KERNEL']=args.occupancy_kernel
env['EVENT_LAB_RETRACTION_COUNT']='words'
env['EVENT_LAB_RETRACTION_FACTOR']='faces'
env['EVENT_LAB_RETRACTION_NORMALIZE']=args.retraction_normalize
env['EVENT_LAB_PRODUCT']=args.product
env['EVENT_LAB_VIEW_KERNEL']=args.view_kernel
env['EVENT_LAB_VIEW_REUSE']=args.view_reuse
env['EVENT_LAB_VIEW_NORMALIZE']=args.view_normalize
start=time.time();p=subprocess.run(['cargo','test','--offline']+(['--release'] if args.release else [])+['shared_semantics','--','--exact','--nocapture','--test-threads=1'],cwd=folder,capture_output=True,text=True,env=env)
assert digests=={str(p.relative_to(LAB)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((LAB/'src').glob('*.rs'))},'Sources changed during check'
log=Path(args.output).stem+'.log';(LAB/'results'/log).write_text(p.stdout+'\n'+p.stderr)
result={'passed':p.returncode==0,'elapsed_s':time.time()-start,'profile':('release' if args.release else 'debug')+'; not a performance measurement','lab_sources':digests,'shared_modules':names,'packed_map':args.packed_map,'transfer_import':args.transfer_import,'essential_kernel':args.essential_kernel,'essential_layout':args.essential_layout,'occupancy_kernel':args.occupancy_kernel,'product_mode':args.product,'view_kernel':args.view_kernel,'view_reuse':args.view_reuse,'view_normalize':args.view_normalize,'log':log,'retraction_count':'words','retraction_factor':'faces','retraction_normalize':args.retraction_normalize}
(LAB/'results'/args.output).write_text(json.dumps(result,indent=2)+'\n')
print(p.stdout[-3500:]);print(p.stderr[-3500:]);print(json.dumps(result));raise SystemExit(p.returncode)
