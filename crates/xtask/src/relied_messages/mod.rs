//! `cargo xtask relied-messages` — git's messages the product relies on
//! (`platitude-core/src/relied_messages.rs`), held to git's own sources.
//!
//! The table writes each message as the forms that say it, and names for
//! each form the file of git's tree it is written in and words on that
//! line. The ledger (`crates/xtask/relied-messages.txt`)
//! holds every such line of every tree read — each release of git from the
//! minimum on and `master`, of Git for Windows and its `main`, Apple Git,
//! the minimum container's Ubuntu, and OpenSSH for what git passes on of
//! it — grouped where the lines are the same, so a change between two
//! releases is one entry read off it (gitの扱い.md §git の文言に依存する判定).
//!
//! - `check` holds the table to the ledger: a form added or changed is
//!   checked against the releases before the gate takes it. Offline.
//! - `audit` fetches every tree and writes the ledger again.
//! - `watch` is the weekly run: the heads, and any release the ledger has
//!   not read, against the ledger — red where a line the product reads
//!   changed, naming the releases that carry the change. Every maintenance
//!   release is in `master`'s history, so a change that reaches any of
//!   them reaches `master` too.
//! - `master` builds `master`'s git and runs each form's witnesses under
//!   it: a change of the path to the words, which no line shows, is caught
//!   where a test has git walk it.

mod ledger;
mod master;
mod sources;
mod table;

use std::path::Path;

use crate::command::{self, Permission, Where};

pub(crate) static CHECK: command::Command = command::Command {
    id: "relied-messages.check",
    call: "relied-messages check",
    purpose: "the table of git's words agrees with the ledger the audit wrote",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static AUDIT: command::Command = command::Command {
    id: "relied-messages.audit",
    call: "relied-messages audit",
    purpose: "read every release of git, Git for Windows, Apple Git, the container's Ubuntu and \
              OpenSSH, and write the ledger of the lines the table names",
    run_in: Where::Seat,
    needs: &["the network, through git"],
    permission: Permission::Plain,
};

pub(crate) static WATCH: command::Command = command::Command {
    id: "relied-messages.watch",
    call: "relied-messages watch",
    purpose: "the weekly run: the heads and any new release against the ledger, red only where \
              a line the table names changed",
    run_in: Where::Either,
    needs: &["the network, through git"],
    permission: Permission::Plain,
};

pub(crate) static MASTER: command::Command = command::Command {
    id: "relied-messages.master",
    call: "relied-messages master",
    purpose: "build master's git and run the table's witnesses under it",
    run_in: Where::Either,
    needs: &[
        "Linux with a C compiler, make and zlib's headers (`cargo xtask linux relied-messages master` \
         elsewhere)",
        "the network, through git",
    ],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&CHECK, &AUDIT, &WATCH, &MASTER];

/// The ledger, beside the other generated files the gate reads.
pub(crate) const LEDGER: &str = "crates/xtask/relied-messages.txt";

pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let root = crate::tree::workspace_root();
    match args.first().map(String::as_str) {
        Some("check") => check(&root),
        Some("audit") => audit(&root),
        Some("watch") => watch(&root),
        Some("master") => master::run(&root),
        _ => Err("relied-messages needs check, audit, watch or master".to_string()),
    }
}

/// The table against the ledger. Whether each witness is a test of the
/// suite by its full name is the suite's own test to say
/// (`relied_witnesses`), off the list its binary gives.
fn check(root: &Path) -> Result<(), String> {
    let forms = table::read(root)?;
    let ledger = ledger::Ledger::read(root)?;
    let differing = ledger.differs_from(&forms);
    if !differing.is_empty() {
        return Err(format!(
            "the table and {LEDGER} differ over {}: run `{}` and commit what it writes",
            differing.join(", "),
            AUDIT.line()
        ));
    }
    println!(
        "relied-messages: {} forms, as the ledger read them ({})",
        forms.len(),
        ledger.read_summary()
    );
    Ok(())
}

fn audit(root: &Path) -> Result<(), String> {
    let forms = table::read(root)?;
    let cache = sources::Cache::open()?;
    let lines = sources::Lines::of_this_tree(root)?;
    cache.fetch_all(&lines)?;
    let trees = cache.trees(&lines)?;
    let read = sources::read_all(&cache, &trees, &forms)?;
    let ledger = ledger::Ledger::of(&forms, &trees, &read, &cache)?;
    ledger.unwritten_at_the_heads()?;
    std::fs::write(root.join(LEDGER), ledger.render()).map_err(|e| format!("{LEDGER}: {e}"))?;
    println!("{}", ledger.changes_summary());
    println!("wrote {LEDGER}: {}", ledger.read_summary());
    Ok(())
}

fn watch(root: &Path) -> Result<(), String> {
    let forms = table::read(root)?;
    let ledger = ledger::Ledger::read(root)?;
    let differing = ledger.differs_from(&forms);
    if !differing.is_empty() {
        return Err(format!(
            "the table and {LEDGER} differ over {} — the gate holds them together, so this \
             commit did not pass it",
            differing.join(", ")
        ));
    }
    let cache = sources::Cache::open()?;
    let lines = sources::Lines::of_this_tree(root)?;
    let heads = cache.fetch_heads(&lines, &ledger)?;
    let new_releases = cache.fetch_unread_releases(&lines, &ledger)?;
    let mut looked_at = heads.clone();
    looked_at.extend(new_releases.iter().cloned());
    let read = sources::read_all(&cache, &looked_at, &forms)?;
    let changes = ledger.changes_in(&forms, &heads, &new_releases, &read);
    if changes.is_empty() {
        println!(
            "relied-messages: no line the table names changed — {} heads, {} releases the ledger had \
             not read",
            heads.len(),
            new_releases.len()
        );
        return Ok(());
    }
    // Which trees hold each change's lines: a full read of every release,
    // which only a change is worth.
    cache.fetch_all(&lines)?;
    let every = cache.trees(&lines)?;
    let read_every = sources::read_all(&cache, &every, &forms)?;
    for change in &changes {
        println!("{}", change.report(&every, &read_every, &cache));
    }
    let mut changed: Vec<&str> = changes.iter().map(|change| change.form()).collect();
    changed.dedup();
    Err(format!(
        "git's sources changed the lines of {}: read the lines above, put the classifier right, \
         and run `{}`",
        changed.join(", "),
        AUDIT.line()
    ))
}
