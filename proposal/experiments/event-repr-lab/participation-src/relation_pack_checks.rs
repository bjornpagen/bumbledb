//! Explicit pair enumeration and matrix witnesses; no factorized Event oracle.
use super::carrier::*;
use super::legal::{Checked,Domain,Domains};
use super::native::Row;
use super::relation_pack::Schedule;
use super::relation_pack::{Executor,Fault,Program};
use std::collections::{BTreeMap,BTreeSet};

pub struct Fixture {pub domains:Domains,pub bank:Vec<Vec<u64>>,pub raw:[Vec<Row>;2],pub groups:usize}
pub fn fixture(width:u32,fanout:usize,p:Program)->Fixture {
    fixture_shape(width,fanout,p,"balanced")
}
pub fn fixture_shape(width:u32,fanout:usize,p:Program,shape:&str)->Fixture {
    let size=1u64<<width;
    let d=Domains::new(width,1,vec![Domain::Below(size-3),Domain::Values((0..size).filter(|v|v%3!=1).collect())]).unwrap();
    let n=1usize<<d.dimensions();let side=size as usize-1;let block=8usize.min(size as usize);let bank_size=16;
    let bank=(0..2).flat_map(|family|(0..bank_size).map(move |seed|(family,seed))).map(|(family,seed)|bits(n,|w| {
        let x=w&side;let y=(w>>width)&side;let e=w>>(3*width);
        if family==0 {x/block==y/block && x%7!=(seed+e)%7 && ((y+seed+e)%block==x%block || (y+seed+2*e+1)%block==x%block)}
        else if p==Program::Compose {x/block==y/block && ((y+2*seed+e)%block==(3*x)%block || (y+seed+e+1)%block==x%block)}
        else {x/block==y/block || ((x+seed+e)%5!=0 && (y+seed+2*e)%7!=0)}
    })).collect();
    assert!(matches!(shape,"balanced"|"skew"|"unmatched"));
    let groups=if shape=="unmatched" {48} else {16};
    let raw=std::array::from_fn(|family| {
        let mut rows=vec![];
        for g in 0..groups {
            let count=match shape {
                "balanced"=>fanout,
                "skew"=>if (g%4==0)==(family==0) {4*fanout} else {1},
                "unmatched"=>if g<16 {fanout} else if family==0 && g<32 {fanout} else if family==1 && g>=32 {4*fanout} else {0},
                _=>unreachable!(),
            };
            for i in 0..count {rows.push([g as u64,i as u64,1,((g*7+i*3+family*5)%bank_size+family*bank_size) as u64]);}
        }rows
    });
    Fixture{domains:d,bank,raw,groups}
}
fn value(d:&Domains,words:&[u64],e:usize,x:usize,y:usize)->bool {
    let z=d.domain(e).minimum().unwrap() as usize;
    at(words,x|(y<<d.width())|(z<<(2*d.width()))|(e<<(3*d.width())))
}
fn role(d:&Domains,words:&[u64])->bool {
    let size=1usize<<d.width();
    for e in 0..d.environments() {for x in 0..size {for y in 0..size {
        if !d.domain(e).contains(x as u64)||!d.domain(e).contains(y as u64) {continue;}
        let expected=value(d,words,e,x,y);
        for z in 0..size {if d.domain(e).contains(z as u64) && at(words,x|(y<<d.width())|(z<<(2*d.width()))|(e<<(3*d.width())))!=expected {return false;}}
    }}}true
}
fn pair(d:&Domains,a:&[u64],b:&[u64],p:Program)->Vec<u64> {
    let n=1usize<<d.dimensions();let width=d.width();let size=1usize<<width;
    let mut table=vec![false;d.environments()*size*size];
    for e in 0..d.environments() {for x in 0..size {for z in 0..size {
        if !d.domain(e).contains(x as u64)||!d.domain(e).contains(z as u64) {continue;}
        table[(e*size+x)*size+z]=match p {
            Program::Compose=>(0..size).any(|y|d.domain(e).contains(y as u64)&&value(d,a,e,x,y)&&value(d,b,e,y,z)),
            Program::Residual=>(0..size).all(|y|!d.domain(e).contains(y as u64)||!value(d,a,e,y,x)||value(d,b,e,y,z)),
        };
    }}}
    bits(n,|w|d.contains(w as u64)&&table[((w>>(3*width))*size+(w&(size-1)))*size+((w>>width)&(size-1))])
}
pub fn oracle(data:&[Vec<Row>;2],groups:usize,bank:&BTreeMap<Id,Vec<u64>>,d:&Domains,p:Program)->Result<(Vec<Option<Vec<u64>>>,usize),Vec<Fault>> {
    let data=data.each_ref().map(|rs|rs.iter().copied().collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>());
    let roles:BTreeMap<_,_>=bank.iter().map(|(&r,w)|(r,role(d,w))).collect();
    let mut output:Vec<Option<Vec<u64>>>=vec![None;groups];let mut count=0;let mut faults=BTreeSet::new();
    let mut pairs=BTreeMap::new();
    for a in &data[0] {for b in &data[1] {
        if a[0]!=b[0]||a[2]!=b[2] {continue;}
        count+=1;let mut ok=true;
        if a[0]>=groups as u64 {faults.insert(Fault::Group(a[0]));ok=false;}
        if a[2]!=1 {faults.insert(Fault::Scope(a[2]));ok=false;}
        for r in [a[3],b[3]] {match roles.get(&r) {
            None=>{faults.insert(Fault::Event(r));ok=false;},Some(false)=>{faults.insert(Fault::Role(r));ok=false;},_=>{}
        }}
        if !ok {continue;}
        let computed=pairs.entry((a[3],b[3])).or_insert_with(||pair(d,&bank[&a[3]],&bank[&b[3]],p));
        match &mut output[a[0] as usize] {
            None=>output[a[0] as usize]=Some(computed.clone()),
            Some(old)=>for (x,y) in old.iter_mut().zip(computed.iter()) {if p==Program::Compose {*x|=*y;} else {*x&=*y;}},
        }
    }}
    if faults.is_empty() {Ok((output,count))} else {Err(faults.into_iter().collect())}
}
fn case<C:Carrier>(input:&Checked<C>,data:&[Vec<Row>;2],groups:usize,bank:&BTreeMap<Id,Vec<u64>>,d:&Domains)->usize {
    let published=bank.keys().copied().collect();let mut checks=0;
    for p in [Program::Compose,Program::Residual] {let expected=oracle(data,groups,bank,d,p);
        for memo in [false,true] {for schedule in [Schedule::Complete,Schedule::Staged,Schedule::IndexRight,Schedule::IndexSmall] {
            let mut c=input.clone();c.memo(memo);let mut engine=Executor::new(data,schedule);
            for _ in 0..2 {
                match (&expected,engine.run(&mut c,groups,&published,p)) {
                    (Err(a),Err(b))=>assert_eq!(*a,b,"{} {p:?} {schedule:?}",C::NAME),
                    (Ok((out,rows)),Ok(actual))=>{
                        assert_eq!(actual.represented_bindings,*rows);
                        assert_eq!(actual.events.iter().map(|r|r.map(|r|c.carrier().export(r))).collect::<Vec<_>>(),*out,"{} {p:?} {schedule:?}",C::NAME);
                        if schedule!=Schedule::Complete {
                            assert_eq!(actual.summary_rows,2*actual.present());assert_eq!(actual.native_emissions,actual.branch_emissions+actual.present());
                            let unique=data.each_ref().map(|r|r.iter().copied().collect::<BTreeSet<_>>());
                            let base=unique.iter().map(BTreeSet::len).sum::<usize>();
                            if schedule==Schedule::Staged {assert_eq!(actual.branch_emissions,base);assert!(actual.roster_branch.is_none());}
                            else {
                                let first=if schedule==Schedule::IndexSmall && data[0].len()<data[1].len() {0} else {1};
                                assert_eq!(actual.roster_branch,Some(first));assert_eq!(actual.branch_emissions,base+unique[first].len());
                                assert_eq!(actual.staged_capacity_bytes,0);
                                assert_eq!(actual.map_entries,unique[first].iter().map(|r|(r[0],r[2])).collect::<BTreeSet<_>>().len());
                            }
                        }
                        else {assert_eq!(actual.native_emissions,*rows);}
                    },(a,b)=>panic!("{} {p:?} {schedule:?}: {a:?} versus {b:?}",C::NAME),
                }checks+=1;
            }
        }}
    }checks
}
fn carrier<C:Carrier>() {
    let d=Domains::new(2,1,vec![Domain::Below(3),Domain::Values(vec![0,2,3])]).unwrap();let n=1usize<<d.dimensions();
    let mut checks=0;
    for layout in ["face-major","bit-major"] {
        let mut c=C::legal_space(&d,d.order(layout)).unwrap();
        let words:Vec<_>=(0..11).map(|i|bits(n,|w|match i {
            0=>false,1=>true,
            8=>(w&3)==0 && ((w>>2)&3)==2 && (w>>6)==0,
            9=>(w&3)==2 && ((w>>2)&3)==3 && (w>>6)==1,
            10=>(w&3)==1 && ((w>>2)&3)==0 && (w>>6)==0,
            _=>{let x=w&3;let y=(w>>2)&3;let e=w>>6;(x+i+e)%4==y || (y+i+2*e)%4==x}
        })).collect();
        let roots:Vec<_>=words.iter().map(|w|c.import(w)).collect();
        let scratch=c.variable(4);let mut bank:BTreeMap<_,_>=roots.iter().map(|&r|(r,c.export(r))).collect();bank.insert(scratch,c.export(scratch));
        let input=Checked::new(c,&d,false,"gated").unwrap();
        for i in [8,9,10] {assert!(input.carrier().count(roots[i])>0);}
        for selection in 0..16 {
            let data=std::array::from_fn(|b|(0..2).filter(|&i|selection>>(2*b+i)&1!=0).map(|i|[0,i as u64,1,roots[2*b+i]]).collect());
            checks+=case(&input,&data,2,&bank,&d);
        }
        for seed in 0..24u64 {
            let data=std::array::from_fn(|b| {
                let mut rows=vec![];
                for g in 0..4 {for i in 0..3 {let h=mix(seed*37+g*13+i*7+b as u64*31);if h%4==0 {continue;}
                    let row=[g,i,if h%17==0 {2} else {1},roots[h as usize%8]];rows.push(row);if i==0 {rows.push(row);}
                }}rows
            });checks+=case(&input,&data,4,&bank,&d);
        }
        for b in 0..2 {for bad in [u64::MAX,scratch] {for (g,scope) in [(0,1),(9,1),(0,9)] {
            let mut data:[Vec<Row>;2]=std::array::from_fn(|_|vec![[0,0,1,roots[1]]]);
            data[b].push([g,1,scope,bad]);checks+=case(&input,&data,2,&bank,&d);
            if (g,scope)!=(0,1) {data[1-b].push([g,2,scope,roots[1]]);checks+=case(&input,&data,2,&bank,&d);}
        }}}
        // Separate possible transitions cannot replace a shared intermediate,
        // and environments must never be existentially erased.
        for other in [9,10] {
            let data=[vec![[0,0,1,roots[8]]],vec![[0,0,1,roots[other]]]];
            checks+=case(&input,&data,2,&bank,&d);
            let (expected,_)=oracle(&data,2,&bank,&d,Program::Compose).unwrap();
            assert!(expected[0].as_ref().unwrap().iter().all(|w|*w==0));assert!(expected[1].is_none());
        }
        // The row-count estimate deliberately chooses the wrong side after
        // whole-row deduplication; the choice must affect cost only.
        let duplicated=[vec![[0,0,1,roots[1]];16],vec![[0,0,1,roots[2]],[0,1,1,roots[3]]]];
        checks+=case(&input,&duplicated,2,&bank,&d);
        let mut special=input.clone();let empty=special.empty();let full=special.full();
        let composed=special.compose(empty,full).unwrap();let residual=special.residual(empty,full).unwrap();
        assert_eq!(special.root(composed).unwrap(),special.carrier().empty());
        assert_eq!(special.root(residual).unwrap(),special.carrier().full());
    }
    for shape in ["balanced","skew","unmatched"] {
        let f=fixture_shape(3,3,Program::Compose,shape);let mut c=C::legal_space(&f.domains,f.domains.order("bit-major")).unwrap();
        let ids:Vec<_>=f.bank.iter().map(|w|c.import(w)).collect();let bank=ids.iter().map(|&r|(r,c.export(r))).collect();
        let data:[Vec<Row>;2]=f.raw.each_ref().map(|rs|rs.iter().map(|r|[r[0],r[1],r[2],ids[r[3] as usize]]).collect());
        let input=Checked::new(c,&f.domains,false,"gated").unwrap();
        checks+=case(&input,&data,f.groups,&bank,&f.domains);
        checks+=case(&input,&[data[1].clone(),data[0].clone()],f.groups,&bank,&f.domains);
    }
    println!("EVENT_LAB {{\"kind\":\"participation_verification\",\"candidate\":\"{}\",\"native_runs\":{checks},\"passed\":true}}",C::NAME);
}
pub fn all() {
    carrier::<super::finite::Finite<super::finite::Dense>>();carrier::<super::packed::Packed<8>>();
    carrier::<super::essential::Essential<9>>();carrier::<super::retraction::Retraction<9>>();
    carrier::<super::range_carrier::RangeCarrier<0>>();carrier::<super::range_carrier::RangeCarrier<2>>();
}
