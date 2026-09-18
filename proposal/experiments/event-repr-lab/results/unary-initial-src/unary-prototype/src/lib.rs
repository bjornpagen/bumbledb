//! Ordered Shannon diagrams with a competing inline unary-chain record.
//! This is a raw manager: no Event ownership/transport or native Free Join yet.
use rustc_hash::{FxHashMap,FxHashSet};

pub type Ref=u32;
const CHAIN:u64=1<<63;
const MAX_CHAIN:u32=10;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Mode { Plain, Chains }

#[repr(C)]
#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash)]
struct Node { variables:u64, edges:[Ref;2] }
const _:()=assert!(std::mem::size_of::<Node>()==16);

#[derive(Clone)]
pub struct Arena {
    order:Vec<u32>,rank:Vec<usize>,mode:Mode,
    nodes:Vec<Node>,unique:FxHashMap<Node,Ref>,
    applications:FxHashMap<(u8,Ref,Ref),Ref>,
    cofactors:FxHashMap<(Ref,u32,bool),Ref>,
    projections:FxHashMap<(Ref,u64),Ref>,
    pub apply_misses:usize,pub cofactor_misses:usize,pub projection_misses:usize,
}

fn trivial(op:u8,a:Ref,b:Ref)->Option<Ref> {
    let unary=|x,code:u8|match code&3 {0=>0,1=>x^1,2=>x,_=>1};
    Some(match op {
        0=>0,15=>1,12=>a,10=>b,3=>a^1,5=>b^1,
        _ if a<2=>unary(b,op>>(2*a)),
        _ if b<2=>unary(a,((op>>b)&1)|(((op>>(2+b))&1)<<1)),
        _ if a==b=>unary(a,(op&1)|((op>>2)&2)),
        _ if a==(b^1)=>unary(a,op>>1),
        _=>return None,
    })
}

impl Arena {
    pub fn new(order:Vec<u32>,mode:Mode)->Self {
        assert!(order.len()<63);
        let mut rank=vec![usize::MAX;order.len()];
        for (i,&v) in order.iter().enumerate() {
            assert!((v as usize)<order.len() && rank[v as usize]==usize::MAX);
            rank[v as usize]=i;
        }
        Self {order,rank,mode,nodes:vec![Node{variables:0,edges:[0,0]}],unique:FxHashMap::default(),
            applications:FxHashMap::default(),cofactors:FxHashMap::default(),projections:FxHashMap::default(),
            apply_misses:0,cofactor_misses:0,projection_misses:0}
    }
    pub fn variables(&self,r:Ref)->u64 {self.nodes[(r/2) as usize].variables&!CHAIN}
    fn top(&self,mask:u64)->u32 {*self.order.iter().find(|&&v|mask>>v&1!=0).expect("nonconstant")}
    fn intern(&mut self,node:Node)->Ref {
        if let Some(&r)=self.unique.get(&node) {return r;}
        assert!(self.nodes.len()<2_000_000,"RESOURCE_CAP: unary nodes");
        let r=self.nodes.len() as Ref*2;self.nodes.push(node);self.unique.insert(node,r);r
    }
    fn chain_len(&self,node:Node)->u32 {
        ((node.variables&!CHAIN)&!self.variables(node.edges[0])).count_ones()
    }
    fn mk(&mut self,v:u32,mut lo:Ref,mut hi:Ref)->Ref {
        if lo==hi {return lo;}
        let children=self.variables(lo)|self.variables(hi);
        assert!(children>>v&1==0);
        debug_assert!(self.order.iter().take(self.rank[v as usize]).all(|&w|children>>w&1==0));
        let flip=lo&1;lo^=flip;hi^=flip;
        let variables=children|(1<<v);
        if self.mode==Mode::Chains {
            // Priority resolves the variable/constant boundary: XOR first.
            // Codes: x&g, x&!g, !x&g, x|g, x XOR g. Tail is regular.
            let unary=if lo==(hi^1) {Some((4,lo))}
                else if lo==0 {Some((hi&1,hi&!1))}
                else if hi<2 {Some((2+hi,lo))} else {None};
            if let Some((mut letters,mut tail))=unary {
                let child=self.nodes[(tail/2) as usize];
                if child.variables&CHAIN!=0 && self.chain_len(child)<MAX_CHAIN {
                    letters|=child.edges[1]<<3;tail=child.edges[0];
                }
                return self.intern(Node{variables:variables|CHAIN,edges:[tail,letters]})^flip;
            }
        }
        self.intern(Node{variables,edges:[lo,hi]})^flip
    }
    pub fn variable(&mut self,v:u32)->Ref {self.mk(v,0,1)}
    fn children(&mut self,r:Ref)->[Ref;2] {
        let node=self.nodes[(r/2) as usize];let flip=r&1;
        if node.variables&CHAIN==0 {return [node.edges[0]^flip,node.edges[1]^flip];}
        let top=self.top(self.variables(r));
        let tail=if self.chain_len(node)==1 {node.edges[0]} else {
            // Reify a shortened block on demand, even after unreachable
            // suffix records have been collected. No hidden back-reference.
            self.intern(Node{variables:node.variables&!(1<<top),edges:[node.edges[0],node.edges[1]>>3]})
        };
        let [a,b]=match node.edges[1]&7 {
            0=>[0,tail],1=>[0,tail^1],2=>[tail,0],3=>[tail,1],4=>[tail,tail^1],_=>unreachable!(),
        };
        [a^flip,b^flip]
    }
    fn at(&mut self,r:Ref,v:u32)->[Ref;2] {
        if self.variables(r)>>v&1==0 {[r,r]} else {
            assert_eq!(self.top(self.variables(r)),v);self.children(r)
        }
    }
    pub fn apply(&mut self,mut op:u8,mut a:Ref,mut b:Ref)->Ref {
        assert!(op<16);
        if let Some(r)=trivial(op,a,b) {return r;}
        if a>b {std::mem::swap(&mut a,&mut b);op=(op&9)|((op&2)<<1)|((op&4)>>1);}
        if let Some(&r)=self.applications.get(&(op,a,b)) {return r;}
        self.apply_misses+=1;
        let v=self.top(self.variables(a)|self.variables(b));
        let aa=self.at(a,v);let bb=self.at(b,v);
        let lo=self.apply(op,aa[0],bb[0]);let hi=self.apply(op,aa[1],bb[1]);
        let out=self.mk(v,lo,hi);self.applications.insert((op,a,b),out);out
    }
    pub fn cofactor(&mut self,r:Ref,v:u32,high:bool)->Ref {
        if self.variables(r)>>v&1==0 {return r;}
        let regular=r&!1;
        if let Some(&out)=self.cofactors.get(&(regular,v,high)) {return out^(r&1);}
        self.cofactor_misses+=1;
        let top=self.top(self.variables(r));let [a,b]=self.children(regular);
        let out=if top==v {if high {b} else {a}} else {
            let a=self.cofactor(a,v,high);let b=self.cofactor(b,v,high);self.mk(top,a,b)
        };
        self.cofactors.insert((regular,v,high),out);out^(r&1)
    }
    pub fn exists(&mut self,r:Ref,hidden:u64)->Ref {
        let hidden=hidden&self.variables(r);if hidden==0 {return r;}
        if let Some(&out)=self.projections.get(&(r,hidden)) {return out;}
        self.projection_misses+=1;
        let v=self.top(self.variables(r));let [a,b]=self.children(r);
        let a=self.exists(a,hidden&!(1<<v));let b=self.exists(b,hidden&!(1<<v));
        let out=if hidden>>v&1!=0 {self.apply(14,a,b)} else {self.mk(v,a,b)};
        self.projections.insert((r,hidden),out);out
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
    pub fn evaluate(&self,r:Ref,world:u64)->bool {
        if r<2 {return r!=0;}
        let node=self.nodes[(r/2) as usize];let mut flip=r&1!=0;
        if node.variables&CHAIN==0 {
            let v=self.top(self.variables(r));return self.evaluate(node.edges[(world>>v&1) as usize],world)^flip;
        }
        let axes=self.variables(r)&!self.variables(node.edges[0]);let mut letters=node.edges[1];
        for &v in &self.order {
            if axes>>v&1==0 {continue;}
            let x=world>>v&1!=0;
            match letters&7 {
                0=>if !x {return flip;},
                1=>{if !x {return flip;} flip=!flip;},
                2=>if x {return flip;},
                3=>if x {return !flip;},
                4=>flip^=x,_=>unreachable!(),
            }
            letters>>=3;
        }
        self.evaluate(node.edges[0],world)^flip
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
    /// Uniform FULL raw cube only. Reads packed labels without reifying suffixes.
    pub fn raw_uniform_count(&self,r:Ref)->u64 {
        fn visit(c:&Arena,r:Ref,memo:&mut FxHashMap<Ref,u64>)->u64 {
            if r<2 {return r as u64;}
            let regular=r&!1;let dims=c.variables(r).count_ones();
            let value=if let Some(&n)=memo.get(&regular) {n} else {
                let node=c.nodes[(r/2) as usize];
                let n=if node.variables&CHAIN==0 {
                    node.edges.iter().map(|&e|visit(c,e,memo)<<(dims-1-c.variables(e).count_ones())).sum()
                } else {
                    let mut n=visit(c,node.edges[0],memo);
                    let mut d=c.variables(node.edges[0]).count_ones();
                    for i in (0..c.chain_len(node)).rev() {
                        n=match (node.edges[1]>>(3*i))&7 {0|2=>n,1=>(1u64<<d)-n,3=>(1u64<<d)+n,4=>1u64<<d,_=>unreachable!()};
                        d+=1;
                    }
                    n
                };
                memo.insert(regular,n);n
            };
            if r&1!=0 {(1u64<<dims)-value} else {value}
        }
        visit(self,r,&mut FxHashMap::default())<<(self.order.len() as u32-self.variables(r).count_ones())
    }
    /// Copy only physical reachable records; omit operation caches. This
    /// measures real post-collection storage, not an in-place concurrent GC.
    pub fn compact(&self,roots:&[Ref])->(Self,Vec<Ref>) {
        fn copy(src:&Arena,dst:&mut Arena,r:Ref,map:&mut FxHashMap<Ref,Ref>)->Ref {
            if r<2 {return r;}
            let regular=r&!1;
            if let Some(&out)=map.get(&regular) {return out^(r&1);}
            let mut node=src.nodes[(r/2) as usize];
            node.edges[0]=copy(src,dst,node.edges[0],map);
            if node.variables&CHAIN==0 {node.edges[1]=copy(src,dst,node.edges[1],map);}
            let out=dst.intern(node);map.insert(regular,out);out^(r&1)
        }
        let mut out=Self::new(self.order.clone(),self.mode);let mut map=FxHashMap::default();
        let roots=roots.iter().map(|&r|copy(self,&mut out,r,&mut map)).collect();(out,roots)
    }
    pub fn nodes(&self)->usize {self.nodes.len()}
    pub fn reachable(&self,roots:&[Ref])->usize {
        let mut seen=FxHashSet::default();let mut pending=roots.to_vec();
        while let Some(r)=pending.pop() {
            if r<2 || !seen.insert(r/2) {continue;}
            let node=self.nodes[(r/2) as usize];pending.push(node.edges[0]);
            if node.variables&CHAIN==0 {pending.push(node.edges[1]);}
        }
        seen.len()
    }
    pub fn bytes(&self)->usize {
        self.nodes.capacity()*16+self.unique.capacity()*21
            +self.applications.capacity()*(std::mem::size_of::<((u8,Ref,Ref),Ref)>()+1)
            +self.cofactors.capacity()*(std::mem::size_of::<((Ref,u32,bool),Ref)>()+1)
            +self.projections.capacity()*(std::mem::size_of::<((Ref,u64),Ref)>()+1)
            +self.order.capacity()*4+self.rank.capacity()*std::mem::size_of::<usize>()
    }
}

#[cfg(test)]
mod tests;
