use super::carrier::*;
use super::native;
use super::pack::{Executor,Schedule};
use std::collections::{BTreeMap,BTreeSet};
use std::hint::black_box;
use std::time::Instant;

pub fn bench<C:Carrier>(scenario:&str) {
    let trials=super::trials();let groups=64;
    let fanout:usize=std::env::var("EVENT_PACK_FANOUT").unwrap_or_else(|_|"4".into()).parse().unwrap();
    assert!((1..=16).contains(&fanout));
    let (n,bank)=super::workload(scenario);let support=super::scenario_support(scenario,n);
    let start=Instant::now();let mut base=C::new(&support,n);
    let ids:Vec<_>=bank.iter().map(|v|base.import(v)).collect();let build_s=start.elapsed().as_secs_f64();
    let rows=native::data("clover",groups,fanout,bank.len());
    let data:[Vec<native::Row>;3]=rows.each_ref().map(|rs|rs.iter().map(|r|[r[0],r[1],r[2],ids[r[3] as usize]]).collect());
    let mut by_id=BTreeMap::new();
    for (words,&id) in bank.iter().zip(&ids) {
        let actual:Vec<_>=words.iter().zip(&support).map(|(w,s)|w&s).collect();
        assert_eq!(base.export(id),actual);
        if let Some(old)=by_id.insert(id,actual.clone()) {assert_eq!(old,actual);}
    }
    let (expected,expected_rows)=super::pack_checks::oracle(&data,groups,&by_id,bank[0].len()).unwrap();
    let expected_checksum=expected.iter().enumerate().map(|(i,b)|b.as_ref().map_or(0,|b|b.iter().map(|w|w.count_ones() as u64).sum::<u64>()*(i as u64+1))).sum::<u64>();
    let expected_present=expected.iter().filter(|v|v.is_some()).count();
    let valid:BTreeSet<_>=ids.iter().copied().collect();
    let schedules=if std::env::var("EVENT_PACK_REVERSE").ok().as_deref()==Some("1") {
        [Schedule::Factorized,Schedule::Complete]
    } else {[Schedule::Complete,Schedule::Factorized]};
    for schedule in schedules {for cached in [false,true] {
        let setup=Instant::now();let mut engine=Executor::new(&data,"clover",schedule).unwrap();
        let setup_s=setup.elapsed().as_secs_f64();engine.prime();
        let input=Memo::new(base.clone(),cached);let mut fresh=Vec::new();let mut warm=Vec::new();
        let mut current=None;let mut input_stats=None;
        let verify=|c:&C,out:&super::pack::Output| {
            assert_eq!(out.represented_bindings,expected_rows);assert_eq!(out.present(),expected_present);
            let actual:Vec<_>=out.events.iter().map(|r|r.map(|v|c.export(v))).collect();assert_eq!(actual,expected);
        };
        for i in 0..trials {
            let mut c=input.clone();
            let stats=(c.inner.nodes(),c.bytes());if let Some(old)=input_stats {assert_eq!(old,stats);}input_stats=Some(stats);
            let start=Instant::now();let out=engine.run(&mut c,groups,&valid).unwrap();let checksum=out.checksum(&c.inner);
            fresh.push(start.elapsed().as_secs_f64());black_box(checksum);assert_eq!(checksum,expected_checksum);
            if i==0 {verify(&c.inner,&out);}current=Some(c);
        }
        let mut c=current.unwrap();let mut loops=1;
        loop {
            let start=Instant::now();
            for _ in 0..loops {let out=engine.run(&mut c,groups,&valid).unwrap();black_box(out.checksum(&c.inner));}
            if start.elapsed().as_secs_f64()>=0.02 || loops>=4096 {break;}loops*=2;
        }
        for _ in 0..trials {
            let start=Instant::now();let mut checksum=0;
            for _ in 0..loops {let out=engine.run(&mut c,groups,&valid).unwrap();checksum=out.checksum(&c.inner);black_box(checksum);}
            warm.push(start.elapsed().as_secs_f64()/loops as f64);assert_eq!(checksum,expected_checksum);
        }
        let out=engine.run(&mut c,groups,&valid).unwrap();verify(&c.inner,&out);
        let (input_nodes,input_bytes)=input_stats.unwrap();
        println!("EVENT_LAB {{\"kind\":\"factorized_pack_free_join\",\"candidate\":\"{}\",\"scenario\":\"{scenario}\",\"schedule\":\"{}\",\"memo\":{cached},\"fanout\":{fanout},\"worlds\":{n},\"admissible_worlds\":{},\"groups\":{groups},\"present_groups\":{},\"represented_bindings\":{},\"native_emissions\":{},\"branch_emissions\":{},\"summary_rows\":{},\"summary_colt_bytes\":{},\"staged_id_capacity_bytes\":{},\"input_nodes\":{input_nodes},\"final_nodes\":{},\"input_bytes_est\":{input_bytes},\"final_bytes_est\":{},\"retained_input_colt_bytes\":{},\"build_s\":{build_s},\"native_setup_s\":{setup_s},\"fresh_s\":{fresh:?},\"warm_s\":{warm:?},\"warm_loops\":{loops},\"checksum\":{expected_checksum},\"verified\":true}}",
            C::NAME,schedule.name(),support.iter().map(|w|w.count_ones() as u64).sum::<u64>(),out.present(),out.represented_bindings,
            out.native_emissions,out.branch_emissions,out.summary_rows,out.summary_colt_bytes,out.staged_id_capacity_bytes,
            c.inner.nodes(),c.bytes(),engine.retained_colt_bytes());
    }}
}
