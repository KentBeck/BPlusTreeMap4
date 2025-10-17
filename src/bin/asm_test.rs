use bplustree::BPlusTreeMap;
use std::hint::black_box;

#[inline(never)]
fn iterate_range(map: &BPlusTreeMap<i32, i32>, start: i32, end: i32) -> u64 {
    let mut sum = 0u64;
    for (k, v) in map.range(start..end) {
        sum = sum.wrapping_add(*k as u64 + *v as u64);
    }
    sum
}

fn main() {
    let mut map = BPlusTreeMap::with_cache_lines(2, 2);

    for i in 0..1000 {
        map.insert(i, i * 2);
    }

    let result = iterate_range(&map, 100, 200);
    black_box(result);
}
