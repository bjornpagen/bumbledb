//! Raw sparse shared-exit selectors. No native Event/Free Join claim.
use rustc_hash::{FxHashMap,FxHashSet};
pub type Ref=u32;
const RANGE:u64=1<<63;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Mode {Plain,Ranges}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Kernel {Steps,Groups}
#[repr(C)]
#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash)]
struct Node {variables:u64,edges:[Ref;2]}
const _:()=assert!(std::mem::size_of::<Node>()==16);
#[derive(Clone)]
pub struct Arena {
    order:Vec<u32>,rank:Vec<usize>,mode:Mode,kernel:Kernel,
    nodes:Vec<Node>,unique:FxHashMap<Node,Ref>,
    applications:FxHashMap<(u8,Ref,Ref),Ref>,
    cofactors:FxHashMap<(Ref,u32,bool),Ref>,projections:FxHashMap<(Ref,u64),Ref>,
    pub apply_misses:usize,pub cofactor_misses:usize,pub projection_misses:usize,
    pub grouped_splits:usize,pub grouped_axes:usize,
}
fn trivial(op:u8,a:Ref,b:Ref)->Option<Ref> {
    let unary=|x,code:u8|match code&3 {0=>0,1=>x^1,2=>x,_=>1};
    Some(match op {
        0=>0,15=>1,12=>a,10=>b,3=>a^1,5=>b^1,
        _ if a<2=>unary(b,op>>(2*a)),
        _ if b<2=>unary(a,((op>>b)&1)|(((op>>(2+b))&1)<<1)),
        _ if a==b=>unary(a,(op&1)|((op>>2)&2)),
        _ if a==(b^1)=>unary(a,op>>1),_=>return None,
    })
}
impl Arena {
    pub fn new(order:Vec<u32>,mode:Mode)->Self {Self::with_kernel(order,mode,Kernel::Steps)}
    pub fn with_kernel(order:Vec<u32>,mode:Mode,kernel:Kernel)->Self {
        assert!(order.len()<63);let mut rank=vec![usize::MAX;order.len()];
        for (i,&v) in order.iter().enumerate() {
            assert!((v as usize)<order.len() && rank[v as usize]==usize::MAX);rank[v as usize]=i;
        }
        Self {order,rank,mode,kernel,nodes:vec![Node{variables:0,edges:[0,0]}],
            unique:FxHashMap::default(),applications:FxHashMap::default(),cofactors:FxHashMap::default(),
            projections:FxHashMap::default(),apply_misses:0,cofactor_misses:0,projection_misses:0,
            grouped_splits:0,grouped_axes:0}
    }
    pub fn variables(&self,r:Ref)->u64 {self.nodes[(r/2) as usize].variables&!RANGE}
    fn top(&self,mask:u64)->u32 {*self.order.iter().find(|&&v|mask>>v&1!=0).expect("nonconstant")}
    pub(super) fn selector(&self,r:Ref)->u64 {
        if r<2 {return 0;}
        let node=self.nodes[(r/2) as usize];
        // Exact dependencies recover a sparse selector without a label arena.
        self.variables(r)&!(self.variables(node.edges[0])|self.variables(node.edges[1]))
    }
    pub(super) fn edges(&self,r:Ref)->[Ref;2] {self.nodes[(r/2) as usize].edges.map(|e|e^(r&1))}
    fn intern(&mut self,node:Node)->Ref {
        if let Some(&r)=self.unique.get(&node) {return r;}
        assert!(self.nodes.len()<2_000_000,"RESOURCE_CAP: range nodes");
        let r=self.nodes.len() as Ref*2;self.nodes.push(node);self.unique.insert(node,r);r
    }
    fn mk(&mut self,v:u32,lo:Ref,hi:Ref)->Ref {self.block(1<<v,lo,hi)}
    /// IF any selected bit THEN hi ELSE lo. All selected coordinates precede
    /// both children. Normalize low before the deterministic maximal merge.
    pub(super) fn block(&mut self,mut axes:u64,mut lo:Ref,mut hi:Ref)->Ref {
        if axes==0 || lo==hi {return lo;}
        let deps=self.variables(lo)|self.variables(hi);
        assert_eq!(axes&deps,0);
        debug_assert!(self.order.iter().filter(|&&v|axes>>v&1!=0)
            .all(|&v|self.order.iter().take(self.rank[v as usize]).all(|&w|deps>>w&1==0)));
        if self.mode==Mode::Plain && axes.count_ones()>1 {
            let vars:Vec<_>=self.order.iter().copied().filter(|&v|axes>>v&1!=0).collect();
            for v in vars.into_iter().rev() {lo=self.mk(v,lo,hi);}return lo;
        }
        let flip=lo&1;lo^=flip;hi^=flip;
        if self.mode==Mode::Ranges && lo>=2 {
            let child=self.nodes[(lo/2) as usize];
            if child.edges[1]==hi {axes|=self.selector(lo);lo=child.edges[0];}
        }
        let variables=axes|self.variables(lo)|self.variables(hi);
        let tag=if axes.count_ones()>1 {RANGE} else {0};
        self.intern(Node{variables:variables|tag,edges:[lo,hi]})^flip
    }
    pub fn variable(&mut self,v:u32)->Ref {self.mk(v,0,1)}
    /// Two OR-selector cofactors; an absent operand is constant on the segment.
    fn split(&mut self,r:Ref,axes:u64)->[Ref;2] {
        if self.variables(r)&axes==0 {return [r,r];}
        let selected=self.selector(r);assert_eq!(axes&selected,axes);
        let [lo,hi]=self.edges(r);
        [self.block(selected&!axes,lo,hi),hi]
    }
    fn children(&mut self,r:Ref)->[Ref;2] {self.split(r,1<<self.top(self.variables(r)))}
    fn segment(&self,a:Ref,b:Ref)->u64 {
        let da=self.variables(a);let db=self.variables(b);let v=self.top(da|db);
        if self.kernel==Kernel::Steps {return 1<<v;}
        let sa=self.selector(a);let sb=self.selector(b);
        if sa>>v&1!=0 && sb>>v&1!=0 {
            let different=sa^sb;
            let stop=if different==0 {usize::MAX} else {self.rank[self.top(different) as usize]};
            self.order.iter().take(stop.min(self.order.len())).fold(0,|m,&w|m|((sa&sb)&(1<<w)))
        } else {
            let (s,other)=if sa>>v&1!=0 {(sa,db)} else {(sb,da)};
            let stop=if other==0 {self.order.len()} else {self.rank[self.top(other) as usize]};
            self.order.iter().take(stop).fold(0,|m,&w|m|(s&(1<<w)))
        }
    }
    pub fn apply(&mut self,mut op:u8,mut a:Ref,mut b:Ref)->Ref {
        assert!(op<16);if let Some(r)=trivial(op,a,b) {return r;}
        if a>b {std::mem::swap(&mut a,&mut b);op=(op&9)|((op&2)<<1)|((op&4)>>1);}
        if let Some(&r)=self.applications.get(&(op,a,b)) {return r;}
        self.apply_misses+=1;
        let axes=self.segment(a,b);assert_ne!(axes,0);
        if axes.count_ones()>1 {self.grouped_splits+=1;self.grouped_axes+=axes.count_ones() as usize;}
        let aa=self.split(a,axes);let bb=self.split(b,axes);
        let lo=self.apply(op,aa[0],bb[0]);let hi=self.apply(op,aa[1],bb[1]);
        let r=self.block(axes,lo,hi);self.applications.insert((op,a,b),r);r
    }
    pub fn cofactor(&mut self,r:Ref,v:u32,high:bool)->Ref {
        if self.variables(r)>>v&1==0 {return r;}
        let regular=r&!1;
        if let Some(&out)=self.cofactors.get(&(regular,v,high)) {return out^(r&1);}
        self.cofactor_misses+=1;
        let out=if self.kernel==Kernel::Groups {
            let axes=self.selector(regular);let [lo,hi]=self.edges(regular);
            if axes>>v&1!=0 {if high {hi} else {self.block(axes&!(1<<v),lo,hi)}} else {
                let lo=self.cofactor(lo,v,high);let hi=self.cofactor(hi,v,high);self.block(axes,lo,hi)
            }
        } else {
            let top=self.top(self.variables(r));let [lo,hi]=self.children(regular);
            if top==v {if high {hi} else {lo}} else {
                let lo=self.cofactor(lo,v,high);let hi=self.cofactor(hi,v,high);self.mk(top,lo,hi)
            }
        };
        self.cofactors.insert((regular,v,high),out);out^(r&1)
    }
    pub fn exists(&mut self,r:Ref,hidden:u64)->Ref {
        let hidden=hidden&self.variables(r);if hidden==0 {return r;}
        if let Some(&out)=self.projections.get(&(r,hidden)) {return out;}
        self.projection_misses+=1;
        let axes=if self.kernel==Kernel::Groups {self.selector(r)} else {1<<self.top(self.variables(r))};
        let [lo,hi]=self.split(r,axes);
        let lo=self.exists(lo,hidden&!axes);let hi=self.exists(hi,hidden&!axes);
        let lo=if hidden&axes!=0 {self.apply(14,lo,hi)} else {lo};
        let out=self.block(axes&!hidden,lo,hi);self.projections.insert((r,hidden),out);out
    }
    pub fn substitute(&mut self,r:Ref,selectors:&[Ref])->Ref {
        assert_eq!(selectors.len(),self.order.len());
        fn visit(c:&mut Arena,r:Ref,s:&[Ref],memo:&mut FxHashMap<Ref,Ref>)->Ref {
            if r<2 {return r;}
            let regular=r&!1;if let Some(&out)=memo.get(&regular) {return out^(r&1);}
            let v=c.top(c.variables(r));let [a,b]=c.children(regular);
            let a=visit(c,a,s,memo);let b=visit(c,b,s,memo);
            let lo=c.apply(8,s[v as usize]^1,a);let hi=c.apply(8,s[v as usize],b);
            let out=c.apply(14,lo,hi);memo.insert(regular,out);out^(r&1)
        }
        visit(self,r,selectors,&mut FxHashMap::default())
    }
    pub fn evaluate(&self,mut r:Ref,world:u64)->bool {
        while r>=2 {let axes=self.selector(r);r=self.edges(r)[(world&axes!=0) as usize];}r!=0
    }
    pub fn import(&mut self,truth:&[bool])->Ref {
        assert_eq!(truth.len(),1<<self.order.len());
        fn visit(c:&mut Arena,t:&[bool],remaining:&[u32])->Ref {
            if remaining.is_empty() {return t[0] as Ref;}
            let v=*c.order.iter().find(|v|remaining.contains(v)).unwrap();
            let p=remaining.iter().position(|&w|w==v).unwrap();
            let insert=|w:usize,b:usize|(w&((1<<p)-1))|(b<<p)|((w>>p)<<(p+1));
            let lo:Vec<_>=(0..t.len()/2).map(|i|t[insert(i,0)]).collect();
            let hi:Vec<_>=(0..t.len()/2).map(|i|t[insert(i,1)]).collect();
            let rest:Vec<_>=remaining.iter().copied().filter(|&w|w!=v).collect();
            let a=visit(c,&lo,&rest);let b=visit(c,&hi,&rest);c.mk(v,a,b)
        }
        let axes:Vec<_>=(0..self.order.len() as u32).collect();visit(self,truth,&axes)
    }
    /// FULL raw-cube count; no assumption about a designated probability law.
    pub fn raw_uniform_count(&self,r:Ref)->u64 {
        fn visit(c:&Arena,r:Ref,memo:&mut FxHashMap<Ref,u64>)->u64 {
            if r<2 {return r as u64;}
            let regular=r&!1;let dims=c.variables(r).count_ones();
            let value=if let Some(&n)=memo.get(&regular) {n} else {
                let k=c.selector(regular).count_ones();let [lo,hi]=c.edges(regular);
                let l=visit(c,lo,memo)<<(dims-k-c.variables(lo).count_ones());
                let h=visit(c,hi,memo)<<(dims-k-c.variables(hi).count_ones());
                let value=l+((1u64<<k)-1)*h;memo.insert(regular,value);value
            };
            if r&1!=0 {(1u64<<dims)-value} else {value}
        }
        visit(self,r,&mut FxHashMap::default())<<(self.order.len() as u32-self.variables(r).count_ones())
    }
    pub fn compact(&self,roots:&[Ref])->(Self,Vec<Ref>) {
        fn copy(src:&Arena,dst:&mut Arena,r:Ref,map:&mut FxHashMap<Ref,Ref>)->Ref {
            if r<2 {return r;}
            let regular=r&!1;if let Some(&out)=map.get(&regular) {return out^(r&1);}
            let mut node=src.nodes[(r/2) as usize];
            node.edges=node.edges.map(|e|copy(src,dst,e,map));
            let out=dst.intern(node);map.insert(regular,out);out^(r&1)
        }
        let mut out=Self::with_kernel(self.order.clone(),self.mode,self.kernel);let mut map=FxHashMap::default();
        let roots=roots.iter().map(|&r|copy(self,&mut out,r,&mut map)).collect();(out,roots)
    }
    pub fn nodes(&self)->usize {self.nodes.len()}
    pub fn reachable(&self,roots:&[Ref])->usize {
        let mut seen=FxHashSet::default();let mut pending=roots.to_vec();
        while let Some(r)=pending.pop() {
            if r<2 || !seen.insert(r/2) {continue;}pending.extend(self.nodes[(r/2) as usize].edges);
        }seen.len()
    }
    pub fn bytes(&self)->usize {
        self.nodes.capacity()*16+self.unique.capacity()*21
            +self.applications.capacity()*(std::mem::size_of::<((u8,Ref,Ref),Ref)>()+1)
            +self.cofactors.capacity()*(std::mem::size_of::<((Ref,u32,bool),Ref)>()+1)
            +self.projections.capacity()*(std::mem::size_of::<((Ref,u64),Ref)>()+1)
            +self.order.capacity()*4+self.rank.capacity()*std::mem::size_of::<usize>()
    }
}

