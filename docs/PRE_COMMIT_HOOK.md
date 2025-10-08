# Pre-Commit Hook Documentation

## Overview

A pre-commit hook has been installed to automatically enforce code quality standards before each commit. This ensures that all committed code is properly formatted, builds successfully, and passes all tests.

## What the Hook Does

The pre-commit hook runs three checks automatically:

### 1. Code Formatting ✅
- Runs `cargo fmt --check` to verify code formatting
- Ensures all Rust code follows standard formatting conventions
- **Blocks commit if:** Code is not properly formatted

**To fix:** Run `cargo fmt` before committing

### 2. Build Verification ✅
- Runs `cargo build --quiet` to verify the project compiles
- Catches compilation errors before they reach the repository
- **Blocks commit if:** Code doesn't compile

**To fix:** Fix compilation errors shown in the output

### 3. Test Suite ✅
- Runs `cargo test --quiet` to execute all 235 tests
- Ensures no regressions are introduced
- **Blocks commit if:** Any test fails

**To fix:** Fix failing tests before committing

## Performance

The hook typically completes in **5-10 seconds**:
- Formatting check: ~1 second
- Build: ~2-3 seconds (incremental builds are fast)
- Tests: ~2-5 seconds

This is a small investment for maintaining code quality!

## Usage

### Normal Workflow

The hook runs automatically when you commit:

```bash
git add .
git commit -m "Your commit message"
```

If all checks pass, you'll see:
```
🔍 Running pre-commit checks...
📝 Checking code formatting...
✅ Code formatting OK
🔨 Building project...
✅ Build successful
🧪 Running tests...
✅ All tests passed

✅ All pre-commit checks passed!
🚀 Proceeding with commit...
```

### If Checks Fail

**Formatting failure:**
```bash
❌ Code formatting check failed!
💡 Run 'cargo fmt' to fix formatting issues.
```

**Solution:**
```bash
cargo fmt
git add .
git commit -m "Your message"
```

**Build failure:**
```bash
❌ Build failed!
```

**Solution:** Fix the compilation errors, then try again.

**Test failure:**
```bash
❌ Tests failed!
💡 Fix failing tests before committing.
```

**Solution:** Fix the failing tests, then try again.

## Bypassing the Hook (Not Recommended)

In rare cases where you need to commit without running checks:

```bash
git commit --no-verify -m "Your message"
```

**⚠️ Warning:** Only use `--no-verify` if you have a very good reason:
- Emergency hotfix that will be fixed in next commit
- Work-in-progress commit on a feature branch
- Temporary commit that will be squashed later

**Never** use `--no-verify` for commits to the main branch!

## Installation

The hook is already installed at `.git/hooks/pre-commit` and is executable.

### Manual Installation

If you need to reinstall the hook:

1. Copy the hook file:
```bash
cp .git/hooks/pre-commit.sample .git/hooks/pre-commit
```

2. Make it executable:
```bash
chmod +x .git/hooks/pre-commit
```

3. Edit the file to match the current implementation (see `.git/hooks/pre-commit`)

### Sharing with Team

**Note:** Git hooks are not committed to the repository (they live in `.git/hooks/`).

To share the hook with your team:

1. Keep the hook script in a committed location (e.g., `scripts/pre-commit`)
2. Add installation instructions to the README
3. Team members run: `cp scripts/pre-commit .git/hooks/ && chmod +x .git/hooks/pre-commit`

Alternatively, use a tool like [pre-commit](https://pre-commit.com/) for automatic hook management.

## Benefits

### Code Quality ✅
- Ensures consistent code formatting across all commits
- Catches compilation errors before they reach CI/CD
- Prevents test regressions from being committed

### Developer Experience ✅
- Fast feedback loop (5-10 seconds vs waiting for CI)
- Reduces "oops" commits that break the build
- Maintains clean git history

### Team Productivity ✅
- Reduces code review time (no formatting discussions)
- Prevents broken builds on main branch
- Ensures all code is tested before commit

## Troubleshooting

### Hook doesn't run

**Check if it's executable:**
```bash
ls -la .git/hooks/pre-commit
```

Should show: `-rwxr-xr-x` (note the `x` for executable)

**Make it executable:**
```bash
chmod +x .git/hooks/pre-commit
```

### Hook runs but fails unexpectedly

**Check cargo is available:**
```bash
which cargo
```

**Check you're in the right directory:**
```bash
pwd
# Should be: /path/to/BPlusTreeMap4
```

### Hook is too slow

The hook uses incremental builds and quiet mode to be as fast as possible. If it's still too slow:

1. Consider using `--no-verify` for WIP commits on feature branches
2. Squash commits before merging to main
3. Ensure you have a fast SSD and enough RAM

## Future Enhancements

Potential improvements to consider:

- [ ] Add clippy lints (currently disabled due to existing warnings)
- [ ] Add check for TODO/FIXME comments
- [ ] Add check for debug print statements
- [ ] Add benchmark regression detection
- [ ] Add documentation coverage check
- [ ] Integrate with pre-commit framework

## Summary

The pre-commit hook is a lightweight, fast quality gate that ensures:
- ✅ All code is properly formatted
- ✅ All code compiles successfully  
- ✅ All tests pass

This maintains high code quality with minimal developer friction.

---

**Last Updated:** October 8, 2025  
**Hook Version:** 1.0  
**Maintained by:** BPlusTreeMap4 team

