export function parseLosslessJson(source) {
  return JSON.parse(source, (_key, value, context) => {
    if (typeof value === "number" && !Number.isSafeInteger(value) && context?.source) {
      return context.source;
    }
    return value;
  });
}

export function formatAddress(value, prefix = "0x") {
  if (typeof value === "string" && /^\d+$/.test(value)) {
    return `${prefix}${BigInt(value).toString(16).toUpperCase()}`;
  }
  if (typeof value === "number" && Number.isFinite(value) && value >= 0) {
    return `${prefix}${Math.trunc(value).toString(16).toUpperCase()}`;
  }
  return "—";
}

export function formatBytes(value) {
  if (!Number.isFinite(value) || value < 0) return "—";
  if (value < 1024) return `${value} B`;
  const units = ["KiB", "MiB", "GiB"];
  let amount = value;
  let unit = "B";
  for (const candidate of units) {
    amount /= 1024;
    unit = candidate;
    if (amount < 1024 || candidate === "GiB") break;
  }
  return `${amount.toFixed(1)} ${unit}`;
}

export function validateBenchmarkReport(report) {
  if (!report || report.schema_version !== 1 || report.kind !== "benchmark") {
    throw new Error("Unsupported NEXUS benchmark report schema.");
  }
  if (typeof report.benchmark !== "string" || !report.workload || typeof report.workload !== "object" || Array.isArray(report.workload)) {
    throw new Error("Benchmark report is missing its workload identity.");
  }
  const workloadValues = Object.values(report.workload);
  if (workloadValues.length > 32 || workloadValues.some((value) => value !== null && !["string", "number", "boolean"].includes(typeof value))) {
    throw new Error("Benchmark workload must contain only bounded scalar fields.");
  }
  const samples = report.samples_ns;
  const count = report.statistics?.sample_count;
  if (!Array.isArray(samples) || samples.length === 0 || samples.length > 10000 || count !== samples.length) {
    throw new Error("Benchmark report has invalid timing samples.");
  }
  if (samples.some((sample) => typeof sample !== "number" || !Number.isFinite(sample) || sample < 0)) {
    throw new Error("Benchmark samples must be finite nonnegative numbers.");
  }
  for (const field of ["min_ns", "median_ns", "mean_ns", "max_ns", "stddev_ns", "mean_ci95_low_ns", "mean_ci95_high_ns"]) {
    if (typeof report.statistics[field] !== "number" || !Number.isFinite(report.statistics[field])) {
      throw new Error(`Benchmark statistic ${field} is invalid.`);
    }
  }
  if (!report.environment || typeof report.environment !== "object" || Array.isArray(report.environment)) {
    throw new Error("Benchmark report is missing environment metadata.");
  }
  return report;
}

function stableJson(value) {
  if (Array.isArray(value)) return `[${value.map(stableJson).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableJson(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

export function compareBenchmarkReports(current, baseline, thresholdPercent = 10, minimumSamples = 30) {
  validateBenchmarkReport(current);
  validateBenchmarkReport(baseline);
  if (current.benchmark !== baseline.benchmark || stableJson(current.workload) !== stableJson(baseline.workload)) {
    return { status: "incompatible", reason: "workload differs" };
  }
  const environmentFields = ["os", "os_version", "architecture", "cpu", "rustc", "build_profile"];
  if (environmentFields.some((field) => current.environment?.[field] !== baseline.environment?.[field])) {
    return { status: "incompatible", reason: "environment differs" };
  }
  const a = baseline.statistics;
  const b = current.statistics;
  const medianChange = a.median_ns > 0 ? ((b.median_ns / a.median_ns) - 1) * 100 : null;
  let status = "no_regression";
  if (Math.min(current.samples_ns.length, baseline.samples_ns.length) < minimumSamples) {
    status = "insufficient_samples";
  } else if (medianChange !== null && medianChange <= -thresholdPercent) {
    status = b.mean_ci95_high_ns < a.mean_ci95_low_ns ? "improvement_candidate" : "inconclusive";
  } else if (medianChange !== null && medianChange >= thresholdPercent) {
    status = b.mean_ci95_low_ns > a.mean_ci95_high_ns ? "regression_candidate" : "inconclusive";
  }
  return { status, medianChange, thresholdPercent, minimumSamples };
}
