use super::carrier::*;
use super::range_carrier::RangeCarrier;
use super::packed::Packed;
use super::observation::CountPlan;

fn candidate<const M:u8>() {
    type Baseline=Packed<8>;
    super::semantic_checks::verify::<RangeCarrier<M>>();
    super::legal_checks::symbolic::<RangeCarrier<M>>();
    super::diagonal_checks::symbolic::<RangeCarrier<M>>();
    let mut fixtures=super::transfer_checks::fixtures();
    super::transfer_checks::source::<RangeCarrier<M>>(&mut fixtures);
    super::transfer_checks::target::<RangeCarrier<M>>(&fixtures);
    super::transfer_checks::target::<Baseline>(&fixtures);
    super::transfer_checks::table_modes::<RangeCarrier<M>>();
    super::transfer_checks::compute_owner::<RangeCarrier<M>>();
    let mut packets=Vec::new();
    super::transfer_checks::high_source::<RangeCarrier<M>>(&mut packets);
    super::transfer_checks::high_source::<Baseline>(&mut packets);
    super::transfer_checks::high_target::<RangeCarrier<M>>(&packets);
    super::transfer_checks::high_target::<Baseline>(&packets);
    // An OR selector and both children share parameter names. Check every
    // coefficient against worlds, also after projection changes dependence.
    let n=18;let cells=1usize<<n;
    for order in [(0..n).collect(),(0..n).rev().collect()] {
        let mut c=RangeCarrier::<M>::full_space(n,order).unwrap();
        let raw=bits(cells,|w| if w&0x3fff!=0 {0x6b97u64>>(w>>14)&1!=0} else {0xa639u64>>(w>>14)&1!=0});
        let root=c.import(&raw);let projected=c.exists(root,(1<<3)|(1<<9)|(1<<16));
        let neg=c.not(root);
        for group_of in [vec![0;n as usize],(0..n as usize).map(|v|v%2).collect(),(0..n as usize).map(|v|v%3).collect()] {
            let mut plan=CountPlan::new(group_of);c.prepare_spectrum(&mut plan);
            for r in [root,neg,projected,c.full(),c.empty()] {
                let words=c.export(r);let mut expected=plan.zero();
                for w in 0..cells {if at(&words,w) {expected[plan.index(w)]+=1;}}
                assert_eq!(c.spectrum(r,&plan),expected);
            }
        }
    }
    let mut shared=RangeCarrier::<M>::full_space(40,(0..40).collect()).unwrap();
    let one=shared.variable(39);let mut all=shared.full();
    for v in 0..40 {let x=shared.variable(v);all=shared.op(8,all,x);}
    super::observation_checks::high(shared,all,one);
    // 61-coordinate full-cube algebra and one shared unknown parameter.
    let mut c=RangeCarrier::<M>::full_space(61,(0..61).collect()).unwrap();
    let mut any=c.empty();let mut all=c.full();
    for v in 0..61 {let x=c.variable(v);any=c.op(14,any,x);all=c.op(8,all,x);}
    let mut plan=CountPlan::new(vec![0;61]);c.prepare_spectrum(&mut plan);
    let a=c.spectrum(any,&plan);let b=c.spectrum(all,&plan);
    assert_eq!(a.iter().sum::<u64>(),(1u64<<61)-1);
    assert_eq!(a[0],0);assert_eq!(a[1],61);assert_eq!(a[60],61);assert_eq!(a[61],1);
    assert_eq!(b.iter().sum::<u64>(),1);assert_eq!(b[61],1);
    println!("RANGE_ADMISSION {{\"passed\":true,\"candidate\":\"{}\",\"coefficient_cases\":30,\"symbolic_dimensions\":61}}",RangeCarrier::<M>::NAME);
}

pub fn all() {candidate::<0>();candidate::<1>();candidate::<2>();}
