#!/usr/bin/env python3
"""
Mock instrumented JS engine for testing coverage-guided fuzzing.
Prints edges:<N> to stdout and optionally writes to a file.
Simulates crashes based on content or random probability.
"""

import argparse
import hashlib
import os
import random
import sys
import time


def parse_args():
    parser = argparse.ArgumentParser(
        description="Mock instrumented JS engine for testing kre8ntemjs coverage workflow"
    )
    parser.add_argument(
        "js_file",
        nargs="?",
        help="JavaScript file to 'execute' (positional or --js)"
    )
    parser.add_argument(
        "--js",
        dest="js_file_flag",
        help="JavaScript file to 'execute' (flag form)"
    )
    parser.add_argument(
        "--mode",
        choices=["rand", "inc"],
        default=os.getenv("MOCK_EDGES_MODE", "rand"),
        help="Edge count mode: 'rand' (random) or 'inc' (deterministic increment based on file)"
    )
    parser.add_argument(
        "--min",
        type=int,
        default=int(os.getenv("MOCK_EDGES_MIN", "100")),
        help="Minimum edge count (rand mode)"
    )
    parser.add_argument(
        "--max",
        type=int,
        default=int(os.getenv("MOCK_EDGES_MAX", "10000")),
        help="Maximum edge count (rand mode)"
    )
    parser.add_argument(
        "--crash-rate",
        type=float,
        default=float(os.getenv("MOCK_CRASH_RATE", "0.0")),
        help="Probability of simulating a crash (0.0-1.0)"
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=int(os.getenv("MOCK_SEED", "42")),
        help="Random seed for reproducibility"
    )
    parser.add_argument(
        "--sleep-ms",
        type=int,
        default=int(os.getenv("MOCK_SLEEP_MS", "0")),
        help="Sleep duration in milliseconds (simulate execution time)"
    )
    parser.add_argument(
        "--write-file",
        action="store_true",
        help="Write edge count to file (use with --edges-file)"
    )
    parser.add_argument(
        "--edges-file",
        default="/tmp/kre8_edges.txt",
        help="Path to write edge count (default: /tmp/kre8_edges.txt)"
    )

    return parser.parse_args()


def compute_edge_count(js_file, mode, min_edges, max_edges, seed):
    """Compute edge count based on mode."""
    random.seed(seed)

    if mode == "rand":
        return random.randint(min_edges, max_edges)
    elif mode == "inc":
        # Deterministic increment based on file size and hash
        try:
            with open(js_file, "rb") as f:
                content = f.read()
            file_hash = int(hashlib.sha256(content).hexdigest()[:8], 16)
            file_size = len(content)
            # Normalize to reasonable range
            base = min_edges + (file_hash % (max_edges - min_edges))
            size_bonus = min(file_size * 10, max_edges // 2)
            return base + size_bonus
        except Exception:
            return min_edges
    else:
        return min_edges


def should_crash(js_file, crash_rate):
    """Determine if this execution should crash."""
    # Check for CRASH token in file
    try:
        with open(js_file, "r", encoding="utf-8", errors="ignore") as f:
            content = f.read()
        if "CRASH" in content or "throw" in content:
            return True
    except Exception:
        pass

    # Random crash based on probability
    return random.random() < crash_rate


def main():
    args = parse_args()

    # Determine JS file path
    js_file = args.js_file_flag or args.js_file
    if not js_file:
        print("Error: No JavaScript file provided", file=sys.stderr)
        sys.exit(1)

    if not os.path.exists(js_file):
        print(f"Error: File not found: {js_file}", file=sys.stderr)
        sys.exit(1)

    # Simulate execution time
    if args.sleep_ms > 0:
        time.sleep(args.sleep_ms / 1000.0)

    # Compute edge count
    edges = compute_edge_count(js_file, args.mode, args.min, args.max, args.seed)

    # Print to stdout
    print(f"edges:{edges}")

    # Optionally write to file
    if args.write_file or args.edges_file != "/tmp/kre8_edges.txt":
        try:
            with open(args.edges_file, "w") as f:
                f.write(f"edges:{edges}\n")
        except Exception as e:
            print(f"Warning: Failed to write edges file: {e}", file=sys.stderr)

    # Check if we should crash
    if should_crash(js_file, args.crash_rate):
        print("ReferenceError: mock_var is not defined", file=sys.stderr)
        print("  at <anonymous>:1:1", file=sys.stderr)
        sys.exit(1)

    # Success
    sys.exit(0)


if __name__ == "__main__":
    main()
