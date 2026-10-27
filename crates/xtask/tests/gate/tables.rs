//! What the two tables' own moves owe, and what a line's key is held to:
//! a row added or retiered runs where the gate owes it now, a row whose
//! promise moved runs again, and a comment moved in what a run reads
//! keeps its green (反映前テストの機械化.md §動詞の絞り込みは比較表示に留める /
//! §キャッシュとスタンプ).

use crate::support::{Sandbox, tiered, without_always};

/// A line the host alone owed, moved to `fonts`: the container's run is
/// owed now and the host's green stands.
#[test]
fn a_line_moved_to_fonts_owes_its_container_run_and_keeps_the_hosts_green() {
    let sb = tiered("to-fonts");
    sb.write(
        &sb.seat,
        "crates/xtask/verb-tiers.txt",
        "fonts\tstash --preset basic\t-\tboth sides\n\
         fonts\tstash-open --preset basic\t-\tboth sides now\n\
         whole\tstash-menu --preset basic\tstash --preset basic\tpicture: the menu standing\n",
    );
    sb.commit_all(&sb.seat, "chore: fonts decide it", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(
        text.contains("cached [host ] verify stash-open --preset basic")
            && text.contains("run    [linux] verify-linux stash-open --preset basic"),
        "{text}"
    );
    assert!(
        !text.contains("verify stash --preset basic"),
        "a row that stood: {text}"
    );
    sb.gate_ok(&sb.seat, &[]);
    assert_eq!(
        without_always(&sb.ran()),
        crate::support::set(&["verify-linux stash-open --preset basic"])
    );
}

/// A whole line made a pre-merge one runs before a merge from then on —
/// now, since no stamp answers for it.
#[test]
fn a_whole_line_made_a_pre_merge_one_runs_now() {
    let sb = tiered("promoted");
    sb.write(
        &sb.seat,
        "crates/xtask/verb-tiers.txt",
        "fonts\tstash --preset basic\t-\tboth sides\n",
    );
    sb.commit_all(&sb.seat, "chore: the menu before a merge", &[]);
    sb.gate_ok(&sb.seat, &[]);
    assert_eq!(
        without_always(&sb.ran()),
        crate::support::set(&["verify stash-menu --preset basic"])
    );
}

/// A line retired to a twin — its row gone, a twin row in its place — runs
/// no more, and nothing else runs for the tables' move.
#[test]
fn a_line_retired_to_a_twin_runs_no_more() {
    let sb = tiered("twinned");
    sb.write(
        &sb.seat,
        "crates/xtask/verb-lines.txt",
        "*\tStashPane\nstash --preset basic\tPeek\nstash-menu --preset basic\t-\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/verb-tiers.txt",
        "fonts\tstash --preset basic\t-\tboth sides\n\
         whole\tstash-menu --preset basic\tstash --preset basic\tpicture: the menu standing\n\
         twin\tstash-open --preset basic\tstash --preset basic\tthe same run\n",
    );
    sb.commit_all(&sb.seat, "chore: open is the same run as stash", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(!text.contains("verify stash-open"), "{text}");
    assert!(!text.contains("  run    "), "{text}");
}

/// A row whose promise moved is a new contract: the line runs again, and
/// the rows that stood do not.
#[test]
fn a_rows_promise_moved_runs_that_line_again_and_no_other() {
    let sb = tiered("promise-moved");
    sb.write(
        &sb.seat,
        "crates/xtask/verb-lines.txt",
        "*\tStashPane\nstash --preset basic\tPeek\n\
         stash-menu --preset basic\t-\n\
         stash-open --preset basic\tPeek\n",
    );
    sb.commit_all(&sb.seat, "chore: open promises the popup too", &[]);
    // The tables' move owes the row that moved; the key of its step moved
    // with its contract, so the green it had answers for nothing.
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(
        text.contains("run    [host ] verify stash-open --preset basic"),
        "{text}"
    );
    assert!(!text.contains("verify stash --preset basic"), "{text}");
}

/// A comment moved in what a run reads moves no key: the harness's
/// greens stand, though every line is owed again.
#[test]
fn a_comment_moved_in_the_harness_keeps_every_lines_green() {
    let sb = Sandbox::new("harness-comment");
    sb.write_pane(&sb.seat, 2);
    sb.commit_all(&sb.seat, "feat(app-ui): pane", &[]);
    sb.gate_ok(&sb.seat, &[]);
    assert!(sb.ran().contains("verify stash --preset basic"));
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/auto/Driver.qml",
        "// The harness's one driver, said in a comment.\nItem {}\n",
    );
    sb.commit_all(&sb.seat, "docs(app): a note on the driver", &[]);
    let text = sb.gate_ok(&sb.seat, &[]);
    assert!(
        text.contains("cached verify stash --preset basic") && text.contains("cached shipped"),
        "{text}"
    );
    assert!(!sb.ran().contains("verify stash --preset basic"));
    // The same move with code in it runs the line.
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/auto/Driver.qml",
        "// The harness's one driver, said in a comment.\nItem { width: 1 }\n",
    );
    sb.commit_all(&sb.seat, "feat(app): the driver widens", &[]);
    sb.gate_ok(&sb.seat, &[]);
    assert!(sb.ran().contains("verify stash --preset basic"));
}

/// A fixture moved runs every line: the presets are what a run stands on.
#[test]
fn a_fixture_moved_runs_every_line() {
    let sb = Sandbox::new("fixture-moved");
    sb.write_pane(&sb.seat, 2);
    sb.commit_all(&sb.seat, "feat(app-ui): pane", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let _ = sb.ran();
    sb.write(
        &sb.seat,
        "crates/xtask/src/demo/mod.rs",
        "pub fn build() { let _ = 2; }\n",
    );
    sb.commit_all(&sb.seat, "feat(xtask): the preset grows a file", &[]);
    sb.gate_ok(&sb.seat, &[]);
    assert!(
        sb.ran().contains("verify stash --preset basic"),
        "{:?}",
        sb.ran()
    );
}
