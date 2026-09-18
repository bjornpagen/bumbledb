use super::carrier::*;
use super::legal::Checked;
use super::native::Row;
use super::pack::Schedule;
use super::relation_pack::{Executor,Program};
use super::relation_pack_checks::{fixture,oracle};
use std::collections::{BTreeMap,BTreeSet};
use std::hint::black_box;
use std::time::Instant;

pub fn bench<C:Carrier>() {
    let width=std::env::var("EVENT_PACK_WIDTH").unwrap_or_else(|_|"4".into()).parse().unwrap();
    let fanout=std::env::var("EVENT_PACK_FANOUT").unwrap_or_else(|_|"4".into()).parse().unwrap();
    assert!((3..=6).contains(&width) && (1..=16).contains(&fanout));
    let layout=std::env::var("EVENT_LAB_LAYOUT").unwrap_or_else(|_|"bit-major".into());
    let trials=super::trials();
    let schedules=if std::env::var("EVENT_PACK_REVERSE").ok().as_deref()==Some("1") {[Schedule::Factorized,Schedule::Complete]} else {[Schedule::Complete,Schedule::Factorized]};
    for program in [Program::Compose,Program::Residual] {
        let f=fixture(width,fanout,program);let start=Instant::now();
        let mut carrier=C::legal_space(&f.domains,f.domains.order(&layout)).unwrap();
        let ids:Vec<_>=f.bank.iter().map(|b|carrier.import(b)).collect();
        let by_id:BTreeMap<_,_>=ids.iter().map(|&r|(r,carrier.export(r))).collect();
        let published:BTreeSet<_>=ids.iter().copied().collect();
        let input=Checked::new(carrier,&f.domains,false,"gated").unwrap();let build_s=start.elapsed().as_secs_f64();
        let data:[Vec<Row>;2]=f.raw.each_ref().map(|rs|rs.iter().map(|r|[r[0],r[1],r[2],ids[r[3] as usize]]).collect());
        let (expected,bindings)=oracle(&data,f.groups,&by_id,&f.domains,program).unwrap();
        let expected_checksum=expected.iter().enumerate().map(|(g,w)|w.as_ref().map_or(0,|w|w.iter().map(|w|w.count_ones() as u64).sum::<u64>()*(g as u64+1))).sum::<u64>();
        let verify=|c:&Checked<C>,out:&super::relation_pack::Output| {
            assert_eq!(out.represented_bindings,bindings);
            assert_eq!(out.events.iter().map(|r|r.map(|r|c.carrier().export(r))).collect::<Vec<_>>(),expected);
        };
        for schedule in schedules {for memo in [false,true] {
            let setup=Instant::now();let mut engine=Executor::new(&data,schedule);let setup_s=setup.elapsed().as_secs_f64();engine.prime();
            let mut fresh=vec![];let mut warm=vec![];let mut current=None;let mut input_stats=None;
            for _ in 0..trials {
                let mut c=input.clone();c.memo(memo);let stats=(c.carrier().nodes(),c.bytes());
                if let Some(old)=input_stats {assert_eq!(old,stats);}input_stats=Some(stats);
                let start=Instant::now();let out=engine.run(&mut c,f.groups,&published,program).unwrap();let checksum=out.checksum(c.carrier());
                fresh.push(start.elapsed().as_secs_f64());assert_eq!(black_box(checksum),expected_checksum);verify(&c,&out);current=Some(c);
            }
            let mut c=current.unwrap();let mut loops=1;
            loop {
                let start=Instant::now();for _ in 0..loops {let out=engine.run(&mut c,f.groups,&published,program).unwrap();black_box(out.checksum(c.carrier()));}
                if start.elapsed().as_secs_f64()>=0.02 || loops>=4096 {break;}loops*=2;
            }
            for _ in 0..trials {
                let start=Instant::now();let mut checksum=0;
                for _ in 0..loops {let out=engine.run(&mut c,f.groups,&published,program).unwrap();checksum=out.checksum(c.carrier());black_box(checksum);}
                warm.push(start.elapsed().as_secs_f64()/loops as f64);assert_eq!(checksum,expected_checksum);
            }
            let out=engine.run(&mut c,f.groups,&published,program).unwrap();verify(&c,&out);let (input_nodes,input_bytes)=input_stats.unwrap();
            println!("EVENT_LAB {{\"kind\":\"relation_pack_free_join\",\"candidate\":\"{}\",\"program\":\"{}\",\"schedule\":\"{}\",\"layout\":\"{layout}\",\"width\":{width},\"environments\":2,\"fanout\":{fanout},\"memo\":{memo},\"groups\":{},\"present_groups\":{},\"worlds\":{},\"admissible_worlds\":{},\"represented_bindings\":{},\"native_emissions\":{},\"branch_emissions\":{},\"summary_rows\":{},\"staged_capacity_bytes\":{},\"summary_colt_bytes\":{},\"retained_input_colt_bytes\":{},\"input_nodes\":{input_nodes},\"final_nodes\":{},\"input_bytes_est\":{input_bytes},\"final_bytes_est\":{},\"build_s\":{build_s},\"native_setup_s\":{setup_s},\"fresh_s\":{fresh:?},\"warm_s\":{warm:?},\"warm_loops\":{loops},\"checksum\":{expected_checksum},\"verified\":true}}",
                C::NAME,program.name(),schedule.name(),f.groups,out.present(),1usize<<f.domains.dimensions(),f.domains.population(),out.represented_bindings,
                out.native_emissions,out.branch_emissions,out.summary_rows,out.staged_capacity_bytes,out.summary_colt_bytes,engine.colt_bytes(),c.carrier().nodes(),c.bytes());
        }}
    }
}
