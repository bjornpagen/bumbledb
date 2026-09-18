//! Complete, staged and scalar-participation-index schedules of typed relations.
use super::carrier::*;
use super::legal::{Checked,Relation};
use super::native::{Native,Row};
use std::collections::{BTreeMap,BTreeSet};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Schedule {Complete,Staged,IndexRight,IndexSmall}
impl Schedule {
    pub fn name(self)->&'static str {match self {Self::Complete=>"complete",Self::Staged=>"staged",Self::IndexRight=>"index-right",Self::IndexSmall=>"index-small"}}
}
#[derive(Default)]
struct Entry {counts:[usize;2],folds:[Option<Relation>;2],bad:bool,count_overflow:bool}
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
    pub map_entries:usize,pub map_payload_bytes:usize,pub roster_branch:Option<usize>,
}
impl Output {
    pub fn present(&self)->usize {self.events.iter().filter(|v|v.is_some()).count()}
    pub fn checksum<C:RegionOps>(&self,c:&C)->u64 {
        self.events.iter().enumerate().map(|(g,r)|r.map_or(0,|r|c.count(r)*(g as u64+1))).sum()
    }
}
pub struct Executor {schedule:Schedule,complete:Option<Native>,branches:Vec<Native>,small_branch:usize}
impl Executor {
    pub fn new(data:&[Vec<Row>;2],schedule:Schedule)->Self {
        let complete=Native::pair(data);
        assert_eq!(complete.separator().expect("pair query must have a separator").key_count(),2);
        // An input-row estimate only. Whole-row duplicates can change which
        // side is actually smaller, but cannot make either choice unsound.
        let small_branch=if data[0].len()<data[1].len() {0} else {1};
        match schedule {
            Schedule::Complete=>Self{schedule,complete:Some(complete),branches:vec![],small_branch},
            _=>Self{schedule,complete:None,branches:data.iter().map(|r|Native::single(r)).collect(),small_branch},
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
            summary_rows:0,staged_capacity_bytes:0,summary_colt_bytes:0,map_entries:0,map_payload_bytes:0,roster_branch:None};
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
        } else if self.schedule==Schedule::Staged {
            let mut staged:BTreeMap<(u64,u64),[Vec<Id>;2]>=BTreeMap::new();
            for (i,branch) in self.branches.iter_mut().enumerate() {
                branch.run(|[g,scope,r,_,_]| {
                    staged.entry((g,scope)).or_default()[i].push(r);out.branch_emissions+=1;out.native_emissions+=1;
                });
            }
            let mut summaries:[Vec<Row>;2]=std::array::from_fn(|_|vec![]);let mut counts=BTreeMap::new();
            out.map_entries=staged.len();out.map_payload_bytes=staged.len()*(std::mem::size_of::<(u64,u64)>()+std::mem::size_of::<[Vec<Id>;2]>());
            for (&(g,scope),branches) in &staged {
                out.staged_capacity_bytes+=branches.iter().map(|b|b.capacity()*8).sum::<usize>();
                if branches.iter().any(Vec::is_empty) {continue;}
                let mut ok=validate_key(g,scope,groups,&mut faults);
                let mut resolved:[Vec<Relation>;2]=std::array::from_fn(|_|vec![]);
                for (i,rs) in branches.iter().enumerate() {for &r in rs {
                    match resolve(c,r,published,&mut faults) {Some(r)=>resolved[i].push(r),None=>ok=false}
                }}
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
            out.map_payload_bytes+=counts.len()*(std::mem::size_of::<(u64,u64)>()+std::mem::size_of::<usize>());
            let mut finish=Native::pair(&summaries);
            finish.run(|[g,scope,a,b,_]| {
                out.native_emissions+=1;add_count(&mut out,counts[&(g,scope)],&mut faults);
                let a=c.admit(a).unwrap();let b=c.admit(b).unwrap();
                let r=program.compute(c,a,b);accumulate(c,&mut out,g as usize,r,program);
            });
            out.summary_colt_bytes=finish.retained_colt_bytes();
        } else {
            let first=if self.schedule==Schedule::IndexSmall {self.small_branch} else {1};
            let second=1-first;out.roster_branch=Some(first);
            let mut index:BTreeMap<(u64,u64),Entry>=BTreeMap::new();
            // No Event is resolved during this roster-only first pass.
            self.branches[first].run(|[g,scope,_,_,_]| {
                out.branch_emissions+=1;out.native_emissions+=1;
                increment(index.entry((g,scope)).or_default(),first);
            });
            self.branches[second].run(|[g,scope,r,_,_]| {
                out.branch_emissions+=1;out.native_emissions+=1;
                if let Some(e)=index.get_mut(&(g,scope)) {
                    increment(e,second);reduce_row(c,e,second,g,scope,r,groups,published,program,&mut faults);
                }
            });
            self.branches[first].run(|[g,scope,r,_,_]| {
                out.branch_emissions+=1;out.native_emissions+=1;
                let e=index.get_mut(&(g,scope)).unwrap();
                if e.counts[second]>0 {reduce_row(c,e,first,g,scope,r,groups,published,program,&mut faults);}
            });
            out.map_entries=index.len();out.map_payload_bytes=index.len()*(std::mem::size_of::<(u64,u64)>()+std::mem::size_of::<Entry>());
            let mut summaries:[Vec<Row>;2]=std::array::from_fn(|_|vec![]);
            for (&(g,scope),e) in &mut index {
                if e.counts[second]==0 {continue;}
                if e.count_overflow || e.counts[0].checked_mul(e.counts[1]).is_none() {faults.insert(Fault::Multiplicity);e.bad=true;}
                if e.bad {continue;}
                for i in 0..2 {summaries[i].push([g,0,scope,c.root(e.folds[i].unwrap()).unwrap()]);out.summary_rows+=1;}
            }
            let mut finish=Native::pair(&summaries);
            finish.run(|[g,scope,a,b,_]| {
                out.native_emissions+=1;let e=&index[&(g,scope)];add_count(&mut out,e.counts[0]*e.counts[1],&mut faults);
                let a=c.admit(a).unwrap();let b=c.admit(b).unwrap();let r=program.compute(c,a,b);accumulate(c,&mut out,g as usize,r,program);
            });
            out.summary_colt_bytes=finish.retained_colt_bytes();
        }
        if faults.is_empty() {Ok(out)} else {Err(faults.into_iter().collect())}
    }
}
fn increment(e:&mut Entry,branch:usize) {
    match e.counts[branch].checked_add(1) {Some(n)=>e.counts[branch]=n,None=>e.count_overflow=true}
}
fn reduce_row<C:RegionOps>(c:&mut Checked<C>,e:&mut Entry,branch:usize,g:u64,scope:u64,r:Id,groups:usize,published:&BTreeSet<Id>,program:Program,faults:&mut BTreeSet<Fault>) {
    let key_ok=validate_key(g,scope,groups,faults);
    let resolved=resolve(c,r,published,faults); // Never skip later participating faults.
    e.bad|=!key_ok || resolved.is_none();
    if !e.bad {
        let op=if branch==0 {14} else {program.right_op()};
        let old=e.folds[branch].unwrap_or_else(||if op==14 {c.empty()} else {c.full()});
        e.folds[branch]=Some(c.boolean(op,old,resolved.unwrap()).unwrap());
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
