//! Exact count/compact construction probe. No elapsed-time or Free Join claims.
use event_difference_prototype::{Arena,Basis,Kernel};

#[derive(Clone,Copy)]
enum Gate { And(usize,usize), Not(usize) }

fn evaluate(inputs: usize, gates: &[Gate], output: usize, world: u64) -> bool {
    let mut values:Vec<_>=(0..inputs).map(|i|world>>i&1!=0).collect();
    for gate in gates {
        values.push(match *gate { Gate::And(a,b)=>values[a]&&values[b], Gate::Not(a)=>!values[a] });
    }
    values[output]
}

fn polynomial(inputs: usize, gates: &[Gate], output: usize) -> Vec<u64> {
    let n=inputs+gates.len(); let mut terms=Vec::new();
    for (i,gate) in gates.iter().enumerate() {
        let y=1u64<<(n+i); let z=1u64<<(inputs+i);
        terms.push(y|z);
        match *gate {
            Gate::And(a,b)=>terms.push(y|(1<<a)|(1<<b)),
            Gate::Not(a)=>{terms.push(y);terms.push(y|(1<<a));}
        }
    }
    let y=1u64<<(n+gates.len());
    terms.extend([y,y|(1<<output)]);
    terms
}

fn check(inputs:usize,gates:&[Gate],output:usize,reverse:bool,kernel:Kernel,exhaustive:bool,emit:bool) {
    let n=inputs+gates.len();let m=gates.len()+1;let dimensions=n+m;
    let terms=polynomial(inputs,gates,output);
    assert!(terms.len()<=3*gates.len()+2);
    assert!(terms.iter().all(|t|t.count_ones()<=3));
    let mut order:Vec<_>=(0..dimensions as u32).collect();if reverse {order.reverse();}
    let mut c=Arena::with_kernel(order,vec![Basis::Positive;dimensions],kernel);
    let root=c.from_anf(terms.clone());
    assert!(c.reachable(&[root])<=dimensions*terms.len());
    let satisfying=(0..1u64<<inputs).filter(|&x|evaluate(inputs,gates,output,x)).count() as u64;
    let expected=(1u64<<(dimensions-1))-(satisfying<<(m-1));
    if exhaustive {
        let mut count=0;
        for w in 0..1u64<<dimensions {
            let p=terms.iter().filter(|&&t|w&t==t).count()&1!=0;
            assert_eq!(c.evaluate(root,w),p);count+=p as u64;
        }
        assert_eq!(count,expected);
    }
    let input_nodes=c.nodes();let input_bytes=c.bytes();
    if emit { eprintln!("COUNTING_PHASE {{\"phase\":\"input_verified\",\"nodes\":{input_nodes},\"bytes\":{input_bytes},\"terms\":{}}}",terms.len()); }
    let observed=c.raw_uniform_count(root);
    assert_eq!(observed,expected);
    assert_eq!(c.raw_uniform_count(root^1),(1u64<<dimensions)-expected);
    if emit {
        println!("COUNTING_SHAPE {{\"passed\":true,\"gates\":{},\"dimensions\":{dimensions},\"reverse\":{reverse},\"kernel\":\"{kernel:?}\",\"terms\":{},\"input_nodes\":{input_nodes},\"input_bytes\":{input_bytes},\"final_nodes\":{},\"final_bytes\":{},\"apply_misses\":{},\"cofactor_misses\":{},\"circuit_solutions\":{satisfying},\"polynomial_ones\":{observed}}}",
            gates.len(),terms.len(),c.nodes(),c.bytes(),c.apply_misses,c.cofactor_misses);
    }
}

fn small() {
    fn visit(gates:&mut Vec<Gate>,remaining:usize,cases:&mut usize) {
        if remaining==0 {
            for output in 0..2+gates.len() {
                for reverse in [false,true] { for kernel in [Kernel::Coefficients,Kernel::Cofactors] {
                    check(2,gates,output,reverse,kernel,true,false);*cases+=1;
                }}
            }
        } else {
            for a in 0..2+gates.len() {
                gates.push(Gate::Not(a));visit(gates,remaining-1,cases);gates.pop();
                for b in a..2+gates.len() {
                    gates.push(Gate::And(a,b));visit(gates,remaining-1,cases);gates.pop();
                }
            }
        }
    }
    let mut cases=0;
    for g in 0..=2 {visit(&mut Vec::new(),g,&mut cases);}
    assert_eq!(cases,788);
    println!("COUNTING_SMALL {{\"passed\":true,\"circuits_and_outputs\":197,\"manager_cases\":{cases},\"orders\":2,\"kernels\":2}}");
}

fn main() {
    let args:Vec<_>=std::env::args().collect();
    if args[1]=="small" {small();return;}
    let count:usize=args[1].parse().unwrap();assert!(count<=28);
    let reverse=args[2]=="reverse";
    let kernel=if args[3]=="coefficients" {Kernel::Coefficients} else {assert_eq!(args[3],"cofactors");Kernel::Cofactors};
    let mut seed=20260918u64;
    let mut random=||{seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;seed};
    let mut gates=Vec::new();
    for i in 0..count {
        let a=random() as usize%(4+i);let b=random() as usize%(4+i);
        gates.push(if random()%3==0 {Gate::Not(a)} else {Gate::And(a,b)});
    }
    check(4,&gates,3+count,reverse,kernel,false,true);
}
