// Mencegah jendela konsol tambahan muncul di Windows pada build release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    swaloka_lib::run()
}
