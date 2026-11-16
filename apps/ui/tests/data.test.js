import test from "node:test";
import assert from "node:assert/strict";
import { compareBenchmarkReports, formatAddress, formatBytes, parseLosslessJson, validateBenchmarkReport } from "../web/data.js";

function report(samples, environment = {}) {
  const mean = samples.reduce((sum, value) => sum + value, 0) / samples.length;
  const sorted = [...samples].sort((a, b) => a - b);
  const median = sorted[Math.floor(sorted.length / 2)];
  return {
    schema_version: 1,
    kind: "benchmark",
    benchmark: "metadata",
    workload: { format: "pe", input_bytes: 4096 },
    samples_ns: samples,
    statistics: {
      sample_count: samples.length,
      min_ns: sorted[0],
      median_ns: median,
      mean_ns: mean,
      max_ns: sorted.at(-1),
      stddev_ns: 0,
      mean_ci95_low_ns: mean,
      mean_ci95_high_ns: mean,
    },
    environment: { os: "windows", os_version: "10", architecture: "x86_64", cpu: "test", rustc: "test", build_profile: "release", ...environment },
  };
}

test("large NIR addresses remain exact and render as hexadecimal", () => {
  const value = parseLosslessJson('{"entry_point":18446744073709551615}');
  assert.equal(value.entry_point, "18446744073709551615");
  assert.equal(formatAddress(value.entry_point), "0xFFFFFFFFFFFFFFFF");
});

test("byte sizes are readable and invalid values are explicit", () => {
  assert.equal(formatBytes(1024), "1.0 KiB");
  assert.equal(formatBytes(-1), "—");
});

test("benchmark import validates schema and classifies matching reports", () => {
  const baseline = validateBenchmarkReport(report(Array(30).fill(1000)));
  const same = compareBenchmarkReports(report(Array(30).fill(1000)), baseline);
  assert.equal(same.status, "no_regression");
  const changedEnvironment = compareBenchmarkReports(report(Array(30).fill(1000), { cpu: "other" }), baseline);
  assert.equal(changedEnvironment.status, "incompatible");
  assert.throws(() => validateBenchmarkReport({ kind: "profile" }), /schema/);
  const nestedWorkload = report(Array(30).fill(1000));
  nestedWorkload.workload.untrusted = { nested: true };
  assert.throws(() => validateBenchmarkReport(nestedWorkload), /scalar fields/);
});
