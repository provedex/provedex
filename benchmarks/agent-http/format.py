#!/usr/bin/env python3
"""Format one oha JSON result as a single line for the bench summary."""
import json
import os
import sys

label = os.environ["LABEL"]
path = os.environ["OUT"]
d = json.load(open(path))
sec = d["summary"]
pct = d["latencyPercentiles"]
print(
    f"{label:<48} "
    f"median={pct['p50']*1000:7.2f} ms  "
    f"p95={pct['p95']*1000:7.2f} ms  "
    f"p99={pct['p99']*1000:7.2f} ms  "
    f"rps={sec['requestsPerSec']:8.1f}"
)
