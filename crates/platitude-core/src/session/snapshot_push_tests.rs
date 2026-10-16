//! What a menu's `push` row on a branch the worktree is not on stands on
//! (`RefsSnapshot::branch_push`): the toolbar's table, read off the row,
//! the branch's own mark and the counts of where a mark sends it.

use super::{BranchItem, BranchPushFacts, RefsSnapshot};
use crate::oid::Oid;
use crate::remote::{BranchPushRemote, PushDefault, PushStanding, PushTrack};

fn oid(byte: u8) -> Oid {
    Oid::from_hex(&[byte; 40]).expect("an oid")
}

fn branch(short: &str, at: u8) -> BranchItem {
    BranchItem {
        short: short.into(),
        full: format!("refs/heads/{short}").into(),
        oid: oid(at),
        has_remote: false,
        is_head: false,
        upstream: "".into(),
        upstream_gone: "".into(),
        upstream_oid: None,
        upstream_drifted: false,
        held_elsewhere: false,
        tracked_by: "".into(),
        ahead: 0,
        behind: 0,
    }
}

/// `short` tracking `upstream`, which is here, `ahead` / `behind` it.
fn tracking(short: &str, upstream: &str, ahead: u32, behind: u32) -> BranchItem {
    BranchItem {
        upstream: upstream.into(),
        ahead,
        behind,
        ..branch(short, b'1')
    }
}

fn remote_branch(short: &str, at: u8) -> BranchItem {
    BranchItem {
        full: format!("refs/remotes/{short}").into(),
        ..branch(short, at)
    }
}

/// A repository with `origin` and `fork`, the remote branches named, and
/// `locals` (sorted here, as the listing sorts them).
fn snapshot(mut locals: Vec<BranchItem>, remotes: &[(&str, u8)]) -> RefsSnapshot {
    locals.sort_by(|a, b| a.short.cmp(&b.short));
    let mut remotes: Vec<BranchItem> = remotes
        .iter()
        .map(|(short, at)| remote_branch(short, *at))
        .collect();
    remotes.sort_by(|a, b| a.short.cmp(&b.short));
    RefsSnapshot {
        locals,
        remotes,
        remote_names: vec!["fork".to_string(), "origin".to_string()],
        ..RefsSnapshot::default()
    }
}

fn marked(branch: &str, remote: &str) -> BranchPushRemote {
    BranchPushRemote {
        branch: branch.to_string(),
        remote: remote.to_string(),
    }
}

#[test]
fn a_tracked_branch_stands_against_its_upstream_as_the_toolbar_would() {
    let at = snapshot(
        vec![
            tracking("ahead", "origin/ahead", 2, 0),
            tracking("behind", "origin/behind", 0, 3),
            tracking("level", "origin/level", 0, 0),
            tracking("split", "origin/trunk", 1, 1),
        ],
        &[
            ("origin/ahead", b'a'),
            ("origin/behind", b'b'),
            ("origin/level", b'c'),
            ("origin/trunk", b'd'),
        ],
    );
    assert_eq!(
        at.branch_push("ahead", "origin"),
        BranchPushFacts {
            standing: PushStanding::Ready,
            target: "origin/ahead".to_string(),
            measured: "origin/ahead".to_string(),
            lease: Some(oid(b'a')),
            ahead: 2,
            behind: 0,
            upstream: "origin/ahead".to_string(),
        }
    );
    assert_eq!(
        at.branch_push("behind", "origin").standing,
        PushStanding::Behind
    );
    assert_eq!(
        at.branch_push("level", "origin").standing,
        PushStanding::Clean
    );
    let split = at.branch_push("split", "origin");
    assert_eq!(split.standing, PushStanding::Diverged);
    assert_eq!(
        (split.target.as_str(), split.lease),
        ("origin/trunk", Some(oid(b'd'))),
        "the upstream names the branch over there, and the overwrite is leased to its tip"
    );
}

#[test]
fn a_branch_with_nothing_to_count_against_asks_where_it_goes() {
    let at = snapshot(
        vec![
            branch("fresh", b'1'),
            BranchItem {
                upstream_gone: "origin/gone".into(),
                ..branch("pruned", b'2')
            },
        ],
        &[],
    );
    let fresh = at.branch_push("fresh", "origin");
    assert_eq!(fresh.standing, PushStanding::Publish);
    assert_eq!((fresh.upstream.as_str(), fresh.lease), ("", None));
    let pruned = at.branch_push("pruned", "origin");
    assert_eq!(pruned.standing, PushStanding::Publish);
    assert_eq!(
        pruned.upstream, "origin/gone",
        "the question opens where the branch already points"
    );
    assert_eq!(
        at.branch_push("nowhere", "origin"),
        BranchPushFacts::default(),
        "no such branch, nothing to send"
    );
}

/// `remote = .`: the upstream is a branch here, which a first push's
/// answer would replace with a remote one.
#[test]
fn a_branch_measured_against_one_here_has_nothing_to_send() {
    let at = snapshot(
        vec![
            branch("main", b'1'),
            BranchItem {
                upstream_oid: Some(oid(b'1')),
                ahead: 1,
                ..branch("topic", b'2')
            },
        ],
        &[],
    );
    assert_eq!(
        at.branch_push("topic", "origin").standing,
        PushStanding::Closed
    );
}

/// The branch's own mark, then the repository's: the counts that speak are
/// the destination's, and with no tracking ref there no count speaks.
#[test]
fn a_mark_sends_the_branch_elsewhere_and_its_counts_go_with_it() {
    let mut at = snapshot(
        vec![
            tracking("counted", "origin/counted", 1, 0),
            tracking("unfetched", "origin/unfetched", 1, 0),
            tracking("home", "origin/home", 1, 0),
        ],
        &[
            ("origin/counted", b'a'),
            ("origin/unfetched", b'b'),
            ("origin/home", b'c'),
            ("fork/counted", b'e'),
        ],
    );
    at.branch_push_remotes = vec![
        marked("counted", "fork"),
        marked("home", "origin"),
        marked("unfetched", "fork"),
    ];
    at.push_tracks = vec![(
        "counted".to_string(),
        PushTrack {
            tracking: "fork/counted".to_string(),
            ahead: 1,
            behind: 2,
        },
    )];
    assert_eq!(
        at.branch_push("counted", "origin"),
        BranchPushFacts {
            standing: PushStanding::Diverged,
            target: "fork/counted".to_string(),
            measured: "fork/counted".to_string(),
            lease: Some(oid(b'e')),
            ahead: 1,
            behind: 2,
            upstream: "origin/counted".to_string(),
        },
        "ready against the upstream, diverged where the push goes"
    );
    let unfetched = at.branch_push("unfetched", "origin");
    assert_eq!(unfetched.standing, PushStanding::Elsewhere);
    assert_eq!(
        (
            unfetched.target.as_str(),
            unfetched.measured.as_str(),
            unfetched.lease
        ),
        ("fork/unfetched", "", None)
    );
    assert_eq!(
        at.branch_push("home", "origin").standing,
        PushStanding::Ready,
        "a mark on the upstream's own remote leaves the upstream's counts speaking"
    );

    at.branch_push_remotes.clear();
    at.push_default = Some(PushDefault {
        remote: "fork".to_string(),
        local: true,
    });
    assert_eq!(
        at.branch_push("home", "origin").standing,
        PushStanding::Elsewhere,
        "the repository's mark sends every branch there"
    );
}
