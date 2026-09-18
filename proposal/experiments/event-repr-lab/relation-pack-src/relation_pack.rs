//! Two typed relational reductions over the certified scalar binding product.
use super::carrier::*;
use super::legal::{Checked,Relation};
use super::native::{Native,Row};
use super::pack::Schedule;
use std::collections::{BTreeMap,BTreeSet};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Program {Compose,Residual}
impl Program {
    pub fn name(self)->&'static str {match self {Self::Compose=>"compose",Self::Residual=>"residual"}}
    pub fn output_op(self)->u8 {if self==Self::Compose {14} else {8}}
    pub fn right_op(self)->u8 {self.output_op()}
    fn compute<C:RegionOps>(self,c:&mut Checked<C>,a:Relation,b:Relation)->Relation {
        match self {Self::Compose=>c.compose(a,b),Self::Residual=>c.residual(a,b)}.unwrap()
    }
}
#[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord)]
pub enum Fault {Group(u64),Scope(u64),Event(Id),Role(Id),Multiplicity}
#[derive(Debug)]
pub struct Output {
    pub events:Vec<Option<Id>>,pub represented_bindings:usize,pub native_emissions:usize,
    pub branch_emissions:usize,pub summary_rows:usize,pub staged_capacity_bytes:usize,pub summary_colt_bytes:usize,
}
impl Output {
    pub fn present(&self)->usize {self.events.iter().filter(|v|v.is_some()).count()}
    pub fn checksum<C:RegionOps>(&self,c:&C)->u64 {
        self.events.iter().enumerate().map(|(g,r)|r.map_or(0,|r|c.count(r)*(g as u64+1))).sum()
    }
}
pub struct Executor {schedule:Schedule,complete:Option<Native>,branches:Vec<Native>}
impl Executor {
    pub fn new(data:&[Vec<Row>;2],schedule:Schedule)->Self {
        // Certificate comes from precisely the normalized IR used for this plan.
        let complete=Native::pair(data);
        assert_eq!(complete.separator().expect("pair query must have a separator").key_count(),2);
        match schedule {
            Schedule::Complete=>Self{schedule,complete:Some(complete),branches:vec![]},
            Schedule::Factorized=>Self{schedule,complete:None,branches:data.iter().map(|r|Native::single(r)).collect()},
        }
    }
    pub fn prime(&mut self) {
        if let Some(c)=&mut self.complete {c.run(|_|{});}
        for b in &mut self.branches {b.run(|_|{});}
    }
    pub fn colt_bytes(&self)->usize {
        self.complete.as_ref().map_or(0,Native::retained_colt_bytes)+self.branches.iter().map(Native::retained_colt_bytes).sum::<usize>()
    }
    pub fn run<C:RegionOps>(&mut self,c:&mut Checked<C>,groups:usize,published:&BTreeSet<Id>,program:Program)->Result<Output,Vec<Fault>> {
        let mut out=Output{events:vec![None;groups],represented_bindings:0,native_emissions:0,branch_emissions:0,
            summary_rows:0,staged_capacity_bytes:0,summary_colt_bytes:0};
        let mut faults=BTreeSet::new();
        if self.schedule==Schedule::Complete {
            self.complete.as_mut().unwrap().run(|[g,scope,a,b,_]| {
                out.native_emissions+=1;add_count(&mut out,1,&mut faults);
                let ok=validate_key(g,scope,groups,&mut faults);
                let ra=resolve(c,a,published,&mut faults);let rb=resolve(c,b,published,&mut faults);
                if let (true,Some(a),Some(b))=(ok,ra,rb) {
                    let r=program.compute(c,a,b);accumulate(c,&mut out,g as usize,r,program);
                }
            });
        } else {
            let mut staged:BTreeMap<(u64,u64),[Vec<Id>;2]>=BTreeMap::new();
            for (i,branch) in self.branches.iter_mut().enumerate() {
                branch.run(|[g,scope,r,_,_]| {
                    staged.entry((g,scope)).or_default()[i].push(r);out.branch_emissions+=1;out.native_emissions+=1;
                });
            }
            let mut summaries:[Vec<Row>;2]=std::array::from_fn(|_|vec![]);let mut counts=BTreeMap::new();
            for (&(g,scope),branches) in &staged {
                out.staged_capacity_bytes+=branches.iter().map(|b|b.capacity()*8).sum::<usize>();
                if branches.iter().any(Vec::is_empty) {continue;}
                let mut ok=validate_key(g,scope,groups,&mut faults);
                let mut resolved:[Vec<Relation>;2]=std::array::from_fn(|_|vec![]);
                for (i,rs) in branches.iter().enumerate() {for &r in rs {
                    match resolve(c,r,published,&mut faults) {Some(r)=>resolved[i].push(r),None=>ok=false}
                }}
                // Include this additional staging explicitly; this is a capacity
                // census, not peak query memory (maps/images/plans also allocate).
                out.staged_capacity_bytes+=resolved.iter().map(|b|b.capacity()*std::mem::size_of::<Relation>()).sum::<usize>();
                let n=branches[0].len().checked_mul(branches[1].len());
                if n.is_none() {faults.insert(Fault::Multiplicity);ok=false;}
                if !ok {continue;}
                counts.insert((g,scope),n.unwrap());
                for (i,rs) in resolved.iter().enumerate() {
                    let op=if i==0 {14} else {program.right_op()};
                    let mut r=if op==14 {c.empty()} else {c.full()};
                    for &v in rs {r=c.boolean(op,r,v).unwrap();}
                    summaries[i].push([g,0,scope,c.root(r).unwrap()]);out.summary_rows+=1;
                }
            }
            let mut finish=Native::pair(&summaries);
            finish.run(|[g,scope,a,b,_]| {
                out.native_emissions+=1;add_count(&mut out,counts[&(g,scope)],&mut faults);
                let a=c.admit(a).unwrap();let b=c.admit(b).unwrap();
                let r=program.compute(c,a,b);accumulate(c,&mut out,g as usize,r,program);
            });
            out.summary_colt_bytes=finish.retained_colt_bytes();
        }
        if faults.is_empty() {Ok(out)} else {Err(faults.into_iter().collect())}
    }
}
fn add_count(out:&mut Output,n:usize,faults:&mut BTreeSet<Fault>) {
    match out.represented_bindings.checked_add(n) {Some(n)=>out.represented_bindings=n,None=>{faults.insert(Fault::Multiplicity);}}
}
fn validate_key(g:u64,scope:u64,groups:usize,faults:&mut BTreeSet<Fault>)->bool {
    let mut ok=true;
    if g>=groups as u64 {faults.insert(Fault::Group(g));ok=false;}
    if scope!=1 {faults.insert(Fault::Scope(scope));ok=false;}
    ok
}
fn resolve<C:RegionOps>(c:&mut Checked<C>,r:Id,published:&BTreeSet<Id>,faults:&mut BTreeSet<Fault>)->Option<Relation> {
    if !published.contains(&r) {faults.insert(Fault::Event(r));return None;}
    match c.admit(r) {Ok(r)=>Some(r),Err(_)=>{faults.insert(Fault::Role(r));None}}
}
fn accumulate<C:RegionOps>(c:&mut Checked<C>,out:&mut Output,g:usize,r:Relation,p:Program) {
    let r=match out.events[g] {Some(old)=>{let old=c.admit(old).unwrap();c.boolean(p.output_op(),old,r).unwrap()},None=>r};
    out.events[g]=Some(c.root(r).unwrap());
}
