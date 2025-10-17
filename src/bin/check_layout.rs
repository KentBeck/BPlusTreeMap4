use bplustree::BPlusTreeMap;

fn main() {
    let _map: BPlusTreeMap<u64, u64> = BPlusTreeMap::new(128).expect("new");

    // Access internal layout info via debug or other means
    println!("Created BPlusTreeMap with capacity 128");
    println!("Size of u64: {}", std::mem::size_of::<u64>());
    println!("Align of u64: {}", std::mem::align_of::<u64>());

    // The actual layout info is private, but we can infer from memory usage
    // A leaf with cap=128 should have:
    // - NodeHdr (5 bytes)
    // - 2 sibling pointers (16 bytes)
    // - 128 keys (1024 bytes)
    // - 128 values (1024 bytes)
    // Total: ~2069 bytes
    // With 64-byte alignment, this rounds up to 2112 bytes (33 cache lines)

    println!("\nExpected leaf node size:");
    println!("  Without alignment: ~2069 bytes");
    println!("  With 64-byte alignment: ~2112 bytes (33 cache lines)");
    println!("  Overhead: ~43 bytes (2%)");
}
