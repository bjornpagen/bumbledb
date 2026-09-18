//! Scoped/anchored admission adapter for the sparse-selector candidate.
//! The three modes share exactly the same owner/support/transport contract.
use super::carrier::*;
use super::range_raw::{Arena,Mode,Kernel,Ref};
use super::observation::{CountPlan,Spectrum};
use super::transfer::{Builder,Packet};
use rustc_hash::FxHashMap;

#[derive(Clone)]
pub struct RangeCarrier<const M:u8> {
    raw:Arena,n:usize,order:Vec<u32>,support:Ref,anchor:u64,population:u64,
}
fn axes(mut mask:u64)->Vec<u32> {
    let mut out=Vec::new();while mask!=0 {let v=mask.trailing_zeros();out.push(v);mask&=mask-1;}out
}
impl<const M:u8> RangeCarrier<M> {
    pub fn symbolic(dimensions:u32,order:Vec<u32>)->Self {
        assert!(M<=2 && dimensions<63);assert_eq!(order.len(),dimensions as usize);
        validate_order(dimensions,&order);
        Self {raw:Arena::with_kernel(order.clone(),if M==0 {Mode::Plain} else {Mode::Ranges},
                if M==2 {Kernel::Groups} else {Kernel::Steps}),
            n:1usize<<dimensions,order,support:1,anchor:0,population:1u64<<dimensions}
    }
    fn seal(&mut self,raw:Ref)->Id {
        let selected=self.raw.apply(8,self.support,raw);
        let flip=self.raw.evaluate(selected,self.anchor);
        let representative=if flip {self.raw.apply(4,self.support,selected)} else {selected};
        (representative as Id)<<1 | flip as Id
    }
    fn true_root(&mut self,event:Id)->Ref {
        let r=(event>>1) as Ref;
        if event&1==0 {r} else {self.raw.apply(4,self.support,r)}
    }
    fn local_table(&mut self,coordinates:&[u32],words:&[u64])->Ref {
        fn visit<const M:u8>(c:&mut RangeCarrier<M>,coordinates:&[u32],words:&[u64],base:usize)->Ref {
            if coordinates.is_empty() {return at(words,base) as Ref;}
            let v=*c.order.iter().find(|v|coordinates.contains(v)).unwrap();
            let pos=coordinates.iter().position(|&w|w==v).unwrap();
            let mut lo=Vec::new();let mut hi=Vec::new();
            let insert=|w:usize,b:usize|(w&((1<<pos)-1))|(b<<pos)|((w>>pos)<<(pos+1));
            // Local tables only: no ambient cube is allocated for symbolic owners.
            let cells=1usize<<(coordinates.len()-1);lo.resize(cells.div_ceil(64),0);hi.resize(cells.div_ceil(64),0);
            for w in 0..cells {
                lo[w/64]|=(at(words,insert(w,0)) as u64)<<(w%64);
                hi[w/64]|=(at(words,insert(w,1)) as u64)<<(w%64);
            }
            let rest:Vec<_>=coordinates.iter().copied().filter(|&w|w!=v).collect();
            let lo=visit(c,&rest,&lo,0);let hi=visit(c,&rest,&hi,0);
            c.raw.block(1<<v,lo,hi)
        }
        visit(self,coordinates,words,0)
    }
    fn transport(&self,r:Ref,out:&mut Builder,memo:&mut FxHashMap<Ref,Ref>)->Ref {
        if r<2 {return r;}
        let regular=r&!1;if let Some(&v)=memo.get(&regular) {return v^(r&1);}
        let [lo,hi]=self.raw.edges(regular);let selected=self.raw.selector(regular);
        let mut lo=self.transport(lo,out,memo);let hi=self.transport(hi,out,memo);
        for &v in self.order.iter().rev() {
            if selected>>v&1!=0 {lo=out.branch(v,lo,hi);}
        }
        memo.insert(regular,lo);lo^(r&1)
    }
    /// Exact coefficient vector by successes in named parameter groups.
    /// Selector and child coordinates are disjoint, not their parameter names.
    fn spectrum_local(&self,r:Ref,plan:&CountPlan,memo:&mut FxHashMap<Ref,Spectrum>)->Spectrum {
        if r<2 {return if r==0 {plan.zero()} else {plan.one()};}
        if let Some(s)=memo.get(&r) {return s.clone();}
        let [lo,hi]=self.raw.edges(r);let selected=self.raw.selector(r);
        let children=self.raw.variables(lo)|self.raw.variables(hi);
        let mut l=self.spectrum_local(lo,plan,memo);let mut h=self.spectrum_local(hi,plan,memo);
        plan.smooth(&mut l,axes(children&!self.raw.variables(lo)));
        plan.smooth(&mut h,axes(children&!self.raw.variables(hi)));
        let mut all=h.clone();plan.smooth(&mut all,axes(selected));
        // All selected assignments minus the unique all-zero assignment.
        for i in 0..all.len() {all[i]=all[i]-h[i]+l[i];}
        memo.insert(r,all.clone());all
    }
    fn spectrum_full(&self,r:Ref,plan:&CountPlan,memo:&mut FxHashMap<Ref,Spectrum>)->Spectrum {
        let mut out=self.spectrum_local(r,plan,memo);
        plan.smooth(&mut out,(0..self.dimensions()).filter(|&v|self.raw.variables(r)>>v&1==0));out
    }
}
impl<const M:u8> RegionOps for RangeCarrier<M> {
    const NAME:&'static str=match M {0=>"range-shannon",1=>"range-step",2=>"range-group",_=>"range-invalid"};
    fn dimensions(&self)->u32 {self.order.len() as u32}
    fn variable(&mut self,v:u32)->Id {assert!(v<self.dimensions());let r=self.raw.variable(v);self.seal(r)}
    fn import(&mut self,input:&[u64])->Id {
        assert!(self.dimensions()<=20,"RESOURCE_CAP: explicit range import");
        let cells=1usize<<self.dimensions();let mut words=input.to_vec();words.resize(cells.div_ceil(64),0);
        let r=self.local_table(&(0..self.dimensions()).collect::<Vec<_>>(),&words);self.seal(r)
    }
    fn import_table(&mut self,coords:&[u32],words:&[u64])->Option<Id> {
        if coords.len()>20 || words.len()!=(1usize<<coords.len()).div_ceil(64) {return None;}
        let mut mask=0;
        for &v in coords {if v>=self.dimensions() || mask>>v&1!=0 {return None;}mask|=1u64<<v;}
        let r=self.local_table(coords,words);Some(self.seal(r))
    }
    fn op(&mut self,op:u8,a:Id,b:Id)->Id {
        assert!(op<16);if let Some(r)=trivial(op,a,b) {return r;}
        let mut code=0;
        for cell in 0..4 {
            let old=cell ^ ((a&1) as u8*2) ^ (b&1) as u8;
            code|=((op>>old)&1)<<cell;
        }
        let flip=code&1;if flip!=0 {code^=15;}
        let r=self.raw.apply(code,(a>>1) as Ref,(b>>1) as Ref);(r as Id)<<1 | flip as Id
    }
    fn not(&mut self,a:Id)->Id {a^1}
    fn empty(&self)->Id {0}
    fn full(&self)->Id {1}
    fn export(&self,a:Id)->Vec<u64> {
        assert!(self.dimensions()<=20,"RESOURCE_CAP: explicit range export");
        bits(self.n,|w|self.raw.evaluate(self.support,w as u64) &&
            (self.raw.evaluate((a>>1) as Ref,w as u64) ^ (a&1!=0)))
    }
    fn exists(&mut self,a:Id,hidden:u64)->Id {
        let r=self.true_root(a);let r=self.raw.exists(r,hidden);self.seal(r)
    }
    fn permute(&mut self,a:Id,map:&Permutation)->Id {
        assert_eq!(map.destinations.len(),self.order.len());
        let r=self.true_root(a);let selectors:Vec<_>=map.destinations.iter().map(|&v|self.raw.variable(v)).collect();
        let r=self.raw.substitute(r,&selectors);self.seal(r)
    }
    fn bytes(&self)->usize {self.raw.bytes()+self.order.capacity()*4+std::mem::size_of::<Self>()}
    fn nodes(&self)->usize {self.raw.nodes()}
    fn count(&self,a:Id)->u64 {
        let n=self.raw.raw_uniform_count((a>>1) as Ref);if a&1==0 {n} else {self.population-n}
    }
    fn packet(&self,names:&[u64],roots:&[Id])->Packet {
        assert_eq!(names.len(),self.order.len());let mut out=Builder::new(names.to_vec(),self.order.clone());
        let mut memo=FxHashMap::default();let support=self.transport(self.support,&mut out,&mut memo);
        let roots=roots.iter().map(|&r|self.transport((r>>1) as Ref,&mut out,&mut memo) ^ (r&1) as Ref).collect();
        out.finish(support,roots)
    }
    fn prepare_spectrum(&self,plan:&mut CountPlan) {assert_eq!(plan.group_of.len(),self.order.len());}
    fn spectrum(&self,a:Id,plan:&CountPlan)->Spectrum {
        assert_eq!(plan.group_of.len(),self.order.len());let mut memo=FxHashMap::default();
        let part=self.spectrum_full((a>>1) as Ref,plan,&mut memo);
        if a&1==0 {part} else {plan.subtract(self.spectrum_full(self.support,plan,&mut memo),&part)}
    }
}
impl<const M:u8> Carrier for RangeCarrier<M> {
    fn full_space(dimensions:u32,order:Vec<u32>)->Result<Self,&'static str> {Ok(Self::symbolic(dimensions,order))}
    fn legal_space(domains:&super::legal::Domains,order:Vec<u32>)->Result<Self,&'static str> {
        let mut out=Self::symbolic(domains.dimensions(),order);let full=domains.formula(&mut out);
        let support=out.true_root(full);let population=out.raw.raw_uniform_count(support);
        assert_eq!(population,domains.population());let anchor=domains.anchor();assert!(out.raw.evaluate(support,anchor));
        out.support=support;out.population=population;out.anchor=anchor;Ok(out)
    }
    fn new(support:&[u64],n:usize)->Self {
        Self::with_order(support,n,(0..n.next_power_of_two().trailing_zeros()).rev().collect())
    }
    fn with_order(support:&[u64],n:usize,order:Vec<u32>)->Self {
        let mut out=Self::symbolic(n.next_power_of_two().trailing_zeros(),order);
        assert_eq!(support.len(),n.div_ceil(64));let mut words=support.to_vec();
        if n%64!=0 {*words.last_mut().unwrap()&=(1u64<<(n%64))-1;}
        out.population=words.iter().map(|v|v.count_ones() as u64).sum();
        out.anchor=words.iter().enumerate().find_map(|(i,&w)|(w!=0).then(||(i*64+w.trailing_zeros() as usize) as u64)).expect("nonempty support");
        words.resize((1usize<<out.dimensions()).div_ceil(64),0);
        out.support=out.local_table(&(0..out.dimensions()).collect::<Vec<_>>(),&words);out.n=n;out
    }
}
