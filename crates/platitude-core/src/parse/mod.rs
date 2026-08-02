//! Parsers for machine-readable git output.
//!
//! Only stable formats are parsed (`--porcelain=v2`, `-z`, `--format=`);
//! human-oriented output is never interpreted.

pub mod diff;
pub mod log;
pub mod name_status;
