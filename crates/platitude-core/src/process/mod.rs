//! git subprocess execution — the single place that spawns `git`.
//!
//! Policy (.claude/rules/core.md / 実装計画 §2):
//! - argument vectors only, never shell strings
//! - fixed config arguments and environment keep output machine-readable,
//!   prompt-free and lock-friendly
//! - every run is cancellable and (optionally) time-limited; the process is
//!   killed when either fires
//! - on Windows no console window is shown

mod child;
mod command;
mod executor;
mod program;
mod slots;

pub use command::{CommandEnd, CommandObserver, GitCommand, GitOutput, Kept, literal_pathspec};
pub use executor::{DEFAULT_TIMEOUT, GitExecutor};
pub use program::{default_program_path, same_program, spawnable};
pub use slots::{
    DEFAULT_CONCURRENCY_CEILING, DEFAULT_CONCURRENCY_FLOOR, Limits, MAX_CONCURRENCY,
    OVERTAKEN_LIMIT, Pace, Priority, Report, Slot, Slots, concurrency, default_concurrency,
};
