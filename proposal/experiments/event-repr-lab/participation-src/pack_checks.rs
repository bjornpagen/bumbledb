use super::carrier::*;
use super::native::Row;
use super::pack::{Executor,Fault,Schedule};
use std::collections::{BTreeMap,BTreeSet};

type Expected=(Vec<Option<Vec<u64>>>,usize);
pub fn oracle(data:&[Vec<Row>;3],groups:usize,bank:&BTreeMap<Id,Vec<u64>>,words:usize)->Result<Expected,Vec<Fault>> {
    let data:[Vec<Row>;3]=data.each_ref().map(|rows|rows.iter().copied().collect::<BTreeSet<_>>().into_iter().collect());
    let mut out:Vec<Option<Vec<u64>>>=vec![None;groups];let mut faults=BTreeSet::new();let mut count=0;
    for a in &data[0] {for b in &data[1] {for c in &data[2] {
        if a[0]!=b[0] || a[0]!=c[0] || a[2]!=b[2] || a[2]!=c[2] {continue;}
        count+=1;let mut ok=true;
        if a[0]>=groups as u64 {faults.insert(Fault::Group(a[0]));ok=false;}
        if a[2]!=1 {faults.insert(Fault::Scope(a[2]));ok=false;}
        for r in [a[3],b[3],c[3]] {if !bank.contains_key(&r) {faults.insert(Fault::Event(r));ok=false;}}
        if !ok {continue;}
        let value=out[a[0] as usize].get_or_insert_with(||vec![0;words]);
        for i in 0..words {value[i]|=bank[&a[3]][i]&!(bank[&b[3]][i]|bank[&c[3]][i]);}
    }}}
    if faults.is_empty() {Ok((out,count))} else {Err(faults.into_iter().collect())}
}

fn case<C:Carrier>(base:&C,data:&[Vec<Row>;3],groups:usize,bank:&BTreeMap<Id,Vec<u64>>,memo:bool)->usize {
    let valid=bank.keys().copied().collect();let words=base.export(base.full()).len();
    let expected=oracle(data,groups,bank,words);let mut checks=0;
    for schedule in [Schedule::Complete,Schedule::Factorized] {
        let mut engine=Executor::new(data,"clover",schedule).unwrap();
        let mut c=Memo::new(base.clone(),memo);
        // Repeat on the same executors to expose stale summaries and caches.
        for _ in 0..2 {
            match (&expected,engine.run(&mut c,groups,&valid)) {
                (Err(a),Err(b))=>assert_eq!(*a,b,"{} {schedule:?}",C::NAME),
                (Ok((values,count)),Ok(out))=> {
                    assert_eq!(out.represented_bindings,*count);
                    let actual:Vec<_>=out.events.iter().map(|r|r.map(|r|c.inner.export(r))).collect();
                    assert_eq!(&actual,values,"{} {schedule:?}",C::NAME);
                    assert_eq!(out.present(),values.iter().filter(|v|v.is_some()).count());
                    if schedule==Schedule::Factorized {
                        assert_eq!(out.summary_rows,3*out.present());
                        assert_eq!(out.native_emissions,out.branch_emissions+out.present());
                    } else {assert_eq!(out.native_emissions,*count);}
                },
                (a,b)=>panic!("{} {schedule:?} mismatch: {a:?} {b:?}",C::NAME),
            }
            checks+=1;
        }
    }checks
}

fn carrier<C:Carrier>() {
    let support=bits(128,|w|w%5!=0);let mut base=C::new(&support,128);
    let bank:Vec<_>=(0..8).map(|i|bits(128,|w|match i {0=>false,1=>true,_=>mix(w as u64+i*17)%5<3})).collect();
    let roots:Vec<_>=bank.iter().map(|b|base.import(b)).collect();
    let values:BTreeMap<_,_>=roots.iter().map(|&r|(r,base.export(r))).collect();let mut checks=0;
    for memo in [false,true] {
        // All branch-presence combinations with empty/full and partial Events.
        for selection in 0..64u64 {
            let data=std::array::from_fn(|branch|(0..2).filter(|&i|selection>>(2*branch+i)&1!=0)
                .map(|i|[0,i as u64,1,roots[(branch*2+i)%8]]).collect());
            checks+=case(&base,&data,2,&values,memo);
        }
        // Distinct local keys carrying equal Event values remain distinct rows;
        // duplicate entire rows are deduplicated by the native image path.
        for seed in 0..32u64 {
            let data=std::array::from_fn(|branch| {
                let mut rows=Vec::new();
                for g in 0..4u64 {for i in 0..3u64 {
                    let h=mix(seed*131+g*17+i*5+branch as u64*71);
                    if h%4==0 {continue;}
                    let r=[g,i,if h%13==0 {2} else {1},roots[(h as usize/4)%8]];
                    rows.push(r);if i==0 {rows.push(r);}
                }}rows
            });
            checks+=case(&base,&data,4,&values,memo);
        }
        for branch in 0..3 {
            for corrupt in 0..3 {
                // The first row saturates that branch's accumulator. A later
                // invalid participating row must still fail the entire query.
                let mut data:[Vec<Row>;3]=std::array::from_fn(|b|vec![[0,0,1,roots[if b==0 {1} else {0}]]]);
                let mut bad=[0,1,1,u64::MAX];
                if corrupt==1 {bad=[3,1,1,u64::MAX];}
                if corrupt==2 {bad=[0,1,9,u64::MAX];}
                data[branch].push(bad);
                // Only corrupt==0 participates. The other rows have no joined
                // companions at their group/scope and must not be evaluated.
                checks+=case(&base,&data,2,&values,memo);
                if corrupt!=0 {
                    for b in 0..3 {if b!=branch {data[b].push([bad[0],2,bad[2],roots[1]]);}}
                    checks+=case(&base,&data,2,&values,memo);
                }
            }
        }
        let mut data:[Vec<Row>;3]=std::array::from_fn(|_|vec![[0,0,1,roots[1]]]);
        data[0][0][3]=roots[0];
        checks+=case(&base,&data,2,&values,memo); // present empty, plus absent group 1
        assert!(Executor::new(&data,"triangle",Schedule::Factorized).is_err());
    }
    println!("EVENT_LAB {{\"kind\":\"factorized_pack_verification\",\"candidate\":\"{}\",\"native_runs\":{checks},\"passed\":true}}",C::NAME);
}

pub fn all() {
    carrier::<super::finite::Finite<super::finite::Dense>>();
    carrier::<super::packed::Packed<8>>();
    carrier::<super::essential::Essential<9>>();
    carrier::<super::retraction::Retraction<9>>();
    carrier::<super::range_carrier::RangeCarrier<0>>();
    carrier::<super::range_carrier::RangeCarrier<2>>();
}
