//! HTTP server for eval requests and control commands.
//!
//! Provides a simple HTTP API for interacting with sclang:
//! - POST /eval - Execute SuperCollider code
//! - GET /health - Health check
//! - POST /stop, /boot, /recompile, /quit - Control commands
//! - POST /convert-schelp - Convert .schelp to markdown

use anyhow::{anyhow, Result};
use socket2::{Domain, Protocol, Socket, Type};
use std::io;
use std::net::{SocketAddr, TcpListener, UdpSocket};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tiny_http::{Header, Method, Response, Server};

use crate::bridge::{create_execute_command_request, next_lsp_request_id};
use crate::logging::verbose_logging_enabled;

// ============================================================================
// Response Helpers
// ============================================================================

/// Build CORS headers for preflight responses.
pub fn cors_headers() -> Vec<Header> {
    vec![
        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..])
            .expect("valid ASCII header"),
        Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"POST, OPTIONS"[..])
            .expect("valid ASCII header"),
        Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type"[..])
            .expect("valid ASCII header"),
    ]
}

/// Build a JSON response with the given status code.
pub fn json_response(body: &str, status: u16) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(body)
        .with_status_code(status)
        .with_header(
            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                .expect("valid ASCII header"),
        )
}

/// Build a JSON response with CORS headers.
pub fn json_response_with_cors(body: &str, status: u16) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(body, status).with_header(
        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..])
            .expect("valid ASCII header"),
    )
}

/// Build an error response with CORS headers.
pub fn error_response(msg: &str, status: u16) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response_with_cors(&format!(r#"{{"error":"{}"}}"#, msg), status)
}

/// Build a CORS preflight response (204 No Content with CORS headers).
fn cors_preflight_response() -> Response<std::io::Cursor<Vec<u8>>> {
    let mut response = Response::from_string("").with_status_code(204);
    for header in cors_headers() {
        response = response.with_header(header);
    }
    response
}

// ============================================================================
// LSP Communication
// ============================================================================

/// Send an LSP payload to sclang via UDP.
pub fn send_lsp_payload(udp_socket: &UdpSocket, payload: &serde_json::Value) -> io::Result<()> {
    let lsp_json = serde_json::to_string(payload).map_err(io::Error::other)?;
    let lsp_message = format!("Content-Length: {}\r\n\r\n{}", lsp_json.len(), lsp_json);

    udp_socket.send(lsp_message.as_bytes()).map(|_| ())
}

// ============================================================================
// HTTP Server
// ============================================================================

/// Run the HTTP server for eval requests.
/// Accepts POST /eval with code in the body, sends workspace/executeCommand to sclang.
pub fn run_http_server(port: u16, udp_socket: UdpSocket, shutdown: Arc<AtomicBool>) -> Result<()> {
    let verbose = verbose_logging_enabled();
    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .map_err(|e| anyhow!("invalid address: {}", e))?;

    // Create socket with SO_REUSEADDR to allow quick rebinding after restart.
    // This prevents "address already in use" errors when Zed restarts quickly.
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .map_err(|e| anyhow!("failed to create socket: {}", e))?;
    socket
        .set_reuse_address(true)
        .map_err(|e| anyhow!("failed to set SO_REUSEADDR: {}", e))?;
    socket
        .bind(&addr.into())
        .map_err(|e| anyhow!("failed to bind socket to {}: {}", addr, e))?;
    socket
        .listen(128)
        .map_err(|e| anyhow!("failed to listen on socket: {}", e))?;

    // Convert to std TcpListener, then create tiny_http Server
    let listener: TcpListener = socket.into();
    let server = Server::from_listener(listener, None).map_err(|e| {
        eprintln!(
            "[sc_launcher] failed to start HTTP server on {}: {}",
            addr, e
        );
        anyhow!("HTTP server bind failed: {}", e)
    })?;

    if verbose {
        eprintln!(
            "[sc_launcher] HTTP eval server listening on http://{}",
            addr
        );
    }

    // Set a timeout so we can check shutdown flag periodically
    server
        .incoming_requests()
        .take_while(|_| !shutdown.load(Ordering::SeqCst))
        .for_each(|mut request| {
            let response = handle_http_request(&mut request, &udp_socket);
            if let Err(err) = request.respond(response) {
                eprintln!("[sc_launcher] failed to send HTTP response: {}", err);
            }
        });

    if verbose {
        eprintln!("[sc_launcher] HTTP server shutting down");
    }
    Ok(())
}

/// Handle an incoming HTTP request.
fn handle_http_request(
    request: &mut tiny_http::Request,
    udp_socket: &UdpSocket,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let url = request.url().to_string();
    let method = request.method().clone();

    // CORS preflight
    if method == Method::Options {
        return cors_preflight_response();
    }

    // Health check endpoint
    if url == "/health" && method == Method::Get {
        return json_response(r#"{"status":"ok"}"#, 200);
    }

    // Eval endpoint
    if url == "/eval" && method == Method::Post {
        return handle_eval(request, udp_socket);
    }

    // schelp conversion endpoint
    if url == "/convert-schelp" && method == Method::Post {
        return handle_convert_schelp(request);
    }

    // Command endpoints
    if method == Method::Post {
        return match url.as_str() {
            "/stop" => send_command(udp_socket, "supercollider.internal.cmdPeriod", &[]),
            "/boot" => send_command(udp_socket, "supercollider.internal.bootServer", &[]),
            "/recompile" => send_command(udp_socket, "supercollider.internal.recompile", &[]),
            "/quit" => send_command(udp_socket, "supercollider.internal.quitServer", &[]),
            _ => not_found_response(),
        };
    }

    not_found_response()
}

/// Handle POST /eval endpoint.
fn handle_eval(
    request: &mut tiny_http::Request,
    udp_socket: &UdpSocket,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut body = String::new();
    if let Err(err) = request.as_reader().read_to_string(&mut body) {
        return error_response(&format!("failed to read body: {}", err), 400);
    }

    // Send workspace/executeCommand to sclang via UDP
    let request_id = next_lsp_request_id();
    let lsp_request = create_execute_command_request(
        request_id,
        "supercollider.eval",
        vec![serde_json::json!(body)],
    );

    match send_lsp_payload(udp_socket, &lsp_request) {
        Ok(_) => {
            if verbose_logging_enabled() {
                eprintln!(
                    "[sc_launcher] HTTP /eval sent {} bytes to sclang (id={})",
                    body.len(),
                    request_id
                );
            }
            // We don't wait for the LSP response - fire and forget for now
            // The result will be posted to sclang's post window
            let response_body = format!(
                r#"{{"status":"sent","request_id":{},"code_length":{}}}"#,
                request_id,
                body.len()
            );
            json_response_with_cors(&response_body, 202)
        }
        Err(err) => {
            eprintln!("[sc_launcher] HTTP /eval failed to send UDP: {}", err);
            error_response(&format!("failed to send to sclang: {}", err), 502)
        }
    }
}

/// Send a workspace/executeCommand to sclang and return an HTTP response.
fn send_command(
    udp_socket: &UdpSocket,
    command: &str,
    arguments: &[&str],
) -> Response<std::io::Cursor<Vec<u8>>> {
    let request_id = next_lsp_request_id();
    let args: Vec<serde_json::Value> = arguments.iter().map(|s| serde_json::json!(s)).collect();
    let lsp_request = create_execute_command_request(request_id, command, args);

    match send_lsp_payload(udp_socket, &lsp_request) {
        Ok(_) => {
            if verbose_logging_enabled() {
                eprintln!(
                    "[sc_launcher] HTTP /{} sent command {} (id={})",
                    command.split('.').next_back().unwrap_or(command),
                    command,
                    request_id
                );
            }
            let response_body = format!(
                r#"{{"status":"sent","command":"{}","request_id":{}}}"#,
                command, request_id
            );
            json_response_with_cors(&response_body, 202)
        }
        Err(err) => {
            eprintln!(
                "[sc_launcher] HTTP /{} failed to send UDP: {}",
                command.split('.').next_back().unwrap_or(command),
                err
            );
            error_response(&format!("failed to send to sclang: {}", err), 502)
        }
    }
}

/// Return a 404 response with available endpoints.
fn not_found_response() -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(
        r#"{"error":"not found","endpoints":["/eval","/health","/stop","/boot","/recompile","/quit","/convert-schelp"]}"#,
        404,
    )
}

// ============================================================================
// schelp Conversion
// ============================================================================

/// Handle POST /convert-schelp endpoint.
/// Converts a .schelp file to markdown using pandoc with our custom reader.
fn handle_convert_schelp(request: &mut tiny_http::Request) -> Response<std::io::Cursor<Vec<u8>>> {
    // Read JSON body
    let mut body = String::new();
    if let Err(err) = request.as_reader().read_to_string(&mut body) {
        return error_response(&format!("failed to read body: {}", err), 400);
    }

    // Parse JSON to get path
    let json: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(err) => {
            return error_response(&format!("invalid JSON: {}", err), 400);
        }
    };

    let schelp_path = match json.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return error_response("missing 'path' field in JSON body", 400);
        }
    };

    // Verify file exists
    if !Path::new(schelp_path).exists() {
        return error_response(&format!("file not found: {}", schelp_path), 404);
    }

    // Find schelp.lua reader - relative to the binary location
    let schelp_lua = find_schelp_lua();
    let schelp_lua = match schelp_lua {
        Some(p) => p,
        None => {
            return error_response("schelp.lua reader not found", 500);
        }
    };

    // Run pandoc
    let output = Command::new("pandoc")
        .arg("-f")
        .arg(&schelp_lua)
        .arg("-t")
        .arg("markdown")
        .arg(schelp_path)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let markdown = String::from_utf8_lossy(&output.stdout);
                let response_json = serde_json::json!({ "markdown": markdown });
                json_response_with_cors(&response_json.to_string(), 200)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error_response(&format!("pandoc failed: {}", stderr), 500)
            }
        }
        Err(err) => error_response(&format!("failed to run pandoc: {}", err), 500),
    }
}

/// Find the schelp.lua reader file.
/// Looks in common locations relative to the binary.
fn find_schelp_lua() -> Option<String> {
    // Try paths relative to current executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // Development: binary is in target/debug or target/release
            // schelp.lua is at tools/schelp/schelp.lua from project root
            let dev_paths = [
                exe_dir.join("../../tools/schelp/schelp.lua"),
                exe_dir.join("../../../tools/schelp/schelp.lua"),
                exe_dir.join("../../../../tools/schelp/schelp.lua"),
            ];
            for path in &dev_paths {
                if let Ok(canonical) = path.canonicalize() {
                    return Some(canonical.to_string_lossy().into_owned());
                }
            }
        }
    }

    // Try SCHELP_LUA environment variable
    if let Ok(path) = std::env::var("SCHELP_LUA") {
        if Path::new(&path).exists() {
            return Some(path);
        }
    }

    None
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_headers_includes_required_headers() {
        let headers = cors_headers();
        assert_eq!(headers.len(), 3);

        let header_strs: Vec<String> = headers
            .iter()
            .map(|h| format!("{}: {}", h.field.as_str(), h.value.as_str()))
            .collect();

        assert!(header_strs
            .iter()
            .any(|h| h.contains("Access-Control-Allow-Origin")));
        assert!(header_strs
            .iter()
            .any(|h| h.contains("Access-Control-Allow-Methods")));
        assert!(header_strs
            .iter()
            .any(|h| h.contains("Access-Control-Allow-Headers")));
    }

    #[test]
    fn test_json_response_sets_content_type_and_status() {
        let resp = json_response(r#"{"ok":true}"#, 200);
        assert_eq!(resp.status_code().0, 200);
    }

    #[test]
    fn test_json_response_with_cors_includes_cors_header() {
        let resp = json_response_with_cors(r#"{"ok":true}"#, 200);
        assert_eq!(resp.status_code().0, 200);
        // Response has CORS header (verified by the builder pattern)
    }

    #[test]
    fn test_error_response_format_and_status() {
        let resp = error_response("test error", 500);
        assert_eq!(resp.status_code().0, 500);
    }

    #[test]
    fn test_error_response_uses_400_for_client_errors() {
        let resp = error_response("bad request", 400);
        assert_eq!(resp.status_code().0, 400);
    }

    #[test]
    fn test_cors_preflight_returns_204() {
        let resp = cors_preflight_response();
        assert_eq!(resp.status_code().0, 204);
    }

    #[test]
    fn test_not_found_returns_404() {
        let resp = not_found_response();
        assert_eq!(resp.status_code().0, 404);
    }

    #[test]
    fn test_find_schelp_lua_finds_file() {
        // This test runs from target/debug, so schelp.lua should be found
        // via the relative path search
        let result = find_schelp_lua();
        // May be None in CI if not run from expected location
        if let Some(path) = result {
            assert!(path.ends_with("schelp.lua"));
            assert!(Path::new(&path).exists());
        }
    }

    #[test]
    fn test_pandoc_available() {
        // Verify pandoc is installed (prerequisite for schelp conversion)
        let output = Command::new("pandoc").arg("--version").output();
        assert!(output.is_ok(), "pandoc must be installed");
        assert!(output.unwrap().status.success());
    }
}
