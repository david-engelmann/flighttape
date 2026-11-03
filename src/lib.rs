//! flighttape: flight tapes for agent runs.
//!
//! A library crate with a thin command line binary.
//! The modules mirror the behavioral spec's division:
//! value and json carry the tape's data, identity its
//! canonical form and digest, and one module per
//! command family carries the work.

pub mod identity;
pub mod json;
pub mod value;

/// The tape format version this crate speaks.
pub const TAPE_VERSION: &str = "1.0";

/// Placeholder until the command line lands in this
/// round; returns the usage exit code.
pub fn run_cli() -> i32 {
    2
}
