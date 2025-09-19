#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT_DIR"

mkdir -p logs
TS=$(date -u +%Y%m%d-%H%M%SZ)
LOG="logs/test-$TS.txt"

echo "[run_and_log_tests] Running cargo test ... this may take a while"
set +e
cargo test --all --no-fail-fast | tee "$LOG"
STATUS=$?
set -e

# Aggregate results across all test binaries
PASSED=0
FAILED=0
IGNORED=0
MEASURED=0
FILTERED=0

while IFS= read -r line; do
  # Expect lines like:
  # test result: ok. 12 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 0.00s
  # test result: FAILED. 8 passed; 17 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
  if [[ "$line" =~ test\ result: ]]; then
    # Extract numbers by field names (BSD sed -E)
    p=$(echo "$line" | sed -E 's/.*: (ok|FAILED)\. ([0-9]+) passed; .*/\2/')
    f=$(echo "$line" | sed -E 's/.* passed; ([0-9]+) failed; .*/\1/')
    i=$(echo "$line" | sed -E 's/.* failed; ([0-9]+) ignored; .*/\1/')
    m=$(echo "$line" | sed -E 's/.* ignored; ([0-9]+) measured; .*/\1/')
    flt=$(echo "$line" | sed -E 's/.* measured; ([0-9]+) filtered.*/\1/')
    PASSED=$((PASSED + ${p:-0}))
    FAILED=$((FAILED + ${f:-0}))
    IGNORED=$((IGNORED + ${i:-0}))
    MEASURED=$((MEASURED + ${m:-0}))
    FILTERED=$((FILTERED + ${flt:-0}))
  fi
done < "$LOG"

ISO_TS=$(date -u +%Y-%m-%dT%H:%M:%SZ)

echo "- $ISO_TS: passed=$PASSED, failed=$FAILED, ignored=$IGNORED, measured=$MEASURED, filtered=$FILTERED" >> docs/TEST_PROGRESS.md

echo "[run_and_log_tests] Summary appended to docs/TEST_PROGRESS.md"
echo "[run_and_log_tests] Exit status: $STATUS (non-zero typically indicates some tests failed)"
exit 0
