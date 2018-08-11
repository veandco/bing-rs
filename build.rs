extern crate cbindgen;

// std
use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_item_prefix("CAPI_")
        .with_include_guard("bing_rs_h")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("bing-rs.h");
}