"""Unit tests for the dependency-free benchmark comparison utility."""
import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("compare_benchmarks.py")
SPEC = importlib.util.spec_from_file_location("compare_benchmarks", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class ComparisonTests(unittest.TestCase):
    def report(self, samples):
        return {
            "schema_version": 1,
            "kind": "benchmark",
            "benchmark": "parse",
            "workload": {"format": "pe", "input_bytes": 100},
            "samples_ns": samples,
            "environment": {"os": "windows", "os_version": "test-os", "architecture": "x86_64", "cpu": "test", "rustc": "test", "build_profile": "release"},
        }

    def test_classifies_stable_and_clear_regression_runs(self):
        baseline = self.report([1000.0] * 30)
        same = MODULE.compare(self.report([1000.0] * 30), baseline, 10.0, 30)
        slower = MODULE.compare(self.report([1500.0] * 30), baseline, 10.0, 30)
        self.assertEqual(same["status"], "no_regression")
        self.assertEqual(slower["status"], "regression_candidate")
        self.assertEqual(slower["median_change_percent"], 50.0)

    def test_noise_and_small_sample_count_are_not_regressions(self):
        distribution = [100.0, 500.0] * 15
        baseline = self.report(distribution)
        noisy = MODULE.compare(self.report([100.0, 500.0] * 15), baseline, 10.0, 30)
        short = MODULE.compare(self.report([300.0] * 5), baseline, 10.0, 30)
        self.assertEqual(noisy["status"], "no_regression")
        self.assertEqual(short["status"], "insufficient_samples")

    def test_rejects_mismatched_environment(self):
        baseline = self.report([100.0] * 30)
        current = self.report([100.0] * 30)
        current["environment"]["cpu"] = "different"
        with self.assertRaisesRegex(ValueError, "environment field cpu"):
            MODULE.compare(current, baseline, 10.0, 30)


if __name__ == "__main__":
    unittest.main()
