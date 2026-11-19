"""Shared measurement helpers for the derive benchmark fixtures.

`measure.py` measures one fixture and records a result file; `ablate.py` runs
the same measurement over a set of throwaway variants. Both go through here so
the numbers are produced identically.
"""

import json
import os
import pathlib
import re
import resource
import statistics
import subprocess
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
BENCH = ROOT / "bench"

PASSES = (
    "expand_crate",
    "type_check_crate",
    "borrow_check_crate",
    "codegen_crate",
    "total",
)

# Recorded alongside the table (the instantiation fixture lives or dies by
# these), but not printed so the table stays comparable across rounds.
EXTRA_PASSES = (
    "monomorphization_collector_root_collections",
    "monomorphization_collector_graph_walk",
)

# `~/.cargo/config.toml` routes rustc through `kache`, which replays cached
# stderr, so every repeated measurement came back byte-identical. Bypass it.
ENV = {**os.environ, "CARGO_BUILD_RUSTC_WRAPPER": ""}


def cargo(fixture, *args):
    return subprocess.run(
        ["cargo", *args],
        cwd=fixture,
        env=ENV,
        capture_output=True,
        text=True,
        check=True,
    )


def fixture_metadata(fixture):
    """Read the `fixture.json` a generator writes, if there is one."""
    path = pathlib.Path(fixture) / "fixture.json"
    return json.loads(path.read_text()) if path.exists() else {}


def build_deps(fixture):
    """Build dependencies (including the proc macro) outside the measured runs."""
    cargo(fixture, "build", "--quiet")


def expand(fixture):
    """Return `(bytes, non_whitespace_bytes)` of the fully expanded crate."""
    expanded = subprocess.run(
        ["cargo", "+nightly", "rustc", "--lib", "--quiet", "--", "-Zunpretty=expanded"],
        cwd=fixture,
        env=ENV,
        capture_output=True,
        text=True,
        check=True,
    )
    text = expanded.stdout
    return len(text.encode()), len("".join(text.split()).encode())


def time_crate(fixture, iters):
    """Time `iters` clean compiles of the fixture and return per-pass min/median."""
    samples = []
    for _ in range(iters):
        (pathlib.Path(fixture) / "src" / "lib.rs").touch()
        before = resource.getrusage(resource.RUSAGE_CHILDREN)
        start = time.perf_counter()
        run = subprocess.run(
            ["cargo", "+nightly", "rustc", "--lib", "--quiet", "--", "-Ztime-passes"],
            cwd=fixture,
            env=ENV,
            capture_output=True,
            text=True,
        )
        wall = time.perf_counter() - start
        after = resource.getrusage(resource.RUSAGE_CHILDREN)
        if run.returncode != 0:
            raise RuntimeError(run.stderr)

        passes = {
            name: float(value)
            for value, name in re.findall(r"time:\s+([\d.]+);[^\t]*\t(\S+)", run.stderr)
            if name in PASSES or name in EXTRA_PASSES
        }
        # Peak resident memory rustc reaches, which tracks the AST and the
        # expanded crate rather than the macro's own allocation traffic.
        rss = re.search(r"rss:\s+(\d+)MB ->\s+(\d+)MB[^\n]*\ttotal", run.stderr)
        if rss:
            passes["rss_start_mb"] = float(rss.group(1))
            passes["rss_peak_mb"] = float(rss.group(2))
        passes["wall"] = wall
        passes["cpu"] = (after.ru_utime - before.ru_utime) + (
            after.ru_stime - before.ru_stime
        )
        samples.append(passes)

    return {
        name: {
            "min": min(s[name] for s in samples),
            "median": statistics.median(s[name] for s in samples),
        }
        for name in samples[0]
    }


def measure(fixture, label, iters):
    """Measure one fixture end to end and return the result dict."""
    fixture = pathlib.Path(fixture)
    build_deps(fixture)
    expanded_bytes, expanded_tokens = expand(fixture)
    return {
        "label": label,
        "fixture": fixture.name,
        "iters": iters,
        "rustc": subprocess.run(
            ["rustc", "+nightly", "-V"], capture_output=True, text=True
        ).stdout.strip(),
        "fixture_metadata": fixture_metadata(fixture),
        "expanded_bytes": expanded_bytes,
        "expanded_tokens": expanded_tokens,
        "passes": time_crate(fixture, iters),
    }


def write_result(result):
    out = BENCH / "results" / f"{result['label']}.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, indent=2) + "\n")
    return out


def print_result(result):
    meta = result.get("fixture_metadata") or {}
    print(f"fixture         {result['fixture']}")
    print(f"derives         {meta.get('derives', 'n/a')}")
    print(f"instantiations  {meta.get('instantiations', 0):,}")
    print(f"expanded_bytes  {result['expanded_bytes']:,}")
    print(f"expanded_tokens {result['expanded_tokens']:,}")
    print(f"{'pass':22} {'min':>8} {'median':>8}")
    for name in PASSES:
        if name not in result["passes"]:
            continue
        value = result["passes"][name]
        print(f"{name:22} {value['min']:8.3f} {value['median']:8.3f}")
