#!/bin/bash
set -e

echo "Building profile_range..."
cargo build --release --bin profile_range

echo ""
echo "Running perf profiler on BPlusTreeMap range queries..."
perf record -F 999 --call-graph dwarf -o profile_range.data \
    target/release/profile_range

echo ""
echo "Generating report..."
perf report -i profile_range.data --stdio > profile_range_report.txt

echo ""
echo "Profile saved to profile_range.data"
echo "Report saved to profile_range_report.txt"
