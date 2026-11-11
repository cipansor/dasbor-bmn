#![recursion_limit = "512"]

pub mod analitik;
pub mod app;
pub mod auth;
pub mod catalog;
pub mod components;
pub mod config;
pub mod demo;
pub mod dto;
pub mod fakta;
pub mod fs_aman;
pub mod model;
pub mod pages;
pub mod report;
pub mod schema;
pub mod server_fns;
pub mod sinkron;
pub mod sldk;
pub mod store;

/// Waktu sekarang dalam format tanggal-jam (WIB).
pub fn now_string() -> String {
    #[cfg(feature = "ssr")]
    {
        (chrono::Utc::now() + chrono::Duration::hours(7))
            .format("%Y-%m-%d %H:%M WIB")
            .to_string()
    }
    #[cfg(not(feature = "ssr"))]
    {
        String::new()
    }
}

/// Tahun dan bulan berjalan menurut Waktu Indonesia Barat (UTC+7).
#[cfg(feature = "ssr")]
pub fn tanggal_kini() -> (i32, u32) {
    use chrono::Datelike;
    let now = chrono::Utc::now() + chrono::Duration::hours(7);
    (now.year(), now.month())
}

/// Tanggal hari ini (WIB) dalam bentuk `YYYY-MM-DD`.
#[cfg(feature = "ssr")]
pub fn hari_ini() -> String {
    (chrono::Utc::now() + chrono::Duration::hours(7))
        .format("%Y-%m-%d")
        .to_string()
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
