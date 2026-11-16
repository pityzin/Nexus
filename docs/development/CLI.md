# NEXUS CLI

Binary inspection commands read a file as data. They never execute it.

```powershell
nexus inspect .\sample.exe
nexus headers .\sample.exe
nexus sections .\sample.exe
nexus info .\sample.exe
nexus inspect .\sample.elf --format json
nexus headers .\sample.elf --format json
nexus sections .\sample.elf --format json
```

`--format` accepts `text` (default) or `json` and follows the file path. JSON is deterministic NIR v1. `nexus info` without a path continues to show NEXUS runtime/build information. Unsupported formats and malformed input exit with code 2 and one JSON error object on stderr. Successful commands exit 0.

`NEXUS_MAX_INPUT_BYTES` changes the default 512 MiB per-file cap, up to 2 GiB. Invalid values are reported instead of silently ignored.

## Local browser workbench

```powershell
nexus ui
```

The command binds a local server to `127.0.0.1` on an ephemeral port and prints the URL to open. Select a file in the browser to inspect it; the UI sends its bytes to the existing Rust parser and does not send a path. The UI cap is 128 MiB or `NEXUS_MAX_INPUT_BYTES`, whichever is lower. Stop the server with Ctrl+C. This phase ships a local browser workbench, not a native Tauri package.

## Profiling and benchmarks

Profiling is opt-in and runs only when these commands are called; ordinary inspection commands do not collect timing data.

```powershell
nexus profile .\sample.exe --format json
nexus benchmark .\sample.exe --warmup 20 --iterations 1000 --format json > parse-run.json
nexus benchmark native-ffi --warmup 20 --iterations 1000 --format json
nexus benchmark memory-allocation --warmup 20 --iterations 1000 --format json
python tools/python/compare_benchmarks.py benchmarks/baselines/windows-x64/pe-metadata-parse.json parse-run.json
```

`profile` reports file snapshot and metadata parse wall times. `benchmark` measures repeated in-memory PE/ELF metadata parsing after loading the file once, verifies each parse result against the initial result, and exports raw samples plus min/median/mean/max, sample standard deviation, an approximate 95% mean interval, throughput, workload, and environment metadata. Sample counts are bounded to 10,000. Use identical input and environment metadata for comparisons. The comparator requires at least 30 samples by default and labels regressions only when the median threshold and non-overlapping approximate intervals both agree; it does not fail unless `--fail-on-regression` is supplied.

`benchmark native-ffi` measures batches of calls to the implemented native integer ABI. `benchmark memory-allocation` measures bounded 4 KiB vector allocation/free pairs. Both are microbenchmarks, report operation throughput and batch timing, and do not provide allocator telemetry or a Rust-only comparison baseline. Their result schema identifies them separately from parser results.

These commands do not profile CPU call stacks, allocation counts, worker utilization, or Phases 03/04 analysis stages. The report identifies those workloads as unavailable rather than treating parser timings as a proxy for them.
# Plugin commands

`nexus plugins` lists plugins compiled into this CLI build. `nexus export <file> --plugin metadata-summary` parses a supported PE/ELF file and emits the selected plugin's JSON summary. The command uses the same bounded read-only loader as `inspect`; it does not execute the input.

This release includes only the `metadata-summary` example plugin. Third-party plugins can implement the Rust SDK trait and register with `nexus-plugin-host` when building a host, but this CLI does not load external libraries or plugin files.

