// Integration-style tests for sc_launcher. We include the main module so we can
// exercise internal functions without turning the crate into a full library target.
#![cfg(test)]

#[path = "../src/main.rs"]
#[allow(dead_code)]
mod launcher;

use launcher::*;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddrV4, TcpListener, TcpStream, UdpSocket};
use std::sync::Mutex;
use std::thread;
use std::time::Duration as StdDuration;

/// Mutex to serialize HTTP server tests.
/// Prevents port reuse race conditions when tests run in parallel.
static HTTP_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Attempt to connect; return false if permission denied so callers can early-return.
fn connect_or_skip(sock: &UdpSocket, addr: SocketAddrV4) -> bool {
    match sock.connect(addr) {
        Ok(_) => true,
        Err(err) if err.kind() == ErrorKind::PermissionDenied => {
            eprintln!("[test] UDP connect permission denied; skipping test");
            false
        }
        Err(err) => panic!("UDP connect failed: {err}"),
    }
}

/// Create a pair of UDP sockets (receiver + sender) or return None if sandboxed.
fn udp_pair() -> Option<(UdpSocket, SocketAddrV4, UdpSocket)> {
    let receiver = match UdpSocket::bind(SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, 0)) {
        Ok(s) => s,
        Err(err) if err.kind() == ErrorKind::PermissionDenied => {
            eprintln!("[test] UDP bind permission denied; skipping test");
            return None;
        }
        Err(err) => panic!("UDP bind failed: {err}"),
    };
    let receiver_addr = match receiver.local_addr().unwrap() {
        std::net::SocketAddr::V4(v4) => v4,
        _ => panic!("expected IPv4 localhost"),
    };
    let sender = match UdpSocket::bind(SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, 0)) {
        Ok(s) => s,
        Err(err) if err.kind() == ErrorKind::PermissionDenied => {
            eprintln!("[test] UDP bind permission denied; skipping test");
            return None;
        }
        Err(err) => panic!("UDP bind failed: {err}"),
    };
    Some((receiver, receiver_addr, sender))
}

/// Verify graceful_shutdown_child terminates and reaps a running child without panicking.
#[test]
#[cfg(unix)]
fn shutdown_reaps_running_child() {
    // Spawn a long-running benign process as a stand-in for sclang.
    let mut child = std::process::Command::new("sleep")
        .arg("60")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("spawn sleep");

    // UDP socket to satisfy shutdown request path.
    let Some((_recv, receiver_addr, sock)) = udp_pair() else {
        let _ = child.kill();
        let _ = child.wait();
        return;
    };
    if !connect_or_skip(&sock, receiver_addr) {
        let _ = child.kill();
        let _ = child.wait();
        return;
    }

    let status =
        graceful_shutdown_child(&mut child, &sock, StdDuration::from_millis(50), 42).unwrap();

    // Child should no longer be running and must be reapable.
    assert!(
        child.try_wait().unwrap().is_some(),
        "child should be exited after shutdown"
    );
    // Exit status may reflect signal termination; just ensure it exists.
    assert!(
        status.success() || status.code().is_none(),
        "shutdown should complete with an exit status"
    );
}

/// Verify shutdown is tolerant of already-exited children.
#[test]
#[cfg(unix)]
fn shutdown_handles_already_exited_child() {
    let mut child = std::process::Command::new("true")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    // Allow the process to exit.
    let _ = child.wait();

    let Some((_recv, receiver_addr, sock)) = udp_pair() else {
        return;
    };
    if !connect_or_skip(&sock, receiver_addr) {
        return;
    }

    let status =
        graceful_shutdown_child(&mut child, &sock, StdDuration::from_millis(10), 99).unwrap();
    assert!(
        child.try_wait().unwrap().is_some(),
        "already-exited child remains exited"
    );
    assert!(
        status.success() || status.code().is_none(),
        "shutdown on exited child should succeed"
    );
}

/// Pick a free port and return it along with an open listener to prevent reuse.
/// Caller must drop the listener just before the server binds.
fn pick_free_port() -> (u16, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    (port, listener)
}

/// Wait for HTTP server to become ready, with retry logic.
/// Returns Ok(()) when server accepts connections, Err after timeout.
fn wait_for_server(port: u16, timeout: StdDuration) -> Result<(), String> {
    let start = std::time::Instant::now();
    let mut last_error = None;
    while start.elapsed() < timeout {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(_) => return Ok(()),
            Err(e) => last_error = Some(e),
        }
        thread::sleep(StdDuration::from_millis(10));
    }
    Err(format!(
        "Server not ready after {:?}, last error: {:?}",
        timeout, last_error
    ))
}

fn http_request(port: u16, req: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.write_all(req.as_bytes()).unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
    let mut buf = String::new();
    stream.read_to_string(&mut buf).unwrap();
    buf
}

fn status_line(body: &str) -> Option<String> {
    body.lines().next().map(|s| s.trim().to_string())
}

#[test]
fn http_health_and_shutdown() {
    // Serialize HTTP tests to prevent port reuse race conditions
    // Use unwrap_or_else to recover from poisoned mutex (previous test panic)
    let _lock = HTTP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    // Create UDP socket pair. The receiver (_recv) is needed to get a valid address
    // for connecting the sender, but isn't used in this test. RAII handles cleanup.
    let Some((_recv, receiver_addr, udp_sender)) = udp_pair() else {
        return;
    };
    if !connect_or_skip(&udp_sender, receiver_addr) {
        return;
    }

    let (port, listener) = pick_free_port();
    drop(listener); // Release port for server to bind
    thread::sleep(StdDuration::from_millis(10)); // Give OS time to release port
    let shutdown_clone = shutdown.clone();
    let handle = thread::spawn(move || run_http_server(port, udp_sender, shutdown_clone));

    // Wait for server to be ready with retry logic (longer timeout for CI/loaded systems)
    wait_for_server(port, StdDuration::from_secs(10)).expect("HTTP server failed to start");

    let resp = http_request(
        port,
        "GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );
    let line = status_line(&resp).unwrap_or_default();
    assert!(
        line.contains("200"),
        "expected 200 status, got line: {}",
        line
    );

    // Signal shutdown and send a final request to unblock the server
    shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
    let _ = http_request(
        port,
        "GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    handle
        .join()
        .expect("HTTP server thread panicked")
        .expect("HTTP server returned error");
}

#[test]
fn http_eval_sends_udp() {
    // Serialize HTTP tests to prevent port reuse race conditions
    // Use unwrap_or_else to recover from poisoned mutex (previous test panic)
    let _lock = HTTP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    // Create UDP socket pair. The receiver is used to verify UDP payload delivery.
    let Some((receiver, receiver_addr, udp_sender)) = udp_pair() else {
        return;
    };
    if !connect_or_skip(&udp_sender, receiver_addr) {
        return;
    }

    let (port, listener) = pick_free_port();
    drop(listener); // Release port for server to bind
    thread::sleep(StdDuration::from_millis(10)); // Give OS time to release port
    let shutdown_clone = shutdown.clone();
    let handle = thread::spawn(move || run_http_server(port, udp_sender, shutdown_clone));

    // Wait for server to be ready with retry logic (longer timeout for CI/loaded systems)
    wait_for_server(port, StdDuration::from_secs(10)).expect("HTTP server failed to start");

    let body = "1+1";
    let req = format!(
        "POST /eval HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let resp = http_request(port, &req);
    let line = status_line(&resp).unwrap_or_default();
    assert!(
        line.contains("202"),
        "expected 202 status, got line: {}",
        line
    );

    // Confirm UDP payload was emitted to the receiver.
    receiver
        .set_read_timeout(Some(StdDuration::from_secs(1)))
        .unwrap();
    let mut buf = [0u8; 8192];
    let received = receiver.recv(&mut buf).unwrap();
    let payload = String::from_utf8_lossy(&buf[..received]);
    assert!(
        payload.contains("supercollider.eval"),
        "expected eval command over UDP, got: {}",
        payload
    );

    // Signal shutdown and send a final request to unblock the server
    shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
    let _ = http_request(
        port,
        "GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    handle
        .join()
        .expect("HTTP server thread panicked")
        .expect("HTTP server returned error");
}

#[test]
fn duplicate_spawn_guard_blocks_second_run() {
    // Simulate an in-progress run
    IS_RUNNING.store(true, std::sync::atomic::Ordering::SeqCst);
    let args = Args {
        sclang_path: None,
        conf_yaml_path: None,
        mode: Mode::Lsp,
        log_level: None,
        http_port: 0,
    };
    let res = run_lsp_bridge("/bin/echo", &args);
    // Clear guard for other tests
    IS_RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
    assert!(
        res.is_err(),
        "second concurrent run should be rejected by guard"
    );
}

#[test]
fn pid_file_write_and_remove() {
    // Write PID file
    let launcher_pid = 12345;
    let sclang_pid = 67890;
    write_pid_file(launcher_pid, sclang_pid).expect("write_pid_file should succeed");

    // Verify file exists and contains expected JSON
    let pid_path = std::env::temp_dir().join("sc_launcher.pid");
    assert!(pid_path.exists(), "PID file should exist after write");

    let content = std::fs::read_to_string(&pid_path).expect("read PID file");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("parse PID JSON");
    assert_eq!(parsed["launcher_pid"], launcher_pid);
    assert_eq!(parsed["sclang_pid"], sclang_pid);

    // Remove PID file
    remove_pid_file();
    assert!(
        !pid_path.exists(),
        "PID file should be removed after cleanup"
    );
}
