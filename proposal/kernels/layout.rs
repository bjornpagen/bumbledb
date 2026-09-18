// Standalone candidate-layout check; not native bumbledb types.
use std::mem::{align_of, size_of};
#[repr(transparent)] struct BddRef(u32);
#[repr(C, align(16))] struct BddNode { variable: u32, low: BddRef, high: BddRef, reserved: u32 }
#[repr(C)] struct RegionPair { side0: BddRef, side1: BddRef }
#[repr(C)] struct EventKey { space: u64, region: u64 }
fn main() {
    assert_eq!(size_of::<BddRef>(), 4);
    assert_eq!(size_of::<BddNode>(), 16);
    assert_eq!(align_of::<BddNode>(), 16);
    assert_eq!(size_of::<RegionPair>(), 8);
    assert_eq!(size_of::<EventKey>(), 16);
    println!("BddRef=4 BddNode=16/alignment16 RegionPair=8 EventKey=16");
}
