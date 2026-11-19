#!/usr/bin/env python3
"""Measures `#[derive(Type)]` expansion cost for `bench/derive-perf`.

Usage: bench/measure.py <label> [iterations]

Writes `bench/results/<label>.json` and prints a summary of:
  * expanded code size (bytes of `-Zunpretty=expanded`) — deterministic;
  * `-Ztime-passes` timings for expansion, type-check and codegen;
  * wall clock and CPU time of the whole `cargo rustc` invocation.

Pass `--check` to only compile.
"""

import json
import os
import pathlib
import re
import resource
import statistics
import subprocess
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "bench" / "derive-perf"

label = sys.argv[1]
iters = int(sys.argv[2]) if len(sys.argv) > 2 else 9

PASSES = (
    "expand_crate",
    "type_check_crate",
    "borrow_check_crate",
    "codegen_crate",
    "total",
)

# `~/.cargo/config.toml` routes rustc through `kache`, which replays cached
# stderr, so every repeated measurement came back byte-identical. Bypass it.
ENV = {**os.environ, "CARGO_BUILD_RUSTC_WRAPPER": ""}


def cargo(*args, capture=True):
    return subprocess.run(
        ["cargo", *args],
        cwd=FIXTURE,
        env=ENV,
        capture_output=capture,
        text=True,
        check=True,
    )


# Build dependencies (including the proc macro) outside the measured runs.
cargo("build", "--quiet")

expanded = subprocess.run(
    ["cargo", "+nightly", "rustc", "--lib", "--quiet", "--", "-Zunpretty=expanded"],
    cwd=FIXTURE,
    env=ENV,
    capture_output=True,
    text=True,
    check=True,
)
expanded_bytes = len(expanded.stdout.encode())
# Pretty-printed indentation is most of `expanded_bytes` and costs rustc
# nothing, so also report the bytes rustc actually has to chew on.
expanded_tokens = len("".join(expanded.stdout.split()).encode())

samples = []
for _ in range(iters):
    (FIXTURE / "src" / "lib.rs").touch()
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.perf_counter()
    run = subprocess.run(
        ["cargo", "+nightly", "rustc", "--lib", "--quiet", "--", "-Ztime-passes"],
        cwd=FIXTURE,
        env=ENV,
        capture_output=True,
        text=True,
    )
    wall = time.perf_counter() - start
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    if run.returncode != 0:
        sys.exit(run.stderr)

    passes = {
        name: float(value)
        for value, name in re.findall(r"time:\s+([\d.]+);[^\t]*\t(\S+)", run.stderr)
        if name in PASSES
    }
    passes["wall"] = wall
    passes["cpu"] = (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime)
    samples.append(passes)

result = {
    "label": label,
    "iters": iters,
    "rustc": subprocess.run(["rustc", "+nightly", "-V"], capture_output=True, text=True).stdout.strip(),
    "expanded_bytes": expanded_bytes,
    "expanded_tokens": expanded_tokens,
    "passes": {
        name: {
            "min": min(s[name] for s in samples),
            "median": statistics.median(s[name] for s in samples),
        }
        for name in samples[0]
    },
}

out = ROOT / "bench" / "results" / f"{label}.json"
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(result, indent=2) + "\n")

print(f"expanded_bytes  {expanded_bytes:,}")
print(f"expanded_tokens {expanded_tokens:,}")
print(f"{'pass':22} {'min':>8} {'median':>8}")
for name, value in result["passes"].items():
    print(f"{name:22} {value['min']:8.3f} {value['median']:8.3f}")
