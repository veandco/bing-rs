extern crate cbindgen;

// std
use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_item_prefix("CAPI_")
        .with_include_guard("bing_rs_h")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_path.join("bing-rs.h"));
}
