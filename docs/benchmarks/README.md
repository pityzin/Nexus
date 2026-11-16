# Benchmark protocol

The standalone Criterion.rs suite compares the assembly-backed native add ABI call with scalar Rust wrapping addition and compares native/Rust scalar checksum loops over fixed 64-byte, 4-KiB, and 1-MiB inputs. Run it with `cargo bench --manifest-path benchmarks/rust/Cargo.toml --bench native_interop`. Criterion is kept out of the application workspace so product builds do not require benchmark-only dependencies.

`nexus benchmark <file>` measures the currently implemented PE/ELF metadata parser in memory, emits raw samples and statistical summaries, and checks output equivalence on every iteration. `nexus profile <file>` measures read-snapshot and parse wall time for one operation. The throughput field is an equivalent container-size rate: this parser reads metadata and validates ranges, but does not scan section payload bytes. `nexus benchmark native-ffi` measures a bounded batch of native ABI additions; `nexus benchmark memory-allocation` measures bounded 4 KiB vector allocation/free pairs. These workloads are distinct and must only be compared against the same benchmark name and workload. The allocation measurement does not instrument the allocator. Save JSON output as a baseline only with its input size, format, build, CPU, OS, compiler, and sample settings intact. Set `NEXUS_OS_VERSION` in PowerShell to include an explicit OS version. Use `tools/python/compare_benchmarks.py` to compare same-environment reports. The tool requires at least 30 samples and reports a candidate regression only when the median threshold and approximate 95% mean intervals both support it. Re-run noisy comparisons; the tool is advisory and does not affect builds by default.

## Initial measured baseline

The checked-in Windows x64 reports under `benchmarks/baselines/windows-x64` are measured release runs, not fabricated targets. They were captured on Windows 10.0.19045.0 (build 19045.6466) with Rust 1.98.0, x86-64, the CPU environment identifier `AMD64 Family 25 Model 80 Stepping 0, AuthenticAMD`, and the release NEXUS CLI as the PE workload. Each used 100 warmups and 1,000 samples.

For metadata parsing (305,152-byte PE with five sections), median was 2,300 ns (mean 3,449.8 ns; min 1,500 ns; max 742,000 ns; sample standard deviation 23,491.9 ns), with an equivalent container-size rate of 132,674,782,609 bytes/s. One one-shot profile measured a 330,500 ns file snapshot and a 14,900 ns initial parse; these include cold-path effects and are not directly comparable to warmed samples.

For the native ABI microbenchmark, a batch of 256 calls had median 1,000 ns (mean 1,085.1 ns; min 600 ns; max 33,600 ns), about 256 million calls/s derived from batch median. For the allocation workload, a batch of 256 allocate/free pairs requesting 4 KiB each had median 13,100 ns (mean 15,281.5 ns; min 9,000 ns; max 296,200 ns), about 19.5 million pairs/s. That is operation throughput, not live memory or allocator event telemetry.

The PE rate is not a physical scan rate; only headers, section metadata, and bounds were processed. These are single-machine initial runs and should not be generalized to other workloads. The large maximum samples show scheduling/noise outliers. No optimization was made from them; no stable hot-path bottleneck has been established.

Reproduce from PowerShell after building Release:

```powershell
$env:NEXUS_OS_VERSION = [Environment]::OSVersion.VersionString
$report = target\x86_64-pc-windows-msvc\release\nexus.exe benchmark target\x86_64-pc-windows-msvc\release\nexus.exe --warmup 100 --iterations 1000 --format json | Out-String
[IO.File]::WriteAllText('.\current-run.json', $report, [Text.UTF8Encoding]::new($false))
python .\tools\python\compare_benchmarks.py .\benchmarks\baselines\windows-x64\pe-metadata-parse.json .\current-run.json
```

Compare only matching workload and environment fields. The initial run compared with itself produces `no_regression`; this validates the comparator contract and is not evidence of improvement.

Current profiling is opt-in and reports wall time and workload-specific throughput. CPU sampling, allocator event counts, peak memory, worker utilization, and analyses from missing Phases 03/04 are not measured.

For every result, record CPU model, operating system/version, Rust/C++ compiler versions where relevant, target, optimization configuration, benchmark revision (the current workspace is not a Git repository), dataset or workload, warm-up/sample settings, and whether the machine was otherwise idle. Keep the raw Criterion output with the run artifacts. Do not compare results from different machines without labeling the difference.

Future decoding, CFG, symbols, relocation, and cross-reference benchmarks require those analysis stages to exist. They should use fixed, checksummed corpora and report bytes or instructions processed as appropriate. Google Benchmark may be introduced with a meaningful C++ workload. No results are fabricated.
