
# TemuJsX — Template-based JS Engine Fuzzer (from scratch)

A clean-room, modular reimplementation of a high-level, **template-based** JavaScript engine fuzzer you can fully own and customize.

## Features (MVP)
- Seed → Template extraction via simple, pluggable rules (regex-based starter).
- High-level mutations: insertion, deletion, substitution, fusion.
- Concretization of placeholders with a context-aware generator (MVP minimal context).
- Engine runner adapters (shell out to `d8`, `jsc`, `js`/`spidermonkey`) with timeouts.
- Crash triage: exit codes + stderr fingerprint, auto-save repro cases.
- CLI for batch fuzzing over a seed corpus.

> This is intentionally minimal to get you started. Each module has `TODO` markers for deeper engines, coverage, and smarter extraction/mutation.

## Quick start
```bash
# 1) Install Rust
curl https://sh.rustup.rs -sSf | sh

# 2) Build
cd temujsx
cargo build

# 3) Run a quick smoke fuzz (requires a JS engine, e.g. V8's d8 on PATH)
cargo run -p temujsx_cli --   --engine-cmd "d8"   --seeds ./seeds   --out ./artifacts   --iters 1000 --timeout 500ms
```

## CLI
```
temujsx --engine-cmd <cmd> --seeds <dir> --out <dir> [--iters N] [--timeout 500ms]
```

## Layout
- `crates/temujsx_core` — core traits, template data types, extractor/mutator/concretizer, runner utils.
- `crates/temujsx_cli` — command-line entry point.

## Roadmap (suggested)
- [ ] Coverage adapters (sanitizer-coverage, pcguard, or engine-native flags).
- [ ] Smarter, AST-level extraction (Tree-sitter) and placeholder design.
- [ ] Dataflow-aware substitution heuristics.
- [ ] Structured fusion/splicing using CFG/DFG.
- [ ] Minimization and delta-debugging.
- [ ] Distributed scheduling.
- [ ] Crash dedup via stack hashes and root-cause clustering.

## License
MIT or Apache-2.0, at your option.
