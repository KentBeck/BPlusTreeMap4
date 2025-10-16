#!/bin/bash
set -e

echo "Building profile_range_detailed..."
cargo build --release --bin profile_range_detailed

echo ""
echo "Running with cargo flamegraph for line-level profiling..."
echo "This will generate a flamegraph showing hotspots"

# Use cargo flamegraph if available, otherwise use perf
if command -v cargo-flamegraph &> /dev/null; then
    cargo flamegraph --bin profile_range_detailed -o flamegraph_range.svg
    echo "Flamegraph saved to flamegraph_range.svg"
else
    echo "cargo-flamegraph not found. Using perf instead..."
    
    # Check if perf is available
    if command -v perf &> /dev/null; then
        perf record -F 999 --call-graph dwarf -o profile_range_detailed.data \
            target/release/profile_range_detailed
        
        echo ""
        echo "Generating annotated source..."
        perf annotate -i profile_range_detailed.data --stdio > profile_range_detailed_annotate.txt
        
        echo ""
        echo "Profile saved to profile_range_detailed.data"
        echo "Annotated source saved to profile_range_detailed_annotate.txt"
    else
        echo "Neither cargo-flamegraph nor perf available. Just running the binary..."
        target/release/profile_range_detailed
    fi
fi
