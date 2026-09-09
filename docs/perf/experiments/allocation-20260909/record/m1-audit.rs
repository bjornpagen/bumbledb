use super::*;
use crate::alloc_counter::{AllocWindow, snapshot};

fn delta(before: AllocWindow, after: AllocWindow) -> (u64, u64, u64, u64) {
    (
        after.allocs - before.allocs,
        after.deallocs - before.deallocs,
        after.alloc_bytes - before.alloc_bytes,
        after.dealloc_bytes - before.dealloc_bytes,
    )
}

fn ownership(colt: &Colt) -> [usize; 10] {
    let map = colt.maps[0];
    [
        map.nbuckets,
        map.len as usize,
        colt.ctrl.len(),
        colt.ctrl.capacity(),
        colt.buckets.len(),
        colt.buckets.capacity(),
        colt.dense.len(),
        colt.dense.capacity(),
        colt.ctrl.len() + 8 * colt.buckets.len() + 4 * colt.dense.len(),
        colt.retained_bytes(),
    ]
}

#[test]
#[ignore = "untimed map construction ownership and request census; no tracing"]
fn construction_ownership() {
    for width in [0, 1, 2, 3, 4, 5, 8] {
        for distinct in [25, 26] {
            for grouped in [false, true] {
                let image = fixture(width, distinct, grouped);
                let work = crate::WorkContext::new();
                let mut colt = Colt::new(all(&image), &[], levels(width));
                colt.bind(Some(&work));
                let before = snapshot().window;
                colt.force_root().unwrap();
                let cold = delta(before, snapshot().window);
                let owners = ownership(&colt);
                let mut sibling = Colt::new(all(&image), &[], levels(width));
                sibling.bind(Some(&work));
                let before = snapshot().window;
                let old = sibling.clone_bound_from(&colt, Vec::new()).unwrap();
                let cloned = delta(before, snapshot().window);
                drop(old);
                assert_eq!(sibling.ctrl, colt.ctrl);
                assert_eq!(sibling.buckets, colt.buckets);
                assert_eq!(sibling.dense, colt.dense);
                let clone_owners = ownership(&sibling);
                drop(colt.reset(all(&image)));
                let before = snapshot().window;
                colt.force_root().unwrap();
                let reused = delta(before, snapshot().window);
                assert_eq!(reused, (0, 0, 0, 0));
                assert_eq!(ownership(&colt), owners);
                // Semantic validation/inspection allocations are outside all windows.
                check_rows(&mut colt, &image, width);
                check_rows(&mut sibling, &image, width);
                println!(
                    "MAP width={width} distinct={distinct} grouped={grouped} owners={owners:?} clone_owners={clone_owners:?} cold={cold:?} clone={cloned:?} reused={reused:?}"
                );
            }
        }
    }
    println!("PASS construction ownership; 28 cells; exact rows, clone, zero-request reuse");
}
