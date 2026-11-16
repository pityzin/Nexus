"""Black-box tests for the loopback UI bridge; analyzed fixtures remain inert bytes."""
import http.client
import json
import os
import re
import subprocess
import time
import unittest
from typing import Optional


def make_elf64() -> bytes:
    image = bytearray(64)
    image[:7] = b"\x7fELF\x02\x01\x01"
    image[16:18] = (2).to_bytes(2, "little")
    image[18:20] = (62).to_bytes(2, "little")
    image[20:24] = (1).to_bytes(4, "little")
    image[24:32] = (0x400000).to_bytes(8, "little")
    image[52:54] = (64).to_bytes(2, "little")
    image[54:56] = (56).to_bytes(2, "little")
    image[58:60] = (64).to_bytes(2, "little")
    return bytes(image)


class UiBridgeTests(unittest.TestCase):
    process: Optional[subprocess.Popen] = None
    host = ""
    port = 0

    @classmethod
    def setUpClass(cls) -> None:
        executable = os.environ.get("NEXUS_BIN")
        if not executable:
            raise RuntimeError("Set NEXUS_BIN to the built NEXUS CLI path")
        cls.process = subprocess.Popen(
            [executable, "ui"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
        assert cls.process.stdout is not None
        line = cls.process.stdout.readline().strip()
        match = re.fullmatch(r"NEXUS interface: http://(localhost):(3001)/", line)
        if not match:
            raise RuntimeError(f"UI service did not start: {line!r}")
        _, port_text = match.groups()
        cls.host = "127.0.0.1"
        cls.port = int(port_text)
        cls.process.stdout.readline()  # Upload limit and shutdown instructions.
        time.sleep(0.05)
        if cls.process.poll() is not None:
            raise RuntimeError("NEXUS UI process exited during startup")

    @classmethod
    def tearDownClass(cls) -> None:
        if cls.process is not None and cls.process.poll() is None:
            cls.process.terminate()
            cls.process.wait(timeout=5)

    def request(self, method: str, path: str, body: bytes = b"", headers=None):
        connection = http.client.HTTPConnection(self.host, self.port, timeout=5)
        connection.request(method, path, body=body, headers=headers or {})
        response = connection.getresponse()
        result = response.status, dict(response.getheaders()), response.read()
        connection.close()
        return result

    def test_static_app_and_capability_status(self) -> None:
        status, headers, body = self.request("GET", "/")
        self.assertEqual(status, 200)
        self.assertIn("default-src 'self'", headers["Content-Security-Policy"])
        self.assertIn(b"NEXUS", body)
        status, _, status_body = self.request("GET", "/api/v1/status")
        self.assertEqual(status, 200)
        capabilities = json.loads(status_body)
        self.assertEqual(capabilities["supported_formats"], ["pe", "elf"])
        self.assertIn("binary_metadata", capabilities["implemented"])
        self.assertIn("cfg", capabilities["unavailable"])
        self.assertEqual(capabilities["web_port"], 3001)
        self.assertIn("metadata_plugin_export", capabilities["implemented"])
        status, _, registry = self.request("GET", "/api/v1/plugins")
        self.assertEqual(status, 200)
        plugins = json.loads(registry)
        self.assertEqual(plugins["plugin_api_version"], 1)
        self.assertEqual(plugins["plugins"][0]["id"], "metadata-summary")
        self.assertEqual(self.request("GET", "/app.js")[0], 200)
        self.assertEqual(self.request("GET", "/styles.css")[0], 200)

    def test_real_parse_endpoint_returns_nir_and_rejects_untrusted_origin(self) -> None:
        content_type = {"Content-Type": "application/octet-stream", "Origin": f"http://{self.host}:{self.port}"}
        status, _, body = self.request("POST", "/api/v1/inspect", make_elf64(), content_type)
        self.assertEqual(status, 200, body.decode("utf-8", errors="replace"))
        envelope = json.loads(body)
        metadata = json.loads(envelope["metadata_json"])
        self.assertEqual(metadata["format"], "elf")
        self.assertEqual(metadata["architecture"], "x86_64")
        self.assertEqual(metadata["file_size"], 64)
        self.assertGreaterEqual(envelope["parse_elapsed_ns"], 0)

        attacker = {"Content-Type": "application/octet-stream", "Origin": "http://attacker.invalid"}
        status, _, error_body = self.request("POST", "/api/v1/inspect", make_elf64(), attacker)
        self.assertEqual(status, 403)
        self.assertEqual(json.loads(error_body)["error"]["code"], "origin_rejected")

    def test_metadata_plugin_endpoint_exports_the_parsed_fixture(self) -> None:
        headers = {"Content-Type": "application/octet-stream", "Origin": "http://localhost:3001"}
        status, _, body = self.request("POST", "/api/v1/plugins/metadata-summary", make_elf64(), headers)
        self.assertEqual(status, 200, body.decode("utf-8", errors="replace"))
        report = json.loads(body)
        self.assertEqual(report["plugin"], "metadata-summary")
        self.assertEqual(report["format"], "elf")
        self.assertEqual(report["architecture"], "x86_64")
        self.assertEqual(report["file_size"], 64)

    def test_port_conflict_is_reported_without_stopping_the_running_server(self) -> None:
        executable = os.environ["NEXUS_BIN"]
        competing = subprocess.run(
            [executable, "ui"], capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=5
        )
        self.assertEqual(competing.returncode, 2)
        self.assertIn("required NEXUS web port 3001 is already in use", competing.stderr)
        self.assertEqual(self.request("GET", "/api/v1/status")[0], 200)

    def test_malformed_input_and_unknown_routes_are_errors(self) -> None:
        headers = {"Content-Type": "application/octet-stream", "Origin": f"http://{self.host}:{self.port}"}
        status, _, body = self.request("POST", "/api/v1/inspect", b"not executable", headers)
        self.assertEqual(status, 422)
        self.assertEqual(json.loads(body)["error"]["code"], "binary_parse_failed")
        self.assertEqual(self.request("GET", "/missing")[0], 404)


if __name__ == "__main__":
    unittest.main()
