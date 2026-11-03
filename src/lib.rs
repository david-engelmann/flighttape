//! flighttape: flight tapes for agent runs.
//!
//! A library crate with a thin command line binary.
//! The modules mirror the behavioral spec's division:
//! value and json carry the tape's data, identity its
//! canonical form and digest, and one module per
//! command family carries the work.

pub mod cli;
pub mod compare;
pub mod convert;
pub mod dumps;
pub mod helptext;
pub mod identity;
pub mod outcome;
pub mod play;
pub mod rollup;
pub mod screen;
pub mod slice;
pub mod json;
pub mod pyrepr;
pub mod timeparse;
pub mod validate;
pub mod value;

/// The tape format version this crate speaks.
pub const TAPE_VERSION: &str = "1.0";

/// The command line entry, in the cli module.
pub fn run_cli() -> i32 {
    // A generous stack: parsed values nest as deep as
    // their files, and cloning or dropping a deep
    // value recurses with its depth.
    let child = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(crate::cli::run_cli)
        .expect("the CLI thread starts");
    child.join().unwrap_or(1)
}
