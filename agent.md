# Agent Notes

This repository is being brought up as a raw-memory B+ tree map. The plan is to import API tests from the prior project, get them compiling, and then iteratively implement functionality until tests pass.

## Test Progress Tracking

- Log: see `docs/TEST_PROGRESS.md` for timestamped pass/fail counts aggregated across all test binaries.
- Script: run `scripts/run_and_log_tests.sh` to execute the full test suite and append a summary line to the log. The script:
  - runs `cargo test --all --no-fail-fast`,
  - saves detailed output under `logs/test-YYYYmmdd-HHMMSSZ.txt`,
  - extracts and sums `passed/failed/ignored/measured/filtered` from all `test result:` lines,
  - appends a timestamped summary to `docs/TEST_PROGRESS.md`.

## Current Status

- API surface compiles with stubs; most runtime tests fail (expected) until functionality is implemented.
- Next milestones:
  - Implement leaf-root insert/get/remove without splits.
  - Add leaf splits + root promotion.
  - Implement branch descent and splitting.
  - Add deletion with borrow/merge.
  - Implement iterators and range (DoubleEndedIterator over linked leaves).
  - Replace invariant stubs with real validation.

