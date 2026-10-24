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
mod work;

#[cfg(all(test, not(feature = "icon-artwork")))]
#[path = "../tests/unit/artwork_free.rs"]
mod artwork_free;
mod generation;
mod named_inputs;
pub use named_inputs::{NamedInput, generate_named, inspect_named_inputs};
mod module_inventory;
mod resource_inventory;
pub use generation::{Error, Generated, Generator, Options, OutputFormat, generate};
pub use icons::IconMode;
pub use module_inventory::{ModuleInventoryEntry, inspect_modules};
pub use resource_inventory::{
    InputInventory, ResourceInventoryEntry, inspect_input, inspect_resources,
};
pub use svg::AddressFormat;

#[cfg(not(target_arch = "wasm32"))]
pub fn run_cli() -> Result<(), String> {
    cli::run()
}

#[cfg(target_arch = "wasm32")]
mod wasm;
