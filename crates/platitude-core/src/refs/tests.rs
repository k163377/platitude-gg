use super::*;

const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SHA_T: &str = "1111111111111111111111111111111111111111";

fn line(fields: [&str; 7]) -> Vec<u8> {
    let mut v = fields.join("\u{0}").into_bytes();
    v.push(b'\n');
    v
}

#[test]
fn parses_the_three_namespaces() {
    let mut bytes = line([
        "refs/heads/main",
        "commit",
        SHA_A,
        "",
        "refs/remotes/origin/main",
        "*",
        "1700000100",
    ]);
    bytes.extend(line([
        "refs/remotes/origin/main",
        "commit",
        SHA_A,
        "",
        "",
        "",
        "1700000100",
    ]));
    bytes.extend(line([
        "refs/tags/v1.0",
        "tag",
        SHA_T,
        SHA_B,
        "",
        "",
        "1700000200",
    ]));

    let refs = parse_refs(&bytes);
    assert_eq!(refs.len(), 3);

    let main = &refs[0];
    assert_eq!(main.kind, RefKind::LocalBranch);
    assert_eq!(main.short, "main");
    assert!(main.is_head);
    assert_eq!(main.upstream.as_deref(), Some("refs/remotes/origin/main"));
    assert_eq!(main.commit_oid().to_hex(), SHA_A);
    assert_eq!(main.created_unix, 1_700_000_100);

    let remote = &refs[1];
    assert_eq!(remote.kind, RefKind::RemoteBranch);
    assert_eq!(remote.short, "origin/main");
    assert!(!remote.is_head);

    let tag = &refs[2];
    assert_eq!(tag.kind, RefKind::Tag);
    assert_eq!(tag.short, "v1.0");
    assert_eq!(tag.target.to_hex(), SHA_T, "annotated tag object");
    assert_eq!(tag.commit_oid().to_hex(), SHA_B, "peeled commit");
}

#[test]
fn skips_remote_head_symref() {
    let bytes = line(["refs/remotes/origin/HEAD", "commit", SHA_A, "", "", "", "0"]);
    assert!(parse_refs(&bytes).is_empty());
}

#[test]
fn skips_malformed_lines_but_keeps_the_rest() {
    let mut bytes = b"garbage-without-fields\n".to_vec();
    bytes.extend(line(["refs/heads/ok", "commit", SHA_A, "", "", "", "1"]));
    let refs = parse_refs(&bytes);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].short, "ok");
}

#[test]
fn branch_with_slash_in_name_keeps_full_short_name() {
    let bytes = line([
        "refs/heads/feature/deep/name",
        "commit",
        SHA_A,
        "",
        "",
        "",
        "1",
    ]);
    let refs = parse_refs(&bytes);
    assert_eq!(refs[0].short, "feature/deep/name");
}

fn entry(kind: RefKind, name: &str, short: &str, upstream: Option<&str>) -> RefEntry {
    RefEntry {
        name: crate::Name::from(name),
        short: crate::Name::from(short),
        kind,
        target: Oid::from_hex_str(SHA_A).expect("valid test sha"),
        peeled: None,
        upstream: upstream.map(crate::Name::from),
        is_head: false,
        created_unix: 0,
    }
}

/// The upstream is the whole of the question. A remote branch of the same
/// name is a different branch — git reads `branch.<name>.merge` and says
/// nothing about a matching name, and so does this.
#[test]
fn remote_state_comes_from_the_upstream_and_nowhere_else() {
    let refs = vec![
        // upstream configured and alive
        entry(
            RefKind::LocalBranch,
            "refs/heads/main",
            "main",
            Some("refs/remotes/origin/main"),
        ),
        // upstream configured but the remote branch is gone
        entry(
            RefKind::LocalBranch,
            "refs/heads/dead",
            "dead",
            Some("refs/remotes/origin/dead"),
        ),
        // no upstream, though origin does have a same-named branch
        entry(
            RefKind::LocalBranch,
            "refs/heads/feature/x",
            "feature/x",
            None,
        ),
        // truly local
        entry(RefKind::LocalBranch, "refs/heads/local", "local", None),
        entry(
            RefKind::RemoteBranch,
            "refs/remotes/origin/main",
            "origin/main",
            None,
        ),
        entry(
            RefKind::RemoteBranch,
            "refs/remotes/origin/feature/x",
            "origin/feature/x",
            None,
        ),
    ];
    let remotes = RemoteBranches::index(&refs);
    let with_remote: HashSet<&str> = refs
        .iter()
        .filter(|r| r.kind == RefKind::LocalBranch)
        .filter(|r| remotes.has_counterpart(r))
        .map(|r| r.name.as_str())
        .collect();
    assert!(with_remote.contains("refs/heads/main"));
    assert!(!with_remote.contains("refs/heads/dead"), "[gone] upstream");
    assert!(
        !with_remote.contains("refs/heads/feature/x"),
        "origin/feature/x is a branch of the same name, not this branch's"
    );
    assert!(!with_remote.contains("refs/heads/local"));
}

fn at(mut e: RefEntry, sha: &str) -> RefEntry {
    e.target = Oid::from_hex_str(sha).expect("valid test sha");
    e
}

#[test]
fn folds_the_upstream_sharing_the_commit() {
    let refs = vec![
        entry(
            RefKind::LocalBranch,
            "refs/heads/main",
            "main",
            Some("refs/remotes/origin/main"),
        ),
        entry(
            RefKind::RemoteBranch,
            "refs/remotes/origin/main",
            "origin/main",
            None,
        ),
    ];
    let folded = RemoteBranches::index(&refs).folded_into_local(&refs);
    assert!(folded.contains("refs/remotes/origin/main"));
}

#[test]
fn keeps_an_upstream_left_behind_on_another_commit() {
    let refs = vec![
        entry(
            RefKind::LocalBranch,
            "refs/heads/topic",
            "topic",
            Some("refs/remotes/origin/topic"),
        ),
        at(
            entry(
                RefKind::RemoteBranch,
                "refs/remotes/origin/topic",
                "origin/topic",
                None,
            ),
            SHA_B,
        ),
    ];
    assert!(
        RemoteBranches::index(&refs)
            .folded_into_local(&refs)
            .is_empty(),
        "a drifted upstream is another row's chip"
    );
}

#[test]
fn keeps_a_second_remote_that_is_not_the_upstream() {
    let refs = vec![
        entry(
            RefKind::LocalBranch,
            "refs/heads/main",
            "main",
            Some("refs/remotes/origin/main"),
        ),
        entry(
            RefKind::RemoteBranch,
            "refs/remotes/origin/main",
            "origin/main",
            None,
        ),
        entry(
            RefKind::RemoteBranch,
            "refs/remotes/fork/main",
            "fork/main",
            None,
        ),
    ];
    let folded = RemoteBranches::index(&refs).folded_into_local(&refs);
    assert!(folded.contains("refs/remotes/origin/main"));
    assert!(!folded.contains("refs/remotes/fork/main"), "not the badge");
}

/// A branch with no upstream folds nothing, however many remotes carry
/// its name: a same-named remote branch is a different branch, and it
/// keeps the chip that says so.
#[test]
fn folds_nothing_without_an_upstream() {
    let alone = vec![
        entry(
            RefKind::LocalBranch,
            "refs/heads/feature/x",
            "feature/x",
            None,
        ),
        entry(
            RefKind::RemoteBranch,
            "refs/remotes/origin/feature/x",
            "origin/feature/x",
            None,
        ),
    ];
    assert!(
        RemoteBranches::index(&alone)
            .folded_into_local(&alone)
            .is_empty(),
        "one same-named remote is still not this branch's"
    );

    let several = vec![
        entry(RefKind::LocalBranch, "refs/heads/main", "main", None),
        entry(
            RefKind::RemoteBranch,
            "refs/remotes/origin/main",
            "origin/main",
            None,
        ),
        entry(
            RefKind::RemoteBranch,
            "refs/remotes/fork/main",
            "fork/main",
            None,
        ),
    ];
    assert!(
        RemoteBranches::index(&several)
            .folded_into_local(&several)
            .is_empty(),
        "and neither of two is"
    );
}

#[test]
fn a_remote_ref_splits_at_the_remote_name_not_the_first_slash() {
    let names = ["origin", "my", "my/fork"];
    assert_eq!(
        split_remote_ref("origin/main", names),
        Some(("origin", "main"))
    );
    assert_eq!(
        split_remote_ref("origin/feature/x", names),
        Some(("origin", "feature/x"))
    );
    // The longest configured name wins: `my/fork/main` lives on
    // `my/fork`, not on `my` with a branch called `fork/main`.
    assert_eq!(
        split_remote_ref("my/fork/main", names),
        Some(("my/fork", "main"))
    );
    assert_eq!(split_remote_ref("my/other", names), Some(("my", "other")));
}

#[test]
fn a_name_no_remote_owns_does_not_split() {
    let names = ["origin"];
    assert_eq!(split_remote_ref("fork/main", names), None);
    // The bare remote name is not a remote branch.
    assert_eq!(split_remote_ref("origin", names), None);
    assert_eq!(split_remote_ref("", names), None);
    assert_eq!(split_remote_ref("origin/main", []), None);
}

#[test]
fn the_first_slash_answers_where_no_configured_remote_does() {
    // The configured list wins where it speaks…
    assert_eq!(
        split_remote_ref_or_first_slash("my/fork/main", ["my", "my/fork"]),
        Some(("my/fork", "main"))
    );
    // …and the ref's own first slash answers where it does not: a list
    // still loading, or a remote gone from configuration.
    assert_eq!(
        split_remote_ref_or_first_slash("fork/main", ["origin"]),
        Some(("fork", "main"))
    );
    assert_eq!(
        split_remote_ref_or_first_slash("fork/feature/x", []),
        Some(("fork", "feature/x"))
    );
    // No slash at all is still not a remote branch.
    assert_eq!(split_remote_ref_or_first_slash("main", []), None);
}
