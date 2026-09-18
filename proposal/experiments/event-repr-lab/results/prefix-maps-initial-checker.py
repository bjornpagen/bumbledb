"""Check arbitrary maps on legal prefix owners against a finite pointwise oracle."""
from pathlib import Path
import argparse,hashlib,json,os,subprocess,time
from prepare import LAB,SCRATCH
ap=argparse.ArgumentParser();ap.add_argument('--storage',choices=['slab','enum'],default='slab');ap.add_argument('--output',required=True);args=ap.parse_args()
folder=SCRATCH/'prefix-maps-checker';(folder/'src').mkdir(parents=True,exist_ok=True)
(folder/'Cargo.toml').write_text('''[package]
name = "event-prefix-maps-checker"
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
names=['carrier','essential','essential_raw','diagram','finite','packed','blocks','relational_core','legal','observation','dependencies','transfer','occupancy','signature','contraction','retraction']
code='#![allow(dead_code)]\n'+''.join('#[path = '+json.dumps(str(LAB/'src'/(name+'.rs')))+']\nmod '+name+';\n' for name in names)+r'''
use carrier::*;
use legal::{Domain,Domains};
use retraction::Retraction;
fn permutations(xs: &mut [u32], at: usize, out: &mut Vec<Vec<u32>>) {
    if at==xs.len() { out.push(xs.to_vec());return; }
    for i in at..xs.len() { xs.swap(at,i);permutations(xs,at+1,out);xs.swap(at,i); }
}
fn maps<const K:u32,const PREFIX:bool>() {
    let d=Domains::new(2,0,vec![Domain::Below(3)]).unwrap();
    let mut c=Retraction::<K,PREFIX>::legal_space(&d,d.order("bit-major")).unwrap();
    let swap=|w:u64| (w & !3) | ((w&1)<<1) | ((w>>1)&1);
    let commutes=(0..64).all(|w|c.decode_world(swap(w))==swap(c.decode_world(w)));
    assert_eq!(commutes,!PREFIX,"the policies trade symmetry for prefix structure");
    let mut all=vec![];permutations(&mut [0,1,2,3,4,5],0,&mut all);assert_eq!(all.len(),720);
    let mut cases=0;
    for seed in 0..8 {
        let data=bits(64,|w| match seed {0=>false,1=>true,2=>w&1!=0,3=>w&2!=0,_=>mix(w as u64+seed*29)%7<3});
        let event=c.import(&data);
        for destinations in &all {
            let map=Permutation::new(destinations.clone()).unwrap();
            let got=c.permute(event,&map);
            let want=bits(64,|w| {
                let input=destinations.iter().enumerate().fold(0u64,|v,(i,&j)|v|(((w as u64>>j)&1)<<i));
                d.contains(w as u64)&&d.contains(input)&&at(&data,input as usize)
            });
            assert_eq!(c.export(got),want,"pointwise map");
            assert_eq!(got,c.import(&want),"the mapped result must be normalized, even when its legal export was already correct");
            cases+=1;
        }
    }
    println!("EVENT_LAB {{\"kind\":\"prefix_map_verification\",\"candidate\":\"{}\",\"cases\":{},\"bit_swap_commutes\":{},\"passed\":true}}",Retraction::<K,PREFIX>::NAME,cases,commutes);
}
#[test]
fn prefix_maps() { maps::<9,false>();maps::<3,true>();maps::<6,true>();maps::<9,true>(); }
'''
(folder/'src/lib.rs').write_text(code)
digests={str(p.relative_to(LAB)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((LAB/'src').glob('*.rs'))}
env=os.environ.copy();env.update(EVENT_LAB_ESSENTIAL_KERNEL='derived',EVENT_LAB_ESSENTIAL_LAYOUT=args.storage,EVENT_LAB_OCCUPANCY_KERNEL='words',EVENT_LAB_RETRACTION_COUNT='words')
start=time.time();p=subprocess.run(['cargo','test','--offline','--','--nocapture','--test-threads=1'],cwd=folder,capture_output=True,text=True,env=env)
assert digests=={str(p.relative_to(LAB)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((LAB/'src').glob('*.rs'))}
log=Path(args.output).stem+'.log';(LAB/'results'/log).write_text(p.stdout+'\n'+p.stderr)
result=dict(passed=p.returncode==0,elapsed_s=time.time()-start,profile='debug; correctness only',lab_sources=digests,shared_modules=names,essential_layout=args.storage,log=log,checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),generated_rust_sha256=hashlib.sha256(code.encode()).hexdigest())
(LAB/'results'/args.output).write_text(json.dumps(result,indent=2)+'\n');print(p.stdout[-2500:]);print(p.stderr[-2500:]);raise SystemExit(p.returncode)
