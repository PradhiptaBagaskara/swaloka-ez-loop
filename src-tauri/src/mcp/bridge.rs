//! Jembatan stdio ke endpoint MCP aplikasi yang sedang berjalan.
//!
//! Dipakai klien yang hanya bisa stdio: aplikasi dijalankan dengan argumen
//! `--mcp-stdio`, lalu jembatan ini meneruskan JSON-RPC dari stdin ke endpoint
//! loopback aplikasi dan menulis balasannya ke stdout. Jembatan tidak
//! membangun `AppState` sendiri, jadi run yang dimulai agen tetap berbagi satu
//! proses dan satu queue card dengan pengguna. Bila aplikasi belum jalan, ia
//! berhenti dengan pesan yang jelas alih-alih memulai proses penulis kedua.

use std::io::{BufRead, Write};

/// Pesan saat server tidak bisa dihubungi.
const HELP: &str = "Kontrol AI tidak aktif.\nBuka aplikasi Swaloka, lalu aktifkan \"Kontrol AI (MCP)\" di Pengaturan, sebelum menjalankan mode stdio.";

struct Runtime {
    port: u16,
    token: String,
}

/// Jalankan jembatan. Mengembalikan kode keluar proses.
pub fn run() -> i32 {
    let runtime = match read_runtime() {
        Ok(runtime) => runtime,
        Err(message) => {
            eprintln!("{message}");
            return 1;
        }
    };

    let endpoint = format!("http://127.0.0.1:{}/mcp", runtime.port);
    let authorization = format!("Bearer {}", runtime.token);

    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();

    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }
        match forward(&endpoint, &authorization, &line) {
            Ok(Some(payload)) => {
                let _ = writeln!(out, "{payload}");
                let _ = out.flush();
            }
            // Notifikasi tidak berbalas.
            Ok(None) => {}
            Err(message) => {
                eprintln!("{message}");
                return 1;
            }
        }
    }

    0
}

/// Baca port dan token server yang sedang jalan dari file runtime.
fn read_runtime() -> Result<Runtime, String> {
    let path = crate::storage::paths::mcp_runtime_file();
    let text = std::fs::read_to_string(&path).map_err(|_| HELP.to_string())?;
    let data: serde_json::Value = serde_json::from_str(&text).map_err(|_| HELP.to_string())?;
    let port = data
        .get("port")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| HELP.to_string())? as u16;
    let token = data
        .get("token")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| HELP.to_string())?
        .to_string();
    Ok(Runtime { port, token })
}

/// Teruskan satu pesan JSON-RPC ke endpoint aplikasi; `None` bila tak berbalas.
fn forward(endpoint: &str, authorization: &str, payload: &str) -> Result<Option<String>, String> {
    let mut response = ureq::post(endpoint)
        .header("Authorization", authorization)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .send(payload)
        .map_err(|error| format!("Gagal menghubungi aplikasi MCP: {error}\n{HELP}"))?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("Balasan MCP tidak terbaca: {error}"))?;
    Ok(extract_body(&body))
}

/// Ambil payload JSON dari balasan: body JSON biasa, atau event SSE (`data:`).
fn extract_body(body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return Some(trimmed.to_string());
    }
    trimmed
        .lines()
        .filter_map(|line| line.strip_prefix("data:").map(str::trim))
        .rfind(|payload| !payload.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_json_body_passes_through() {
        assert_eq!(
            extract_body("  {\"jsonrpc\":\"2.0\"}  "),
            Some(String::from("{\"jsonrpc\":\"2.0\"}"))
        );
    }

    #[test]
    fn empty_body_has_no_response() {
        assert_eq!(extract_body("   \n "), None, "notifikasi tidak berbalas");
    }

    #[test]
    fn sse_body_uses_the_last_data_frame() {
        let body = "event: message\ndata: {\"a\":1}\n\nevent: message\ndata: {\"b\":2}\n";
        assert_eq!(extract_body(body), Some(String::from("{\"b\":2}")));
    }
}
