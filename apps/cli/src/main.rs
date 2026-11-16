use nexus_api::API_VERSION;
use nexus_core::nexus_binary_loader::{parse_bytes, BinaryFile, BinaryLimits};
use nexus_core::nexus_binary_loader::{BinaryMetadata, BinarySection, FormatHeaders};
use nexus_core::{default_registry, load_binary, native_add, CoreConfig};
use nexus_example_metadata_plugin::MetadataSummaryPlugin;
use nexus_plugin_api::{PluginInput, PLUGIN_API_VERSION};
use nexus_plugin_host::{PluginHost, PluginLimits};
use nexus_profiling::{
    environment_json, measure, statistics_json, SamplingConfig, Statistics, REPORT_SCHEMA_VERSION,
};
use std::{ffi::OsString, hint::black_box, path::Path, time::Instant};

mod ui;

const HELP: &str = r#"NEXUS - Modular Binary Analysis & Execution Engine

USAGE:
    nexus <COMMAND>

COMMANDS:
    info                 Show runtime information
    info <file>          Show parsed binary information
    inspect <file>       Detect format and show a summary
    headers <file>       Show PE/ELF headers
    sections <file>      Show section metadata and permissions
    profile <file>       Measure file-read and parse wall time
    ui                   Start the loopback analysis interface
    plugins              List compiled, trusted Rust plugins
    export <file> --plugin <id>  Run a metadata export plugin
    <command> <file> --format json   Emit structured JSON
    modules              List registered analysis modules
    benchmark <file>     Measure PE/ELF metadata parsing
    benchmark native-ffi Measure native ABI call throughput
    benchmark memory-allocation Measure bounded allocation throughput
    help                 Show this message
    --version            Show version

Analyzed files are read as data and are never executed."#;

fn run(args: &[OsString]) -> Result<String, String> {
    if args.first().and_then(|value| value.to_str()) == Some("plugins") {
        return run_plugins(args);
    }
    if args.first().and_then(|value| value.to_str()) == Some("export") {
        return run_plugin_export(args);
    }
    if args
        .first()
        .and_then(|value| value.to_str())
        .is_some_and(|command| matches!(command, "benchmark" | "profile"))
    {
        return run_performance_command(args);
    }
    match args.len() {
        0 => Ok(HELP.to_owned()),
        1 => match args[0]
            .to_str()
            .ok_or_else(|| "command name is not valid Unicode".to_owned())?
        {
            "--help" | "-h" | "help" => Ok(HELP.to_owned()),
            "--version" | "-V" => Ok(format!(
                "nexus {} (API {})",
                env!("CARGO_PKG_VERSION"),
                API_VERSION
            )),
            "info" => {
                let config = CoreConfig::from_env().map_err(|e| e.to_string())?;
                let native = native_add(20, 22).map_err(|e| format!("native service: {e}"))?;
                Ok(format!("NEXUS {}\nAPI version: {}\nNative ABI: available (20 + 22 = {native})\nMaximum configured input: {} bytes\nAnalysis mode: static; input binaries are never executed", env!("CARGO_PKG_VERSION"), API_VERSION, config.resource_limits.max_input_bytes))
            }
            "modules" => {
                let registry = default_registry().map_err(|e| format!("module registry: {e}"))?;
                let mut output = String::from("NAME\tVERSION\tCAPABILITIES\n");
                for module in registry.list() {
                    let capabilities = module
                        .capabilities
                        .iter()
                        .map(|cap| cap.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    output.push_str(&format!(
                        "{}\t{}\t{}\n",
                        module.name, module.version, capabilities
                    ));
                }
                Ok(output.trim_end().to_owned())
            }
            "benchmark" => Err(benchmark_usage().to_owned()),
            "profile" => Err("usage: nexus profile <file> [--format text|json]".to_owned()),
            command => Err(format!("unknown command: {command}\n\n{HELP}")),
        },
        2 | 4 => {
            let command = args[0]
                .to_str()
                .ok_or_else(|| "command name is not valid Unicode".to_owned())?;
            if !matches!(command, "inspect" | "headers" | "sections" | "info") {
                return Err(format!("unknown command: {command}\n\n{HELP}"));
            }
            let output_format = if args.len() == 2 {
                "text"
            } else {
                if args[2] != "--format" {
                    return Err("expected --format json".into());
                }
                args[3]
                    .to_str()
                    .ok_or_else(|| "output format is not valid Unicode".to_owned())?
            };
            if !matches!(output_format, "text" | "json") {
                return Err("output format must be text or json".into());
            }
            let config = CoreConfig::from_env().map_err(|e| e.to_string())?;
            let metadata = load_binary(Path::new(&args[1]), config.resource_limits)
                .map_err(|e| e.to_string())?;
            Ok(render(command, output_format, &metadata))
        }
        _ => Err(format!("unknown command or invalid arguments\n\n{HELP}")),
    }
}

fn builtin_plugins() -> Result<PluginHost, String> {
    let mut host = PluginHost::new(PLUGIN_API_VERSION, PluginLimits::default())
        .map_err(|error| error.to_string())?;
    host.register(Box::new(MetadataSummaryPlugin))
        .map_err(|error| error.to_string())?;
    Ok(host)
}

fn run_plugins(args: &[OsString]) -> Result<String, String> {
    if args.len() != 1 {
        return Err("usage: nexus plugins".to_owned());
    }
    let host = builtin_plugins()?;
    let mut output = String::from("ID\tVERSION\tSTATE\tEXTENSION POINT\tCAPABILITIES\n");
    for status in host.list() {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            status.descriptor.id(),
            status.descriptor.version(),
            if status.enabled {
                "enabled"
            } else {
                "disabled"
            },
            status.descriptor.extension_point().as_str(),
            status.descriptor.capabilities().join(", ")
        ));
    }
    Ok(output.trim_end().to_owned())
}

fn run_plugin_export(args: &[OsString]) -> Result<String, String> {
    if args.len() != 4 || args[2] != "--plugin" {
        return Err("usage: nexus export <file> --plugin <id>".to_owned());
    }
    let plugin_id = args[3]
        .to_str()
        .ok_or_else(|| "plugin id is not valid Unicode".to_owned())?;
    let config = CoreConfig::from_env().map_err(|error| error.to_string())?;
    let metadata = load_binary(Path::new(&args[1]), config.resource_limits)
        .map_err(|error| error.to_string())?;
    let section_count = u32::try_from(metadata.sections.len())
        .map_err(|_| "section count does not fit the plugin API".to_owned())?;
    let input = PluginInput::new(
        metadata.format.as_str(),
        metadata.architecture.as_str(),
        metadata.bitness,
        metadata.file_size,
        metadata.entry_point,
        section_count,
    );
    builtin_plugins()?
        .execute(plugin_id, input)
        .map_err(|error| error.to_string())
}

fn run_performance_command(args: &[OsString]) -> Result<String, String> {
    let command = args[0]
        .to_str()
        .ok_or_else(|| "command name is not valid Unicode".to_owned())?;
    if args.len() < 2 || args[1] == "--help" {
        return Ok(match command {
            "benchmark" => benchmark_usage().into(),
            _ => "usage: nexus profile <file> [--format text|json]".into(),
        });
    }
    let mut output_format = "text";
    let mut warmups = 5_usize;
    let mut iterations = 30_usize;
    let mut index = 2;
    while index < args.len() {
        let option = args[index]
            .to_str()
            .ok_or_else(|| "option is not valid Unicode".to_owned())?;
        index += 1;
        match option {
            "--format" => {
                let value = args.get(index).ok_or("--format requires text or json")?;
                output_format = value
                    .to_str()
                    .ok_or_else(|| "format is not valid Unicode".to_owned())?;
                index += 1;
                if !matches!(output_format, "text" | "json") {
                    return Err("format must be text or json".into());
                }
            }
            "--iterations" if command == "benchmark" => {
                iterations = parse_count(args.get(index), "--iterations")?;
                index += 1;
            }
            "--warmup" if command == "benchmark" => {
                warmups = parse_count(args.get(index), "--warmup")?;
                index += 1;
            }
            _ => return Err(format!("unsupported option for {command}: {option}")),
        }
    }

    if command == "benchmark" {
        let workload = args[1]
            .to_str()
            .ok_or_else(|| "benchmark workload is not valid Unicode".to_owned())?;
        if matches!(workload, "native-ffi" | "memory-allocation") {
            return run_microbenchmark(workload, output_format, warmups, iterations);
        }
    }

    let config = CoreConfig::from_env().map_err(|error| error.to_string())?;
    let max_file_size = u64::try_from(config.resource_limits.max_input_bytes)
        .map_err(|_| "configured input limit does not fit in u64".to_owned())?;
    let io_start = Instant::now();
    let input = BinaryFile::open(
        &args[1],
        BinaryLimits::new(max_file_size).map_err(|e| e.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let read_ns = io_start.elapsed().as_nanos();
    let parse_start = Instant::now();
    let metadata = input.parse().map_err(|error| error.to_string())?;
    let parse_ns = parse_start.elapsed().as_nanos();

    if command == "profile" {
        return Ok(render_profile(
            output_format,
            &metadata,
            input.len() as u64,
            read_ns,
            parse_ns,
        ));
    }

    let sampling = SamplingConfig::new(warmups, iterations).map_err(|error| error.to_string())?;
    let expected = metadata.clone();
    let samples = measure(sampling, || {
        let parsed = parse_bytes(input.bytes()).map_err(|error| error.to_string())?;
        if parsed != expected {
            return Err("parser output changed between repeated runs".to_owned());
        }
        Ok::<(), String>(())
    })
    .map_err(|error| error.to_string())?;
    let stats = Statistics::from_samples(&samples).map_err(|error| error.to_string())?;
    Ok(render_benchmark(
        output_format,
        &metadata,
        input.len() as u64,
        sampling,
        &samples,
        &stats,
    ))
}

fn parse_count(value: Option<&OsString>, option: &str) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("{option} requires an integer"))?
        .to_str()
        .ok_or_else(|| format!("{option} value is not valid Unicode"))?
        .parse::<usize>()
        .map_err(|_| format!("{option} requires a positive integer"))
}

fn benchmark_usage() -> &'static str {
    "usage: nexus benchmark <file|native-ffi|memory-allocation> [--warmup N] [--iterations N] [--format text|json]"
}

fn run_microbenchmark(
    workload: &str,
    output_format: &str,
    warmups: usize,
    iterations: usize,
) -> Result<String, String> {
    const OPERATIONS_PER_SAMPLE: u64 = 256;
    const ALLOCATION_BYTES: usize = 4096;
    let sampling = SamplingConfig::new(warmups, iterations).map_err(|error| error.to_string())?;
    let samples = measure(sampling, || {
        match workload {
            "native-ffi" => {
                for _ in 0..OPERATIONS_PER_SAMPLE {
                    let result = native_add(black_box(20), black_box(22))
                        .map_err(|error| error.to_string())?;
                    if black_box(result) != 42 {
                        return Err("native ABI returned an inconsistent result".to_owned());
                    }
                }
            }
            "memory-allocation" => {
                for index in 0..OPERATIONS_PER_SAMPLE {
                    let mut buffer = Vec::<u8>::new();
                    buffer
                        .try_reserve_exact(ALLOCATION_BYTES)
                        .map_err(|error| format!("bounded sample allocation failed: {error}"))?;
                    buffer.push(index as u8);
                    black_box(buffer);
                }
            }
            _ => return Err("unsupported microbenchmark workload".to_owned()),
        }
        Ok::<(), String>(())
    })
    .map_err(|error| error.to_string())?;
    let stats = Statistics::from_samples(&samples).map_err(|error| error.to_string())?;
    Ok(render_microbenchmark(
        workload,
        output_format,
        sampling,
        &samples,
        &stats,
        OPERATIONS_PER_SAMPLE,
        ALLOCATION_BYTES,
    ))
}

fn render_microbenchmark(
    workload: &str,
    output_format: &str,
    sampling: SamplingConfig,
    samples: &[u128],
    stats: &Statistics,
    operations_per_sample: u64,
    allocation_bytes: usize,
) -> String {
    let benchmark = match workload {
        "native-ffi" => "native_add_i64_ffi",
        _ => "heap_alloc_dealloc_4096",
    };
    let operations_per_second = if stats.median_ns == 0.0 {
        0.0
    } else {
        operations_per_sample as f64 * 1_000_000_000.0 / stats.median_ns
    };
    if output_format == "json" {
        let raw_samples = samples
            .iter()
            .map(u128::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let workload_details = if workload == "native-ffi" {
            format!("{{\"operations_per_sample\":{operations_per_sample}}}")
        } else {
            format!("{{\"allocation_pairs_per_sample\":{operations_per_sample},\"requested_bytes_per_allocation\":{allocation_bytes}}}")
        };
        return format!(
            "{{\"schema_version\":{REPORT_SCHEMA_VERSION},\"kind\":\"benchmark\",\"benchmark\":{},\"captured_at_unix_ms\":{},\"workload\":{},\"sampling\":{{\"warmups\":{},\"iterations\":{}}},\"statistics\":{},\"throughput_operations_per_second\":{},\"samples_ns\":[{}],\"coverage\":[{}],\"unavailable\":[\"CPU sampling\",\"allocator telemetry\",\"instruction decoding\",\"NIR transforms\",\"CFG\",\"symbols\",\"relocations\",\"cross-references\",\"data flow\",\"parallel analysis\"],\"environment\":{}}}",
            json_string(benchmark),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            workload_details,
            sampling.warmup_iterations,
            sampling.sample_iterations,
            statistics_json(stats),
            operations_per_second,
            raw_samples,
            json_string(benchmark),
            environment_json(),
        );
    }
    format!(
        "Benchmark: {benchmark}\nWarmups: {}; samples: {}; operations per sample: {operations_per_sample}\nMin batch: {} ns\nMedian batch: {:.1} ns\nMean batch: {:.1} ns\nMax batch: {} ns\nStddev: {:.1} ns\nThroughput: {:.2} operations/s\nMemory metric: {}",
        sampling.warmup_iterations,
        sampling.sample_iterations,
        stats.min_ns,
        stats.median_ns,
        stats.mean_ns,
        stats.max_ns,
        stats.stddev_ns,
        operations_per_second,
        if workload == "memory-allocation" {
            format!("{} allocation/free pairs of {allocation_bytes} bytes per sample (requested size; allocator telemetry unavailable)", operations_per_sample)
        } else {
            "not applicable".to_owned()
        },
    )
}

fn render_profile(
    output_format: &str,
    metadata: &BinaryMetadata,
    bytes: u64,
    read_ns: u128,
    parse_ns: u128,
) -> String {
    if output_format == "json" {
        return format!(
            "{{\"schema_version\":{REPORT_SCHEMA_VERSION},\"kind\":\"profile\",\"coverage\":\"file_read_and_metadata_parse_wall_time\",\"format\":{},\"architecture\":{},\"input_bytes\":{},\"section_count\":{},\"stages\":[{{\"name\":\"read_snapshot\",\"elapsed_ns\":{read_ns}}},{{\"name\":\"metadata_parse\",\"elapsed_ns\":{parse_ns}}}],\"memory_metrics\":\"unsupported\",\"cpu_sampling\":\"unsupported\",\"environment\":{}}}",
            json_string(metadata.format.as_str()),
            json_string(metadata.architecture.as_str()),
            bytes,
            metadata.sections.len(),
            environment_json(),
        );
    }
    format!(
        "Profile (wall time only)\nFormat: {}\nArchitecture: {}\nInput: {} bytes\nSections: {}\nRead snapshot: {} ns\nMetadata parse: {} ns\nMemory metrics: unsupported\nCPU sampling: unsupported",
        metadata.format.as_str(), metadata.architecture.as_str(), bytes, metadata.sections.len(), read_ns, parse_ns
    )
}

fn render_benchmark(
    output_format: &str,
    metadata: &BinaryMetadata,
    bytes: u64,
    sampling: SamplingConfig,
    samples: &[u128],
    stats: &Statistics,
) -> String {
    let throughput = if stats.median_ns == 0.0 {
        0.0
    } else {
        bytes as f64 * 1_000_000_000.0 / stats.median_ns
    };
    if output_format == "json" {
        let raw_samples = samples
            .iter()
            .map(u128::to_string)
            .collect::<Vec<_>>()
            .join(",");
        return format!(
            "{{\"schema_version\":{REPORT_SCHEMA_VERSION},\"kind\":\"benchmark\",\"benchmark\":\"binary_metadata_parse\",\"captured_at_unix_ms\":{},\"workload\":{{\"format\":{},\"architecture\":{},\"input_bytes\":{},\"section_count\":{}}},\"sampling\":{{\"warmups\":{},\"iterations\":{}}},\"statistics\":{},\"throughput_bytes_per_second\":{},\"throughput_basis\":\"container_size_equivalent; metadata parser does not scan section payloads\",\"samples_ns\":[{}],\"coverage\":[\"PE/ELF metadata parsing\"],\"unavailable\":[\"instruction decoding\",\"NIR instruction transforms\",\"CFG\",\"symbols\",\"relocations\",\"cross-references\",\"data flow\",\"parallel analysis\",\"allocation counts\"],\"environment\":{}}}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            json_string(metadata.format.as_str()),
            json_string(metadata.architecture.as_str()),
            bytes,
            metadata.sections.len(),
            sampling.warmup_iterations,
            sampling.sample_iterations,
            statistics_json(stats),
            throughput,
            raw_samples,
            environment_json(),
        );
    }
    format!(
        "Binary metadata parse benchmark\nFormat: {}\nArchitecture: {}\nInput: {} bytes; {} sections\nWarmups: {}; samples: {}\nMin: {} ns\nMedian: {:.1} ns\nMean: {:.1} ns\nMax: {} ns\nStddev: {:.1} ns\n95% mean CI: [{:.1}, {:.1}] ns\nEquivalent container-size throughput: {:.2} bytes/s (metadata parser does not scan section payloads)",
        metadata.format.as_str(), metadata.architecture.as_str(), bytes, metadata.sections.len(),
        sampling.warmup_iterations, sampling.sample_iterations, stats.min_ns, stats.median_ns,
        stats.mean_ns, stats.max_ns, stats.stddev_ns, stats.mean_ci95_low_ns,
        stats.mean_ci95_high_ns, throughput
    )
}

fn render(command: &str, output_format: &str, metadata: &BinaryMetadata) -> String {
    if output_format == "json" {
        return match command {
            "headers" => format!(
                "{{\"schema_version\":1,\"format\":{},\"headers\":{}}}",
                nexus_core::nexus_binary_loader::nexus_nir::json_string(metadata.format.as_str()),
                metadata.headers_json()
            ),
            "sections" => format!(
                "{{\"schema_version\":1,\"format\":{},\"sections\":{}}}",
                nexus_core::nexus_binary_loader::nexus_nir::json_string(metadata.format.as_str()),
                metadata.sections_json()
            ),
            _ => metadata.inspect_json(),
        };
    }
    match command {
        "headers" => render_headers(metadata),
        "sections" => render_sections(&metadata.sections),
        _ => format!("Format: {}\nArchitecture: {} ({}-bit, {})\nFile size: {} bytes\nEntry point: {} 0x{:x}\nSections: {}", metadata.format.as_str().to_uppercase(), metadata.architecture.as_str(), metadata.bitness, metadata.endianness.as_str(), metadata.file_size, metadata.entry_point_kind.as_str(), metadata.entry_point, metadata.sections.len()),
    }
}

fn render_headers(metadata: &BinaryMetadata) -> String {
    match &metadata.headers {
        FormatHeaders::Pe(h) => {
            let mut out=format!("PE headers\nPE offset: 0x{:x}\nMachine: 0x{:04x}\nSections: {}\nTimestamp: 0x{:08x}\nCOFF characteristics: 0x{:04x}\nOptional header: 0x{:x} ({} bytes)\nEntry point RVA: 0x{:x}\nImage base: 0x{:x}\nSection alignment: 0x{:x}\nFile alignment: 0x{:x}\nImage size: 0x{:x}\nHeaders size: 0x{:x}\nSubsystem: {}\nDLL characteristics: 0x{:04x}\nData directories: {}", h.pe_offset,h.machine,h.section_count,h.timestamp,h.characteristics,h.optional_magic,h.optional_header_size,h.entry_point_rva,h.image_base,h.section_alignment,h.file_alignment,h.image_size,h.headers_size,h.subsystem,h.dll_characteristics,h.data_directories.len());
            for directory in &h.data_directories { out.push_str(&format!("\n  {}: {} 0x{:x}, size 0x{:x}",directory.name,directory.address_kind.as_str(),directory.address,directory.size)); }
            out
        }
        FormatHeaders::Elf(h) => format!("ELF headers\nClass: {}-bit\nEndianness: {}\nOS ABI: {}\nABI version: {}\nType: {} ({})\nMachine: 0x{:04x}\nVersion: {}\nEntry point: 0x{:x}\nFlags: 0x{:x}\nHeader size: {}\nProgram headers: offset 0x{:x}, entry size {}, count {}\nSection headers: offset 0x{:x}, entry size {}, count {}\nSection name string table index: {}",h.class_bits,h.endianness.as_str(),h.os_abi,h.abi_version,elf_type(h.file_type),h.file_type,h.machine,h.version,metadata.entry_point,h.flags,h.header_size,h.program_header_offset,h.program_header_entry_size,h.program_header_count,h.section_header_offset,h.section_header_entry_size,h.section_header_count,h.section_name_string_table_index),
    }
}

fn render_sections(sections: &[BinarySection]) -> String {
    let mut out = String::from(
        "INDEX\tNAME\tVIRTUAL ADDRESS\tVIRTUAL SIZE\tRAW OFFSET\tRAW SIZE\tR/W/X\tFLAGS\n",
    );
    for s in sections {
        out.push_str(&format!(
            "{}\t{}\t0x{:x}\t0x{:x}\t0x{:x}\t0x{:x}\t{}/{}/{}\t0x{:x}\n",
            s.index,
            s.name,
            s.virtual_address,
            s.virtual_size,
            s.raw_data_offset,
            s.raw_data_size,
            if s.permissions.read { 'r' } else { '-' },
            if s.permissions.write { 'w' } else { '-' },
            if s.permissions.execute { 'x' } else { '-' },
            s.flags
        ));
    }
    out.trim_end().to_owned()
}

fn elf_type(value: u16) -> &'static str {
    match value {
        0 => "NONE",
        1 => "REL",
        2 => "EXEC",
        3 => "DYN",
        4 => "CORE",
        _ => "OTHER",
    }
}

fn main() {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.first().and_then(|value| value.to_str()) == Some("ui") {
        if args.len() == 2 && args[1] == "--help" {
            println!("usage: nexus ui");
            return;
        }
        if args.len() != 1 {
            eprintln!("{{\"level\":\"error\",\"event\":\"command_failed\",\"message\":\"usage: nexus ui\"}}");
            std::process::exit(2);
        }
        if let Err(error) = ui::serve() {
            eprintln!(
                "{{\"level\":\"error\",\"event\":\"ui_server_failed\",\"message\":{}}}",
                json_string(&error)
            );
            std::process::exit(2);
        }
        return;
    }
    match run(&args) {
        Ok(output) => println!("{output}"),
        Err(message) => {
            eprintln!(
                "{{\"level\":\"error\",\"event\":\"command_failed\",\"message\":{}}}",
                json_string(&message)
            );
            std::process::exit(2);
        }
    }
}

fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", ch as u32));
            }
            ch => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests {
    use super::run;
    use std::ffi::OsString;
    fn args(values: &[&str]) -> Vec<std::ffi::OsString> {
        values.iter().map(std::ffi::OsString::from).collect()
    }
    #[test]
    fn exposes_help_version_and_real_registered_modules() {
        let help = run(&args(&["--help"])).unwrap();
        assert!(
            help.contains("COMMANDS:") && help.contains("Start the loopback analysis interface")
        );
        assert!(help.contains("metadata export plugin"));
        assert!(run(&args(&["--version"])).unwrap().starts_with("nexus "));
        let modules = run(&args(&["modules"])).unwrap();
        assert!(
            modules.contains("formats.pe")
                && modules.contains("formats.elf")
                && !modules.contains("formats.macho")
        );
    }

    #[test]
    fn lists_and_runs_the_compiled_metadata_export_plugin() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let plugins = run(&args(&["plugins"])).unwrap();
        assert!(plugins.contains("metadata-summary"));
        assert!(plugins.contains("metadata_export"));

        let mut bytes = vec![0_u8; 64];
        bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
        bytes[18..20].copy_from_slice(&62_u16.to_le_bytes());
        bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
        bytes[24..32].copy_from_slice(&0x400000_u64.to_le_bytes());
        bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
        bytes[54..56].copy_from_slice(&56_u16.to_le_bytes());
        bytes[58..60].copy_from_slice(&64_u16.to_le_bytes());
        let path = std::env::temp_dir().join(format!(
            "nexus-plugin-cli-{}-{}.elf",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, bytes).unwrap();
        let result = run(&[
            OsString::from("export"),
            path.as_os_str().to_owned(),
            OsString::from("--plugin"),
            OsString::from("metadata-summary"),
        ]);
        let _ = std::fs::remove_file(path);
        let output = result.unwrap();
        assert!(output.contains("\"format\":\"elf\""));
        assert!(output.contains("\"architecture\":\"x86_64\""));
        assert!(output.contains("\"entry_point\":4194304"));
    }
    #[test]
    fn rejects_unsupported_commands_with_a_useful_error() {
        assert!(run(&args(&["analyze"]))
            .unwrap_err()
            .contains("unknown command"));
        assert!(run(&args(&["benchmark"]))
            .unwrap()
            .contains("nexus benchmark <file|native-ffi|memory-allocation>"));
    }

    #[test]
    fn json_error_strings_escape_control_characters() {
        assert_eq!(super::json_string("a\"b\\c\n"), r#""a\"b\\c\n""#);
    }
}
