//! What a copy has to answer the way the repository it was copied from
//! would. Every test here runs real git against a real preset: what is
//! being checked is git's own reading of a tree that moved.

use std::path::Path;

use super::{Entry, holds, leaf, walk};

/// git's answer in `dir`, or the reason it had none — a test that cannot
/// ask has failed.
fn git(dir: &Path, arguments: &[&str]) -> String {
    let dir = dir.display().to_string();
    crate::subprocess::git_query(&dir, arguments)
        .unwrap_or_else(|| panic!("git {arguments:?} in {dir} answered nothing"))
}

/// The one segment a copy and its template differ in.
fn run_leaf(work: &Path) -> String {
    work.parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().to_string())
        .expect("a work tree stands in a run's root")
}

/// Every file under `dir` that names `needle`.
fn naming(dir: &Path, needle: &str) -> Vec<String> {
    let mut found = Vec::new();
    walk(dir, &mut |path, kind| {
        if kind == Entry::File
            && std::fs::read(path).is_ok_and(|bytes| holds(&bytes, needle.as_bytes()))
        {
            found.push(path.display().to_string());
        }
        Ok(())
    })
    .expect("the tree reads");
    found
}

/// Two runs of one preset are the same repository, down to the commit
/// names — which is the whole claim a template makes, and the thing two
/// builds could never say: a build stamps its commits off the clock, so
/// two of them agree on the shape and on nothing else.
///
/// The preset is one with something of everything to disagree about: an
/// origin with an upstream, tags pushed and unpushed, and a commit past
/// the last push.
#[test]
fn two_runs_of_a_preset_are_the_same_repository() {
    let first = crate::demo::create("tags", None).expect("a run of the preset");
    let second = crate::demo::create("tags", None).expect("a second run of it");

    for question in [
        &["log", "--format=%H %s", "--all"][..],
        &["status", "--porcelain=v2", "--branch"][..],
        &["tag", "--list"][..],
        &["for-each-ref", "--format=%(refname) %(objectname)"][..],
    ] {
        assert_eq!(
            git(&first, question),
            git(&second, question),
            "two runs disagreed on git {question:?}"
        );
    }
}

/// A copy's `origin` is its own. Left as the template's, the first verb
/// to push would write into the repository every later run is copied
/// from.
#[test]
fn a_copy_pushes_to_its_own_origin() {
    let work = crate::demo::create("tags", None).expect("a run of the preset");
    let url = git(&work, &["remote", "get-url", "origin"]);
    assert!(
        url.contains(&run_leaf(&work)),
        "origin is not this run's: {url}"
    );
    assert!(
        !url.contains(&leaf("tags", "repo")),
        "origin still names the template: {url}"
    );
    // And it is a URL git can still fetch from, which the fetch
    // below proves.
    git(&work, &["fetch", "origin"]);
}

/// A linked worktree stands on two files naming an absolute path — its
/// own `.git`, and the `gitdir` pointing back at it — and git writes
/// both in a spelling of its own (the profile's long name, where this
/// process hands it the short one). A copy that kept either would have
/// git answering about the template from inside the copy.
#[test]
fn a_copy_of_linked_worktrees_stands_on_its_own_paths() {
    let work = crate::demo::create("worktrees", None).expect("a run of the preset");
    let root = work.parent().expect("a run root");
    let mine = run_leaf(&work);
    let template = leaf("worktrees", "repo");

    let list = git(&work, &["worktree", "list"]);
    assert!(list.contains(&mine), "no worktree of this run's: {list}");
    assert!(
        !list.contains(&template),
        "a worktree still stands in the template: {list}"
    );

    let stragglers = naming(root, &template);
    assert!(
        stragglers.is_empty(),
        "these still name the template: {stragglers:?}"
    );
}

/// The rule that keeps a rebind honest: git's own files may name the
/// directory the repository sits in, and rewriting a tracked one
/// would leave the copy dirty where the template was clean. Every
/// preset a template is made of has to pass it, so the check is that
/// the presets under test hold nothing but metadata.
#[test]
fn only_gits_own_files_name_the_directory_they_were_built_in() {
    for preset in [
        "tags",
        "worktrees",
        "worktree-detached",
        "stashes",
        "shallow",
    ] {
        let work = crate::demo::create(preset, None).expect("a run of the preset");
        let root = work.parent().expect("a run root");
        for path in naming(root, &run_leaf(&work)) {
            assert!(
                super::under_git_metadata(root, Path::new(&path)),
                "{preset}: {path} names its own directory and git tracks it"
            );
        }
    }
}

/// A copy of a repository with something uncommitted in it answers the
/// same `status` — the index it was copied with names files by the stat
/// the copy has changed, and git has to be left to notice that for
/// itself.
#[test]
fn a_copy_of_an_unclean_tree_is_unclean_in_the_same_way() {
    let first = crate::demo::create("dirty", None).expect("a run of the preset");
    let second = crate::demo::create("dirty", None).expect("a second run of it");
    let status = git(&first, &["status", "--porcelain=v2"]);
    assert!(!status.is_empty(), "the preset is meant to be dirty");
    assert_eq!(status, git(&second, &["status", "--porcelain=v2"]));
    // Asked twice, because the first status is the one that rewrites the
    // index it found stale: a tree that only reads clean once reads as a
    // change to whatever runs second.
    assert_eq!(status, git(&second, &["status", "--porcelain=v2"]));
}

/// Runs start together and any number of them can find the template
/// missing at once. Each builds its own, one rename wins, and every run
/// ends up holding a copy of the same repository — which their commit
/// names say and nothing else would.
#[test]
fn runs_started_together_are_handed_one_template() {
    let start = std::sync::Arc::new(std::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let start = start.clone();
            std::thread::spawn(move || {
                start.wait();
                crate::demo::create("one-commit", None).expect("a run of the preset")
            })
        })
        .collect();
    let runs: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().expect("a building thread"))
        .collect();

    let roots: std::collections::BTreeSet<_> = runs.iter().map(|work| run_leaf(work)).collect();
    assert_eq!(roots.len(), runs.len(), "two runs shared a root");
    let commits: std::collections::BTreeSet<_> = runs
        .iter()
        .map(|work| git(work, &["log", "--format=%H", "--all"]))
        .collect();
    assert_eq!(commits.len(), 1, "each run built its own repository");
}

/// `--at` names a directory anywhere on the machine, and a template's
/// paths are under the one root the runs share — so a named root is
/// built in, and comes out standing on itself.
#[test]
fn a_named_root_is_built_where_it_was_named() {
    let root = crate::verify::claim_dir(&std::env::temp_dir().join("pgg-demo"), "named-root")
        .expect("a root of this test's own");
    let work = crate::demo::create("one-commit", Some(root.clone())).expect("a run at the root");
    assert_eq!(work, root.join("repo"));
    assert!(git(&work, &["log", "--format=%H"]).len() >= 40);
}

/// A bare repository is git's own by what it holds; a directory whose
/// name merely ends in `.git` is a directory a preset could commit, and
/// what it holds is content.
#[test]
fn a_bare_repository_is_gits_own_and_a_name_ending_in_git_is_not() {
    let root = crate::verify::claim_dir(&std::env::temp_dir().join("pgg-demo"), "metadata")
        .expect("a root of this test's own");
    let bare = root.join("origin.git");
    std::fs::create_dir_all(bare.join("objects")).expect("a bare repository's objects");
    std::fs::write(bare.join("HEAD"), "ref: refs/heads/main\n").expect("its HEAD");
    let worn = root.join("vendor.git");
    std::fs::create_dir_all(&worn).expect("a directory that only looks like one");
    std::fs::write(worn.join("notes.txt"), "tracked\n").expect("something tracked in it");

    assert!(super::under_git_metadata(&root, &bare.join("config")));
    assert!(super::under_git_metadata(
        &root,
        &root.join("repo/.git/config")
    ));
    assert!(super::under_git_metadata(&root, &root.join("topic/.git")));
    assert!(!super::under_git_metadata(&root, &worn.join("notes.txt")));
    assert!(!super::under_git_metadata(
        &root,
        &root.join("repo/src/app.txt")
    ));
    std::fs::remove_dir_all(&root).expect("the root this test made");
}

#[test]
fn a_needle_is_found_where_it_is_and_nowhere_else() {
    assert!(holds(b"gitdir: /tmp/pgg-demo/basic-ab", b"basic-ab"));
    assert!(holds(b"basic-ab", b"basic-ab"));
    assert!(!holds(b"basic-a", b"basic-ab"));
    assert!(!holds(b"", b"basic-ab"));
    assert!(!holds(b"basic-ab", b""));
    // The false start that a byte-at-a-time search gets wrong: the first
    // byte matches twice and only the second run is the needle.
    assert!(holds(b"bbasic-ab", b"basic-ab"));
}
