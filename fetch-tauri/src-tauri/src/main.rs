// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use fetch_common::bin::init_ort;
use fetch_core::embeddable::session_pool::init_querying;

fn main() {
    init_ort().expect("Failed initializing ort");
    init_querying();
    fetch_tauri_lib::run()
}
