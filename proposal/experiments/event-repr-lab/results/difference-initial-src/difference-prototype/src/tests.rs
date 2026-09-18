use super::*;

fn truth(mask: u8) -> Vec<bool> { (0..8).map(|w| mask >> w & 1 != 0).collect() }
fn mask(c: &Arena, r: Ref) -> u8 { (0..8).fold(0, |m,w| m | ((c.evaluate(r,w) as u8) << w)) }
fn op_truth(op: u8, a: u8, b: u8) -> u8 {
    (0..8).fold(0, |m,w| m | (((op >> ((((a >> w)&1)<<1)|((b >> w)&1))) & 1) << w))
}

#[test]
fn all_small_functions() {
    let orders = [[0,1,2],[0,2,1],[1,0,2],[1,2,0],[2,0,1],[2,1,0]];
    let modes = [[Basis::Shannon;3], [Basis::Positive;3], [Basis::Negative;3],
        [Basis::Shannon,Basis::Positive,Basis::Negative]];
    let mut binary_cases = 0usize;
    let mut projection_cases = 0usize;
    let mut map_cases = 0usize;
    let mut completed_cases = 0usize;
    for order in orders {
        for basis in modes {
            let mut c = Arena::new(order.to_vec(),basis.to_vec());
            let roots: Vec<_> = (0..=255).map(|m|c.import(&truth(m))).collect();
            assert_eq!(roots.iter().copied().collect::<FxHashSet<_>>().len(),256);
            for a in 0..=255u8 {
                assert_eq!(mask(&c,roots[a as usize]),a);
                assert_eq!(roots[a as usize]^1,roots[(a^255) as usize]);
                let actual_deps=(0..3).fold(0,|m,v|m|if (0..8).any(|w|(a>>w)&1!=(a>>(w^(1<<v)))&1) {1<<v} else {0});
                assert_eq!(c.variables(roots[a as usize]),actual_deps);
                for v in 0..3 {
                    for high in [false,true] {
                        let result=c.cofactor(roots[a as usize],v,high);
                        let expected=(0..8).fold(0,|m,w|m|(((a>>((w&!(1<<v))|((high as u32)<<v)))&1)<<w));
                        assert_eq!(result,roots[expected as usize]);
                    }
                }
                for hidden in 0..8u64 {
                    let expected=(0..8).fold(0u8,|m,w|m|(((0..8).any(|s|s & !hidden == w & !hidden && a >> s & 1 != 0) as u8)<<w));
                    let result=c.exists(roots[a as usize],hidden);
                    assert_eq!(result,roots[expected as usize]);
                    projection_cases+=1;
                }
                for b in 0..=255u8 {
                    for op in 0..16 {
                        let result=c.apply(op,roots[a as usize],roots[b as usize]);
                        assert_eq!(result,roots[op_truth(op,a,b) as usize],"{order:?} {basis:?} op {op}, {a}, {b}");
                        binary_cases+=1;
                    }
                }
            }
            // All coordinate maps, including aliases, plus complemented and
            // nonlinear selectors. Target selectors are substituted simultaneously.
            let mut maps:Vec<Vec<Ref>>=(0..27).map(|m| (0..3).map(|k|c.variable((m/3u32.pow(k))%3)).collect()).collect();
            maps.push(vec![roots[0xa6],roots[0x6b],roots[0x97]]);
            maps.push(vec![roots[0x55],roots[0x33],roots[0x0f]]);
            for selectors in &maps {
                let images:Vec<_>=(0..8).map(|w|selectors.iter().enumerate().fold(0,|s,(i,&r)|s|((c.evaluate(r,w) as u8)<<i))).collect();
                for a in 0..=255u8 {
                    let expected=images.iter().enumerate().fold(0,|m,(w,&s)|m|(((a>>s)&1)<<w));
                    assert_eq!(c.substitute(roots[a as usize],selectors),roots[expected as usize]);
                    map_cases+=1;
                }
            }
            // A coupled legal support and its fixed retraction onto legal codes.
            // Input formulas outside support must merge, and every published
            // projected function must equal completion on all raw codes.
            let support=0x6bu8;
            let image:Vec<_>=(0..8).map(|w| if support>>w&1!=0 {w} else {0}).collect();
            let selectors:Vec<_>=(0..3).map(|v|{
                let m=image.iter().enumerate().fold(0,|m,(w,s)|m|(((s>>v)&1)<<w));
                roots[m as usize]
            }).collect();
            for a in 0..=255u8 {
                let normal=image.iter().enumerate().fold(0,|m,(w,s)|m|(((a>>s)&1)<<w));
                let completed=c.substitute(roots[a as usize],&selectors);
                assert_eq!(completed,roots[normal as usize]);
                for hidden in 0..8u64 {
                    let gated=c.apply(8,roots[support as usize],completed);
                    let projected=c.exists(gated,hidden);
                    let out=c.substitute(projected,&selectors);
                    let expected=image.iter().enumerate().fold(0u8,|m,(w,&target)|m|
                        (((0..8u64).any(|s| support>>s&1!=0 && a>>s&1!=0 && s & !hidden == target & !hidden) as u8)<<w));
                    assert_eq!(out,roots[expected as usize]);
                    completed_cases+=1;
                }
            }
        }
    }
    assert_eq!(binary_cases,25_165_824);
    assert_eq!(projection_cases,49_152);
    assert_eq!(map_cases,178_176);
    assert_eq!(completed_cases,49_152);
    println!("DIFFERENCE_CHECK {{\"passed\":true,\"orders\":6,\"basis_schedules\":4,\"functions_per_manager\":256,\"binary_cases\":{binary_cases},\"projection_cases\":{projection_cases},\"map_cases\":{map_cases},\"completed_projection_cases\":{completed_cases}}}");
}

#[test]
fn symbolic_width_and_quantifier_obstruction() {
    for mode in [Basis::Shannon,Basis::Positive,Basis::Negative] {
        let mut c=Arena::new((0..61).collect(),vec![mode;61]);
        let x=c.variable(0);let y=c.variable(60);
        let xor=c.apply(6,x,y);
        assert_eq!(c.variables(xor),(1u64<<60)|1);
        assert_eq!(c.exists(xor,1<<60),1);
        assert_eq!(c.exists(xor^1,1<<60),1);
        let x_and_y=c.apply(8,x,y);
        assert_eq!(c.exists(x_and_y,1<<60),x);
        let mut selectors:Vec<_>=(0..61).map(|v|c.variable(v)).collect();
        selectors[60]=x;
        assert_eq!(c.substitute(xor,&selectors),0);
        // Coefficient-wise projection of y XOR x*true would give !x, not true.
        let wrong=c.apply(6,1,x);
        assert_ne!(wrong,1);
    }
}
