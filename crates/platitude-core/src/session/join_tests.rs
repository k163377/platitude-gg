//! The refs join: what the sidebar and the graph chips are built from,
//! and what one join costs at scale.

#![expect(
    clippy::print_stdout,
    reason = "the scale measurement reports its number to whoever ran it"
)]

use super::build::{RefJoins, build_label_map, build_snapshot};
use super::*;
use crate::remote::RemoteTag;

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
    let joins = RefJoins::new(&refs);
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
    let joins = RefJoins::new(&refs);
    let map = build_label_map(&refs, &head_at(oid(1)), &remote_tags, &joins);
    assert!(map.labels_of(&oid(1), true)[0].here);
    let theirs = map.labels_of(&oid(2), true);
    assert!(!theirs.is_empty(), "the remote's reading");
    assert!(!theirs[0].here);
    assert_eq!(theirs[0].remote, "origin");

    let snapshot = build_snapshot(&refs, &head_at(oid(1)), &remote_tags, &joins);
    assert_eq!(snapshot.tags.len(), 1);
    assert!(snapshot.tags[0].here);
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
    let joins = RefJoins::new(&refs);
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
    let joins = RefJoins::new(&refs);
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
/// `PG_PERF_REPO=<path> cargo test -p platitude-core --release
/// refs_join_at_scale -- --ignored --nocapture`.
///
/// What it times is one `publish_refs` join — the work every poll tick
/// does on top of the two git reads. Recorded in
/// `ci/baseline/refs-join-windows-x64.md`.
#[test]
#[ignore = "needs PG_PERF_REPO pointed at a large repository"]
fn refs_join_at_scale() {
    let repo = std::env::var("PG_PERF_REPO").unwrap_or_else(|_| ".".to_string());
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

    let started = Instant::now();
    let joins = RefJoins::new(&refs);
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
