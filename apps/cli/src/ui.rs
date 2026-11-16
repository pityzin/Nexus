//! Loopback-only, dependency-free HTTP bridge for the static NEXUS interface.
//! It exposes metadata parsing only; there are no filesystem paths or shell APIs.

use nexus_core::{
    nexus_binary_loader::{parse_bytes_with_limit, NIR_SCHEMA_VERSION},
    CoreConfig,
};
use nexus_example_metadata_plugin::MetadataSummaryPlugin;
use nexus_plugin_api::{PluginInput, PLUGIN_API_VERSION};
use nexus_plugin_host::{PluginHost, PluginLimits};
use nexus_profiling::json_string;
use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    time::{Duration, Instant},
};

const INDEX_HTML: &str = include_str!("../../ui/web/index.html");
const APP_JS: &str = include_str!("../../ui/web/app.js");
const DATA_JS: &str = include_str!("../../ui/web/data.js");
const STYLES_CSS: &str = include_str!("../../ui/web/styles.css");
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_UI_UPLOAD_BYTES: usize = 128 * 1024 * 1024;
const UI_PROTOCOL_VERSION: u32 = 1;
pub const UI_PORT: u16 = 3001;

pub fn serve() -> Result<(), String> {
    let config = CoreConfig::from_env().map_err(|error| error.to_string())?;
    let max_upload_bytes = config
        .resource_limits
        .max_input_bytes
        .min(MAX_UI_UPLOAD_BYTES);
    let listener = TcpListener::bind(("127.0.0.1", UI_PORT)).map_err(|error| {
        if error.kind() == io::ErrorKind::AddrInUse {
            format!("required NEXUS web port {UI_PORT} is already in use (http://localhost:{UI_PORT}); no process was stopped and no alternate port was selected")
        } else {
            format!("cannot bind required NEXUS web port {UI_PORT} (http://localhost:{UI_PORT}): {error}")
        }
    })?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("cannot read the local UI address: {error}"))?;
    println!("NEXUS interface: http://localhost:{UI_PORT}/");
    println!("Upload limit: {max_upload_bytes} bytes. Press Ctrl+C to stop.");
    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => handle_connection(stream, address, max_upload_bytes),
            Err(error) => eprintln!("NEXUS UI connection failed: {error}"),
        }
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream, address: SocketAddr, max_upload_bytes: usize) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    let response = match read_request(&mut stream, address, max_upload_bytes) {
        Ok(request) => route(request, address, max_upload_bytes),
        Err((status, message)) => {
            let code = match (status, message.as_str()) {
                (403, message) if message.starts_with("request Origin") => "origin_rejected",
                (403, _) => "host_rejected",
                (413, _) => "upload_too_large",
                _ => "invalid_request",
            };
            json_error(status, code, &message)
        }
    };
    let _ = write_response(&mut stream, response);
}

#[derive(Debug)]
struct Request {
    method: String,
    path: String,
    host: String,
    origin: Option<String>,
    content_type: Option<String>,
    body: Vec<u8>,
}

#[derive(Debug)]
struct Response {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Response {
    fn new(status: u16, content_type: &'static str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            content_type,
            body: body.into(),
        }
    }
}

fn read_request(
    stream: &mut TcpStream,
    address: SocketAddr,
    max_body_bytes: usize,
) -> Result<Request, (u16, String)> {
    let mut buffer = [0_u8; MAX_HEADER_BYTES];
    let mut used = 0;
    let header_end = loop {
        if let Some(index) = buffer[..used]
            .windows(4)
            .position(|part| part == b"\r\n\r\n")
        {
            break index;
        }
        if used == buffer.len() {
            return Err((
                431,
                "request headers exceed the configured limit".to_owned(),
            ));
        }
        let count = stream
            .read(&mut buffer[used..])
            .map_err(|error| (400, format!("cannot read request headers: {error}")))?;
        if count == 0 {
            return Err((
                400,
                "request ended before the headers were complete".to_owned(),
            ));
        }
        used += count;
    };
    let header_bytes = &buffer[..header_end];
    if !header_bytes.is_ascii() {
        return Err((400, "request headers must be ASCII".to_owned()));
    }
    let header_text = std::str::from_utf8(header_bytes)
        .map_err(|_| (400, "request headers are invalid".to_owned()))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| (400, "missing request line".to_owned()))?;
    let mut request_parts = request_line.split_ascii_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| (400, "missing HTTP method".to_owned()))?;
    let path = request_parts
        .next()
        .ok_or_else(|| (400, "missing request path".to_owned()))?;
    let version = request_parts
        .next()
        .ok_or_else(|| (400, "missing HTTP version".to_owned()))?;
    if request_parts.next().is_some() || !matches!(version, "HTTP/1.0" | "HTTP/1.1") {
        return Err((400, "invalid HTTP request line".to_owned()));
    }

    let mut host = None;
    let mut origin = None;
    let mut content_type = None;
    let mut content_length = None;
    let mut transfer_encoding = false;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| (400, "invalid HTTP header".to_owned()))?;
        let value = value.trim();
        match name.trim().to_ascii_lowercase().as_str() {
            "host" => set_header(&mut host, value, "Host")?,
            "origin" => set_header(&mut origin, value, "Origin")?,
            "content-type" => set_header(&mut content_type, value, "Content-Type")?,
            "content-length" => {
                if content_length.is_some() {
                    return Err((400, "duplicate Content-Length header".to_owned()));
                }
                let text = content_length_header(value)?;
                content_length = Some(text);
            }
            "transfer-encoding" => transfer_encoding = true,
            _ => {}
        }
    }
    if transfer_encoding {
        return Err((
            400,
            "transfer-encoded requests are not supported".to_owned(),
        ));
    }
    let host = host.ok_or_else(|| (400, "Host header is required".to_owned()))?;
    let body_length = if method == "POST" {
        content_length.ok_or_else(|| (411, "Content-Length is required".to_owned()))?
    } else {
        content_length.unwrap_or(0)
    };
    if method != "POST" && body_length != 0 {
        return Err((400, "request bodies are only accepted for POST".to_owned()));
    }
    if body_length > max_body_bytes {
        return Err((413, "request body exceeds the configured limit".to_owned()));
    }
    let body_start = header_end + 4;
    let buffered_body = &buffer[body_start..used];
    let host_is_valid = host_matches_address(&host, address);
    let origin_is_valid = if method == "POST" {
        origin
            .as_deref()
            .is_some_and(|value| origin_matches_address(value, address))
    } else {
        true
    };
    if !host_is_valid || !origin_is_valid {
        // Drain small rejected bodies so the peer can receive the structured response
        // before the connection closes. Large bodies are rejected without allocation.
        if body_length <= 64 * 1024 {
            let already_buffered = buffered_body.len().min(body_length);
            discard_body_remainder(stream, body_length - already_buffered)
                .map_err(|error| (400, format!("cannot finish rejected request: {error}")))?;
        }
        return if !host_is_valid {
            Err((403, "Host must match the loopback UI address".to_owned()))
        } else {
            Err((403, "request Origin does not match this UI".to_owned()))
        };
    }
    let copied = buffered_body.len().min(body_length);
    let mut body = Vec::new();
    body.try_reserve_exact(body_length)
        .map_err(|_| (503, "unable to reserve bounded request storage".to_owned()))?;
    body.resize(body_length, 0);
    body[..copied].copy_from_slice(&buffered_body[..copied]);
    if copied < body_length {
        stream
            .read_exact(&mut body[copied..])
            .map_err(|error| (400, format!("request body is incomplete: {error}")))?;
    }
    Ok(Request {
        method: method.to_owned(),
        path: path.to_owned(),
        host,
        origin,
        content_type,
        body,
    })
}

fn discard_body_remainder(stream: &mut TcpStream, mut remaining: usize) -> io::Result<()> {
    let mut discard = [0_u8; 4096];
    while remaining > 0 {
        let count = remaining.min(discard.len());
        stream.read_exact(&mut discard[..count])?;
        remaining -= count;
    }
    Ok(())
}

fn set_header(
    destination: &mut Option<String>,
    value: &str,
    name: &str,
) -> Result<(), (u16, String)> {
    if destination.is_some() {
        return Err((400, format!("duplicate {name} header")));
    }
    *destination = Some(value.to_owned());
    Ok(())
}

fn content_length_header(value: &str) -> Result<usize, (u16, String)> {
    value
        .parse::<usize>()
        .map_err(|_| (400, "invalid Content-Length".to_owned()))
}

fn host_matches_address(host: &str, address: SocketAddr) -> bool {
    host == address.to_string()
        || (address.ip().is_loopback() && address.port() == UI_PORT && host == "localhost:3001")
}

fn origin_matches_address(origin: &str, address: SocketAddr) -> bool {
    origin == format!("http://{address}")
        || (address.ip().is_loopback()
            && address.port() == UI_PORT
            && origin == "http://localhost:3001")
}

fn plugin_host() -> Result<PluginHost, String> {
    let mut host = PluginHost::new(PLUGIN_API_VERSION, PluginLimits::default())
        .map_err(|error| error.to_string())?;
    host.register(Box::new(MetadataSummaryPlugin))
        .map_err(|error| error.to_string())?;
    Ok(host)
}

fn plugins_json() -> Result<String, String> {
    let host = plugin_host()?;
    let rows = host
        .list()
        .into_iter()
        .map(|status| {
            let capabilities = status
                .descriptor
                .capabilities()
                .iter()
                .map(|capability| json_string(capability))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"id\":{},\"version\":{},\"state\":{},\"extension_point\":{},\"min_host_api\":{},\"max_host_api\":{},\"capabilities\":[{}]}}",
                json_string(status.descriptor.id()),
                json_string(status.descriptor.version()),
                json_string(if status.enabled { "enabled" } else { "disabled" }),
                json_string(status.descriptor.extension_point().as_str()),
                status.descriptor.min_host_api(),
                status.descriptor.max_host_api(),
                capabilities,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!(
        "{{\"plugin_api_version\":{PLUGIN_API_VERSION},\"plugins\":[{rows}]}}"
    ))
}

fn route(request: Request, address: SocketAddr, max_upload_bytes: usize) -> Response {
    if !host_matches_address(&request.host, address) {
        return json_error(
            403,
            "host_rejected",
            "Host must match the loopback UI address",
        );
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => Response::new(200, "text/html; charset=utf-8", INDEX_HTML),
        ("GET", "/app.js") => Response::new(
            200,
            "text/javascript; charset=utf-8",
            APP_JS,
        ),
        ("GET", "/data.js") => Response::new(
            200,
            "text/javascript; charset=utf-8",
            DATA_JS,
        ),
        ("GET", "/styles.css") => Response::new(
            200,
            "text/css; charset=utf-8",
            STYLES_CSS,
        ),
        ("GET", "/api/v1/status") => Response::new(
            200,
            "application/json; charset=utf-8",
            format!(
                "{{\"protocol_version\":{UI_PROTOCOL_VERSION},\"engine_version\":{},\"nir_schema_version\":{NIR_SCHEMA_VERSION},\"plugin_api_version\":{PLUGIN_API_VERSION},\"web_port\":{UI_PORT},\"max_upload_bytes\":{},\"supported_formats\":[\"pe\",\"elf\"],\"metadata_architectures\":[\"x86\",\"x86_64\",\"arm\",\"aarch64\",\"unknown\"],\"implemented\":[\"binary_metadata\",\"headers\",\"sections\",\"nir_metadata\",\"parse_wall_time\",\"benchmark_report_import\",\"plugin_discovery\",\"metadata_plugin_export\"],\"unavailable\":[\"disassembly\",\"cfg\",\"symbols\",\"imports_exports\",\"relocations\",\"cross_references\",\"data_flow\",\"memory_telemetry\"]}}",
                json_string(env!("CARGO_PKG_VERSION")),
                max_upload_bytes,
            ),
        ),
        ("GET", "/api/v1/plugins") => match plugins_json() {
            Ok(body) => Response::new(200, "application/json; charset=utf-8", body),
            Err(error) => json_error(500, "plugin_registry_failed", &error),
        },
        ("POST", "/api/v1/inspect") => {
            if !request
                .origin
                .as_deref()
                .is_some_and(|origin| origin_matches_address(origin, address))
            {
                return json_error(403, "origin_rejected", "request Origin does not match this UI");
            }
            if request.content_type.as_deref() != Some("application/octet-stream") {
                return json_error(415, "content_type_rejected", "binary upload must use application/octet-stream");
            }
            if request.body.is_empty() {
                return json_error(422, "empty_input", "select a non-empty binary file");
            }
            let start = Instant::now();
            match parse_bytes_with_limit(&request.body, max_upload_bytes as u64) {
                Ok(metadata) => Response::new(
                    200,
                    "application/json; charset=utf-8",
                    format!(
                        "{{\"protocol_version\":{UI_PROTOCOL_VERSION},\"parse_elapsed_ns\":{},\"metadata_json\":{}}}",
                        start.elapsed().as_nanos(),
                        json_string(&metadata.inspect_json()),
                    ),
                ),
                Err(error) => json_error(422, "binary_parse_failed", &error.to_string()),
            }
        }
        ("POST", "/api/v1/plugins/metadata-summary") => {
            if !request
                .origin
                .as_deref()
                .is_some_and(|origin| origin_matches_address(origin, address))
            {
                return json_error(403, "origin_rejected", "request Origin does not match this UI");
            }
            if request.content_type.as_deref() != Some("application/octet-stream") {
                return json_error(415, "content_type_rejected", "binary upload must use application/octet-stream");
            }
            if request.body.is_empty() {
                return json_error(422, "empty_input", "select a non-empty binary file");
            }
            match parse_bytes_with_limit(&request.body, max_upload_bytes as u64) {
                Ok(metadata) => {
                    let section_count = match u32::try_from(metadata.sections.len()) {
                        Ok(count) => count,
                        Err(_) => return json_error(422, "metadata_out_of_range", "section count exceeds plugin API range"),
                    };
                    let input = PluginInput::new(
                        metadata.format.as_str(),
                        metadata.architecture.as_str(),
                        metadata.bitness,
                        metadata.file_size,
                        metadata.entry_point,
                        section_count,
                    );
                    match plugin_host().and_then(|host| host.execute("metadata-summary", input).map_err(|error| error.to_string())) {
                        Ok(output) => Response::new(200, "application/json; charset=utf-8", output),
                        Err(error) => json_error(500, "plugin_execution_failed", &error),
                    }
                }
                Err(error) => json_error(422, "binary_parse_failed", &error.to_string()),
            }
        }
        ("POST", _) => json_error(404, "route_not_found", "unknown API route"),
        ("GET", _) => Response::new(
            404,
            "text/plain; charset=utf-8",
            "NEXUS UI route not found",
        ),
        _ => json_error(405, "method_not_allowed", "HTTP method is not supported"),
    }
}

fn json_error(status: u16, code: &str, message: &str) -> Response {
    Response::new(
        status,
        "application/json; charset=utf-8",
        format!(
            "{{\"error\":{{\"code\":{},\"message\":{}}}}}",
            json_string(code),
            json_string(message),
        ),
    )
}

fn write_response(stream: &mut TcpStream, response: Response) -> io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Content",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        500 => "Internal Server Error",
        _ => "Error",
    };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'\r\n\r\n",
        response.status,
        reason,
        response.content_type,
        response.body.len(),
    )?;
    stream.write_all(&response.body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(method: &str, path: &str, address: SocketAddr, body: Vec<u8>) -> Request {
        Request {
            method: method.to_owned(),
            path: path.to_owned(),
            host: address.to_string(),
            origin: Some(format!("http://{address}")),
            content_type: Some("application/octet-stream".to_owned()),
            body,
        }
    }

    fn tiny_elf64() -> Vec<u8> {
        let mut bytes = vec![0_u8; 64];
        bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
        bytes[18..20].copy_from_slice(&62_u16.to_le_bytes());
        bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
        bytes[24..32].copy_from_slice(&0x400000_u64.to_le_bytes());
        bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
        bytes[54..56].copy_from_slice(&56_u16.to_le_bytes());
        bytes[58..60].copy_from_slice(&64_u16.to_le_bytes());
        bytes
    }

    #[test]
    fn status_exposes_only_implemented_and_unavailable_features() {
        let address = "127.0.0.1:31000".parse().unwrap();
        let response = route(
            request("GET", "/api/v1/status", address, Vec::new()),
            address,
            1024,
        );
        let body = String::from_utf8(response.body).unwrap();
        assert_eq!(response.status, 200);
        assert!(body.contains("binary_metadata"));
        assert!(body.contains("\"disassembly\""));
        assert!(body.contains("\"web_port\":3001"));
        assert!(body.contains("metadata_plugin_export"));
    }

    #[test]
    fn localhost_port_authority_is_accepted_only_for_the_fixed_loopback_service() {
        let required_address = "127.0.0.1:3001".parse().unwrap();
        assert!(host_matches_address("localhost:3001", required_address));
        assert!(origin_matches_address(
            "http://localhost:3001",
            required_address
        ));
        let other_address = "127.0.0.1:31000".parse().unwrap();
        assert!(!host_matches_address("localhost:3001", other_address));
        assert!(!origin_matches_address(
            "http://localhost:3001",
            other_address
        ));
    }

    #[test]
    fn plugin_discovery_and_export_use_the_real_metadata_host() {
        let address = "127.0.0.1:3001".parse().unwrap();
        let mut plugins_request = request("GET", "/api/v1/plugins", address, Vec::new());
        plugins_request.host = "localhost:3001".to_owned();
        let response = route(plugins_request, address, 1024);
        let body: String = String::from_utf8(response.body).unwrap();
        assert_eq!(response.status, 200);
        assert!(body.contains("metadata-summary"));
        assert!(body.contains("metadata_export"));

        let mut export_request = request(
            "POST",
            "/api/v1/plugins/metadata-summary",
            address,
            tiny_elf64(),
        );
        export_request.host = "localhost:3001".to_owned();
        export_request.origin = Some("http://localhost:3001".to_owned());
        let response = route(export_request, address, 1024);
        let body = String::from_utf8(response.body).unwrap();
        assert_eq!(response.status, 200);
        assert!(body.contains("\"format\":\"elf\""));
        assert!(body.contains("\"architecture\":\"x86_64\""));
    }

    #[test]
    fn inspect_returns_real_nir_metadata_and_parse_timing() {
        let address = "127.0.0.1:31000".parse().unwrap();
        let response = route(
            request("POST", "/api/v1/inspect", address, tiny_elf64()),
            address,
            1024,
        );
        let body = String::from_utf8(response.body).unwrap();
        assert_eq!(response.status, 200);
        assert!(body.contains("\\\"format\\\":\\\"elf\\\""));
        assert!(body.contains("\\\"schema_version\\\":1"));
        assert!(body.contains("\"parse_elapsed_ns\":"));
    }

    #[test]
    fn inspect_rejects_bad_origin_and_malformed_data() {
        let address = "127.0.0.1:31000".parse().unwrap();
        let mut bad_origin = request("POST", "/api/v1/inspect", address, tiny_elf64());
        bad_origin.origin = Some("http://attacker.invalid".to_owned());
        assert_eq!(route(bad_origin, address, 1024).status, 403);
        let bad_data = request("POST", "/api/v1/inspect", address, b"not binary".to_vec());
        assert_eq!(route(bad_data, address, 1024).status, 422);
    }

    #[test]
    fn request_parser_rejects_duplicate_lengths_and_oversized_bodies() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let authority = address.to_string();
        let sender = std::thread::spawn(move || {
            let mut client = TcpStream::connect(address).unwrap();
            write!(client, "POST /api/v1/inspect HTTP/1.1\r\nHost: {authority}\r\nContent-Length: 0\r\nContent-Length: 0\r\n\r\n").unwrap();
        });
        let (mut accepted, _) = listener.accept().unwrap();
        let error = read_request(&mut accepted, address, 1024).unwrap_err();
        assert!(error.1.contains("duplicate Content-Length"));
        sender.join().unwrap();

        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let authority = address.to_string();
        let sender = std::thread::spawn(move || {
            let mut client = TcpStream::connect(address).unwrap();
            write!(client, "POST /api/v1/inspect HTTP/1.1\r\nHost: {authority}\r\nOrigin: http://{authority}\r\nContent-Length: 1025\r\n\r\n").unwrap();
        });
        let (mut accepted, _) = listener.accept().unwrap();
        assert_eq!(
            read_request(&mut accepted, address, 1024).unwrap_err().0,
            413
        );
        sender.join().unwrap();
    }
}
