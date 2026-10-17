// Multi-state/report helpers remain shared source but are not browser API entry points.
#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]

#[cfg(test)]
#[path = "../tests/benchmarks.rs"]
mod benchmarks;
mod classification;
#[cfg(not(target_arch = "wasm32"))]
mod cli;
mod coverage;
mod html;
mod icons;
mod layout;
mod model;
mod plan;
mod provider;
mod semantic;
mod svg;
#[cfg(not(target_arch = "wasm32"))]
mod timing;
#[cfg(target_arch = "wasm32")]
#[path = "timing_browser.rs"]
mod timing;
mod view;

mod generation;
pub use generation::{Error, Generated, Options, OutputFormat, generate};
pub use icons::IconMode;
pub use svg::AddressFormat;

#[cfg(not(target_arch = "wasm32"))]
pub fn run_cli() -> Result<(), String> {
    cli::run()
}

#[cfg(target_arch = "wasm32")]
mod wasm;
