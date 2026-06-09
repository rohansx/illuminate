//! Guards the MCP `initialize` handshake against a stale, hardcoded
//! `serverInfo.version`. The reported version MUST track the crate's
//! `CARGO_PKG_VERSION` (the workspace version) rather than the historical
//! literal `"0.3.0"`.
//!
//! No mocks: this spawns the real `illuminate-mcp` binary over stdio and
//! reads back the genuine `initialize` response, exactly like a client would.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct McpProcess {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
}

impl McpProcess {
    fn spawn(db_path: &str) -> Self {
        let binary = env!("CARGO_BIN_EXE_illuminate-mcp");
        let mut child = Command::new(binary)
            .arg("--db")
            .arg(db_path)
            .env("ILLUMINATE_NO_EMBED", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn illuminate-mcp binary");

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        Self {
            child,
            stdin,
            reader: BufReader::new(stdout),
        }
    }

    fn send(&mut self, msg: Value) {
        let line = serde_json::to_string(&msg).unwrap() + "\n";
        self.stdin.write_all(line.as_bytes()).unwrap();
        self.stdin.flush().unwrap();
    }

    fn recv(&mut self) -> Value {
        let mut line = String::new();
        self.reader.read_line(&mut line).unwrap();
        serde_json::from_str(line.trim()).expect("invalid JSON response")
    }
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn initialize_response() -> Value {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("version.db").to_str().unwrap().to_string();
    let mut mcp = McpProcess::spawn(&db);
    mcp.send(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "clientInfo": {"name": "version-test", "version": "0.1"}
        }
    }));
    mcp.recv()
}

#[test]
fn server_info_version_matches_cargo_pkg_version() {
    let resp = initialize_response();
    let reported = resp["result"]["serverInfo"]["version"]
        .as_str()
        .expect("serverInfo.version must be a string");
    assert_eq!(
        reported,
        env!("CARGO_PKG_VERSION"),
        "serverInfo.version must track CARGO_PKG_VERSION (the workspace version)"
    );
}

#[test]
fn server_info_version_is_not_stale_literal() {
    let resp = initialize_response();
    let reported = resp["result"]["serverInfo"]["version"]
        .as_str()
        .expect("serverInfo.version must be a string");
    assert_ne!(
        reported, "0.3.0",
        "serverInfo.version must no longer be the stale hardcoded literal 0.3.0"
    );
}
