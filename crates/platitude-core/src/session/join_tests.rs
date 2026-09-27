//! The refs join: what the sidebar and the graph chips are built from,
//! and what one join costs at scale.

#![expect(
    clippy::print_stdout,
    reason = "the scale measurement reports its number to whoever ran it"
)]

use super::joins::{RefJoins, WorktreeHolders, build_label_map, build_snapshot};
use super::*;
use crate::remote::RemoteTag;

/// One working copy: no branch is held anywhere else.
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

fn index_of(remote: &str, tags: Vec<RemoteTag>) -> std::sync::Arc<RemoteTagIndex> {
    std::sync::Arc::new(RemoteTagIndex::build(
        tags.into_iter()
            .map(|t| (t.name, t.commit, t.annotated, crate::Name::from(remote))),
    ))
}

/// A drifted pair is two rows on the graph, and the menu's rows that
/// reach the remote read this to say why (デザイン規約 §左メニューの所作 の削除の表).
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

/// The upstream setting read from the other end: a remote-tracking row
/// carries the branch measured against it and its counts, for its opened
/// lines (デザイン規約 §左メニューの所作). A matching name alone joins
/// nothing, as in git (`refs::RemoteBranches::spoken_for`).
#[test]
fn a_reading_carries_the_branch_that_is_measured_against_it() {
    let tracking = |short: &str| RefEntry {
        name: crate::Name::from(format!("refs/remotes/{short}")),
        short: crate::Name::from(short),
        kind: RefKind::RemoteBranch,
        target: oid(1),
        peeled: None,
        upstream: None,
        is_head: false,
        created_unix: 0,
        ahead: 0,
        behind: 0,
    };
    let mut main = branch("main", oid(2));
    main.upstream = Some(crate::Name::from("refs/remotes/origin/main"));
    main.ahead = 2;
    main.behind = 1;
    // Matches `fork/side` by name only: a different branch.
    let stray = branch("side", oid(2));

    let refs = vec![main, stray, tracking("origin/main"), tracking("fork/side")];
    let held = held_by_nobody();
    let joins = RefJoins::new(&refs, &held);
    let snapshot = build_snapshot(
        &refs,
        &head_at(oid(2)),
        &index_of("origin", Vec::new()),
        &joins,
    );
    let read = |short: &str| {
        snapshot
            .remote_named(short)
            .expect("the listing holds this reading")
    };
    assert_eq!(read("origin/main").tracked_by, "main");
    assert_eq!(
        (read("origin/main").ahead, read("origin/main").behind),
        (2, 1),
        "the measurement is the branch's, read from the far side"
    );
    assert_eq!(read("fork/side").tracked_by, "");
    assert_eq!((read("fork/side").ahead, read("fork/side").behind), (0, 0));
    assert_eq!(
        snapshot.local_named("main").expect("the branch").tracked_by,
        "",
        "the setting reads one way on this side"
    );
}

/// An upstream pruned away while the configuration still names it (git's
/// `[gone]`): the name must survive the join, as it is all the row can
/// say (デザイン規約 §左メニューの所作). An upstream configured
/// `remote = .` names a branch here, so nothing is gone.
#[test]
fn a_branch_keeps_the_name_of_an_upstream_that_is_not_here() {
    let remote_tags = index_of("origin", Vec::new());
    let held = held_by_nobody();

    let mut pruned = branch("release-1.2", oid(1));
    pruned.upstream = Some(crate::Name::from("refs/remotes/origin/release-1.2"));
    let alone = vec![pruned];
    let joins = RefJoins::new(&alone, &held);
    let snapshot = build_snapshot(&alone, &head_at(oid(1)), &remote_tags, &joins);
    assert_eq!(
        snapshot.locals[0].upstream, "",
        "there is no reading to speak for"
    );
    assert!(!snapshot.locals[0].has_remote);
    assert_eq!(snapshot.locals[0].upstream_gone, "origin/release-1.2");

    let mut tracks_a_branch_here = branch("topic", oid(1));
    tracks_a_branch_here.upstream = Some(crate::Name::from("refs/heads/main"));
    let here = vec![branch("main", oid(1)), tracks_a_branch_here];
    let joins = RefJoins::new(&here, &held);
    let snapshot = build_snapshot(&here, &head_at(oid(1)), &remote_tags, &joins);
    assert_eq!(
        snapshot.locals[1].upstream_gone, "",
        "the ref it names is a branch in this listing"
    );
}

/// The cloud is on a chip exactly while its remote is folded into it
/// (デザイン規約 §ref の種別); drifted, neither row wears one, while the
/// sidebar still says the branch has a remote.
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

    // No upstream: a same-named remote on the same commit is still a
    // different branch — nothing folds or is badged.
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

/// One bit for both halves: a chip offering a move the row refused would
/// be two answers to one question.
#[test]
fn a_branch_another_copy_holds_is_marked_on_the_row_and_the_chip() {
    let refs = vec![branch("main", oid(1)), branch("feature/topic-a", oid(2))];
    let remote_tags = index_of("origin", Vec::new());
    let mut held = WorktreeHolders::default();
    held.branches.insert("feature/topic-a".to_string(), false);
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
    assert!(
        !map.labels_of(&oid(2), true)[0].locked,
        "the copy holding it was not locked, so the chip wears no padlock"
    );
}

/// The lock rides with the held mark: one lookup answers both
/// (デザイン規約 §ref の種別).
#[test]
fn a_locked_copy_puts_the_padlock_on_the_branch_it_holds() {
    let refs = vec![branch("main", oid(1)), branch("hotfix/urgent", oid(2))];
    let remote_tags = index_of("origin", Vec::new());
    let mut held = WorktreeHolders::default();
    held.branches.insert("hotfix/urgent".to_string(), true);
    let joins = RefJoins::new(&refs, &held);

    let map = build_label_map(&refs, &head_at(oid(1)), &remote_tags, &joins);
    let chip = &map.labels_of(&oid(2), true)[0];
    assert!(chip.held_elsewhere && chip.locked);
    let here = &map.labels_of(&oid(1), true)[0];
    assert!(!here.held_elsewhere && !here.locked);
}

/// No ref there could carry the mark a held branch's chip wears.
#[test]
fn a_copy_standing_on_no_branch_gets_a_chip_of_its_own() {
    let refs = vec![branch("main", oid(1)), tag("v1", oid(2), false)];
    let remote_tags = index_of("origin", Vec::new());
    let mut held = WorktreeHolders::default();
    held.detached.push(super::joins::DetachedCheckout {
        oid: oid(2),
        name: crate::Name::const_new("spike"),
        locked: true,
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
    assert!(
        copy.locked,
        "the padlock is the copy's, whether or not it has a branch out"
    );
    assert!(
        !map.labels_of(&oid(1), true)
            .iter()
            .any(|l| l.kind == LabelKind::Worktree)
    );
    // Tags stay the tail of the run: the TAGS eye cuts them off with a
    // shorter slice, and a chip sorted past them would go with them.
    assert!(
        map.labels_of(&oid(2), false)
            .iter()
            .any(|l| l.kind == LabelKind::Worktree),
        "the eye takes the tag and leaves the working copy"
    );
}

/// `set-url` moves no ref, so the join key must move for it or the held
/// snapshot is resent with the old URL.
#[test]
fn a_changed_remote_url_moves_the_join_key() {
    let remotes = |url: &str| crate::remote::Remotes {
        list: vec![crate::remote::Remote {
            name: "origin".to_string(),
            fetch_url: url.to_string(),
            push_url: url.to_string(),
        }],
        push_default: None,
        checkout_default: None,
    };
    assert_ne!(
        joins::join_key(1, 0, 0, &remotes("https://old.example/repo")),
        joins::join_key(1, 0, 0, &remotes("https://new.example/repo")),
    );
}

/// Finishing a half-set origin mark changes `checkout.defaultRemote`
/// alone, and that change must take away the row offering it.
#[test]
fn a_moved_checkout_mark_moves_the_join_key() {
    let remotes = |checkout: Option<&str>| crate::remote::Remotes {
        list: Vec::new(),
        push_default: None,
        checkout_default: checkout.map(str::to_string),
    };
    assert_ne!(
        joins::join_key(1, 0, 0, &remotes(None)),
        joins::join_key(1, 0, 0, &remotes(Some("fork"))),
    );
}

/// The local label carries the cloud; the remote's reading adds no chip.
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

/// The row that is not here says whose reading it is.
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

/// On the rows that stay too: a tag on a commit a branch also reaches is
/// the chip the walk cannot take away.
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

/// What a tag row opens on, answered off the index rather than the rows
/// (デザイン規約 §左メニューの所作). One row per name however many remotes
/// have it, so four shapes: two carriers agreeing, one carrier elsewhere
/// than here, no carrier, and a name only a remote has.
///
/// The readings are weighed against the reference remote (the one tag
/// rows act on), never the local tag — the copy here is one opinion among
/// them.
#[test]
fn the_snapshot_says_which_remotes_carry_a_tag() {
    let refs = vec![
        branch("main", oid(1)),
        tag("v1", oid(1), false),
        tag("v2", oid(2), false),
        tag("v3-local", oid(1), false),
    ];
    let remote_tags = std::sync::Arc::new(RemoteTagIndex::build(
        [
            ("v1", oid(1), "origin"),
            ("v1", oid(1), "fork"),
            // Elsewhere than here: a carrier all the same.
            ("v2", oid(5), "fork"),
            ("v9", oid(9), "origin"),
        ]
        .into_iter()
        .map(|(name, commit, remote)| {
            (
                crate::Name::from(name),
                commit,
                false,
                crate::Name::from(remote),
            )
        }),
    ));
    let nobody = held_by_nobody();
    let joins = RefJoins::new(&refs, &nobody);
    let snapshot = build_snapshot(&refs, &head_at(oid(1)), &remote_tags, &joins);

    assert_eq!(
        snapshot.tag_remotes("v1", "origin"),
        vec![("fork", false), ("origin", false)],
        "both carriers, each said once and in name order, neither apart"
    );
    assert_eq!(
        snapshot.tag_remotes("v2", "origin"),
        vec![("fork", false)],
        "nobody stands apart from a reading the reference does not have"
    );
    assert!(
        snapshot.tag_remotes("v3-local", "origin").is_empty(),
        "a tag nobody out there has opens on nothing"
    );
    assert_eq!(
        snapshot.tag_remotes("v9", "origin"),
        vec![("origin", false)],
        "a name only a remote has is a row of its own, and says whose"
    );
    assert!(snapshot.tag_remotes("never", "origin").is_empty());

    // With the other carrier as reference; the local tag has no say.
    assert_eq!(
        snapshot.tag_remotes("v2", "fork"),
        vec![("fork", false)],
        "the reference never stands apart from itself"
    );
}

/// Every reading but the reference's is apart, including the one this
/// repository has.
#[test]
fn the_carriers_of_a_tag_stand_apart_from_the_reference_alone() {
    let refs = vec![branch("main", oid(1)), tag("v1", oid(3), false)];
    let remote_tags = std::sync::Arc::new(RemoteTagIndex::build(
        [
            ("v1", oid(2), "origin"),
            ("v1", oid(3), "fork"),
            ("v1", oid(4), "mirror"),
        ]
        .into_iter()
        .map(|(name, commit, remote)| {
            (
                crate::Name::from(name),
                commit,
                false,
                crate::Name::from(remote),
            )
        }),
    ));
    let nobody = held_by_nobody();
    let joins = RefJoins::new(&refs, &nobody);
    let snapshot = build_snapshot(&refs, &head_at(oid(1)), &remote_tags, &joins);

    assert_eq!(
        snapshot.tag_remotes("v1", "origin"),
        vec![("fork", true), ("mirror", true), ("origin", false)],
        "the reading here (fork's) is apart like any other"
    );
}

/// A snapshot over `refs` and the remotes' readings as
/// `(name, commit, remote)`.
fn read_with(refs: &[RefEntry], readings: &[(&str, u8, &str)]) -> RefsSnapshot {
    let remote_tags = std::sync::Arc::new(RemoteTagIndex::build(readings.iter().map(
        |(name, commit, remote)| {
            (
                crate::Name::from(*name),
                oid(*commit),
                false,
                crate::Name::from(*remote),
            )
        },
    )));
    let nobody = held_by_nobody();
    let joins = RefJoins::new(refs, &nobody);
    build_snapshot(refs, &head_at(oid(1)), &remote_tags, &joins)
}

/// The same test for one reading by its commit: the copy here (asked with
/// its own commit) and the reading a graph row carries (asked with that
/// row's). The reference decides wherever it carries the name.
#[test]
fn a_reading_of_a_tag_is_apart_where_the_reference_has_it_elsewhere() {
    let refs = [branch("main", oid(1)), tag("v1", oid(3), false)];
    let snapshot = read_with(
        &refs,
        &[("v1", 2, "origin"), ("v1", 3, "fork"), ("v1", 4, "mirror")],
    );

    assert!(
        snapshot.tag_apart_at("v1", oid(3), "origin"),
        "the copy here stands where origin does not"
    );
    assert!(
        snapshot.tag_apart_at("v1", oid(4), "origin"),
        "so does a reading only another remote carries"
    );
    assert!(
        !snapshot.tag_apart_at("v1", oid(2), "origin"),
        "the reference's own reading is never apart"
    );
    assert!(
        !snapshot.tag_apart_at("v1", oid(3), "fork"),
        "against fork, the copy here agrees"
    );
    assert_eq!(snapshot.tag_weighed_against("v1", "origin"), ["origin"]);
    assert!(!snapshot.tag_apart_at("never", oid(2), "origin"));
    assert!(snapshot.tag_weighed_against("never", "origin").is_empty());
}

/// Without the reference among them, the remotes decide by agreeing: one
/// commit among them is the right one, whoever carries it, and the
/// sentence names every one of them.
#[test]
fn remotes_that_agree_decide_where_the_reference_is_silent() {
    let refs = [branch("main", oid(1)), tag("v2", oid(2), false)];
    let snapshot = read_with(&refs, &[("v2", 5, "fork"), ("v2", 5, "mirror")]);

    assert!(
        snapshot.tag_apart_at("v2", oid(2), "origin"),
        "the copy here stands off the commit both remotes agree on"
    );
    assert!(
        !snapshot.tag_apart_at("v2", oid(5), "origin"),
        "and the remotes that agree are not apart"
    );
    assert_eq!(
        snapshot.tag_remotes("v2", "origin"),
        vec![("fork", false), ("mirror", false)]
    );
    assert_eq!(
        snapshot.tag_weighed_against("v2", "origin"),
        ["fork", "mirror"]
    );
}

/// Remotes that disagree with no reference among them leave no right
/// reading: every holder stands apart, the copy here included.
#[test]
fn remotes_that_disagree_without_the_reference_put_every_holder_apart() {
    let refs = [branch("main", oid(1)), tag("v3", oid(5), false)];
    let snapshot = read_with(&refs, &[("v3", 5, "fork"), ("v3", 6, "mirror")]);

    assert!(
        snapshot.tag_apart_at("v3", oid(5), "origin"),
        "the copy here, though it stands where fork does"
    );
    assert!(snapshot.tag_apart_at("v3", oid(6), "origin"));
    assert_eq!(
        snapshot.tag_remotes("v3", "origin"),
        vec![("fork", true), ("mirror", true)]
    );
    assert!(
        snapshot.tag_weighed_against("v3", "origin").is_empty(),
        "nobody to name"
    );
}

/// Unasked, a delete reaches the reference, else the one remote carrying
/// the name; with several and no reference, no remote can be named.
#[test]
fn a_tag_delete_reaches_the_reference_or_the_only_carrier() {
    let snapshot = read_with(
        &[branch("main", oid(1))],
        &[
            ("both", 2, "origin"),
            ("both", 3, "fork"),
            ("solo", 2, "fork"),
            ("pair", 2, "fork"),
            ("pair", 2, "mirror"),
        ],
    );

    assert_eq!(snapshot.tag_delete_target("both", "origin"), Some("origin"));
    assert_eq!(snapshot.tag_delete_target("solo", "origin"), Some("fork"));
    assert_eq!(
        snapshot.tag_delete_target("pair", "origin"),
        None,
        "two carriers agreeing are still two remotes to pick from"
    );
    assert_eq!(snapshot.tag_delete_target("never", "origin"), None);
}

/// A chip names one remote on its own only where it draws that one
/// remote's reading — not the copy here, not a reading two remotes share.
#[test]
fn a_chip_names_the_one_remote_whose_reading_it_draws() {
    let refs = [branch("main", oid(1)), tag("v1", oid(3), false)];
    let snapshot = read_with(
        &refs,
        &[
            ("v1", 2, "origin"),
            ("v1", 3, "fork"),
            ("v1", 4, "mirror"),
            ("v1", 4, "backup"),
        ],
    );

    assert_eq!(snapshot.tag_aim_at("v1", oid(2)), Some("origin"));
    assert_eq!(
        snapshot.tag_aim_at("v1", oid(3)),
        None,
        "that chip is the copy here, which fork happens to agree with"
    );
    assert_eq!(
        snapshot.tag_aim_at("v1", oid(4)),
        None,
        "two remotes' reading"
    );
    assert_eq!(snapshot.tag_aim_at("v1", oid(9)), None);
}

/// What a tag's menu stands on, for each way the rows reaching over there
/// are decided (デザイン規約 §左メニューの所作 の削除の表).
#[test]
fn a_tag_menu_reaches_the_remote_it_can_name() {
    let refs = [
        branch("main", oid(1)),
        tag("moved", oid(3), false),
        tag("pair-here", oid(2), false),
    ];
    let snapshot = read_with(
        &refs,
        &[
            ("moved", 2, "origin"),
            ("moved", 3, "fork"),
            ("theirs", 5, "fork"),
            ("pair-here", 2, "fork"),
            ("pair-here", 4, "mirror"),
        ],
    );

    // The reference reached, off the commit here: the plain rows are
    // held back and the push is the leased one.
    let moved = snapshot.tag_menu("moved", "origin", "");
    assert_eq!(moved.push_remote, "origin");
    assert_eq!(moved.lease, Some(oid(2)));
    assert_eq!(moved.reach, "origin");
    assert_eq!(moved.held_back, TagDeleteHeld::Drifted);
    assert!(!moved.row_goes, "the copy here stays");

    // Named on its own, fork takes every row, and agrees with here.
    let aimed = snapshot.tag_menu("moved", "origin", "fork");
    assert_eq!(aimed.push_remote, "fork");
    assert_eq!(aimed.lease, None);
    assert_eq!(aimed.reach, "fork");
    assert_eq!(aimed.held_back, TagDeleteHeld::No);

    // Named on its own, origin is reached drifted or not.
    let pointed = snapshot.tag_menu("moved", "origin", "origin");
    assert_eq!(pointed.held_back, TagDeleteHeld::No);
    assert_eq!(pointed.lease, Some(oid(2)));

    // Only fork has it and nothing here does: fork is reached, and the
    // row goes with the one reading there is.
    let theirs = snapshot.tag_menu("theirs", "origin", "");
    assert_eq!(theirs.push_remote, "origin");
    assert_eq!(theirs.reach, "fork");
    assert_eq!(theirs.held_back, TagDeleteHeld::No);
    assert!(theirs.row_goes);

    // Two remotes, no reference: none can be named unasked.
    let pair = snapshot.tag_menu("pair-here", "origin", "");
    assert_eq!(pair.reach, "");
    assert_eq!(pair.held_back, TagDeleteHeld::Unnamed);
    assert_eq!(pair.carriers, ["fork", "mirror"]);
    assert!(!pair.row_goes);
}

/// Times one `publish_refs` join — what every poll tick does on top of
/// the two git reads. How to run it and the record:
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
