//! Structural screen only. The Coup binding formula matches the retained clover
//! fixture, but this executable deliberately does not claim to run Free Join.
use event_difference_prototype::{Arena,Basis,Ref};

fn legal(w:u64)->bool {
    let [a,b,c,d]:[u64;4]=std::array::from_fn(|i|(w>>(4*i))&15);
    [a,b,c,d].iter().all(|&v|v<15&&v!=0&&v!=3)&&a<b&&c<d&&a!=c&&a!=d&&b!=c&&b!=d
}
fn stats(c:&Arena,roots:&[Ref])->(usize,usize,usize,usize,usize,usize) {
    (c.nodes(),c.reachable(roots),c.bytes(),c.apply_misses,c.cofactor_misses,c.projection_misses)
}
fn basis(name:&str,n:u32)->Vec<Basis> {
    (0..n).map(|v|match name {
        "shannon"=>Basis::Shannon,"positive"=>Basis::Positive,"negative"=>Basis::Negative,
        "mixed"=>[Basis::Shannon,Basis::Positive,Basis::Negative][v as usize%3],
        _=>panic!("basis")
    }).collect()
}
fn main() {
    let args:Vec<_>=std::env::args().collect();assert_eq!(args.len(),4);
    let shape=&args[1];let mode=&args[2];let layout=&args[3];
    let n=if shape=="coup" {16} else {12};
    let width=4;
    let faces=if shape=="coup" {4} else {3};
    let order=match layout.as_str() {
        "face-major"=>(0..n).rev().collect(),
        "bit-major"=>(0..width).rev().flat_map(|b|(0..faces).map(move |f|f*width+b)).collect(),
        _=>panic!("layout")
    };
    let mut c=Arena::new(order,basis(mode,n));
    let worlds=1usize<<n;
    let (inputs,outputs,before,checksum,checked)=if shape=="coup" {
        let anchor=(0..worlds as u64).find(|&w|legal(w)).unwrap();
        let decoded:Vec<_>=(0..worlds as u64).map(|w|if legal(w){w}else{anchor}).collect();
        let cards:Vec<_>=(0..15).filter(|&v|v!=0&&v!=3).collect();
        let mut tables=Vec::new();
        for seat in 0..2 {
            for role in 0..5 {
                tables.push(decoded.iter().map(|&w|{
                    let a=(w>>(seat*8))&15;let b=(w>>(seat*8+4))&15;
                    a/3==role||b/3==role
                }).collect::<Vec<bool>>());
            }
            for &card in &cards {
                tables.push(decoded.iter().map(|&w|((w>>(seat*8))&15)==card||((w>>(seat*8+4))&15)==card).collect());
            }
        }
        let inputs:Vec<_>=tables.iter().map(|t|c.import(t)).collect();
        let before=stats(&c,&inputs);
        let mut outputs=vec![0;64];let mut expected=vec![vec![false;worlds];64];
        // Explicitly replay the clover's 64 * 4^3 complete bindings.
        for g in 0..64 {
            for i in 0..4 {for j in 0..4 {for k in 0..4 {
                let a=(g*17+i*13)%inputs.len();
                let b=(g*17+j*13+7)%inputs.len();
                let d=(g*17+k*13+14)%inputs.len();
                let other=c.apply(14,inputs[b],inputs[d]);
                let term=c.apply(8,inputs[a],other^1);
                outputs[g]=c.apply(14,outputs[g],term);
                for w in 0..worlds {expected[g][w]|=tables[a][w]&&!(tables[b][w]||tables[d][w]);}
            }}}
        }
        let mut checksum=0;
        for (i,&r) in outputs.iter().enumerate() {
            for w in 0..worlds {
                assert_eq!(c.evaluate(r,w as u64),expected[i][w]);
                if legal(w as u64)&&expected[i][w] {checksum+=i as u64+1;}
            }
        }
        // In contrast to zero-mass pruning, illegal encodings are explicit;
        // verification compares the completed functions on every raw code.
        (inputs,outputs,before,checksum,64*worlds)
    } else {
        assert_eq!(shape,"composition");
        let size=1usize<<width;
        let rtab:Vec<_>=(0..worlds).map(|w|{
            let x=w&(size-1);let y=(w>>width)&(size-1);
            y==x||y==(x+1)%size
        }).collect();
        let qtab:Vec<_>=(0..worlds).map(|w|{
            let y=(w>>width)&(size-1);let z=(w>>(2*width))&(size-1);
            z==(y+2)%size
        }).collect();
        let r=c.import(&rtab);let q=c.import(&qtab);
        let inputs=vec![r,q];let before=stats(&c,&inputs);
        let joint=c.apply(8,r,q);
        let hidden=((1u64<<width)-1)<<width;
        let composed=c.exists(joint,hidden);
        let mut selectors:Vec<_>=(0..n).map(|v|c.variable(v)).collect();
        for bit in 0..width {selectors.swap(bit as usize,(2*width+bit) as usize);}
        let converse=c.substitute(composed,&selectors);
        let target=c.import(&(0..worlds).map(|w|(w>>(2*width))==7).collect::<Vec<_>>());
        let hits=c.apply(8,composed,target);
        let may=c.exists(hits,((1<<width)-1)<<(2*width));
        let bad=c.apply(8,composed,target^1);
        let all=c.exists(bad,((1<<width)-1)<<(2*width))^1;
        let outputs=vec![composed,converse,may,all];let mut checksum=0;
        for w in 0..worlds {
            let x=w&(size-1);let z=(w>>(2*width))&(size-1);
            let comp=|a,b|(0..size).any(|y|(y==a||y==(a+1)%size)&&b==(y+2)%size);
            let expected=[comp(x,z),comp(z,x),comp(x,7),!(0..size).any(|z|comp(x,z)&&z!=7)];
            for (i,&root) in outputs.iter().enumerate() {
                assert_eq!(c.evaluate(root,w as u64),expected[i]);
                if expected[i] {checksum+=i as u64+1;}
            }
        }
        (inputs,outputs,before,checksum,4*worlds)
    };
    let after=stats(&c,&outputs);
    let live_union=c.reachable(&[inputs.clone(),outputs.clone()].concat());
    println!("DIFFERENCE_SHAPE {{\"passed\":true,\"shape\":\"{shape}\",\"basis\":\"{mode}\",\"layout\":\"{layout}\",\"dimensions\":{n},\"inputs\":{},\"outputs\":{},\"input_records\":{},\"input_reachable\":{},\"input_bytes\":{},\"final_records\":{},\"output_reachable\":{},\"live_union\":{live_union},\"final_bytes\":{},\"query_apply_misses\":{},\"query_cofactor_misses\":{},\"query_projection_misses\":{},\"checked_world_answers\":{checked},\"checksum\":{checksum}}}",inputs.len(),outputs.len(),before.0,before.1,before.2,after.0,after.1,after.2,after.3-before.3,after.4-before.4,after.5-before.5);
}
