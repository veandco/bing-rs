//! # Bing for Rust
//!
//! `bing-rs` is a library for using Bing API via Rust.

#[macro_use]
extern crate error_chain;

// Tokio/Futures Crates
extern crate futures;
extern crate tokio_core;

// Hyper Crates
extern crate hyper;
#[cfg(feature = "rustls")]
extern crate hyper_rustls;
#[cfg(feature = "native-tls")]
extern crate hyper_tls;
#[cfg(feature = "native-tls")]
extern crate native_tls;

// WebSocket Crates
extern crate ws;

// Serde Crates
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

// Url
extern crate url;

// Log
#[macro_use]
extern crate log;

// Chrono
extern crate chrono;

// UUID
extern crate uuid;

// C
extern crate libc;

pub mod errors;
pub mod speech;
