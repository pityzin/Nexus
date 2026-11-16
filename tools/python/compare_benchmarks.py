"""Compare NEXUS benchmark JSON without third-party dependencies."""
import argparse
import json
import math
import statistics
import sys
from pathlib import Path
from typing import Any, Dict, List


def load_report(path: Path) -> Dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if value.get("schema_version") != 1 or value.get("kind") != "benchmark":
        raise ValueError(f"{path} is not a supported NEXUS benchmark report")
    samples = value.get("samples_ns")
    if not isinstance(samples, list) or any(not isinstance(v, (int, float)) or v < 0 for v in samples):
        raise ValueError(f"{path} has invalid sample values")
    if not samples:
        raise ValueError(f"{path} contains no timing samples")
    return value


def mean_ci(samples: List[float]) -> tuple[float, float, float]:
    mean = statistics.fmean(samples)
    if len(samples) < 2:
        return mean, mean, mean
    margin = 1.96 * statistics.stdev(samples) / math.sqrt(len(samples))
    return mean, max(0.0, mean - margin), mean + margin


def compare(current: Dict[str, Any], baseline: Dict[str, Any], threshold_pct: float, min_samples: int) -> Dict[str, Any]:
    for key in ("benchmark", "workload"):
        if current.get(key) != baseline.get(key):
            raise ValueError(f"reports differ in {key}")
    for key in ("os", "os_version", "architecture", "cpu", "rustc", "build_profile"):
        if current.get("environment", {}).get(key) != baseline.get("environment", {}).get(key):
            raise ValueError(f"reports differ in environment field {key}")
    a = [float(value) for value in baseline["samples_ns"]]
    b = [float(value) for value in current["samples_ns"]]
    a_mean, a_low, a_high = mean_ci(a)
    b_mean, b_low, b_high = mean_ci(b)
    a_median = statistics.median(a)
    b_median = statistics.median(b)
    change = ((b_median / a_median) - 1.0) * 100.0 if a_median > 0 else None
    if min(len(a), len(b)) < min_samples:
        status = "insufficient_samples"
    elif change is not None and change <= -threshold_pct:
        status = "improvement_candidate" if b_high < a_low else "inconclusive"
    elif change is not None and change >= threshold_pct:
        status = "regression_candidate" if b_low > a_high else "inconclusive"
    else:
        status = "no_regression"
    return {
        "schema_version": 1,
        "kind": "benchmark_comparison",
        "status": status,
        "threshold_percent": threshold_pct,
        "minimum_samples": min_samples,
        "baseline_samples": len(a),
        "current_samples": len(b),
        "baseline_median_ns": a_median,
        "current_median_ns": b_median,
        "median_change_percent": change,
        "baseline_mean_ci95_ns": [a_low, a_high],
        "current_mean_ci95_ns": [b_low, b_high],
        "note": "Candidate labels use non-overlapping approximate 95% mean intervals and a median threshold; repeat noisy runs before changing code.",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("current", type=Path)
    parser.add_argument("--threshold-percent", type=float, default=10.0)
    parser.add_argument("--minimum-samples", type=int, default=30)
    parser.add_argument("--fail-on-regression", action="store_true")
    options = parser.parse_args()
    if options.threshold_percent < 0 or options.minimum_samples < 2:
        parser.error("threshold must be nonnegative and minimum-samples at least 2")
    try:
        result = compare(load_report(options.current), load_report(options.baseline), options.threshold_percent, options.minimum_samples)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(json.dumps({"kind": "benchmark_comparison_error", "message": str(error)}), file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 1 if options.fail_on_regression and result["status"] == "regression_candidate" else 0


if __name__ == "__main__":
    raise SystemExit(main())
