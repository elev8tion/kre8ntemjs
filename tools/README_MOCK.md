# Mock Instrumented Engine

A Python-based mock JS engine for testing the coverage-guided fuzzing workflow without building an instrumented d8.

## Quick Start

```bash
# Test the mock engine directly
./tools/mock_engine.py --js seeds/example.js

# With file-based output
./tools/mock_engine.py --js seeds/example.js --write-file

# Simulate 10% crash rate
MOCK_CRASH_RATE=0.1 ./tools/mock_engine.py seeds/example.js
```

## Integration with Fuzzer

Use `scorewrap` to integrate the mock engine with the fuzzer:

```bash
# Build scorewrap wrapper
cargo build -p scorewrap

# Run coverage-guided fuzzing with mock engine
cargo run -p kre8ntemjs_cli -- \
  --engine-cmd target/debug/scorewrap \
  --engine-args "--engine $(pwd)/tools/mock_engine.py --engine-args --write-file --edges-file /tmp/kre8_edges.txt" \
  --seeds ./seeds \
  --out ./artifacts \
  --iters 500 \
  --timeout 500ms \
  --score-cmd-args "" \
  --score-regex 'edges:(\d+)' \
  --keep-only-increasing \
  --minimize-by coverage
```

## Configuration

### Modes

- `--mode rand` (default): Random edge count between min/max
- `--mode inc`: Deterministic count based on file size and hash

### Edge Count Range

- `--min <N>`: Minimum edge count (default: 100)
- `--max <N>`: Maximum edge count (default: 10000)

### Crash Simulation

- `--crash-rate <0.0-1.0>`: Probability of crash (default: 0.0)
- Files containing `CRASH` or `throw` keywords will always crash

### Output

- `--write-file`: Enable file output
- `--edges-file <path>`: Output file path (default: /tmp/kre8_edges.txt)

### Performance

- `--sleep-ms <N>`: Simulate execution delay in milliseconds
- `--seed <N>`: Random seed for reproducibility (default: 42)

## Environment Variables

All flags can be set via environment variables:

- `MOCK_EDGES_MODE`: Mode (rand/inc)
- `MOCK_EDGES_MIN`: Minimum edges
- `MOCK_EDGES_MAX`: Maximum edges
- `MOCK_CRASH_RATE`: Crash probability
- `MOCK_SEED`: Random seed
- `MOCK_SLEEP_MS`: Sleep duration

## Examples

### Test file-based coverage tracking
```bash
./tools/mock_engine.py --js test.js --write-file
cat /tmp/kre8_edges.txt
```

### Deterministic mode (same file = same edges)
```bash
./tools/mock_engine.py --mode inc --js test.js
```

### High crash rate for testing crash handling
```bash
MOCK_CRASH_RATE=0.5 ./tools/mock_engine.py test.js
```

### Simulate slow execution
```bash
./tools/mock_engine.py --js test.js --sleep-ms 100
```

## Swapping to Real Engine

When ready to use a real instrumented engine, replace the mock:

```bash
# Mock engine (testing)
--engine-cmd target/debug/scorewrap \
--engine-args "--engine ./tools/mock_engine.py ..."

# Real instrumented d8
--engine-cmd target/debug/scorewrap \
--engine-args "--engine /path/to/instrumented/d8 --engine-args --your-coverage-flags ..."
```

The interface stays the same - only the engine path changes!
