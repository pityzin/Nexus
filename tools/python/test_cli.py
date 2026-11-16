"""Small black-box CLI checks; they only run the NEXUS CLI, never an analyzed file."""
import os
import json
import struct
import subprocess
import tempfile
from typing import Dict, Optional
from pathlib import Path
import unittest


def invoke(*args: str, env: Optional[Dict[str, str]] = None) -> subprocess.CompletedProcess:
    executable = os.environ.get("NEXUS_BIN")
    if not executable:
        raise RuntimeError("Set NEXUS_BIN to the built NEXUS CLI path")
    child_env = os.environ.copy()
    child_env.update(env or {})
    return subprocess.run(
        [executable, *args],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
        env=child_env,
    )


class CliSmokeTests(unittest.TestCase):
    def test_version_and_info(self) -> None:
        version = invoke("--version")
        self.assertEqual(version.returncode, 0, version.stderr)
        self.assertIn("nexus 0.1.0", version.stdout)
        help_output = invoke("--help")
        self.assertEqual(help_output.returncode, 0, help_output.stderr)
        self.assertIn("NEXUS - Modular Binary Analysis", help_output.stdout)
        info = invoke("info")
        self.assertEqual(info.returncode, 0, info.stderr)
        self.assertIn("input binaries are never executed", info.stdout)

    def test_module_listing_and_invalid_command(self) -> None:
        modules = invoke("modules")
        self.assertEqual(modules.returncode, 0, modules.stderr)
        self.assertIn("formats.pe", modules.stdout)
        invalid = invoke("execute")
        self.assertEqual(invalid.returncode, 2)
        self.assertIn("unknown command", invalid.stderr)

    def test_builtin_plugin_exports_real_metadata(self) -> None:
        listed = invoke("plugins")
        self.assertEqual(listed.returncode, 0, listed.stderr)
        self.assertIn("metadata-summary", listed.stdout)
        with tempfile.TemporaryDirectory(prefix="nexus plugin ") as temp_dir:
            path = Path(temp_dir) / "sample.elf"
            path.write_bytes(make_elf64_fixture())
            result = invoke("export", str(path), "--plugin", "metadata-summary")
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(result.stdout)
            self.assertEqual(report["plugin"], "metadata-summary")
            self.assertEqual(report["schema_version"], 1)
            self.assertEqual(report["format"], "elf")
            self.assertEqual(report["architecture"], "x86_64")
            metadata = invoke("inspect", str(path), "--format", "json")
            self.assertEqual(metadata.returncode, 0, metadata.stderr)
            self.assertEqual(report["section_count"], len(json.loads(metadata.stdout)["sections"]))

    def test_invalid_resource_configuration_emits_json_error(self) -> None:
        result = invoke("info", env={"NEXUS_MAX_INPUT_BYTES": "unlimited"})
        self.assertEqual(result.returncode, 2)
        event = json.loads(result.stderr)
        self.assertEqual(event["level"], "error")
        self.assertEqual(event["event"], "command_failed")
        self.assertIn("NEXUS_MAX_INPUT_BYTES", event["message"])

    def test_elf_commands_and_json_output_on_controlled_fixture(self) -> None:
        executable = os.environ.get("NEXUS_BIN")
        self.assertTrue(executable, "Set NEXUS_BIN to the built NEXUS CLI path")
        with tempfile.TemporaryDirectory(prefix="nexus phase02 ") as temp_dir:
            path = Path(temp_dir) / "minimal64.elf"
            path.write_bytes(make_elf64_fixture())
            overview = subprocess.run([executable, "inspect", str(path)], capture_output=True, text=True, encoding="utf-8", errors="replace", check=False)
            self.assertEqual(overview.returncode, 0, overview.stderr)
            self.assertIn("Format: ELF", overview.stdout)
            self.assertIn("Architecture: x86_64", overview.stdout)
            headers = subprocess.run([executable, "headers", str(path)], capture_output=True, text=True, encoding="utf-8", errors="replace", check=False)
            self.assertEqual(headers.returncode, 0, headers.stderr)
            self.assertIn("Program headers:", headers.stdout)
            sections = subprocess.run([executable, "sections", str(path)], capture_output=True, text=True, encoding="utf-8", errors="replace", check=False)
            self.assertEqual(sections.returncode, 0, sections.stderr)
            self.assertIn(".text", sections.stdout)
            for command in ("inspect", "headers", "sections"):
                result = subprocess.run([executable, command, str(path), "--format", "json"], capture_output=True, text=True, encoding="utf-8", errors="replace", check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                document = json.loads(result.stdout)
                self.assertEqual(document["format"], "elf")
                if command == "inspect":
                    self.assertEqual(document["architecture"], "x86_64")
                    self.assertEqual(document["sections"][1]["name"], ".text")
                elif command == "sections":
                    self.assertTrue(document["sections"][1]["permissions"]["execute"])
                else:
                    self.assertEqual(document["headers"]["program_header_count"], 1)

    def test_malformed_input_is_a_structured_error(self) -> None:
        executable = os.environ["NEXUS_BIN"]
        with tempfile.TemporaryDirectory(prefix="nexus malformed ") as temp_dir:
            path = Path(temp_dir) / "bad.bin"
            path.write_bytes(b"MZ\0\0")
            result = subprocess.run([executable, "inspect", str(path)], capture_output=True, text=True, encoding="utf-8", errors="replace", check=False)
            self.assertEqual(result.returncode, 2)
            event = json.loads(result.stderr)
            self.assertEqual(event["level"], "error")
            self.assertIn("truncated", event["message"])

    def test_pe_inspect_headers_sections_and_info(self) -> None:
        executable = os.environ["NEXUS_BIN"]
        with tempfile.TemporaryDirectory(prefix="nexus pe ") as temp_dir:
            path = Path(temp_dir) / "minimal64.exe"
            path.write_bytes(make_pe64_fixture())
            for command in ("inspect", "headers", "sections", "info"):
                result = subprocess.run([executable, command, str(path)], capture_output=True, text=True, encoding="utf-8", errors="replace", check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                if command == "sections":
                    self.assertIn(".text", result.stdout)
                else:
                    self.assertIn("PE", result.stdout)
            result = subprocess.run([executable, "inspect", str(path), "--format", "json"], capture_output=True, text=True, encoding="utf-8", errors="replace", check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)["format"], "pe")

    def test_profile_and_benchmark_json_on_pe_input(self) -> None:
        executable = os.environ["NEXUS_BIN"]
        profile = subprocess.run(
            [executable, "profile", executable, "--format", "json"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
        self.assertEqual(profile.returncode, 0, profile.stderr)
        profile_doc = json.loads(profile.stdout)
        self.assertEqual(profile_doc["kind"], "profile")
        self.assertEqual(profile_doc["format"], "pe")
        self.assertEqual([stage["name"] for stage in profile_doc["stages"]], ["read_snapshot", "metadata_parse"])
        self.assertEqual(profile_doc["memory_metrics"], "unsupported")

        benchmark = subprocess.run(
            [executable, "benchmark", executable, "--warmup", "2", "--iterations", "30", "--format", "json"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
        self.assertEqual(benchmark.returncode, 0, benchmark.stderr)
        report = json.loads(benchmark.stdout)
        self.assertEqual(report["kind"], "benchmark")
        self.assertEqual(report["benchmark"], "binary_metadata_parse")
        self.assertEqual(report["sampling"]["iterations"], 30)
        self.assertEqual(len(report["samples_ns"]), 30)
        self.assertGreater(report["statistics"]["median_ns"], 0)
        self.assertGreater(report["throughput_bytes_per_second"], 0)
        self.assertIn("metadata parser does not scan", report["throughput_basis"])
        self.assertIn("os_version", report["environment"])
        self.assertIn("CFG", report["unavailable"])

    def test_native_and_allocation_microbenchmarks(self) -> None:
        executable = os.environ["NEXUS_BIN"]
        for workload, expected, detail in (
            ("native-ffi", "native_add_i64_ffi", "operations_per_sample"),
            ("memory-allocation", "heap_alloc_dealloc_4096", "allocation_pairs_per_sample"),
        ):
            result = subprocess.run(
                [executable, "benchmark", workload, "--warmup", "2", "--iterations", "30", "--format", "json"],
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(result.stdout)
            self.assertEqual(report["benchmark"], expected)
            self.assertEqual(report["statistics"]["sample_count"], 30)
            self.assertEqual(report["workload"][detail], 256)
            self.assertGreater(report["throughput_operations_per_second"], 0)
            self.assertIn("allocator telemetry", report["unavailable"])


def make_elf64_fixture() -> bytes:
    """Build a tiny synthetic ELF64 file; this fixture is data and is never run."""
    header_size, program_entry_size, section_entry_size = 64, 56, 64
    program_offset, section_offset = header_size, header_size + program_entry_size
    text_offset = 0x140
    names = b"\0.text\0.shstrtab\0"
    string_offset = text_offset + 16
    image = bytearray(string_offset + len(names))
    image[:7] = b"\x7fELF\x02\x01\x01"
    image[7:9] = bytes((3, 1))
    struct.pack_into("<HHIQQQIHHHHHH", image, 16, 2, 62, 1, 0x400123, program_offset, section_offset, 5, header_size, program_entry_size, 1, section_entry_size, 3, 2)
    text_header = section_offset + section_entry_size
    string_header = section_offset + section_entry_size * 2
    struct.pack_into("<IIQQQQIIQQ", image, text_header, 1, 1, 6, 0x400000, text_offset, 16, 0, 0, 16, 0)
    struct.pack_into("<IIQQQQIIQQ", image, string_header, 7, 3, 0, 0, string_offset, len(names), 0, 0, 1, 0)
    image[text_offset:text_offset + 16] = bytes([0x90]) * 16
    image[string_offset:] = names
    return bytes(image)


def make_pe64_fixture() -> bytes:
    """Build a tiny synthetic PE32+ file; its section bytes are never executed."""
    image = bytearray(0x400)
    image[:2] = b"MZ"
    pe_offset = 0x80
    struct.pack_into("<I", image, 0x3C, pe_offset)
    image[pe_offset:pe_offset + 4] = b"PE\0\0"
    coff = pe_offset + 4
    struct.pack_into("<HHIIIHH", image, coff, 0x8664, 1, 0x12345678, 0, 0, 240, 0x22)
    optional = coff + 20
    struct.pack_into("<H", image, optional, 0x20B)
    struct.pack_into("<I", image, optional + 16, 0x1000)
    struct.pack_into("<Q", image, optional + 24, 0x140000000)
    struct.pack_into("<II", image, optional + 32, 0x1000, 0x200)
    struct.pack_into("<IIHH", image, optional + 56, 0x2000, 0x200, 3, 0x140)
    struct.pack_into("<I", image, optional + 108, 1)
    struct.pack_into("<II", image, optional + 112, 0x3000, 40)
    section = optional + 240
    image[section:section + 5] = b".text"
    struct.pack_into("<IIII", image, section + 8, 0x38, 0x1000, 0x200, 0x200)
    struct.pack_into("<I", image, section + 36, 0x60000020)
    return bytes(image)


if __name__ == "__main__":
    unittest.main()
