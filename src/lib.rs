//! # Bing for Rust
//! 
//! `bing-rs` is a library for using Bing API via Rust.

#[macro_use]
extern crate error_chain;

// Tokio/Futures Crates
extern crate tokio_core;
extern crate futures;

// Hyper Crates
extern crate hyper;
#[cfg(feature = "rustls")]
extern crate hyper_rustls;
#[cfg(feature = "native-tls")]
extern crate hyper_tls;
#[cfg(feature = "native-tls")]
extern crate native_tls;

// Serde Crates
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

pub mod error;
pub mod speech;