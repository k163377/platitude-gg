//! What a copy has to answer the way the repository it was copied from
//! would. Every test here runs real git against a real preset: what is
//! being checked is git's own reading of a tree that moved.

use std::path::Path;

use super::{BUILT_BY, Entry, holds, leaf, walk};

/// git's answer in `dir`; a test that cannot ask has failed.
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
        .expect("a worktree stands in a run's root")
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

/// Down to the commit names — the claim a template makes, which two
/// builds could not (a build stamps its commits off the clock). `tags` has
/// something of everything to disagree about: an upstream, tags pushed and
/// unpushed, a commit past the last push.
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

/// Left as the template's, the first verb to push would write into the
/// repository every later run is copied from.
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
    // And still a URL git can fetch from.
    git(&work, &["fetch", "origin"]);
}

/// A linked worktree's `.git` and the `gitdir` pointing back both name an
/// absolute path, in git's own long spelling; a copy that kept either
/// would have git answering about the template.
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

/// These presets name their own directory only in git's metadata, so
/// each gets a template rather than a per-run build (see `rebind`).
#[test]
fn only_gits_own_files_name_the_directory_they_were_built_in() {
    for preset in [
        "tags",
        "worktrees",
        "worktree-detached",
        "nested-worktree",
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

/// The copied index records stats the copy changed; git has to notice
/// that itself and still answer the same `status`.
#[test]
fn a_copy_of_an_unclean_tree_is_unclean_in_the_same_way() {
    let first = crate::demo::create("dirty", None).expect("a run of the preset");
    let second = crate::demo::create("dirty", None).expect("a second run of it");
    let status = git(&first, &["status", "--porcelain=v2"]);
    assert!(!status.is_empty(), "the preset is meant to be dirty");
    assert_eq!(status, git(&second, &["status", "--porcelain=v2"]));
    // Asked twice: the first status rewrites the stale index, and a tree
    // that reads the same only once would change under whatever runs
    // second.
    assert_eq!(status, git(&second, &["status", "--porcelain=v2"]));
}

/// Runs that all find the template missing each build their own, one
/// rename wins, and every run holds a copy of the same repository — which
/// only their commit names can show.
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

/// `--at` can name a directory outside the root templates share, so a
/// named root is built in place.
#[test]
fn a_named_root_is_built_where_it_was_named() {
    let root = crate::verify::claim_dir(&std::env::temp_dir().join("pgg-demo"), "named-root")
        .expect("a root of this test's own");
    let work = crate::demo::create("one-commit", Some(root.clone())).expect("a run at the root");
    assert_eq!(work, root.join("repo"));
    assert!(git(&work, &["log", "--format=%H"]).len() >= 40);
}

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
    // A false start: the first byte matches twice and only the second run
    // is the needle.
    assert!(holds(b"bbasic-ab", b"basic-ab"));
}

/// The modules of the crate a source names (`crate::<module>`), its
/// comments aside — `use crate::{a, b}` counted as both.
fn modules_named(text: &str) -> Vec<String> {
    let mut named = Vec::new();
    for line in text
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
    {
        for (at, _) in line.match_indices("crate::") {
            let rest = &line[at + "crate::".len()..];
            let group = rest
                .strip_prefix('{')
                .and_then(|inside| inside.split_once('}'))
                .map(|(inside, _)| inside);
            let heads: Vec<&str> = match group {
                Some(inside) => inside.split(',').collect(),
                None => vec![rest],
            };
            for head in heads {
                let module: String = head
                    .trim_start()
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !module.is_empty() {
                    named.push(module);
                }
            }
        }
    }
    named
}

/// Held to the code, not to a directory: the demo module's sources (tests
/// aside) plus, of what they name outside it, the PNG writer. Any other
/// module they name has to be justified in `PLACES_OR_DECLARES`, since a
/// source that shapes a template but is left off the list keeps handing
/// out templates built without it.
#[test]
fn the_fingerprint_reads_every_source_a_preset_is_built_by() {
    /// Named by the demo module, and shaping nothing a template holds:
    /// `verify` claims and sweeps the runs' directories, `command`
    /// declares the `demo-repo` verb.
    const PLACES_OR_DECLARES: [&str; 2] = ["verify", "command"];
    /// The PNG writer (`png::rgba`) and what it is built from.
    const PNG_WRITER: [&str; 3] = ["png/checksum.rs", "png/mod.rs", "png/write.rs"];
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut built_by: Vec<String> = PNG_WRITER.iter().map(|name| (*name).to_string()).collect();
    let mut outside = std::collections::BTreeSet::new();
    walk(&src.join("demo"), &mut |path, kind| {
        let name = path.strip_prefix(&src).expect("a file under src");
        let name = name.to_string_lossy().replace('\\', "/");
        if kind == Entry::File && name.ends_with(".rs") && !name.ends_with("/tests.rs") {
            let text = std::fs::read_to_string(path).map_err(|e| format!("{name}: {e}"))?;
            outside.extend(
                modules_named(&text)
                    .into_iter()
                    .filter(|module| module != "demo"),
            );
            built_by.push(name);
        }
        Ok(())
    })
    .expect("the sources read");
    for module in &outside {
        assert!(
            module == "png" || PLACES_OR_DECLARES.contains(&module.as_str()),
            "the demo module names crate::{module}: list its sources in BUILT_BY if they \
             shape a template, or say here why they do not"
        );
    }
    built_by.sort();
    let mut listed: Vec<String> = BUILT_BY
        .iter()
        .map(|(name, _)| (*name).to_string())
        .collect();
    listed.sort();
    assert_eq!(
        listed, built_by,
        "BUILT_BY and the sources a preset is built by disagree"
    );
    for (name, bytes) in BUILT_BY {
        let disk = std::fs::read(src.join(name)).expect("a listed source reads");
        assert!(
            disk == *bytes,
            "{name} is listed with bytes it does not hold"
        );
    }
}

#[test]
fn a_source_names_the_modules_it_reaches() {
    let text = "use crate::command::{self, Where};\n\
                // crate::gate is mentioned, not used\n\
                let out = crate::png::rgba(1, 1, f);\n\
                use crate::{verify, tree};\n";
    assert_eq!(modules_named(text), ["command", "png", "verify", "tree"]);
}
