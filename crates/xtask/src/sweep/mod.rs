//! `cargo xtask sweep` — one generation of build products in `target/`,
//! and nothing else (internal-docs/反映前テストの機械化.md §世代の掃除;
//! the cost in ci/baseline/code-costs-windows-x64.md §build directory の世代).
//!
//! **What is alive is asked of cargo, not of the clock**: a fresh build
//! does not touch a unit's `fingerprint/invoked.timestamp`, so an age is no
//! evidence. The live set is read off `--message-format=json` from
//! [`CANONICAL`], which names fresh units and compiled ones alike.
//!
//! **Run when the generation moves** ([`generation`]): the tail of a gate
//! or a landing sweeps only when the key differs from the one its tree's
//! last sweep stamped. **What was written since the last sweep stays**,
//! live set or not: taking away a configuration somebody used this week
//! would be a rebuild every time.
//!
//! `target/hooks` ([`PROFILES`]) and everything under `target/` that is
//! not cargo's are left alone.

use crate::command::{self, Permission, Where};

mod run;

pub use run::run;
pub(crate) use run::{Tail, at_a_tail, same_file};

pub(crate) static SWEEP: command::Command = command::Command {
    id: "sweep.now",
    call: "sweep",
    purpose: "take away every build product but the generation the canonical cargo lines stand on",
    run_in: Where::Seat,
    needs: &[
        "the lines it reads the live set from compile whatever this tree has not built — \
              once per generation, and the same work a whole gate asks for",
    ],
    permission: Permission::Plain,
};

pub(crate) static DRY: command::Command = command::Command {
    id: "sweep.dry-run",
    call: "sweep --dry-run",
    purpose: "say what a sweep would take away, and take nothing",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

/// What the tail of a gate or a landing runs in the container, against
/// the volume's own rustc, generation key and stamp ([`asks`]).
pub(crate) static IF_MOVED: command::Command = command::Command {
    id: "sweep.if-the-generation-moved",
    call: "sweep --if-the-generation-moved",
    purpose: "sweep this build directory, and only if its own generation key has moved since \
              its own last sweep",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

/// [`IF_MOVED`]'s option, spelled once. The command's own call is held
/// to carrying it (`run::tests`).
const ONLY_IF_MOVED: &str = "--if-the-generation-moved";

pub(crate) static COMMANDS: &[&command::Command] = &[&SWEEP, &DRY, &IF_MOVED];
