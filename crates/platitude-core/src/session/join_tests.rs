//! The refs join: what the sidebar and the graph chips are built from,
//! and what one join costs at scale.

#![expect(
    clippy::print_stdout,
    reason = "the scale measurement reports its number to whoever ran it"
)]

use super::joins::{RefJoins, WorktreeHolders, build_label_map, build_snapshot};
use super::*;
use crate::remote::RemoteTag;

/// The everyday repository: one working copy, so no branch is held
/// anywhere else. The join that reads this has a test of its own below.
fn held_by_nobody() -> WorktreeHolders {
    WorktreeHolders::default()
}

fn tag(name: &str, commit: Oid, annotated: bool) -> RefEntry {
    RefEntry {
        name: crate::Name::from(format!("refs/tags/{name}")),
        short: crate::Name::from(name),
        kind: RefKind::Tag,
        target: commit,
        peeled: annotated.then_some(commit),
        upstream: None,
        is_head: false,
        created_unix: 0,
        ahead: 0,
        behind: 0,
    }
}

fn branch(name: &str, commit: Oid) -> RefEntry {
    RefEntry {
        name: crate::Name::from(format!("refs/heads/{name}")),
        short: crate::Name::from(name),
        kind: RefKind::LocalBranch,
        target: commit,
        peeled: None,
        upstream: None,
        is_head: false,
        created_unix: 0,
        ahead: 0,
        behind: 0,
    }
}

fn oid(byte: u8) -> Oid {
    Oid::from_hex_str(&format!("{byte:02x}").repeat(20)).expect("valid sha")
}

fn head_at(commit: Oid) -> HeadState {
    HeadState {
        branch: Some("main".to_string()),
        oid: Some(commit),
        detached: false,
    }
}

fn index_of(remote: &str, tags: Vec<RemoteTag>) -> RemoteTagIndex {
    RemoteTagIndex::build(
        tags.into_iter()
            .map(|t| (t.name, t.commit, t.annotated, crate::Name::from(remote))),
    )
}

/// The row carries whether the reading it speaks for is standing where
/// the branch is. A pair that has drifted is two rows on the graph rather
/// than one folded chip, and the menu's rows that reach the remote read
/// this to say why instead of running (デザイン規約 §左メニューの所作
/// の削除の表).
#[test]
fn a_branch_says_whether_its_reading_stands_on_the_same_commit() {
    let tracking = |commit: Oid| RefEntry {
        name: crate::Name::from("refs/remotes/origin/main"),
        short: crate::Name::from("origin/main"),
        kind: RefKind::RemoteBranch,
        target: commit,
        peeled: None,
        upstream: None,
        is_head: false,
        created_unix: 0,
        ahead: 0,
        behind: 0,
    };
    let mut main = branch("main", oid(1));
    main.upstream = Some(crate::Name::from("refs/remotes/origin/main"));
    let remote_tags = index_of("origin", Vec::new());
    let held = held_by_nobody();

    let agreed = vec![main.clone(), tracking(oid(1))];
    let joins = RefJoins::new(&agreed, &held);
    let snapshot = build_snapshot(&agreed, &head_at(oid(1)), &remote_tags, &joins);
    assert_eq!(snapshot.locals[0].upstream, "origin/main");
    assert!(!snapshot.locals[0].upstream_drifted);

    let apart = vec![main, tracking(oid(2))];
    let joins = RefJoins::new(&apart, &held);
    let snapshot = build_snapshot(&apart, &head_at(oid(1)), &remote_tags, &joins);
    assert_eq!(
        snapshot.locals[0].upstream, "origin/main",
        "the reading it speaks for is the same one wherever it stands"
    );
    assert!(snapshot.locals[0].upstream_drifted);
}

/// The graph's cloud follows the fold: it is on the chip exactly while the
/// remote it is about is folded into that chip (デザイン規約 §ref の種別).
/// Drifted, the remote keeps a row of its own and neither row wears one —
/// the two rows are the whole of the signal. The sidebar keeps the wider
/// reading of the drift: the branch does have a remote, wherever it is.
/// What neither of them reads is a name — an upstream is the only thing
/// that makes a remote branch this branch's.
#[test]
fn the_graph_cloud_on_a_branch_follows_the_fold() {
    let tracking = |remote: &str, commit: Oid| RefEntry {
        name: crate::Name::from(format!("refs/remotes/{remote}/main")),
        short: crate::Name::from(format!("{remote}/main")),
        kind: RefKind::RemoteBranch,
        target: commit,
        peeled: None,
        upstream: None,
        is_head: false,
        created_unix: 0,
        ahead: 0,
        behind: 0,
    };
    let mut main = branch("main", oid(1));
    main.upstream = Some(crate::Name::from("refs/remotes/origin/main"));
    let remote_tags = index_of("origin", Vec::new());
    let held = held_by_nobody();

    let agreed = vec![main.clone(), tracking("origin", oid(1))];
    let joins = RefJoins::new(&agreed, &held);
    let map = build_label_map(&agreed, &head_at(oid(1)), &remote_tags, &joins);
    let folded = map.labels_of(&oid(1), true);
    assert_eq!(folded.len(), 1, "one chip between the two: {folded:?}");
    assert!(folded[0].has_remote, "the cloud stands for what it folded");

    let apart = vec![main.clone(), tracking("origin", oid(2))];
    let joins = RefJoins::new(&apart, &held);
    let map = build_label_map(&apart, &head_at(oid(1)), &remote_tags, &joins);
    let ours = map.labels_of(&oid(1), true);
    assert!(!ours[0].has_remote, "the remote is not on this row");
    let theirs = map.labels_of(&oid(2), true);
    assert_eq!(theirs[0].text, "origin/main", "the unfolded remote's chip");
    assert!(!theirs[0].has_remote);
    let snapshot = build_snapshot(&apart, &head_at(oid(1)), &remote_tags, &joins);
    assert!(
        snapshot.locals[0].has_remote,
        "the sidebar answers the wider question: the name is out there"
    );

    // No upstream: a remote of the same name, on the very same commit, is
    // still a different branch. Nothing folds, nothing is badged, and the
    // remote says who it is with a chip of its own.
    let mut loose = branch("main", oid(1));
    loose.upstream = None;
    let untracked = vec![loose, tracking("origin", oid(1))];
    let joins = RefJoins::new(&untracked, &held);
    let map = build_label_map(&untracked, &head_at(oid(1)), &remote_tags, &joins);
    let chips = map.labels_of(&oid(1), true);
    assert_eq!(chips.len(), 2, "two branches, two chips: {chips:?}");
    assert!(
        chips.iter().all(|c| !c.has_remote),
        "no upstream, so nothing here has a remote: {chips:?}"
    );
    let snapshot = build_snapshot(&untracked, &head_at(oid(1)), &remote_tags, &joins);
    assert!(
        !snapshot.locals[0].has_remote,
        "and the sidebar says the same — git reports no tracking branch either"
    );
}

/// A branch another working copy has out is marked on **both** halves of
/// the join — the sidebar row and the graph chip read one bit between
/// them, and a chip that offered a move the row refused would be two
/// answers to one question.
#[test]
fn a_branch_another_copy_holds_is_marked_on_the_row_and_the_chip() {
    let refs = vec![branch("main", oid(1)), branch("feature/topic-a", oid(2))];
    let remote_tags = index_of("origin", Vec::new());
    let mut held = WorktreeHolders::default();
    held.branches.insert("feature/topic-a".to_string());
    let joins = RefJoins::new(&refs, &held);

    let snapshot = build_snapshot(&refs, &head_at(oid(1)), &remote_tags, &joins);
    let topic = snapshot
        .locals
        .iter()
        .find(|b| b.short == "feature/topic-a")
        .expect("the branch is listed");
    assert!(topic.held_elsewhere);
    // The copy this session is in never reaches the set, so the branch
    // the reader is standing on is not marked (`note_worktree_holders`).
    let main = snapshot
        .locals
        .iter()
        .find(|b| b.short == "main")
        .expect("the branch is listed");
    assert!(!main.held_elsewhere);

    let map = build_label_map(&refs, &head_at(oid(1)), &remote_tags, &joins);
    assert!(map.labels_of(&oid(2), true)[0].held_elsewhere);
    assert!(!map.labels_of(&oid(1), true)[0].held_elsewhere);
}

/// A working copy standing on no branch is on the graph as a chip of its
/// own — nothing else on the row can say it, since there is no ref there
/// to carry the mark a held branch's chip wears.
#[test]
fn a_copy_standing_on_no_branch_gets_a_chip_of_its_own() {
    let refs = vec![branch("main", oid(1)), tag("v1", oid(2), false)];
    let remote_tags = index_of("origin", Vec::new());
    let mut held = WorktreeHolders::default();
    held.detached.push(super::joins::DetachedCheckout {
        oid: oid(2),
        name: crate::Name::const_new("spike"),
    });
    let joins = RefJoins::new(&refs, &held);
    let map = build_label_map(&refs, &head_at(oid(1)), &remote_tags, &joins);

    let run = map.labels_of(&oid(2), true);
    let copy = run
        .iter()
        .find(|l| l.kind == LabelKind::Worktree)
        .expect("the copy is on the row it is standing on");
    assert_eq!(copy.text.as_str(), "spike");
    assert!(!copy.held_elsewhere, "it names no branch to be holding");
    // The commit a copy is not standing on carries no such chip.
    assert!(
        !map.labels_of(&oid(1), true)
            .iter()
            .any(|l| l.kind == LabelKind::Worktree)
    );
    // **Tags stay the tail of the run**, which is what lets the TAGS eye
    // cut them off with a shorter slice: a chip sorted past them would go
    // out with the tags it was never part of.
    assert!(
        map.labels_of(&oid(2), false)
            .iter()
            .any(|l| l.kind == LabelKind::Worktree),
        "the eye takes the tag and leaves the working copy"
    );
}

/// `set-url` moves no ref, so the URL's only road to the screen is the
/// snapshot — the join key has to move for it, or the held snapshot is
/// resent with the old URL on it.
#[test]
fn a_changed_remote_url_moves_the_join_key() {
    let remotes = |url: &str| crate::remote::Remotes {
        list: vec![crate::remote::Remote {
            name: "origin".to_string(),
            fetch_url: url.to_string(),
            push_url: url.to_string(),
        }],
        push_default: None,
    };
    assert_ne!(
        joins::join_key(1, 0, 0, &remotes("https://old.example/repo")),
        joins::join_key(1, 0, 0, &remotes("https://new.example/repo")),
    );
}

/// A tag both sides agree on is one tag: the local label carries the
/// cloud and the remote's reading adds no second chip.
#[test]
fn an_agreed_tag_gets_one_label() {
    let refs = vec![tag("v1", oid(1), false)];
    let remote_tags = index_of(
        "origin",
        vec![RemoteTag {
            name: crate::Name::const_new("v1"),
            commit: oid(1),
            annotated: false,
        }],
    );
    let nobody = held_by_nobody();
    let joins = RefJoins::new(&refs, &nobody);
    let map = build_label_map(&refs, &head_at(oid(1)), &remote_tags, &joins);
    let labels = map.labels_of(&oid(1), true);
    assert_eq!(labels.len(), 1, "one name, one chip: {labels:?}");
    assert!(labels[0].has_remote, "the cloud says the remote has it");
    assert!(labels[0].here);

    let snapshot = build_snapshot(&refs, &head_at(oid(1)), &remote_tags, &joins);
    assert_eq!(snapshot.tags.len(), 1, "the sidebar lists the name once");
    assert!(snapshot.tags[0].here && snapshot.tags[0].has_remote);
}

/// A tag the remote puts somewhere else stands on both rows, and the
/// one that is not here says whose reading it is.
#[test]
fn a_drifted_tag_stands_on_both_rows() {
    let refs = vec![tag("v1", oid(1), false)];
    let remote_tags = index_of(
        "origin",
        vec![RemoteTag {
            name: crate::Name::const_new("v1"),
            commit: oid(2),
            annotated: false,
        }],
    );
    let nobody = held_by_nobody();
    let joins = RefJoins::new(&refs, &nobody);
    let map = build_label_map(&refs, &head_at(oid(1)), &remote_tags, &joins);
    let ours = map.labels_of(&oid(1), true);
    assert!(ours[0].here);
    assert!(
        !ours[0].has_remote,
        "the two rows are the drift; a cloud here would answer for the wrong one"
    );
    let theirs = map.labels_of(&oid(2), true);
    assert!(!theirs.is_empty(), "the remote's reading");
    assert!(!theirs[0].here);
    assert_eq!(theirs[0].remote, "origin");
    assert!(!theirs[0].has_remote, "and the far half goes bare with it");

    let snapshot = build_snapshot(&refs, &head_at(oid(1)), &remote_tags, &joins);
    assert_eq!(snapshot.tags.len(), 1);
    assert!(snapshot.tags[0].here);
    assert!(
        snapshot.tags[0].has_remote,
        "the sidebar answers the wider question: the name is out there"
    );
}

/// With the tags out of the graph, the chips lose them too — on the rows
/// that stay as well as the rows that go. A tag standing on a commit a
/// branch also reaches is the one the walk cannot take away, and leaving
/// its chip behind is the whole of what the switch would have missed.
#[test]
fn the_chips_lose_their_tags_with_the_graph() {
    let refs = vec![
        branch("main", oid(1)),
        tag("v1", oid(1), false),
        tag("islet", oid(2), false),
    ];
    let nobody = held_by_nobody();
    let joins = RefJoins::new(&refs, &nobody);
    let map = build_label_map(&refs, &head_at(oid(1)), &index_of("origin", vec![]), &joins);

    let both = map.labels_of(&oid(1), true);
    assert_eq!(both.len(), 2, "the branch and the tag: {both:?}");
    let kept = map.labels_of(&oid(1), false);
    assert_eq!(kept.len(), 1, "the tag is cut off the end: {kept:?}");
    assert_eq!(kept[0].kind, LabelKind::LocalBranch);
    assert!(
        map.labels_of(&oid(2), false).is_empty(),
        "a commit nothing but a tag names carries no chip at all"
    );

    let carried: Vec<Oid> = map.commits(false).map(|(commit, _)| commit).collect();
    assert_eq!(
        carried,
        vec![oid(1)],
        "a commit cut down to no chips is not one of the commits carrying them"
    );
}

/// A name only a remote has reaches no graph row, so the sidebar is
/// where it can be read at all.
#[test]
fn a_tag_only_a_remote_has_is_listed_and_marked() {
    let refs = vec![tag("v1", oid(1), false)];
    let remote_tags = index_of(
        "origin",
        vec![RemoteTag {
            name: crate::Name::const_new("v9"),
            commit: oid(9),
            annotated: true,
        }],
    );
    let nobody = held_by_nobody();
    let joins = RefJoins::new(&refs, &nobody);
    let snapshot = build_snapshot(&refs, &head_at(oid(1)), &remote_tags, &joins);
    let v9 = snapshot
        .tags
        .iter()
        .find(|t| t.short == "v9")
        .expect("listed");
    assert!(!v9.here && v9.has_remote && v9.annotated);
    assert_eq!(v9.created_unix, 0, "an advertisement carries no date");
}

/// Ignored: it needs a repository worth measuring. Run it with
/// `PGG_PERF_REPO=<path> cargo test -p platitude-core --release
/// refs_join_at_scale -- --ignored --nocapture`.
///
/// What it times is one `publish_refs` join — the work every poll tick
/// does on top of the two git reads. Recorded in
/// `ci/baseline/refs-join-windows-x64.md`.
#[test]
#[ignore = "needs PGG_PERF_REPO pointed at a large repository"]
fn refs_join_at_scale() {
    let repo = std::env::var("PGG_PERF_REPO").unwrap_or_else(|_| ".".to_string());
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args([
            "for-each-ref",
            refs::REFS_FORMAT_ARG,
            "refs/heads",
            "refs/remotes",
            "refs/tags",
        ])
        .output()
        .expect("git for-each-ref");
    let refs = refs::parse_refs(&out.stdout);
    let head = head_at(refs.first().map(RefEntry::commit_oid).unwrap_or(oid(0)));

    // What a remote carrying every tag this repository has would
    // advertise — the steady state once a fetch has been through.
    let remote_tags = index_of(
        "origin",
        refs.iter()
            .filter(|r| r.kind == RefKind::Tag)
            .map(|r| RemoteTag {
                name: r.short.clone(),
                commit: r.commit_oid(),
                annotated: r.peeled.is_some(),
            })
            .collect(),
    );

    // waits(measured): printed for the record, judged by nothing
    let started = Instant::now();
    let nobody = held_by_nobody();
    let joins = RefJoins::new(&refs, &nobody);
    let snapshot = build_snapshot(&refs, &head, &remote_tags, &joins);
    let labels = build_label_map(&refs, &head, &remote_tags, &joins);
    println!(
        "refs={} tags={} remote_branches={} | one publish_refs join: {:.1} ms \
         ({} sidebar tags, {} labelled commits)",
        refs.len(),
        refs.iter().filter(|r| r.kind == RefKind::Tag).count(),
        refs.iter()
            .filter(|r| r.kind == RefKind::RemoteBranch)
            .count(),
        started.elapsed().as_secs_f64() * 1000.0,
        snapshot.tags.len(),
        labels.len(),
    );
}
