#!/usr/bin/env python3
"""
Test harness for LSP code actions.

Spawns the launcher in LSP mode and verifies that code actions work
without needing Zed. This isolates launcher bugs from editor issues.

Usage:
    ./scripts/test-lsp-codeactions.py [--verbose] [--timeout SECS]
"""

import json
import subprocess
import sys
import os
import time
import argparse
from pathlib import Path

# Colors for output
GREEN = "\033[92m"
RED = "\033[91m"
YELLOW = "\033[93m"
RESET = "\033[0m"
BOLD = "\033[1m"

def log(msg, color=None):
    if color:
        print(f"{color}{msg}{RESET}", file=sys.stderr)
    else:
        print(msg, file=sys.stderr)

def log_ok(msg):
    log(f"✓ {msg}", GREEN)

def log_fail(msg):
    log(f"✗ {msg}", RED)

def log_info(msg):
    log(f"→ {msg}", YELLOW)


class LSPClient:
    """Simple LSP client that communicates with the launcher over stdio."""

    def __init__(self, launcher_path: Path, verbose: bool = False):
        self.launcher_path = launcher_path
        self.verbose = verbose
        self.process = None
        self.request_id = 0

    def start(self, timeout: float = 30.0):
        """Start the launcher process."""
        env = os.environ.copy()
        if self.verbose:
            env["SC_LAUNCHER_VERBOSE"] = "1"

        self.process = subprocess.Popen(
            [str(self.launcher_path), "--mode", "lsp"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
        )

        # Set up non-blocking stderr reading
        import threading
        import queue
        self.stderr_queue = queue.Queue()
        def read_stderr():
            for line in self.process.stderr:
                self.stderr_queue.put(line)
        self.stderr_thread = threading.Thread(target=read_stderr, daemon=True)
        self.stderr_thread.start()

        log_info(f"Spawned launcher (PID {self.process.pid})")

    def stop(self):
        """Stop the launcher process."""
        if self.process:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
            log_info("Stopped launcher")

    def drain_stderr(self):
        """Print any buffered stderr output."""
        while True:
            try:
                line = self.stderr_queue.get_nowait()
                sys.stderr.buffer.write(line)
            except:
                break
        sys.stderr.flush()

    def send_request(self, method: str, params: dict = None) -> int:
        """Send an LSP request and return the request ID."""
        self.request_id += 1
        msg = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
        }
        if params is not None:
            msg["params"] = params

        body = json.dumps(msg)
        header = f"Content-Length: {len(body)}\r\n\r\n"

        if self.verbose:
            log_info(f"Sending: {method} (id={self.request_id})")

        self.process.stdin.write(header.encode())
        self.process.stdin.write(body.encode())
        self.process.stdin.flush()

        return self.request_id

    def send_notification(self, method: str, params: dict = None):
        """Send an LSP notification (no response expected)."""
        msg = {
            "jsonrpc": "2.0",
            "method": method,
        }
        if params is not None:
            msg["params"] = params

        body = json.dumps(msg)
        header = f"Content-Length: {len(body)}\r\n\r\n"

        if self.verbose:
            log_info(f"Sending notification: {method}")

        self.process.stdin.write(header.encode())
        self.process.stdin.write(body.encode())
        self.process.stdin.flush()

    def read_response(self, timeout: float = 60.0) -> dict:
        """Read an LSP response from stdout."""
        start = time.time()

        # Read header
        headers = {}
        while True:
            if time.time() - start > timeout:
                self.drain_stderr()
                raise TimeoutError(f"Timeout waiting for response header (>{timeout}s)")

            line = self.process.stdout.readline()
            if not line:
                self.drain_stderr()
                raise EOFError("Launcher closed stdout")

            line = line.decode().strip()
            if not line:
                break  # End of headers

            if ":" in line:
                key, value = line.split(":", 1)
                headers[key.strip()] = value.strip()

        content_length = int(headers.get("Content-Length", 0))
        if content_length == 0:
            raise ValueError("Missing Content-Length header")

        # Read body
        body = self.process.stdout.read(content_length)
        if len(body) < content_length:
            self.drain_stderr()
            raise EOFError(f"Incomplete body: expected {content_length}, got {len(body)}")

        response = json.loads(body)

        if self.verbose:
            log_info(f"Received: id={response.get('id')} result_keys={list(response.get('result', {}).keys()) if isinstance(response.get('result'), dict) else type(response.get('result')).__name__}")

        return response

    def wait_for_response(self, expected_id: int, timeout: float = 60.0) -> dict:
        """Read responses until we get one with the expected ID."""
        start = time.time()
        while True:
            remaining = timeout - (time.time() - start)
            if remaining <= 0:
                self.drain_stderr()
                raise TimeoutError(f"Timeout waiting for response id={expected_id}")

            response = self.read_response(timeout=remaining)

            # Skip notifications (no id)
            if "id" not in response:
                if self.verbose:
                    log_info(f"Skipping notification: {response.get('method')}")
                continue

            if response["id"] == expected_id:
                return response


def run_test(launcher_path: Path, test_file: Path, verbose: bool, timeout: float) -> bool:
    """Run the LSP code action test."""
    client = LSPClient(launcher_path, verbose=verbose)

    try:
        # Start launcher
        client.start(timeout=timeout)

        # 1. Send initialize request
        log_info("Sending initialize request...")
        init_id = client.send_request("initialize", {
            "processId": os.getpid(),
            "rootUri": f"file://{test_file.parent.parent}",
            "capabilities": {
                "textDocument": {
                    "codeAction": {
                        "codeActionLiteralSupport": {
                            "codeActionKind": {
                                "valueSet": ["source", "quickfix", "refactor"]
                            }
                        }
                    }
                }
            }
        })

        # Wait for initialize response
        init_response = client.wait_for_response(init_id, timeout=timeout)

        if "error" in init_response:
            log_fail(f"Initialize failed: {init_response['error']}")
            return False

        log_ok("Initialize succeeded")

        # Verify code action capability
        caps = init_response.get("result", {}).get("capabilities", {})
        code_action_cap = caps.get("codeActionProvider")
        if not code_action_cap:
            log_fail("Server does not advertise codeActionProvider capability!")
            return False
        log_ok(f"Server has codeActionProvider: {code_action_cap}")

        # Check executeCommandProvider
        exec_cmd = caps.get("executeCommandProvider")
        if exec_cmd:
            commands = exec_cmd.get("commands", [])
            log_ok(f"Server has executeCommandProvider with {len(commands)} commands")
            if verbose:
                for cmd in commands:
                    log_info(f"    Command: {cmd}")
        else:
            log_fail("Server does not advertise executeCommandProvider!")

        # 2. Send initialized notification
        client.send_notification("initialized", {})
        log_ok("Sent initialized notification")

        # 3. Open the test file
        log_info("Opening test file...")
        with open(test_file) as f:
            content = f.read()

        uri = f"file://{test_file}"
        client.send_notification("textDocument/didOpen", {
            "textDocument": {
                "uri": uri,
                "languageId": "supercollider",
                "version": 1,
                "text": content,
            }
        })
        log_ok(f"Opened {test_file.name}")

        # 4. Wait for sclang to be ready (give it time to start)
        log_info("Waiting for sclang to initialize (15s)...")
        time.sleep(15)

        # 5. Request code actions at line 7 ("single line".postln;)
        log_info("Requesting code actions at line 7...")
        ca_start = time.time()
        ca_id = client.send_request("textDocument/codeAction", {
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 6, "character": 0},  # 0-indexed
                "end": {"line": 6, "character": 20},
            },
            "context": {
                "diagnostics": [],
                "triggerKind": 1,  # Invoked
            }
        })

        # 6. Wait for code action response
        log_info(f"Waiting for code action response (id={ca_id})...")
        ca_response = client.wait_for_response(ca_id, timeout=timeout)

        ca_elapsed = time.time() - ca_start
        log_info(f"Code action response received in {ca_elapsed:.3f}s")

        if "error" in ca_response:
            log_fail(f"Code action failed: {ca_response['error']}")
            client.drain_stderr()
            return False

        result = ca_response.get("result", [])

        # 7. Verify we got code actions
        if not isinstance(result, list):
            log_fail(f"Expected array result, got: {type(result).__name__}")
            client.drain_stderr()
            return False

        if len(result) == 0:
            log_fail("No code actions returned!")
            client.drain_stderr()
            return False

        log_ok(f"Got {len(result)} code action(s)")

        # 8. Verify we have an "Evaluate" action
        eval_action = None
        for action in result:
            title = action.get("title", "")
            kind = action.get("kind", "")
            log_info(f"  Action: {title} (kind={kind})")
            if verbose:
                log_info(f"    Full: {json.dumps(action, indent=2)}")
            if "Evaluate" in title:
                eval_action = action

        if not eval_action:
            log_fail("No 'Evaluate' action found in results!")
            client.drain_stderr()
            return False

        log_ok(f"Found evaluate action: {eval_action.get('title')}")

        # Verify the action has required fields
        if "title" not in eval_action:
            log_fail("Action missing 'title' field!")
            return False
        if "kind" not in eval_action:
            log_fail("Action missing 'kind' field!")
            return False
        log_ok(f"Action has required fields: title='{eval_action['title']}', kind='{eval_action['kind']}'")

        # All tests passed!
        log("")
        log(f"{BOLD}{GREEN}ALL TESTS PASSED{RESET}")
        client.drain_stderr()
        return True

    except Exception as e:
        log_fail(f"Test failed with exception: {e}")
        client.drain_stderr()
        import traceback
        traceback.print_exc()
        return False
    finally:
        client.stop()


def main():
    parser = argparse.ArgumentParser(description="Test LSP code actions")
    parser.add_argument("--verbose", "-v", action="store_true", help="Enable verbose output")
    parser.add_argument("--timeout", "-t", type=float, default=60.0, help="Timeout in seconds")
    args = parser.parse_args()

    # Find paths relative to script location
    script_dir = Path(__file__).parent
    project_root = script_dir.parent

    launcher_path = project_root / "server" / "launcher" / "target" / "release" / "sc_launcher"
    if not launcher_path.exists():
        launcher_path = project_root / "server" / "launcher" / "target" / "debug" / "sc_launcher"
    if not launcher_path.exists():
        launcher_path = project_root / "target" / "release" / "sc_launcher"
    if not launcher_path.exists():
        launcher_path = project_root / "target" / "debug" / "sc_launcher"

    if not launcher_path.exists():
        log_fail(f"Launcher not found at {launcher_path}")
        log_info("Run ./scripts/build.sh first")
        sys.exit(1)

    test_file = project_root / "tests" / "eval_test.scd"
    if not test_file.exists():
        log_fail(f"Test file not found: {test_file}")
        sys.exit(1)

    log(f"{BOLD}LSP Code Action Test Harness{RESET}")
    log(f"Launcher: {launcher_path}")
    log(f"Test file: {test_file}")
    log("")

    success = run_test(launcher_path, test_file, args.verbose, args.timeout)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
