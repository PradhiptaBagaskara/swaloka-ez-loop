// Mencegah jendela konsol tambahan muncul di Windows pada build release.
//
// Subsystem GUI hanya mencegah alokasi konsol: saat dijalankan sebagai server
// stdio (spawn oleh klien MCP dengan stdin/stdout berupa pipe), I/O lewat pipe
// tetap bekerja. Menjalankannya langsung di terminal Windows release baru tidak
// menampilkan output — itu satu-satunya konsekuensinya.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Klien yang hanya bisa stdio menjalankan aplikasi dengan `--mcp-stdio`:
    // jalankan jembatan ke server loopback aplikasi yang sedang jalan, bukan GUI.
    if std::env::args().any(|argument| argument == "--mcp-stdio") {
        std::process::exit(swaloka_lib::mcp::bridge::run());
    }
    swaloka_lib::run()
}
