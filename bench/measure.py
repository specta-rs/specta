#!/usr/bin/env python3
"""Measures `#[derive(Type)]` expansion cost for a benchmark fixture.

Usage: bench/measure.py <label> [iterations] [--fixture <name>|--fixture-path <path>]

Defaults to `bench/derive-perf`. `bench/derive-inst` measures the
instantiation-heavy fixture instead.

Writes `bench/results/<label>.json` and prints a summary of:
  * expanded code size (bytes of `-Zunpretty=expanded`) — deterministic;
  * `-Ztime-passes` timings for expansion, type-check and codegen;
  * wall clock and CPU time of the whole `cargo rustc` invocation.

Pass `--check` to only compile.
"""

import argparse
import pathlib
import sys

import harness

parser = argparse.ArgumentParser(description="Measure a derive benchmark fixture.")
parser.add_argument("label", nargs="?", default="run")
parser.add_argument("iterations", nargs="?", type=int, default=9)
parser.add_argument(
    "--fixture",
    default="derive-perf",
    help="fixture directory name under bench/ (default: derive-perf)",
)
parser.add_argument(
    "--fixture-path",
    type=pathlib.Path,
    help="fixture directory (overrides --fixture)",
)
parser.add_argument("--check", action="store_true", help="only compile the fixture")
args = parser.parse_args()

fixture = args.fixture_path or harness.BENCH / args.fixture
if not (fixture / "Cargo.toml").exists():
    sys.exit(f"no fixture at {fixture}; run the matching generator first")

if args.check:
    harness.cargo(fixture, "build")
    sys.exit(0)

result = harness.measure(fixture, args.label, args.iterations)
harness.write_result(result)
harness.print_result(result)
