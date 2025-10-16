#!/bin/bash
set -e

echo "Building profile_range_std..."
cargo build --release --bin profile_range_std

echo ""
echo "Running perf profiler on std::BTreeMap range queries..."
perf record -F 999 --call-graph dwarf -o profile_range_std.data \
    target/release/profile_range_std

echo ""
echo "Generating report..."
perf report -i profile_range_std.data --stdio > profile_range_std_report.txt

echo ""
echo "Profile saved to profile_range_std.data"
echo "Report saved to profile_range_std_report.txt"
