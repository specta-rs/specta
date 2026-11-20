"""Measure the derive's cost at several fixture sizes, for both trees.

    python3 bench/scaling.py <label> [iterations]

Writes `bench/results/scaling-<label>.json`, a list of measurements over
`SCALE` (plain derives) and `INST` (instantiations). Regenerating the fixtures
is part of the run, so the same command reproduces the numbers in the PR for
either checkout: run it once per tree with a different label, then fit
`bench/scaling_fit.py` over the two files.
"""

import json
import os
import pathlib
import subprocess
import sys

import harness

label = sys.argv[1]
iters = int(sys.argv[2]) if len(sys.argv) > 2 else 5

DERIVE_SCALES = (1, 2, 4, 8)
INST_COUNTS = (100, 250, 500, 1000)

results = []


def generate(script, env):
    subprocess.run(
        [sys.executable, str(harness.BENCH / script)],
        cwd=harness.ROOT,
        env={**os.environ, **env},
        check=True,
        capture_output=True,
    )


for scale in DERIVE_SCALES:
    generate("gen.py", {"SCALE": str(scale)})
    result = harness.measure(
        harness.BENCH / "derive-perf", f"{label}-derive-{scale}", iters
    )
    harness.write_result(result)
    results.append(result)
    print(f"derive scale {scale}: {result['fixture_metadata']}", flush=True)

for inst in INST_COUNTS:
    generate("gen_inst.py", {"INST": str(inst)})
    result = harness.measure(
        harness.BENCH / "derive-inst", f"{label}-inst-{inst}", iters
    )
    harness.write_result(result)
    results.append(result)
    print(f"inst {inst}: {result['fixture_metadata']}", flush=True)

out = harness.BENCH / "results" / f"scaling-{label}.json"
out.write_text(json.dumps(results, indent=2) + "\n")
print(f"wrote {out}")

# Leave the fixtures at the size the rest of the harness expects.
generate("gen.py", {"SCALE": "4"})
generate("gen_inst.py", {"INST": "200"})
