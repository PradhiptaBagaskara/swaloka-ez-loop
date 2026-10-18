//! Server MCP (Kontrol AI): mengekspos aksi aplikasi ke klien MCP.
//!
//! Server berjalan di dalam proses aplikasi dan memakai [`AppState`] yang sama
//! dengan antarmuka, jadi run yang dimulai agen tampil di queue card dan memakai
//! penjaga single-run yang sama. Hanya mendengarkan di loopback dan dijaga token
//! per sesi; mati kecuali diaktifkan pengguna. Lihat
//! `openspec/changes/mcp-server`.

pub mod bridge;
pub mod tools;

use std::sync::Mutex;

use axum::extract::{Request, State as AxumState};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::Router;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tauri::AppHandle;

use tools::SwalokaMcp;

/// Port loopback bawaan.
pub const DEFAULT_PORT: u16 = 8787;

/// Status server untuk antarmuka.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub running: bool,
    pub port: u16,
    pub endpoint: Option<String>,
    pub token: Option<String>,
}

struct Running {
    port: u16,
    token: String,
    shutdown: tokio::sync::oneshot::Sender<()>,
}

impl Running {
    fn status(&self) -> McpStatus {
        McpStatus {
            running: true,
            port: self.port,
            endpoint: Some(endpoint(self.port)),
            token: Some(self.token.clone()),
        }
    }
}

fn endpoint(port: u16) -> String {
    format!("http://127.0.0.1:{port}/mcp")
}

fn stopped_status(port: u16) -> McpStatus {
    McpStatus {
        running: false,
        port,
        endpoint: None,
        token: None,
    }
}

/// Pengelola server MCP: menyalakan, mematikan, dan melaporkan statusnya.
pub struct McpServer {
    app: AppHandle,
    running: Mutex<Option<Running>>,
}

impl McpServer {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            running: Mutex::new(None),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Running>> {
        self.running
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    pub fn status(&self) -> McpStatus {
        let guard = self.lock();
        match guard.as_ref() {
            Some(running) => running.status(),
            None => stopped_status(DEFAULT_PORT),
        }
    }

    pub fn is_running(&self) -> bool {
        self.lock().is_some()
    }

    /// Nyalakan server pada port loopback. Idempoten: bila sudah jalan, status
    /// yang sama dikembalikan tanpa memulai server kedua.
    pub fn start(&self, port: u16) -> McpStatus {
        if self.lock().is_some() {
            return self.status();
        }
        let token = new_token();
        let (shutdown, receiver) = tokio::sync::oneshot::channel::<()>();
        let app = self.app.clone();
        let middleware_token = token.clone();
        let selected_port = port;
        write_runtime_file(selected_port, &token);

        tauri::async_runtime::spawn(async move {
            let service = StreamableHttpService::new(
                move || Ok(SwalokaMcp::new(app.clone())),
                LocalSessionManager::default().into(),
                StreamableHttpServerConfig::default()
                    .with_legacy_session_mode(false)
                    .with_json_response(true),
            );
            let router =
                Router::new()
                    .nest_service("/mcp", service)
                    .layer(middleware::from_fn_with_state(
                        middleware_token,
                        require_token,
                    ));

            let address = std::net::SocketAddr::from(([127, 0, 0, 1], selected_port));
            match tokio::net::TcpListener::bind(address).await {
                Ok(listener) => {
                    let shutdown = async move {
                        let _ = receiver.await;
                    };
                    let _ = axum::serve(listener, router)
                        .with_graceful_shutdown(shutdown)
                        .await;
                }
                Err(error) => {
                    eprintln!("MCP: gagal bind {address}: {error}");
                }
            }
        });

        *self.lock() = Some(Running {
            port: selected_port,
            token,
            shutdown,
        });
        self.status()
    }

    /// Matikan server bila sedang jalan.
    pub fn stop(&self) -> McpStatus {
        if let Some(running) = self.lock().take() {
            let _ = running.shutdown.send(());
        }
        let _ = std::fs::remove_file(crate::storage::paths::mcp_runtime_file());
        stopped_status(DEFAULT_PORT)
    }
}

/// Tulis port dan token sesi ke file runtime supaya jembatan stdio bisa
/// menghubungi server yang sedang jalan. Server yang tidak jalan tidak punya
/// file ini, sehingga jembatan memberi pesan yang jelas.
fn write_runtime_file(port: u16, token: &str) {
    let path = crate::storage::paths::mcp_runtime_file();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let payload = serde_json::json!({ "port": port, "token": token });
    let _ = std::fs::write(path, payload.to_string());
}

/// Token acak per sesi (32 byte heksadesimal).
fn new_token() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut token = String::with_capacity(64);
    for _ in 0..32 {
        token.push_str(&format!("{:02x}", rng.random::<u8>()));
    }
    token
}

/// Middleware: tolak permintaan tanpa token sesi yang benar.
async fn require_token(
    AxumState(token): AxumState<String>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let authorized = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(|value| value == format!("Bearer {token}"))
        .unwrap_or(false);
    if authorized {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_is_loopback_only() {
        assert_eq!(endpoint(8787), "http://127.0.0.1:8787/mcp");
    }

    #[test]
    fn tokens_are_64_hex_chars_and_unique() {
        let token = new_token();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|character| character.is_ascii_hexdigit()));
        assert_ne!(new_token(), new_token(), "token berbeda tiap sesi");
    }

    #[test]
    fn stopped_status_hides_endpoint_and_token() {
        let status = stopped_status(DEFAULT_PORT);
        assert!(!status.running);
        assert!(status.endpoint.is_none());
        assert!(status.token.is_none());
    }
}
