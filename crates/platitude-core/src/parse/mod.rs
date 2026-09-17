//! Parsers for machine-readable git output.
//!
//! Only stable formats are parsed (`--porcelain=v2`, `-z`,
//! `--format=`).

pub mod diff;
pub mod log;
pub mod name_status;
