use super::*;

fn truth(mask:u8)->Vec<bool> {(0..8).map(|w|mask>>w&1!=0).collect()}
fn mask(c:&Arena,r:Ref)->u8 {(0..8).fold(0,|m,w|m|((c.evaluate(r,w) as u8)<<w))}
fn op_truth(op:u8,a:u8,b:u8)->u8 {
    (0..8).fold(0,|m,w|m|(((op>>((((a>>w)&1)<<1)|((b>>w)&1)))&1)<<w))
}

#[test]
fn all_small_functions() {
    let orders=[[0,1,2],[0,2,1],[1,0,2],[1,2,0],[2,0,1],[2,1,0]];
    let mut binary=0;let mut projection=0;let mut maps=0;let mut completed=0;
    for kernel in [Kernel::Steps,Kernel::Blocks] {for order in orders {for mode in [Mode::Plain,Mode::Chains] {
        let mut c=Arena::with_kernel(order.to_vec(),mode,kernel);
        let roots:Vec<_>=(0..=255).map(|m|c.import(&truth(m))).collect();
        assert_eq!(roots.iter().copied().collect::<FxHashSet<_>>().len(),256);
        for a in 0..=255u8 {
            let root=roots[a as usize];assert_eq!(mask(&c,root),a);
            assert_eq!(root^1,roots[(a^255) as usize]);
            assert_eq!(c.raw_uniform_count(root),a.count_ones() as u64);
            let deps=(0..3).fold(0,|m,v|m|if (0..8).any(|w|(a>>w)&1!=(a>>(w^(1<<v)))&1) {1<<v} else {0});
            assert_eq!(c.variables(root),deps);
            for v in 0..3 {for high in [false,true] {
                let expected=(0..8).fold(0,|m,w|m|(((a>>((w&!(1<<v))|((high as u32)<<v)))&1)<<w));
                assert_eq!(c.cofactor(root,v,high),roots[expected as usize]);
            }}
            for hidden in 0..8u64 {
                let expected=(0..8).fold(0,|m,w|m|(((0..8).any(|s|s&!hidden==w&!hidden && a>>s&1!=0) as u8)<<w));
                assert_eq!(c.exists(root,hidden),roots[expected as usize]);projection+=1;
            }
            for b in 0..=255u8 {for op in 0..16 {
                assert_eq!(c.apply(op,root,roots[b as usize]),roots[op_truth(op,a,b) as usize]);binary+=1;
            }}
            // A collected single-root graph has no obligation to retain
            // implicit suffix records. Rebuilt cofactors must still work.
            let (mut collected,rs)=c.compact(&[root]);
            assert_eq!(collected.nodes(),c.reachable(&[root])+1);
            for v in 0..3 {for high in [false,true] {
                let expected:Vec<_>=(0..8).map(|w|a>>((w&!(1<<v))|((high as u32)<<v))&1!=0).collect();
                let actual=collected.cofactor(rs[0],v,high);
                assert_eq!(actual,collected.import(&expected));
            }}
        }
        let mut selectors:Vec<Vec<Ref>>=(0..27).map(|m|(0..3).map(|k|c.variable((m/3u32.pow(k))%3)).collect()).collect();
        selectors.push(vec![roots[0xa6],roots[0x6b],roots[0x97]]);
        selectors.push(vec![roots[0x55],roots[0x33],roots[0x0f]]);
        for s in selectors {
            let images:Vec<_>=(0..8).map(|w|s.iter().enumerate().fold(0,|m,(v,&r)|m|((c.evaluate(r,w) as u8)<<v))).collect();
            for a in 0..=255u8 {
                let expected=images.iter().enumerate().fold(0,|m,(w,&x)|m|(((a>>x)&1)<<w));
                assert_eq!(c.substitute(roots[a as usize],&s),roots[expected as usize]);maps+=1;
            }
        }
        let support=0x6bu8;
        let image:Vec<_>=(0..8).map(|w|if support>>w&1!=0 {w} else {0}).collect();
        let decoder:Vec<_>=(0..3).map(|v|roots[image.iter().enumerate().fold(0usize,|m,(w,s)|m|(((s>>v)&1)<<w))]).collect();
        for a in 0..=255u8 {for hidden in 0..8u64 {
            let input=c.substitute(roots[a as usize],&decoder);
            let gated=c.apply(8,input,roots[support as usize]);
            let projected=c.exists(gated,hidden);let out=c.substitute(projected,&decoder);
            let expected=image.iter().enumerate().fold(0u8,|m,(w,&t)|m|
                (((0..8u64).any(|s|support>>s&1!=0 && a>>s&1!=0 && s&!hidden==(t as u64)&!hidden) as u8)<<w));
            assert_eq!(out,roots[expected as usize]);completed+=1;
        }}
    }}}
    assert_eq!((binary,projection,maps,completed),(25165824,49152,178176,49152));
    println!("UNARY_CHECK {{\"passed\":true,\"binary\":{binary},\"projection\":{projection},\"maps\":{maps},\"completed_projection\":{completed},\"orders\":6,\"modes\":2,\"kernels\":2}}");
}

fn step(c:&mut Arena,code:usize,x:Ref,g:Ref)->Ref {
    match code {0=>c.apply(8,x,g),1=>c.apply(8,x,g^1),2=>c.apply(8,x^1,g),3=>c.apply(14,x,g),4=>c.apply(6,x,g),_=>unreachable!()}
}

#[test]
fn long_chains_and_collection() {
    let mut checked=0;
    let mut multi_masks=0;let mut multi_worlds=0;
    for kernel in [Kernel::Steps,Kernel::Blocks] {for n in [9usize,10,11,12,20,21,22,61] {for reverse in [false,true] {for mode in [Mode::Plain,Mode::Chains] {
        let mut order:Vec<_>=(0..n as u32).collect();if reverse {order.reverse();}
        let mut c=Arena::with_kernel(order.clone(),mode,kernel);
        let vars:Vec<_>=(0..n).map(|i|c.variable(order[i])).collect();
        let left=c.apply(8,vars[n-3],vars[n-2]);let right=c.apply(8,vars[n-3]^1,vars[n-1]);
        let mut root=c.apply(14,left,right);
        for i in (0..n-3).rev() {root=step(&mut c,i%5,vars[i],root);}
        let reference=|w:u64| {
            let x=|i:usize|w>>order[i]&1!=0;
            let mut g=if x(n-3) {x(n-2)} else {x(n-1)};
            for i in (0..n-3).rev() {
                g=match i%5 {0=>x(i)&&g,1=>x(i)&&!g,2=>!x(i)&&g,3=>x(i)||g,4=>x(i)^g,_=>unreachable!()};
            }
            g
        };
        let (mut compact,rs)=c.compact(&[root,root^1]);let r=rs[0];
        assert_eq!(compact.nodes(),c.reachable(&[root])+1);
        assert_eq!(rs[1],r^1);
        let nodes=compact.nodes();assert_eq!(compact.raw_uniform_count(r),c.raw_uniform_count(root));
        assert_eq!(compact.raw_uniform_count(r)+compact.raw_uniform_count(r^1),1u64<<n);
        assert_eq!(compact.nodes(),nodes);
        if n==61 {
            println!("UNARY_LONG_SHAPE {{\"passed\":true,\"dimensions\":{n},\"reverse\":{reverse},\"mode\":\"{mode:?}\",\"kernel\":\"{kernel:?}\",\"constructed_records\":{},\"compacted_records\":{nodes},\"compacted_bytes\":{}}}",c.nodes(),compact.bytes());
        }
        if n<=12 {assert_eq!(compact.raw_uniform_count(r),(0..1u64<<n).filter(|&w|reference(w)).count() as u64);}
        let mut seed=20260918u64;
        for _ in 0..300 {
            seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;let w=seed&((1u64<<n)-1);
            assert_eq!(compact.evaluate(r,w),reference(w));
            for &v in &order {
                for high in [false,true] {
                    let co=compact.cofactor(r,v,high);
                    assert_eq!(compact.evaluate(co,w),reference((w&!(1<<v))|((high as u64)<<v)));
                }
                let exists=compact.exists(r,1<<v);
                assert_eq!(compact.evaluate(exists,w),reference(w&!(1<<v))||reference(w|(1<<v)));
                let all=compact.exists(r^1,1<<v)^1;
                assert_eq!(compact.evaluate(all,w),reference(w&!(1<<v))&&reference(w|(1<<v)));
            }
            checked+=1;
        }
        for indices in [[1,n/3,2*n/3,n-1],[0,2,n/2,n-2]] {
            let axes:Vec<_>=indices.iter().map(|&i|order[i]).collect();
            let hidden=axes.iter().fold(0u64,|m,&v|m|(1<<v));
            assert_eq!(hidden.count_ones(),4);
            let possible=compact.exists(r,hidden);let guaranteed=compact.exists(r^1,hidden)^1;
            for _ in 0..128 {
                seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;
                let w=seed&((1u64<<n)-1);let mut may=false;let mut all=true;
                for bits in 0..16u64 {
                    let varied=axes.iter().enumerate().fold(w&!hidden,|m,(i,&v)|m|(((bits>>i)&1)<<v));
                    let value=reference(varied);may|=value;all&=value;
                }
                assert_eq!(compact.evaluate(possible,w),may);
                assert_eq!(compact.evaluate(guaranteed,w),all);multi_worlds+=1;
            }
            multi_masks+=1;
        }
        let all_hidden=(1u64<<n)-1;let count=compact.raw_uniform_count(r);
        assert_eq!(compact.exists(r,all_hidden),(count>0) as Ref);
        assert_eq!(compact.exists(r^1,all_hidden)^1,(count==(1u64<<n)) as Ref);
        let rebuilt_vars:Vec<_>=(0..n).map(|i|compact.variable(order[i])).collect();
        let left=compact.apply(8,rebuilt_vars[n-3],rebuilt_vars[n-2]);
        let right=compact.apply(8,rebuilt_vars[n-3]^1,rebuilt_vars[n-1]);
        let mut rebuilt=compact.apply(14,left,right);
        for i in (0..n-3).rev() {rebuilt=step(&mut compact,i%5,rebuilt_vars[i],rebuilt);}
        assert_eq!(r,rebuilt);
    }}}}
    println!("UNARY_CHAIN_CHECK {{\"passed\":true,\"world_checks\":{checked},\"max_dimensions\":61}}");
    assert_eq!((multi_masks,multi_worlds),(128,16384));
    println!("UNARY_MULTI_CHECK {{\"passed\":true,\"four_axis_masks\":{multi_masks},\"visible_worlds\":{multi_worlds},\"hidden_assignments_per_world\":16}}");
}
