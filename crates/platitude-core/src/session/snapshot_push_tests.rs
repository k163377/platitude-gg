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
        upstream_here: false,
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
            lease: Some(oid(b'a')),
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

/// The branch HEAD is on is the toolbar's to push, off the status tick: the
/// snapshot keeps none of its marks or counts, so it answers nothing.
#[test]
fn the_branch_head_is_on_answers_nothing_here() {
    let at = snapshot(
        vec![BranchItem {
            is_head: true,
            ..tracking("main", "origin/main", 2, 0)
        }],
        &[("origin/main", b'a')],
    );
    assert_eq!(at.branch_push("main", "origin"), BranchPushFacts::default());
}

/// `short` measured against the branch here called `upstream`, `ahead` of
/// it; listed where `listed`, else gone (git's `[gone]`).
fn against_here(short: &str, upstream: &str, ahead: u32, listed: bool) -> BranchItem {
    BranchItem {
        upstream_here: true,
        upstream_oid: listed.then(|| oid(b'1')),
        upstream_gone: if listed { "" } else { upstream }.into(),
        ahead,
        ..branch(short, b'2')
    }
}

/// `remote = .` marked to no remote: nowhere to send it, so the row is the
/// toolbar's `StaysHere` and nothing else — a first push's answer would
/// replace that upstream with a remote one. Gone, it reads as git's
/// `[gone]` anywhere, and the question opens on no remote: the gone name
/// is a branch here's, whatever it looks like.
#[test]
fn a_branch_measured_against_one_here_sends_only_once_that_one_is_gone() {
    let at = snapshot(
        vec![
            branch("main", b'1'),
            against_here("topic", "main", 1, true),
            against_here("orphan", "origin/deleted", 0, false),
        ],
        &[("origin/deleted", b'd')],
    );
    assert_eq!(
        at.branch_push("topic", "origin"),
        BranchPushFacts {
            standing: PushStanding::StaysHere,
            ..BranchPushFacts::default()
        }
    );
    let orphan = at.branch_push("orphan", "origin");
    assert_eq!(
        (orphan.standing, orphan.upstream.as_str(), orphan.lease),
        (PushStanding::Publish, "", None),
        "not origin's `deleted`, which only shares the name"
    );
}

/// A mark sends a branch measured against one here to a remote all the
/// same — the push names the branch there and leaves the upstream be
/// (`remote::plan_push`) —, so its row is the toolbar's form, not out.
#[test]
fn a_mark_sends_a_branch_measured_against_one_here_where_it_says() {
    let mut at = snapshot(
        vec![branch("main", b'1'), against_here("topic", "main", 1, true)],
        &[],
    );
    at.branch_push_remotes = vec![marked("topic", "origin")];
    let own = at.branch_push("topic", "origin");
    assert_eq!(
        (own.standing, own.target.as_str()),
        (PushStanding::Elsewhere, "origin/topic"),
        "the branch's own mark"
    );

    at.branch_push_remotes.clear();
    at.push_default = Some(PushDefault {
        remote: "fork".to_string(),
        local: true,
    });
    let repository = at.branch_push("topic", "origin");
    assert_eq!(
        (repository.standing, repository.target.as_str()),
        (PushStanding::Elsewhere, "fork/topic"),
        "the repository's mark"
    );
}

/// `topic` measured against the branch here called `origin/base`, two
/// commits ahead of it, its own mark on origin: the destination is
/// origin/topic, its counts and tip are what speak, and origin's own
/// `base` — the same short name — plays no part.
#[test]
fn a_branch_here_named_like_a_remote_branch_is_still_a_branch_here() {
    let mut at = snapshot(
        vec![
            branch("origin/base", b'1'),
            against_here("topic", "origin/base", 2, true),
        ],
        &[("origin/base", b'b'), ("origin/topic", b'f')],
    );
    at.branch_push_remotes = vec![marked("topic", "origin")];
    at.push_tracks = vec![(
        "topic".to_string(),
        PushTrack {
            tracking: "origin/topic".to_string(),
            ahead: 0,
            behind: 0,
        },
    )];
    assert_eq!(
        at.branch_push("topic", "origin"),
        BranchPushFacts {
            standing: PushStanding::Clean,
            target: "origin/topic".to_string(),
            lease: Some(oid(b'f')),
            upstream: String::new(),
        },
        "level with origin/topic: nothing to send, however far ahead of the branch here"
    );

    at.branch_push_remotes.clear();
    assert_eq!(
        at.branch_push("topic", "origin").standing,
        PushStanding::StaysHere,
        "unmarked, it does not go to origin for its upstream's name"
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
            lease: Some(oid(b'e')),
            upstream: "origin/counted".to_string(),
        },
        "ready against the upstream, diverged where the push goes, leased to the tip there"
    );
    let unfetched = at.branch_push("unfetched", "origin");
    assert_eq!(unfetched.standing, PushStanding::Elsewhere);
    assert_eq!(
        (unfetched.target.as_str(), unfetched.lease),
        ("fork/unfetched", None)
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

/// Counts read before a mark moved — repeated by a read that could not
/// tell — are about where the branch no longer goes: nothing speaks.
#[test]
fn counts_against_where_a_mark_used_to_send_the_branch_speak_for_nothing() {
    let mut at = snapshot(
        vec![tracking("counted", "origin/counted", 1, 0)],
        &[("origin/counted", b'a'), ("fork/counted", b'e')],
    );
    at.remote_names.push("mirror".to_string());
    at.branch_push_remotes = vec![marked("counted", "mirror")];
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
            standing: PushStanding::Elsewhere,
            target: "mirror/counted".to_string(),
            lease: None,
            upstream: "origin/counted".to_string(),
        },
        "not an overwrite of mirror leased to fork's tip"
    );
}
