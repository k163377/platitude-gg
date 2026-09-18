//! What a report riding an answer leaves the properties QML reads
//! saying — the kind the page picks its two lines from, and the names
//! the first of them is written from (`drain::settle_write`).

use super::*;
use platitude_core::OperationKind as K;

/// The far side keeping a branch is a report — the page reads these
/// four and says so in its own words
/// (`Words.writeReported`).
#[test]
fn a_refusal_the_far_side_made_arrives_as_something_to_report() {
    let mut tab = RepoTab::default();
    tab.settle_write(
        1,
        K::Push,
        "`git push` exited with code 1: remote: error: Cannot delete a protected branch".into(),
        Some(platitude_core::WriteReport::on_remote(
            ReportKind::RemoteDelete,
            "origin",
            "main",
            "Cannot delete a protected branch".into(),
        )),
        0,
        0,
    );
    assert!(tab.write_refused, "nothing happened over there");
    assert_eq!(tab.write_report_kind, "delete");
    assert_eq!(tab.write_report_remote, "origin");
    assert_eq!(tab.write_report_name, "main");
    assert_eq!(tab.write_report_reason, "Cannot delete a protected branch");

    // And it goes with its answer: a report left standing would come
    // back up under the next write.
    tab.settle_write(1, K::Push, String::new(), None, 0, 0);
    assert_eq!(tab.write_report_kind, "");
    assert_eq!(tab.write_report_reason, "");
}

/// A push that was sending says so, since that is the whole of what
/// the sentence turns on.
#[test]
fn a_refused_send_is_told_apart_from_a_refused_delete() {
    let mut tab = RepoTab::default();
    tab.settle_write(
        1,
        K::Push,
        "! [remote rejected]".into(),
        Some(platitude_core::WriteReport::on_remote(
            ReportKind::RemoteUpdate,
            "origin",
            "main",
            "Changes must be made through a pull request.".into(),
        )),
        0,
        0,
    );
    assert_eq!(tab.write_report_kind, "update");
}

/// The other two reports reach the same four properties: nobody over
/// there said anything about either, and the page still has to tell them
/// apart to pick its sentence.
#[test]
fn a_stale_push_and_a_refused_commit_name_themselves_too() {
    let mut tab = RepoTab::default();
    tab.settle_write(
        1,
        K::Push,
        "! [rejected] (fetch first)".into(),
        Some(platitude_core::WriteReport::on_remote(
            ReportKind::Outdated,
            "origin",
            "main",
            "Updates were rejected because the remote contains work that you do not have.".into(),
        )),
        0,
        0,
    );
    assert_eq!(tab.write_report_kind, "outdated");
    assert_eq!(tab.write_report_remote, "origin");
    assert_eq!(tab.write_report_name, "main");

    tab.settle_write(
        1,
        K::Commit,
        "`git commit` exited with code 1".into(),
        Some(platitude_core::WriteReport::local(
            ReportKind::Commit,
            "lint found 1 problem".into(),
        )),
        0,
        0,
    );
    assert_eq!(tab.write_report_kind, "commit");
    assert_eq!(
        (
            tab.write_report_remote.as_str(),
            tab.write_report_name.as_str()
        ),
        ("", ""),
        "nothing over a network and no ref"
    );
    assert_eq!(tab.write_report_reason, "lint found 1 problem");
}
