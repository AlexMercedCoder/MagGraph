#!/usr/bin/env python3
"""Convert index-scale CSV output into a versioned regression report."""

from __future__ import annotations

import argparse
import json
import platform
from pathlib import Path

FIELDS = ("open_ms", "search_ms", "hybrid_ms", "backlinks_ms", "recall_us", "update_us")


def parse_rows(text: str) -> dict[str, dict[str, float]]:
    rows: dict[str, dict[str, float]] = {}
    for line in text.splitlines():
        parts = [part.strip() for part in line.split(",")]
        if len(parts) != 7 or not parts[0].isdigit():
            continue
        rows[parts[0]] = {field: float(value) for field, value in zip(FIELDS, parts[1:], strict=True)}
    return rows


def build_report(rows: dict[str, dict[str, float]], baseline: dict) -> dict:
    comparisons = []
    ok = True
    for size, expected in baseline["tiers"].items():
        actual = rows.get(size)
        if actual is None:
            comparisons.append({"nodes": int(size), "ok": False, "error": "missing benchmark row"})
            ok = False
            continue
        metrics = {}
        tier_ok = True
        for field in FIELDS:
            reference = float(expected[field])
            ratio = actual[field] / reference if reference else 0.0
            metric_ok = ratio <= float(baseline["max_regression_ratio"])
            metrics[field] = {"value": actual[field], "baseline": reference, "ratio": round(ratio, 3), "ok": metric_ok}
            tier_ok = tier_ok and metric_ok
        comparisons.append({"nodes": int(size), "ok": tier_ok, "metrics": metrics})
        ok = ok and tier_ok
    return {
        "schema": "maggraph.benchmark-report.v1",
        "ok": ok,
        "environment": {"system": platform.system(), "machine": platform.machine(), "python": platform.python_version()},
        "baseline_id": baseline["id"],
        "max_regression_ratio": baseline["max_regression_ratio"],
        "comparisons": comparisons,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--baseline", default="planning/benchmark-baseline.json")
    parser.add_argument("--output", default="target/benchmark-report.json")
    args = parser.parse_args()
    rows = parse_rows(Path(args.input).read_text(encoding="utf-8", errors="replace"))
    baseline = json.loads(Path(args.baseline).read_text(encoding="utf-8"))
    report = build_report(rows, baseline)
    target = Path(args.output)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
