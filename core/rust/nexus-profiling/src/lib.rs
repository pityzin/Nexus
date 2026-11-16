//! Opt-in, dependency-free timing and benchmark statistics for existing NEXUS workloads.
//! Normal CLI commands never call this crate, so they incur no profiling overhead.

use std::{fmt, time::Instant};

pub const REPORT_SCHEMA_VERSION: u32 = 1;
pub const MAX_SAMPLES: usize = 10_000;
pub const MAX_WARMUPS: usize = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SamplingConfig {
    pub warmup_iterations: usize,
    pub sample_iterations: usize,
}

impl SamplingConfig {
    pub fn new(warmup_iterations: usize, sample_iterations: usize) -> Result<Self, ProfileError> {
        if warmup_iterations > MAX_WARMUPS {
            return Err(ProfileError::InvalidConfig("warmups exceed 10000"));
        }
        if !(1..=MAX_SAMPLES).contains(&sample_iterations) {
            return Err(ProfileError::InvalidConfig(
                "samples must be between 1 and 10000",
            ));
        }
        Ok(Self {
            warmup_iterations,
            sample_iterations,
        })
    }
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 5,
            sample_iterations: 30,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ProfileError {
    InvalidConfig(&'static str),
    AllocationFailed,
    Workload(String),
    EmptySamples,
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => write!(f, "invalid sampling configuration: {message}"),
            Self::AllocationFailed => f.write_str("unable to reserve benchmark sample storage"),
            Self::Workload(message) => write!(f, "profiled workload failed: {message}"),
            Self::EmptySamples => f.write_str("benchmark requires at least one sample"),
        }
    }
}

impl std::error::Error for ProfileError {}

#[derive(Clone, Debug, PartialEq)]
pub struct Statistics {
    pub sample_count: usize,
    pub min_ns: u128,
    pub median_ns: f64,
    pub mean_ns: f64,
    pub max_ns: u128,
    pub stddev_ns: f64,
    pub mean_ci95_low_ns: f64,
    pub mean_ci95_high_ns: f64,
}

impl Statistics {
    pub fn from_samples(samples: &[u128]) -> Result<Self, ProfileError> {
        if samples.is_empty() {
            return Err(ProfileError::EmptySamples);
        }
        let mut sorted = Vec::new();
        sorted
            .try_reserve_exact(samples.len())
            .map_err(|_| ProfileError::AllocationFailed)?;
        sorted.extend_from_slice(samples);
        sorted.sort_unstable();
        let sum = samples.iter().map(|&x| x as f64).sum::<f64>();
        let mean = sum / samples.len() as f64;
        let median = if sorted.len() % 2 == 0 {
            (sorted[sorted.len() / 2 - 1] as f64 + sorted[sorted.len() / 2] as f64) / 2.0
        } else {
            sorted[sorted.len() / 2] as f64
        };
        let variance = if samples.len() > 1 {
            samples
                .iter()
                .map(|&x| {
                    let delta = x as f64 - mean;
                    delta * delta
                })
                .sum::<f64>()
                / (samples.len() - 1) as f64
        } else {
            0.0
        };
        let stddev = variance.sqrt();
        let margin = 1.96 * stddev / (samples.len() as f64).sqrt();
        Ok(Self {
            sample_count: samples.len(),
            min_ns: sorted[0],
            median_ns: median,
            mean_ns: mean,
            max_ns: *sorted.last().unwrap_or(&sorted[0]),
            stddev_ns: stddev,
            mean_ci95_low_ns: (mean - margin).max(0.0),
            mean_ci95_high_ns: mean + margin,
        })
    }

    /// Returns a regression only for sufficiently sampled, non-overlapping 95% mean intervals.
    pub fn regression_percent(&self, baseline: &Self, min_percent: f64) -> Option<f64> {
        if self.sample_count < 30
            || baseline.sample_count < 30
            || self.mean_ci95_low_ns <= baseline.mean_ci95_high_ns
            || baseline.mean_ns <= 0.0
            || baseline.median_ns <= 0.0
        {
            return None;
        }
        let percent = (self.median_ns / baseline.median_ns - 1.0) * 100.0;
        (percent >= min_percent).then_some(percent)
    }
}

pub fn measure<F, E>(config: SamplingConfig, mut workload: F) -> Result<Vec<u128>, ProfileError>
where
    F: FnMut() -> Result<(), E>,
    E: fmt::Display,
{
    for _ in 0..config.warmup_iterations {
        workload().map_err(|error| ProfileError::Workload(error.to_string()))?;
    }
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(config.sample_iterations)
        .map_err(|_| ProfileError::AllocationFailed)?;
    for _ in 0..config.sample_iterations {
        let start = Instant::now();
        workload().map_err(|error| ProfileError::Workload(error.to_string()))?;
        samples.push(start.elapsed().as_nanos());
    }
    Ok(samples)
}

pub fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            c if c.is_control() => output.push_str(&format!("\\u{:04x}", c as u32)),
            c => output.push(c),
        }
    }
    output.push('"');
    output
}

pub fn environment_json() -> String {
    let cpu = std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "not-reported".into());
    let os_version = std::env::var("NEXUS_OS_VERSION").unwrap_or_else(|_| "not-reported".into());
    format!(
        "{{\"os\":{},\"os_version\":{},\"architecture\":{},\"cpu\":{},\"rustc\":{},\"nexus_version\":{},\"build_profile\":{}}}",
        json_string(std::env::consts::OS),
        json_string(&os_version),
        json_string(std::env::consts::ARCH),
        json_string(&cpu),
        json_string(env!("NEXUS_RUSTC_VERSION")),
        json_string(env!("CARGO_PKG_VERSION")),
        json_string(if cfg!(debug_assertions) { "debug" } else { "release" }),
    )
}

pub fn statistics_json(stats: &Statistics) -> String {
    format!(
        "{{\"sample_count\":{},\"min_ns\":{},\"median_ns\":{},\"mean_ns\":{},\"max_ns\":{},\"stddev_ns\":{},\"mean_ci95_low_ns\":{},\"mean_ci95_high_ns\":{}}}",
        stats.sample_count,
        stats.min_ns,
        stats.median_ns,
        stats.mean_ns,
        stats.max_ns,
        stats.stddev_ns,
        stats.mean_ci95_low_ns,
        stats.mean_ci95_high_ns
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_summary_statistics_and_interval() {
        let stats = Statistics::from_samples(&[10, 20, 30, 40]).unwrap();
        assert_eq!(stats.min_ns, 10);
        assert_eq!(stats.max_ns, 40);
        assert_eq!(stats.median_ns, 25.0);
        assert_eq!(stats.mean_ns, 25.0);
        assert!(stats.stddev_ns > 0.0);
        assert!(stats.mean_ci95_low_ns < stats.mean_ns);
        assert!(stats.mean_ci95_high_ns > stats.mean_ns);
    }

    #[test]
    fn rejects_empty_samples_and_unbounded_configs() {
        assert_eq!(
            Statistics::from_samples(&[]),
            Err(ProfileError::EmptySamples)
        );
        assert!(SamplingConfig::new(0, 0).is_err());
        assert!(SamplingConfig::new(0, MAX_SAMPLES + 1).is_err());
        assert!(SamplingConfig::new(MAX_WARMUPS + 1, 1).is_err());
    }

    #[test]
    fn only_flags_regression_when_confidence_intervals_separate() {
        let baseline = Statistics::from_samples(&[100; 30]).unwrap();
        let noisy = Statistics::from_samples(&[100, 500, 100, 500, 100, 500]).unwrap();
        let slower = Statistics::from_samples(&[200; 30]).unwrap();
        assert_eq!(noisy.regression_percent(&baseline, 10.0), None);
        assert_eq!(slower.regression_percent(&baseline, 10.0), Some(100.0));
    }

    #[test]
    fn measure_runs_warmup_and_samples_and_propagates_failure() {
        let mut calls = 0;
        let samples = measure(SamplingConfig::new(2, 3).unwrap(), || {
            calls += 1;
            Ok::<_, &'static str>(())
        })
        .unwrap();
        assert_eq!(calls, 5);
        assert_eq!(samples.len(), 3);
        let error = measure(SamplingConfig::new(0, 1).unwrap(), || Err("fixture"));
        assert_eq!(error, Err(ProfileError::Workload("fixture".into())));
    }

    #[test]
    fn json_escaping_handles_control_characters() {
        assert_eq!(json_string("a\"\\\n"), r#""a\"\\\n""#);
        assert!(environment_json().starts_with("{\"os\":"));
    }
}
