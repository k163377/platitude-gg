//! git subprocess execution — the single place that spawns `git`
//! (rules/core.md §git サブプロセス規約).

mod child;
mod command;
mod executor;
mod meter;
mod program;
mod slots;

pub use command::{CommandEnd, CommandObserver, GitCommand, GitOutput, Kept, literal_pathspec};
pub use executor::{DEFAULT_TIMEOUT, GitExecutor};
pub use meter::Meter;
pub use program::{default_program_path, same_program, spawnable};
pub use slots::{
    DEFAULT_CONCURRENCY_CEILING, DEFAULT_CONCURRENCY_FLOOR, Limits, MAX_CONCURRENCY,
    OVERTAKEN_LIMIT, Pace, Priority, Report, Slot, Slots, concurrency, default_concurrency,
};
