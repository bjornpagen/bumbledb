//! Exact Pack factorization for the constructed clover query. All scans and
//! the summary join use Native's real Free Join executor. No arbitrary plan
//! receives this license, and neither sink asks to skip a suffix.
use super::carrier::*;
use super::native::{Native,Row};
use std::collections::{BTreeMap,BTreeSet};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Schedule {Complete,Factorized}
impl Schedule {
    pub fn name(self)->&'static str {match self {Self::Complete=>"complete",Self::Factorized=>"factorized"}}
}
#[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord)]
pub enum Fault {Group(u64),Scope(u64),Event(Id),Multiplicity}

#[derive(Debug)]
pub struct Output {
    pub events:Vec<Option<Id>>,
    pub represented_bindings:usize,
    pub native_emissions:usize,
    pub branch_emissions:usize,
    pub summary_rows:usize,
    pub summary_colt_bytes:usize,
    pub staged_id_capacity_bytes:usize,
}
impl Output {
    fn new(groups:usize)->Self {
        Self{events:vec![None;groups],represented_bindings:0,native_emissions:0,
            branch_emissions:0,summary_rows:0,summary_colt_bytes:0,staged_id_capacity_bytes:0}
    }
    pub fn checksum<C:Carrier>(&self,c:&C)->u64 {
        self.events.iter().enumerate().map(|(i,e)|e.map_or(0,|r|c.count(r).wrapping_mul(i as u64+1))).sum()
    }
    pub fn present(&self)->usize {self.events.iter().filter(|e|e.is_some()).count()}
}

pub struct Executor {
    schedule:Schedule,complete:Option<Native>,branches:Vec<Native>,
}
impl Executor {
    /// The fixed clover constructor supplies the Cartesian-fibre certificate:
    /// group and scope are the only shared variables, with no cross-branch
    /// residual or anti-probe. Triangle and other shapes are rejected.
    pub fn new(data:&[Vec<Row>;3],shape:&str,schedule:Schedule)->Result<Self,&'static str> {
        if shape!="clover" {return Err("factorized Pack requires the constructed clover query");}
        Ok(match schedule {
            Schedule::Complete=>Self{schedule,complete:Some(Native::new(data,"clover")),branches:vec![]},
            Schedule::Factorized=>Self{schedule,complete:None,branches:data.iter().map(|rows|Native::single(rows)).collect()},
        })
    }
    pub fn prime(&mut self) {
        if let Some(c)=&mut self.complete {c.run(|_|{});}
        for b in &mut self.branches {b.run(|_|{});}
    }
    pub fn retained_colt_bytes(&self)->usize {
        self.complete.as_ref().map_or(0,Native::retained_colt_bytes)
            +self.branches.iter().map(Native::retained_colt_bytes).sum::<usize>()
    }
    /// `valid` is the immutable set of published fixture roots for owner 1.
    /// Both schedules validate every participating value, even after an
    /// accumulator becomes empty/full. Unmatched groups are never evaluated.
    /// Fault sets deliberately make multiple-invalid-input diagnostics unordered.
    pub fn run<C:Carrier>(&mut self,c:&mut Memo<C>,groups:usize,valid:&BTreeSet<Id>)->Result<Output,Vec<Fault>> {
        let mut out=Output::new(groups);let mut faults=BTreeSet::new();
        if self.schedule==Schedule::Complete {
            self.complete.as_mut().unwrap().run(|[g,scope,a,b,d]| {
                out.native_emissions+=1;
                match out.represented_bindings.checked_add(1) {Some(n)=>out.represented_bindings=n,None=>{faults.insert(Fault::Multiplicity);}}
                let mut ok=validate_group(g,scope,groups,&mut faults);
                for r in [a,b,d] {ok &= validate_event(r,valid,&mut faults);}
                if ok {let e=c.expression(a,b,d);accumulate(c,&mut out,g as usize,e);}
            });
        } else {
            let mut by_group:BTreeMap<(u64,u64),[Vec<Id>;3]>=BTreeMap::new();
            for (branch,engine) in self.branches.iter_mut().enumerate() {
                engine.run(|[g,scope,r,_,_]| {
                    out.branch_emissions+=1;out.native_emissions+=1;
                    by_group.entry((g,scope)).or_default()[branch].push(r);
                });
            }
            let mut summaries:[Vec<Row>;3]=std::array::from_fn(|_|Vec::new());
            let mut multiplicities=BTreeMap::new();
            for (&(g,scope),branches) in &by_group {
                out.staged_id_capacity_bytes+=branches.iter().map(|b|b.capacity()*std::mem::size_of::<Id>()).sum::<usize>();
                if branches.iter().any(Vec::is_empty) {continue;}
                // All these rows occur in a complete binding. Delay evaluation
                // until this is known: an unmatched invalid value is not an error.
                let mut ok=validate_group(g,scope,groups,&mut faults);
                for branch in branches {for &r in branch {ok &= validate_event(r,valid,&mut faults);}}
                let count=branches.iter().try_fold(1usize,|n,b|n.checked_mul(b.len()));
                if count.is_none() {faults.insert(Fault::Multiplicity);ok=false;}
                if !ok {continue;}
                multiplicities.insert((g,scope),count.unwrap());
                for (i,branch) in branches.iter().enumerate() {
                    // Branch presence, not the identity value of the fold,
                    // determines whether a summary row is emitted.
                    let mut value=if i==0 {c.inner.empty()} else {c.inner.full()};
                    for &r in branch {value=c.op(if i==0 {14} else {8},value,r);}
                    summaries[i].push([g,0,scope,value]);out.summary_rows+=1;
                }
            }
            // Planner/image/summary-COLT construction is intentionally inside
            // run, so the query timer charges this first staged implementation.
            let mut finish=Native::new(&summaries,"clover");
            finish.run(|[g,scope,a,b,d]| {
                out.native_emissions+=1;
                let n=multiplicities[&(g,scope)];
                match out.represented_bindings.checked_add(n) {Some(n)=>out.represented_bindings=n,None=>{faults.insert(Fault::Multiplicity);}}
                let e=c.expression(a,b,d);accumulate(c,&mut out,g as usize,e);
            });
            out.summary_colt_bytes=finish.retained_colt_bytes();
        }
        if faults.is_empty() {Ok(out)} else {Err(faults.into_iter().collect())}
    }
}
fn validate_group(g:u64,scope:u64,groups:usize,faults:&mut BTreeSet<Fault>)->bool {
    let mut ok=true;
    if g>=groups as u64 {faults.insert(Fault::Group(g));ok=false;}
    if scope!=1 {faults.insert(Fault::Scope(scope));ok=false;}
    ok
}
fn validate_event(r:Id,valid:&BTreeSet<Id>,faults:&mut BTreeSet<Fault>)->bool {
    if valid.contains(&r) {true} else {faults.insert(Fault::Event(r));false}
}
fn accumulate<C:Carrier>(c:&mut Memo<C>,out:&mut Output,g:usize,e:Id) {
    out.events[g]=Some(match out.events[g] {None=>e,Some(old)=>c.op(14,old,e)});
}
