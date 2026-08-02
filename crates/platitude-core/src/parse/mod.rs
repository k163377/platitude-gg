//! Parsers for machine-readable git output.
//!
//! Only stable formats are parsed (`--porcelain=v2`, `-z`, `--format=`);
//! human-oriented output is never interpreted.

pub mod log;
