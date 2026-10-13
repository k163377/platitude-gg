//! The witnesses the table of relied messages names (`src/relied_messages.rs`,
//! its `- witness:` lines), held to this binary's own list of tests by full
//! name — the names `cargo xtask relied-messages master` hands `--exact`,
//! which selects nothing for a name that is not one of them and passes.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::process::Command;

/// Every witness is a test here, its full name exactly — inner modules
/// (`periodic`) included, and the `#[ignore]`d ones too, which `--list`
/// names.
#[test]
fn every_witness_the_table_names_is_a_test_of_this_binary() {
    let table = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/relied_messages.rs"),
    )
    .expect("the table");
    let witnesses: BTreeSet<&str> = table
        .lines()
        .filter_map(|line| line.trim().strip_prefix("/// - witness: `"))
        .filter_map(|rest| rest.split_once('`'))
        .map(|(witness, _)| witness)
        .collect();
    assert!(!witnesses.is_empty(), "the table names witnesses");

    let listed = Command::new(std::env::current_exe().expect("this binary"))
        .arg("--list")
        .output()
        .expect("the binary lists its tests");
    assert!(listed.status.success(), "--list answered");
    let stdout = String::from_utf8_lossy(&listed.stdout);
    let tests: BTreeSet<&str> = stdout
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .collect();

    let missing: Vec<&&str> = witnesses.difference(&tests).collect();
    assert!(
        missing.is_empty(),
        "witnesses that are no test of this binary by that full name: {missing:?}"
    );
}
