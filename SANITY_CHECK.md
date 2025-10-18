# Coverage-Guided Fuzzing Sanity Checklist

Complete this checklist to verify the coverage-guided fuzzing infrastructure is working correctly.

## 1. Build Components

```bash
# Build core fuzzer
cargo build -p kre8ntemjs_cli
```
**Expected:** ✓ Builds successfully

```bash
# Build scorewrap wrapper
cargo build -p scorewrap
```
**Expected:** ✓ Builds successfully (produces `target/debug/scorewrap`)

## 2. Test Mock Engine

```bash
# Basic execution
./tools/mock_engine.py --js seeds/example.js
```
**Expected:** Prints `edges:<NUMBER>` (e.g., `edges:1924`)

```bash
# File-based output
./tools/mock_engine.py --js seeds/example.js --write-file
cat /tmp/kre8_edges.txt
```
**Expected:** File contains `edges:<NUMBER>`

```bash
# Crash simulation
./tools/mock_engine.py --js seeds/crash_example.js
echo "Exit code: $?"
```
**Expected:** Exit code 1, stderr shows ReferenceError

```bash
# Deterministic mode (run twice)
./tools/mock_engine.py --mode inc seeds/example.js
./tools/mock_engine.py --mode inc seeds/example.js
```
**Expected:** Same edge count both times

## 3. Test Scorewrap Integration

```bash
# File-based mode
target/debug/scorewrap \
  --engine ./tools/mock_engine.py \
  --engine-args="--write-file" \
  --edges-file /tmp/kre8_edges.txt \
  seeds/example.js
```
**Expected:** Prints `edges:<NUMBER>`

```bash
# Regex mode
target/debug/scorewrap \
  --engine ./tools/mock_engine.py \
  --score-regex 'edges:\s*(\d+)' \
  seeds/example.js
```
**Expected:** Prints `edges:<NUMBER>`

## 4. Test End-to-End Coverage-Guided Fuzzing

```bash
# Short run with mock engine
cargo run -p kre8ntemjs_cli -- \
  --engine-cmd target/debug/scorewrap \
  --engine-args="--engine $(pwd)/tools/mock_engine.py --write-file" \
  --seeds ./seeds \
  --out ./test_artifacts \
  --iters 100 \
  --timeout 500ms \
  --score-regex 'edges:(\d+)' \
  --keep-only-increasing \
  --minimize-by coverage
```

**Expected:**
- ✓ Fuzzer runs without errors
- ✓ Fewer artifacts saved than iterations (only coverage-increasing ones)
- ✓ Output shows progress: `iter X | syntax=Y unique_crashes=Z timeouts=W`
- ✓ Check `test_artifacts/` directory for saved crashes

```bash
# Verify artifacts were created
ls -la test_artifacts/
```
**Expected:** Some crash files (fewer than 100 if --keep-only-increasing works)

## 5. Test Minimizer Modes

```bash
# Signature-based minimization (default)
cargo run -p kre8ntemjs_cli -- \
  --engine-cmd target/debug/scorewrap \
  --engine-args="--engine $(pwd)/tools/mock_engine.py --write-file" \
  --seeds ./seeds \
  --out ./test_sig_min \
  --iters 50 \
  --minimize-by signature
```
**Expected:** Creates minimized crash files

```bash
# Coverage-preserving minimization
cargo run -p kre8ntemjs_cli -- \
  --engine-cmd target/debug/scorewrap \
  --engine-args="--engine $(pwd)/tools/mock_engine.py --write-file" \
  --seeds ./seeds \
  --out ./test_cov_min \
  --iters 50 \
  --score-regex 'edges:(\d+)' \
  --minimize-by coverage
```
**Expected:** Creates minimized crash files that preserve coverage

## 6. Verify Dataflow Features

```bash
# Run with dataflow-aware extraction
cargo run -p kre8ntemjs_cli -- \
  --engine-cmd target/debug/scorewrap \
  --engine-args="--engine $(pwd)/tools/mock_engine.py" \
  --seeds ./seeds \
  --out ./test_dataflow \
  --iters 100
```
**Expected:**
- ✓ Runs successfully
- ✓ Variable replacements are biased by DFComp
- ✓ Template generation uses AST analysis

## 7. Performance Baseline

```bash
# Baseline: no coverage guidance
time cargo run -p kre8ntemjs_cli -- \
  --engine-cmd target/debug/scorewrap \
  --engine-args="--engine $(pwd)/tools/mock_engine.py" \
  --seeds ./seeds \
  --out ./test_baseline \
  --iters 500
```

```bash
# Coverage-guided: with --keep-only-increasing
time cargo run -p kre8ntemjs_cli -- \
  --engine-cmd target/debug/scorewrap \
  --engine-args="--engine $(pwd)/tools/mock_engine.py --write-file" \
  --seeds ./seeds \
  --out ./test_coverage \
  --iters 500 \
  --score-regex 'edges:(\d+)' \
  --keep-only-increasing
```

**Expected:** Coverage-guided mode saves significantly fewer artifacts

## 8. Real Engine Migration

When ready to use a real instrumented engine:

```bash
# Step 1: Instrument your d8 build to write /tmp/kre8_edges.txt
# Step 2: Replace mock engine with real engine

cargo run -p kre8ntemjs_cli -- \
  --engine-cmd target/debug/scorewrap \
  --engine-args="--engine /path/to/instrumented/d8 --your-coverage-flags" \
  --seeds ./seeds \
  --out ./real_artifacts \
  --iters 5000 \
  --score-regex 'edges:(\d+)' \
  --keep-only-increasing \
  --minimize-by coverage
```

**Expected:** Same interface, real coverage guidance

## Troubleshooting

### Issue: "edges:0" always printed
**Fix:** Mock engine needs `--write-file` or `--score-regex` to generate edge counts

### Issue: All programs saved (not just coverage-increasing)
**Fix:** Ensure `--keep-only-increasing` flag is set and `--score-regex` matches output

### Issue: Minimizer doesn't preserve coverage
**Fix:** Use `--minimize-by coverage` and ensure score regex is working

### Issue: scorewrap can't find engine
**Fix:** Use absolute paths: `--engine $(pwd)/tools/mock_engine.py`

## Success Criteria

✅ All builds succeed
✅ Mock engine produces consistent edge counts
✅ Scorewrap wrapper correctly parses coverage
✅ Fuzzer runs with `--keep-only-increasing` saves fewer artifacts
✅ Coverage-preserving minimizer works
✅ Dataflow analysis executes without errors
✅ Ready to migrate to real instrumented engine

---

**Note:** Clean up test directories after verification:
```bash
rm -rf test_artifacts test_sig_min test_cov_min test_dataflow test_baseline test_coverage
```
