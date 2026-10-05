//! `cargo xtask package` — a snapshot of the app as a macOS bundle
//! (P5-確認事項 §8): the shipped build and its helper in `Platitude GG.app`,
//! of the Qt they were built against only what [`contents`] names, signed ad
//! hoc, started once offscreen from the bundle alone, and zipped — or put in
//! a DMG only a password opens, the form a run of the public repository
//! keeps.
//!
//! A snapshot, not a release: none of the notices the bundled licences ask
//! for travels with it (P5-確認事項 §3), no Apple identity signs it, and its
//! identifier is not the release's ([`bundle::snapshot_id`]).

mod bundle;
mod contents;
mod run;
mod stand;
mod wrap;

pub(crate) use run::run;

use crate::command::{self, Permission, Where};

pub(crate) static PACKAGE: command::Command = command::Command {
    id: "package.snapshot",
    call: "package",
    purpose: "a snapshot of the app as a macOS bundle: built, its Qt deployed inside, signed ad \
              hoc, started once offscreen from the bundle alone, and zipped or put in an \
              encrypted DMG",
    run_in: Where::Seat,
    needs: &["a Mac, with the Qt the tree pins and Xcode's command line tools"],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&PACKAGE];
