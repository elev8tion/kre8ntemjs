#!/usr/bin/env python3
"""
Mock instrumented JS engine.

- Accepts a JS file as input (last arg or --js path).
- Prints `edges:<N>` to stdout.
- Optionally writes the same to /tmp/kre8_edges.txt (or --edges-file path).
- Exits with 0 for success, non-zero to simulate "crashes".

Environment variables (or flags) to control behavior:
- MOCK_EDGES_MODE=rand|inc      (default: rand)
- MOCK_EDGES_MIN=100            (default: 100)
- MOCK_EDGES_MAX=5000           (default: 5000)
- MOCK_CRASH_RATE=0.05          (default: 0.0)  # probability of simulating a crash
- MOCK_SEED=...                 (default: None) # for reproducibility
- MOCK_SLEEP_MS=...             (default: 0)    # simulate engine runtime

CLI flags override env:
  --mode {rand,inc}
  --min N
  --max N
  --crash-rate R
  --seed S
  --sleep-ms MS
  --edges-file PATH             (default: /tmp/kre8_edges.txt if --write-file set)
  --write-file                  (toggle; write edges to file)
  --js PATH                     (if not provided as trailing arg)
"""

import os, sys, time, argparse, random, hashlib, pathlib

def parse_args():
    p = argparse.ArgumentParser(description="Mock instrumented JS engine")
    p.add_argument("--mode", choices=["rand","inc"], default=os.getenv("MOCK_EDGES_MODE","rand"))
    p.add_argument("--min", type=int, default=int(os.getenv("MOCK_EDGES_MIN","100")))
    p.add_argument("--max", type=int, default=int(os.getenv("MOCK_EDGES_MAX","5000")))
    p.add_argument("--crash-rate", type=float, default=float(os.getenv("MOCK_CRASH_RATE","0.0")))
    p.add_argument("--seed", type=str, default=os.getenv("MOCK_SEED"))
    p.add_argument("--sleep-ms", type=int, default=int(os.getenv("MOCK_SLEEP_MS","0")))
    p.add_argument("--edges-file", type=str, default=None)
    p.add_argument("--write-file", action="store_true")
    p.add_argument("--js", type=str, default=None, help="JS file path (optional if provided as trailing arg)")
    p.add_argument("positional_js", nargs="?", help="JS file path")
    return p.parse_args()

def main():
    args = parse_args()
    js_path = args.js or args.positional_js
    if not js_path:
        print("Usage error: missing JS file (use --js or provide as last arg).", file=sys.stderr)
        sys.exit(2)

    if not os.path.exists(js_path):
        print(f"File not found: {js_path}", file=sys.stderr)
        sys.exit(2)

    # Seed RNG
    if args.mode == "rand":
        if args.seed:
            random.seed(args.seed)
        else:
            try:
                h = hashlib.sha1(pathlib.Path(js_path).read_bytes()).hexdigest()
            except Exception:
                h = str(time.time_ns())
            random.seed(h + str(time.time_ns()))
    # inc mode is deterministic on file props

    # Sleep to simulate runtime
    if args.sleep_ms > 0:
        time.sleep(args.sleep_ms / 1000.0)

    # Decide crash
    crashed = False
    try:
        content = pathlib.Path(js_path).read_text(errors="ignore")
    except Exception:
        content = ""
    if "CRASH" in content:
        crashed = True
    else:
        if random.random() < args.crash_rate:
            crashed = True

    # Compute edges
    if args.mode == "rand":
        lo = max(0, args.min)
        hi = max(args.min, args.max)
        edges = random.randint(lo, hi)
    else:
        st = os.stat(js_path)
        edges = max(args.min, int(st.st_size + (st.st_mtime_ns % 10_000)) % max(args.min, args.max))

    line = f"edges:{edges}"
    print(line)

    if args.write_file:
        path = args.edges_file or "/tmp/kre8_edges.txt"
        try:
            with open(path, "w") as f:
                f.write(line + "\n")
        except Exception as e:
            print(f"warn: failed to write {path}: {e}", file=sys.stderr)

    sys.exit(1 if crashed else 0)

if __name__ == "__main__":
    main()
