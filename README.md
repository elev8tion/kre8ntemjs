
# Kre8ntemJS — Template-based JS Engine Fuzzer

A modular, **template-based** JavaScript engine fuzzer with dataflow analysis and coverage-guided fuzzing capabilities.

## Features
- **AST-based extraction** with Tree-sitter for semantic-aware template generation
- **Dataflow-aware fuzzing** using def/use analysis to bias variable selection
- **Coverage-guided mode** with external scoring and increasing-only gate
- **Dual minimizer modes**: signature-preserving or coverage-preserving
- **Crash deduplication** with SHA1-based signature hashing
- High-level mutations: insertion, deletion, substitution, fusion
- Scope-coherent concretization (reuses in-template identifiers)
- Engine adapters for d8, jsc, spidermonkey with configurable timeouts
- Boring crash filtering (de-prioritizes simple ReferenceErrors)

> Production-ready fuzzing infrastructure with instrumented engine support via `scorewrap`.

## Quick start
```bash
# 1) Install Rust
curl https://sh.rustup.rs -sSf | sh

# 2) Build
cd kre8ntemjs
cargo build

# 3) Run basic fuzzing (requires a JS engine, e.g. V8's d8)
cargo run -p kre8ntemjs_cli -- \
  --engine-cmd "d8" \
  --seeds ./seeds \
  --out ./artifacts \
  --iters 1000 \
  --timeout 500ms
```

## CLI
```
kre8ntemjs_cli --engine-cmd <cmd> --seeds <dir> --out <dir> [--iters N] [--timeout 500ms]
```

## Coverage Guidance (Option B: Instrumented Engine + Wrapper)

We use a tiny wrapper (`tools/scorewrap`) so the fuzzer can stay engine-agnostic. The wrapper:
- Runs the real engine (instrumented).
- Reads a coverage count either from a file (preferred) or by regex.
- Prints `edges:<N>` so the fuzzer can parse it and keep only coverage-increasing programs.

### Build the wrapper
```bash
cargo build -p scorewrap
```

### Testing with Mock Engine

Use the included mock engine to test the coverage workflow without building instrumented d8:

```bash
# Test mock engine directly
./tools/mock_engine.py --js seeds/example.js --write-file

# Run fuzzer with mock engine
cargo run -p kre8ntemjs_cli -- \
  --engine-cmd target/debug/scorewrap \
  --engine-args "--engine $(pwd)/tools/mock_engine.py --engine-args --write-file --edges-file /tmp/kre8_edges.txt" \
  --seeds ./seeds \
  --out ./artifacts \
  --iters 500 \
  --score-regex 'edges:(\d+)' \
  --keep-only-increasing \
  --minimize-by coverage
```

See `tools/README_MOCK.md` for full mock engine documentation.

### Option 1: File-based counter (recommended)
Instrument your engine to write edge count to `/tmp/kre8_edges.txt`:
```bash
cargo run -p kre8ntemjs_cli -- \
  --engine-cmd target/debug/scorewrap \
  --engine-args "--engine /path/to/instrumented/d8 --engine-args --your-flags --edges-file /tmp/kre8_edges.txt" \
  --seeds ./seeds \
  --out ./artifacts \
  --iters 2000 \
  --timeout 500ms \
  --score-cmd-args "" \
  --score-regex 'edges:(\d+)' \
  --keep-only-increasing \
  --minimize-by coverage
```

### Option 2: Regex-based counter
If your engine prints `edges: 12345` to stdout/stderr:
```bash
cargo run -p kre8ntemjs_cli -- \
  --engine-cmd target/debug/scorewrap \
  --engine-args "--engine /path/to/instrumented/d8 --engine-args --your-flags --score-regex edges:\s*(\d+)" \
  --seeds ./seeds \
  --out ./artifacts \
  --iters 2000 \
  --timeout 500ms \
  --score-cmd-args "" \
  --score-regex 'edges:(\d+)' \
  --keep-only-increasing \
  --minimize-by coverage
```

**Benefits:**
- Only saves coverage-increasing test cases
- Minimizer preserves coverage while shrinking crashes
- Works with any instrumented engine (d8, jsc, custom)

## Layout
- `crates/kre8ntemjs_core` — Core fuzzing infrastructure (AST, dataflow, extractor, mutator, concretizer, minimizer)
- `crates/kre8ntemjs_cli` — Command-line fuzzer
- `tools/scorewrap` — Coverage wrapper for instrumented engines

## Implemented Features
- [x] AST-level extraction with Tree-sitter
- [x] Dataflow-aware substitution heuristics (DFComp)
- [x] Coverage-guided fuzzing with external scoring
- [x] Dual minimization modes (signature/coverage)
- [x] Crash deduplication via normalized stack hashing
- [x] Boring crash filtering

## Roadmap
- [ ] Structured fusion/splicing using CFG/DFG analysis
- [ ] Distributed fuzzing coordinator
- [ ] Custom instrumentation integration
- [ ] Taint tracking for input-to-crash correlation

## License
MIT or Apache-2.0, at your option.
