"""Serial, isolated runs: actual Free Join, raw samples, fixed job shuffling."""
from pathlib import Path
import argparse, datetime, hashlib, json, os, platform, random, re, signal, subprocess, time
from prepare import LAB, ROOT, SCRATCH, prepare

CANDIDATES=['prefix64','prefix512','retraction64','retraction512','essential64','essential512','block64','packed512','packed4096','packed64','packed256','dense','dense-dispatched','sparse','roaring','runs','bdd-root','bdd-pair','mdd4-pair','bdd-anchored']
DIRECT_SIGNATURE=['prefix64','prefix512','retraction64','retraction512','essential64','essential512','packed64','packed256','packed512','packed4096','dense','dense-dispatched']
SCENARIOS=['random_4096','overlap_4096','structured_65536','sparse_65536','runs_65536','coup_4290','coup_product_65536','ordered_65536','axes_65536']
RESULTS=LAB/'results'

def check_engine_sources():
    source=json.loads((RESULTS/'source-manifest.json').read_text())
    for path,digest in source['source_sha256'].items():
        if hashlib.sha256((ROOT/path).read_bytes()).hexdigest()!=digest:
            raise RuntimeError('Engine source changed; explicitly refresh with prepare.py: '+path)

def build():
    engine=SCRATCH/'engine'
    if not engine.exists(): prepare()
    # A snapshot is intentional. Refuse to quietly compare against changing engine code.
    check_engine_sources()
    # Detect edits during compilation instead of associating an old binary with
    # newer sources. The running process owns this exact source revision.
    source_hashes={str(p.relative_to(LAB)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((LAB/'src').glob('*.rs'))}
    started_utc=datetime.datetime.now(datetime.timezone.utc).isoformat()
    cmd=['cargo','test','--release','--offline','-p','bumbledb','--lib','--no-run','--message-format=json']
    p=subprocess.run(cmd,cwd=engine,text=True,capture_output=True)
    (RESULTS/'build.log').write_text(p.stderr+'\n'+p.stdout)
    if p.returncode:
        print(p.stderr,flush=True)
        for line in p.stdout.splitlines():
            try: row=json.loads(line)
            except json.JSONDecodeError: continue
            message=row.get('message',{})
            if message.get('rendered'): print(message['rendered'],flush=True)
        raise SystemExit(p.returncode)
    binary=None
    for line in p.stdout.splitlines():
        try: row=json.loads(line)
        except json.JSONDecodeError: continue
        if row.get('executable') and row.get('target',{}).get('name')=='bumbledb': binary=row['executable']
    assert binary
    after_hashes={str(p.relative_to(LAB)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((LAB/'src').glob('*.rs'))}
    if source_hashes != after_hashes:
        raise RuntimeError('Rust sources changed during compilation; refusing to publish build metadata. Rebuild.')
    metadata={
        'binary':binary,'binary_sha256':hashlib.sha256(Path(binary).read_bytes()).hexdigest(),
        'rustc':subprocess.check_output(['rustc','--version','--verbose'],cwd=ROOT,text=True),
        'platform':platform.platform(),'machine':platform.machine(),
        'cpu':subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),
        'memory_bytes':int(subprocess.check_output(['sysctl','-n','hw.memsize'],text=True)),
        'profile':'release, opt-level=3, fat LTO, codegen-units=1, no allocation-counter instrumentation',
        'lab_sources':source_hashes,
        'cargo_lock_sha256':hashlib.sha256((engine/'Cargo.lock').read_bytes()).hexdigest(),
        'built_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'build_started_utc':started_utc,
    }
    (RESULTS/'build.json').write_text(json.dumps(metadata,indent=2)+'\n')
    print('Built isolated native benchmark.',flush=True)
    return binary

def execute(binary,job,index,trials,timeout,log_dir):
    env=os.environ.copy();env.update({key:str(value) for key,value in job.items() if key.startswith('EVENT_LAB_')})
    env['EVENT_LAB_TRIALS']=str(trials)
    # Legacy sweep callers also get the recorded default, never an ambient override.
    env['EVENT_LAB_OCCUPANCY_KERNEL']=str(job.get('EVENT_LAB_OCCUPANCY_KERNEL','scalar'))
    env['EVENT_LAB_ESSENTIAL_LAYOUT']=str(job.get('EVENT_LAB_ESSENTIAL_LAYOUT','enum'))
    env['EVENT_LAB_PRODUCT']=str(job.get('EVENT_LAB_PRODUCT','materialized'))
    env['EVENT_LAB_VIEW_KERNEL']=str(job.get('EVENT_LAB_VIEW_KERNEL','assignments'))
    env['EVENT_LAB_VIEW_REUSE']=str(job.get('EVENT_LAB_VIEW_REUSE','off'))
    env['EVENT_LAB_VIEW_NORMALIZE']=str(job.get('EVENT_LAB_VIEW_NORMALIZE','deferred'))
    env['EVENT_LAB_SCOPED_SUPPORT']=str(job.get('EVENT_LAB_SCOPED_SUPPORT','legal-product'))
    env['EVENT_LAB_LEGAL_DOMAIN']=str(job.get('EVENT_LAB_LEGAL_DOMAIN','below'))
    env['EVENT_LAB_SUPPORT_GATES']=str(job.get('EVENT_LAB_SUPPORT_GATES','gated'))
    env['EVENT_LAB_RETRACTION_COUNT']=str(job.get('EVENT_LAB_RETRACTION_COUNT','words'))
    env['EVENT_LAB_RETRACTION_FACTOR']=str(job.get('EVENT_LAB_RETRACTION_FACTOR','joint'))
    env['EVENT_LAB_RETRACTION_NORMALIZE']=str(job.get('EVENT_LAB_RETRACTION_NORMALIZE','staged'))
    env['EVENT_LAB_REACHABILITY']=str(job.get('EVENT_LAB_REACHABILITY','0'))
    env['EVENT_LAB_READOUT_FAMILY']=str(job.get('EVENT_LAB_READOUT_FAMILY','suffix-1'))
    env['EVENT_LAB_READOUT_FACES']=str(job.get('EVENT_LAB_READOUT_FACES','xy'))
    verify=job.get('verify',False)
    cmd=['/usr/bin/time','-l',binary,'event_repr_lab::semantics' if verify else 'event_repr_lab::experiment','--exact','--nocapture','--test-threads=1']
    if not verify: cmd.append('--ignored')
    load_start=os.getloadavg()
    started=time.time()
    p=subprocess.Popen(cmd,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
    try:
        stdout,stderr=p.communicate(timeout=timeout)
        status='passed' if p.returncode==0 else ('resource_cap' if 'RESOURCE_CAP' in stdout+stderr else 'failed')
    except subprocess.TimeoutExpired:
        # Own only this job's process group; do not leave the timed child running.
        try: os.killpg(p.pid,signal.SIGKILL)
        except ProcessLookupError: pass
        stdout,stderr=p.communicate()
        status='timeout'
    label='verify' if verify else '-'.join(str(job.get(k,'')) for k in ['EVENT_LAB_LANE','EVENT_LAB_CANDIDATE','EVENT_LAB_SCENARIO','EVENT_LAB_PAIRS','EVENT_LAB_ORDER','EVENT_LAB_LAYOUT']).strip('-')
    if verify or job.get('EVENT_LAB_CANDIDATE','').startswith('packed'):
        label+='-'+job.get('EVENT_LAB_PACKED_MAP','local')
    if 'EVENT_LAB_TRANSFER_IMPORT' in job:
        label+='-'+job['EVENT_LAB_TRANSFER_IMPORT']
    if 'EVENT_LAB_TRANSFER_SETUP' in job:
        label+='-'+job['EVENT_LAB_TRANSFER_SETUP']
    label+='-identity-'+job.get('EVENT_LAB_IDENTITY','native')
    if 'EVENT_LAB_CLASSIFY' in job: label+='-classify-'+job['EVENT_LAB_CLASSIFY']
    if verify or job.get('EVENT_LAB_CANDIDATE','').startswith(('essential','retraction','prefix')): label+='-essential-'+job.get('EVENT_LAB_ESSENTIAL_KERNEL','words')+'-'+job.get('EVENT_LAB_ESSENTIAL_LAYOUT','enum')
    if verify or job.get('EVENT_LAB_CANDIDATE','').startswith(('essential','retraction','prefix')): label+='-occupancy-'+job.get('EVENT_LAB_OCCUPANCY_KERNEL','scalar')
    if verify or 'EVENT_LAB_PRODUCT' in job: label+='-product-'+job.get('EVENT_LAB_PRODUCT','materialized')
    if 'EVENT_LAB_VIEW_KERNEL' in job: label+='-view-kernel-'+job['EVENT_LAB_VIEW_KERNEL']
    if 'EVENT_LAB_VIEW_REUSE' in job: label+='-reuse-'+job['EVENT_LAB_VIEW_REUSE']
    if 'EVENT_LAB_VIEW_NORMALIZE' in job: label+='-normalize-'+job['EVENT_LAB_VIEW_NORMALIZE']
    if 'EVENT_LAB_SCOPED_SUPPORT' in job: label+='-support-'+job['EVENT_LAB_SCOPED_SUPPORT']
    if 'EVENT_LAB_LEGAL_DOMAIN' in job: label+='-domain-'+job['EVENT_LAB_LEGAL_DOMAIN']
    if 'EVENT_LAB_SUPPORT_GATES' in job: label+='-gates-'+job['EVENT_LAB_SUPPORT_GATES']
    if 'EVENT_LAB_READOUT_FAMILY' in job: label+='-readout-'+job['EVENT_LAB_READOUT_FAMILY']
    if 'EVENT_LAB_READOUT_FACES' in job: label+='-faces-'+job['EVENT_LAB_READOUT_FACES']
    if 'EVENT_LAB_RETRACTION_NORMALIZE' in job: label+='-normalize-'+job['EVENT_LAB_RETRACTION_NORMALIZE']
    log=log_dir/f'job-{index:03}-{label}.log';log.write_text(stdout+'\n'+stderr)
    match=re.search(r'(\d+)\s+maximum resident set size',stderr)
    record={'job':job,'status':status,'elapsed_s':time.time()-started,'max_rss_bytes':int(match.group(1)) if match else None,'log':str(log.relative_to(RESULTS)),'rows':[],'loadavg_start':load_start,'loadavg_end':os.getloadavg()}
    for line in stdout.splitlines():
        if 'EVENT_LAB ' in line:
            row=json.loads(line.split('EVENT_LAB ',1)[1])
            if row.get('kind') in ['modal_free_join','relations_free_join','product_free_join','scoped_product_free_join','legal_relation_free_join','readout_free_join','law_free_join','transfer_free_join','owned_free_join','elimination']:
                row['layout']=job.get('EVENT_LAB_LAYOUT','face-major')
            if row.get('candidate','').startswith('packed'):
                row['packed_map']=job.get('EVENT_LAB_PACKED_MAP','local')
            if row.get('kind') == 'transfer_free_join':
                row['transfer_import']=job.get('EVENT_LAB_TRANSFER_IMPORT','recursive')
                row['transfer_setup']=job.get('EVENT_LAB_TRANSFER_SETUP','fixture')
            if row.get('kind') == 'owned_free_join':
                row['transfer_import']=job.get('EVENT_LAB_TRANSFER_IMPORT','words')
                row['transfer_setup']='product'
            row['occupancy_kernel']=job.get('EVENT_LAB_OCCUPANCY_KERNEL','scalar')
            row['retraction_count']=job.get('EVENT_LAB_RETRACTION_COUNT','words')
            row['retraction_factor']=job.get('EVENT_LAB_RETRACTION_FACTOR','joint')
            row['retraction_normalize']=job.get('EVENT_LAB_RETRACTION_NORMALIZE','staged')
            row['essential_layout']=job.get('EVENT_LAB_ESSENTIAL_LAYOUT','enum')
            for name in ['input_storage','final_storage']:
                storage=row.get(name)
                if storage is None: continue
                assert storage['layout']==row['essential_layout']
                assert storage['total_bytes_est']==sum(storage[k] for k in ['record_bytes','table_bytes','interner_bytes','metadata_bytes','cache_bytes'])
            row['essential_kernel']=job.get('EVENT_LAB_ESSENTIAL_KERNEL','words')
            row['identity_mode']=job.get('EVENT_LAB_IDENTITY','native')
            row['view_kernel']=job.get('EVENT_LAB_VIEW_KERNEL','assignments')
            row['view_reuse']=job.get('EVENT_LAB_VIEW_REUSE','off')
            row['view_normalize']=job.get('EVENT_LAB_VIEW_NORMALIZE','deferred')
            row['requested_product_mode']=job.get('EVENT_LAB_PRODUCT','materialized')
            if row.get('kind') in ['relations_free_join','product_free_join','scoped_product_free_join','legal_relation_free_join']:
                expected_mode=job.get('EVENT_LAB_PRODUCT','materialized') if row['candidate'].startswith(('essential','retraction','prefix')) else 'materialized'
                assert row['product_mode']==expected_mode
            if row.get('kind') == 'legal_relation_free_join':
                assert row['domain']==job.get('EVENT_LAB_LEGAL_DOMAIN','below')
                expected_gates=job.get('EVENT_LAB_SUPPORT_GATES','gated') if row['product_mode']!='materialized' else 'materialized'
                assert row['support_gates']==expected_gates
            if row.get('kind') == 'readout_free_join':
                assert row['domain']==job.get('EVENT_LAB_LEGAL_DOMAIN','below')
                assert row['readout_family']==job.get('EVENT_LAB_READOUT_FAMILY','suffix-1')
                assert row['readout_faces']==job.get('EVENT_LAB_READOUT_FACES','xy')
            if row.get('kind') == 'signature_free_join':
                assert row['classifier'] == job['EVENT_LAB_CLASSIFY']
            record['rows'].append(row)
    if status=='passed' and not record['rows']: raise RuntimeError('No benchmark output: '+label)
    print(f'{index:03} {label}: {status}, {record["elapsed_s"]:.2f}s',flush=True)
    return record

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--no-build',action='store_true');ap.add_argument('--verify-only',action='store_true');ap.add_argument('--build-only',action='store_true')
    ap.add_argument('--candidate',choices=CANDIDATES);ap.add_argument('--scenario',choices=SCENARIOS)
    ap.add_argument('--lane',choices=['join','classify','elimination','symbolic','modal','relations','products','scoped-products','legal-relations','readouts','laws','transport','owned','symbolic-relations','complement','all'],default='all');ap.add_argument('--trials',type=int,default=7)
    ap.add_argument('--product',choices=['materialized','views','views-inputs','both'],default='materialized')
    ap.add_argument('--legal-domain',choices=['full','below','holes','fibred'],default='below')
    ap.add_argument('--support-gates',choices=['gated','certified'],default='gated')
    ap.add_argument('--readout-family',choices=['suffix-1','suffix-half','whole','non-suffix'],default='suffix-1')
    ap.add_argument('--readout-faces',choices=['x','y','xy'],default='xy')
    ap.add_argument('--retraction-factor',choices=['joint','faces'],default='joint')
    ap.add_argument('--retraction-normalize',choices=['staged','equal','ite'],default='staged')
    ap.add_argument('--classifier',choices=['cells','direct','both'],default='both')
    ap.add_argument('--seed',type=int,default=20260917)
    ap.add_argument('--layout',choices=['face-major','bit-major','pair-major','all'],default='face-major')
    ap.add_argument('--occupancy-kernel',choices=['scalar','words','scratch','both','all'],default='scalar')
    ap.add_argument('--essential-layout',choices=['enum','slab','both'],default='enum')
    ap.add_argument('--essential-kernel',choices=['words','scalar','derived','both','all'],default='words')
    ap.add_argument('--identity',choices=['native','symbolic','table','all'],default='native',help='Matched diagonal constructors: representation-native, symbolic formula, or enumerated table')
    ap.add_argument('--packed-map',choices=['local','recursive'],default='local',help='Matched packed-terminal permutation algorithms in the same binary')
    ap.add_argument('--transfer-import',choices=['words','direct','recursive','both','all'],default='words',help='Matched table import algorithms; both means direct/recursive, all adds word-permutation construction')
    ap.add_argument('--transfer-setup',choices=['fixture','product'],default='fixture',help='Construct target support from fixture bitsets or its declared full-product descriptor')
    ap.add_argument('--timeout',type=int,default=60);ap.add_argument('--output',default='runs.json');args=ap.parse_args()
    if args.lane=='classify' and args.classifier=='direct' and args.candidate and args.candidate not in DIRECT_SIGNATURE:
        ap.error('Direct classification is unavailable for '+args.candidate+'; use --classifier cells for its constructive control.')
    binary=json.loads((RESULTS/'build.json').read_text())['binary'] if args.no_build else build()
    check_engine_sources()
    if args.build_only: return
    metadata=json.loads((RESULTS/'build.json').read_text())
    for path,digest in metadata['lab_sources'].items():
        if hashlib.sha256((LAB/path).read_bytes()).hexdigest()!=digest:
            raise RuntimeError('Lab source changed; rebuild before measuring: '+path)
    assert metadata['binary_sha256']==hashlib.sha256(Path(binary).read_bytes()).hexdigest()
    log_dir=RESULTS/(Path(args.output).stem+'-logs')
    log_dir.mkdir(parents=True,exist_ok=False)
    (log_dir/'build.json').write_text(json.dumps(metadata,indent=2)+'\n')
    transfer_modes=['words','direct','recursive'] if args.transfer_import=='all' else (['direct','recursive'] if args.transfer_import=='both' else [args.transfer_import])
    essential_modes=['words','scalar','derived'] if args.essential_kernel=='all' else ['words','scalar'] if args.essential_kernel=='both' else [args.essential_kernel]
    essential_layouts=['enum','slab'] if args.essential_layout=='both' else [args.essential_layout]
    occupancy_kernels={'both':['scalar','words'],'all':['scalar','words','scratch']}.get(args.occupancy_kernel,[args.occupancy_kernel])
    product_modes=['materialized','views'] if args.product=='both' else [args.product]
    verification_jobs=[(mode,kernel,storage,occupancy,product) for mode in transfer_modes for kernel in essential_modes for storage in essential_layouts for occupancy in occupancy_kernels for product in product_modes]
    records=[execute(binary,{'verify':True,'EVENT_LAB_PACKED_MAP':args.packed_map,'EVENT_LAB_TRANSFER_IMPORT':mode,'EVENT_LAB_ESSENTIAL_KERNEL':kernel,'EVENT_LAB_ESSENTIAL_LAYOUT':storage,'EVENT_LAB_OCCUPANCY_KERNEL':occupancy,'EVENT_LAB_PRODUCT':product,'EVENT_LAB_RETRACTION_FACTOR':args.retraction_factor,'EVENT_LAB_RETRACTION_NORMALIZE':args.retraction_normalize,'EVENT_LAB_IDENTITY':args.identity if args.identity!='all' else 'native'},i,args.trials,args.timeout,log_dir) for i,(mode,kernel,storage,occupancy,product) in enumerate(verification_jobs)]
    out=RESULTS/args.output
    def save(): out.write_text(json.dumps({'seed':args.seed,'trials':args.trials,'timeout_s':args.timeout,'build_metadata':metadata,'runs':records},indent=2)+'\n')
    save()
    if any(r['status']!='passed' for r in records): raise SystemExit('Correctness failed; no performance sweep.')
    jobs=[];candidates=[args.candidate] if args.candidate else CANDIDATES
    layouts=['face-major','bit-major','pair-major'] if args.layout=='all' else [args.layout]
    if not args.verify_only:
        if args.lane in ['all','classify']:
            for name in candidates:
                for scenario in [args.scenario] if args.scenario else SCENARIOS:
                    for mode in (['cells','direct'] if args.classifier=='both' else [args.classifier]):
                        if mode=='direct' and name not in DIRECT_SIGNATURE: continue
                        jobs.append({'EVENT_LAB_LANE':'classify','EVENT_LAB_CANDIDATE':name,'EVENT_LAB_SCENARIO':scenario,'EVENT_LAB_CLASSIFY':mode})
        if args.lane in ['all','join']:
            for name in candidates:
                for scenario in [args.scenario] if args.scenario else SCENARIOS:
                    jobs.append({'EVENT_LAB_LANE':'join','EVENT_LAB_CANDIDATE':name,'EVENT_LAB_SCENARIO':scenario})
        if args.lane in ['all','elimination']:
            for name in candidates:
                for layout in layouts: jobs.append({'EVENT_LAB_LANE':'elimination','EVENT_LAB_CANDIDATE':name,'EVENT_LAB_LAYOUT':layout})
        for lane in ['modal','relations','products','scoped-products','legal-relations','readouts','laws','transport','owned','symbolic-relations','complement']:
            if args.lane in ['all',lane]:
                for name in candidates:
                    if lane=='symbolic-relations' and name in ['dense','dense-dispatched','sparse','roaring','runs']: continue
                    for layout in (['bit-major'] if lane=='symbolic-relations' else layouts if lane!='complement' else ['face-major']):
                        for mode in transfer_modes if lane in ['transport','owned'] else [transfer_modes[0]]:
                            jobs.append({'EVENT_LAB_LANE':lane,'EVENT_LAB_CANDIDATE':name,'EVENT_LAB_LAYOUT':layout,'EVENT_LAB_TRANSFER_IMPORT':mode})
        if args.lane in ['all','symbolic']:
            for name in candidates:
                if name not in ['bdd-root','bdd-pair','mdd4-pair','bdd-anchored','packed64','packed256','packed512','packed4096','block64','essential64','essential512']:continue
                for pairs in [8,12,16,20]:
                    for order in ['grouped','interleaved']:
                        jobs.append({'EVENT_LAB_LANE':'symbolic','EVENT_LAB_CANDIDATE':name,'EVENT_LAB_PAIRS':pairs,'EVENT_LAB_ORDER':order})
    modes=['native','symbolic','table'] if args.identity=='all' else [args.identity]
    jobs=[dict(job,EVENT_LAB_IDENTITY=mode) for job in jobs for mode in modes]
    jobs=[dict(job,EVENT_LAB_ESSENTIAL_KERNEL=kernel) for job in jobs for kernel in (essential_modes if job.get('EVENT_LAB_CANDIDATE','').startswith(('essential','retraction','prefix')) else [essential_modes[0]])]
    jobs=[dict(job,EVENT_LAB_ESSENTIAL_LAYOUT=storage) for job in jobs for storage in (essential_layouts if job.get('EVENT_LAB_CANDIDATE','').startswith(('essential','retraction','prefix')) else [essential_layouts[0]])]
    jobs=[dict(job,EVENT_LAB_OCCUPANCY_KERNEL=mode) for job in jobs for mode in (occupancy_kernels if job.get('EVENT_LAB_CLASSIFY')=='direct' and job.get('EVENT_LAB_CANDIDATE','').startswith(('essential','retraction','prefix')) else [occupancy_kernels[0]])]
    jobs=[dict(job,EVENT_LAB_PRODUCT=mode) for job in jobs for mode in (product_modes if job.get('EVENT_LAB_CANDIDATE','').startswith(('essential','retraction','prefix')) and job.get('EVENT_LAB_LANE') in ['relations','products','scoped-products','legal-relations','owned','laws','symbolic-relations'] else ['materialized'])]
    random.Random(args.seed).shuffle(jobs)
    for i,job in enumerate(jobs,len(records)):
        job['EVENT_LAB_PACKED_MAP']=args.packed_map
        job['EVENT_LAB_RETRACTION_FACTOR']=args.retraction_factor
        job['EVENT_LAB_RETRACTION_NORMALIZE']=args.retraction_normalize
        if job.get('EVENT_LAB_LANE')=='legal-relations':
            job['EVENT_LAB_LEGAL_DOMAIN']=args.legal_domain
            job['EVENT_LAB_SUPPORT_GATES']=args.support_gates
        if job.get('EVENT_LAB_LANE')=='readouts':
            job['EVENT_LAB_LEGAL_DOMAIN']=args.legal_domain
            job['EVENT_LAB_READOUT_FAMILY']=args.readout_family
            job['EVENT_LAB_READOUT_FACES']=args.readout_faces
        if job.get('EVENT_LAB_LANE')=='transport': job['EVENT_LAB_TRANSFER_SETUP']=args.transfer_setup
        record=execute(binary,job,i,args.trials,args.timeout,log_dir);records.append(record);save()
        if record['status']=='failed':raise SystemExit('A candidate failed correctness; inspect '+record['log'])
    print('Saved '+str(out),flush=True)

if __name__=='__main__':main()
