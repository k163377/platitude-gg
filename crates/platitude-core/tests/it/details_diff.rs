//! Commit details and file diffs against real git.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::Oid;
use platitude_core::details::{self, DiffTarget};
use platitude_core::parse::diff::DiffLineKind;

#[tokio::test]
async fn details_of_a_merge_commit_use_the_first_parent() {
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("side.txt", "side\n", "side work");
    repo.git(&["checkout", "main"]);
    repo.commit_file("main.txt", "main\n", "main work");
    repo.git(&[
        "merge",
        "--no-ff",
        "-m",
        "merge side\n\nbody of merge 日本語",
        "side",
    ]);
    let merge_sha = repo.git(&["rev-parse", "HEAD"]);
    let main_sha = repo.git(&["rev-parse", "HEAD^1"]);

    let (executor, cancel) = env();
    let oid = Oid::from_hex_str(&merge_sha).unwrap();
    let d = details::commit_details(&executor, &repo.path, &oid, &cancel)
        .await
        .unwrap();

    assert_eq!(d.oid, oid);
    assert_eq!(d.parents.len(), 2);
    assert_eq!(d.parents[0], Oid::from_hex_str(&main_sha).unwrap());
    assert_eq!(d.author_name, "Test User");
    assert_eq!(d.message, "merge side\n\nbody of merge 日本語");
    // First-parent diff of the merge = what the merge brought in: side.txt.
    let paths: Vec<&str> = d.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, vec!["side.txt"]);
    assert_eq!(d.files[0].status, 'A');
}

#[tokio::test]
async fn details_of_the_root_commit_show_created_files() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file_id("first.txt", "hello\n", "root commit");

    let (executor, cancel) = env();
    let oid = Oid::from_hex_str(&root).unwrap();
    let d = details::commit_details(&executor, &repo.path, &oid, &cancel)
        .await
        .unwrap();
    assert!(d.parents.is_empty());
    assert_eq!(d.files.len(), 1);
    assert_eq!(d.files[0].status, 'A');
    assert_eq!(d.files[0].path, "first.txt");
}

#[tokio::test]
async fn details_read_co_authors_whatever_case_the_trailer_used() {
    let mut repo = TestRepo::init();
    // The spelling tools actually write is `Co-Authored-By`; the one the
    // convention documents is `Co-authored-by`. git's `key=` matches
    // either, and both have to land here. The one in the prose must not:
    // a trailer is a property of the final block, git is what decides
    // that, and the count below is what would go wrong if the message
    // were ever scanned by hand instead.
    let sha = repo.commit_file_id(
        "a.txt",
        "one\n",
        "feat: two hands on it\n\nI wrote Co-authored-by: nobody <n@e.com> in the body\n\n\
         Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>\n\
         co-authored-by: Bob Builder <bob@example.com>",
    );

    let (executor, cancel) = env();
    let oid = Oid::from_hex_str(&sha).unwrap();
    let d = details::commit_details(&executor, &repo.path, &oid, &cancel)
        .await
        .unwrap();

    assert_eq!(d.co_authors.len(), 2);
    assert_eq!(d.co_authors[0].name, "Claude Opus 5");
    assert_eq!(d.co_authors[0].email, "noreply@anthropic.com");
    assert_eq!(d.co_authors[1].name, "Bob Builder");
    assert_eq!(d.co_authors[1].email, "bob@example.com");
    // The trailer stays part of the message: the description box is the
    // editor for what gets saved, so nothing is taken out of it.
    assert!(d.message.contains("Co-Authored-By: Claude Opus 5"));
}

#[tokio::test]
async fn details_report_renames_with_scores() {
    let mut repo = TestRepo::init();
    repo.commit_file("before.txt", "stable content here\n", "add file");
    repo.git(&["mv", "before.txt", "after.txt"]);
    repo.git(&["commit", "-m", "rename it"]);
    let sha = repo.git(&["rev-parse", "HEAD"]);

    let (executor, cancel) = env();
    let oid = Oid::from_hex_str(&sha).unwrap();
    let d = details::commit_details(&executor, &repo.path, &oid, &cancel)
        .await
        .unwrap();
    assert_eq!(d.files.len(), 1);
    let f = &d.files[0];
    assert_eq!(f.status, 'R');
    assert_eq!(f.path, "after.txt");
    assert_eq!(f.orig_path.as_deref(), Some("before.txt"));
    assert_eq!(f.score, Some(100));
}

#[tokio::test]
async fn details_of_a_commit_that_changed_nothing_list_no_files() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "add file");
    repo.git(&["commit", "--allow-empty", "-m", "nothing to see"]);
    let sha = repo.git(&["rev-parse", "HEAD"]);

    let (executor, cancel) = env();
    let oid = Oid::from_hex_str(&sha).unwrap();
    let d = details::commit_details(&executor, &repo.path, &oid, &cancel)
        .await
        .unwrap();
    // The one input where git writes the record and then stops: the file
    // list and the newline in front of it are both absent.
    assert_eq!(d.message, "nothing to see");
    assert!(d.files.is_empty(), "got {:?}", d.files);
}

#[tokio::test]
async fn a_message_that_reads_like_a_file_list_is_not_read_as_one() {
    let mut repo = TestRepo::init();
    // The metadata and the changed files arrive from one `git show`, so
    // the boundary between them has to be the NUL count and nothing else.
    // A commit is free to describe its own diff in prose.
    let sha = repo.commit_file_id(
        "real.txt",
        "one\n",
        "docs: explain the notation\n\nA\tinvented/one.txt\nM\tinvented/two.txt",
    );

    let (executor, cancel) = env();
    let oid = Oid::from_hex_str(&sha).unwrap();
    let d = details::commit_details(&executor, &repo.path, &oid, &cancel)
        .await
        .unwrap();
    assert!(d.message.contains("A\tinvented/one.txt"));
    let paths: Vec<&str> = d.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, vec!["real.txt"]);
}

#[tokio::test]
async fn commit_file_diff_has_hunks_and_line_numbers() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo\nthree\n", "add");
    repo.write_file("f.txt", "one\ntwo changed\nthree\n");
    repo.git(&["commit", "-am", "change line two"]);
    let sha = repo.git(&["rev-parse", "HEAD"]);
    let parent = repo.git(&["rev-parse", "HEAD^"]);

    let (executor, cancel) = env();
    let target = DiffTarget::Commit {
        oid: Oid::from_hex_str(&sha).unwrap(),
        parent: Some(Oid::from_hex_str(&parent).unwrap()),
        path: "f.txt".to_string(),
        orig_path: None,
    };
    let patches = details::file_diff(&executor, &repo.path, &target, &cancel)
        .await
        .unwrap();
    assert_eq!(patches.len(), 1);
    let patch = &patches[0];
    assert_eq!(patch.path(), "f.txt");
    assert_eq!(patch.hunks.len(), 1);
    let lines = &patch.hunks[0].lines;
    let del = lines
        .iter()
        .find(|l| l.kind == DiffLineKind::Deletion)
        .unwrap();
    assert_eq!(del.text, "two");
    assert_eq!(del.old_no, Some(2));
    let add = lines
        .iter()
        .find(|l| l.kind == DiffLineKind::Addition)
        .unwrap();
    assert_eq!(add.text, "two changed");
    assert_eq!(add.new_no, Some(2));
}

#[tokio::test]
async fn staged_and_unstaged_diffs_are_separate() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "committed\n", "base");
    repo.write_file("f.txt", "staged version\n");
    repo.git(&["add", "--", "f.txt"]);
    repo.write_file("f.txt", "worktree version\n");

    let (executor, cancel) = env();

    let staged = details::file_diff(
        &executor,
        &repo.path,
        &DiffTarget::Staged {
            path: "f.txt".to_string(),
            orig_path: None,
        },
        &cancel,
    )
    .await
    .unwrap();
    let staged_adds: Vec<&str> = staged[0].hunks[0]
        .lines
        .iter()
        .filter(|l| l.kind == DiffLineKind::Addition)
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(staged_adds, vec!["staged version"]);

    let unstaged = details::file_diff(
        &executor,
        &repo.path,
        &DiffTarget::Unstaged {
            path: "f.txt".to_string(),
        },
        &cancel,
    )
    .await
    .unwrap();
    let unstaged_adds: Vec<&str> = unstaged[0].hunks[0]
        .lines
        .iter()
        .filter(|l| l.kind == DiffLineKind::Addition)
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(unstaged_adds, vec!["worktree version"]);
}

#[tokio::test]
async fn untracked_file_renders_as_all_additions() {
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "x\n", "base");
    repo.write_file("brand new.txt", "line 1\nline 2\n");

    let (executor, cancel) = env();
    let patches = details::file_diff(
        &executor,
        &repo.path,
        &DiffTarget::Untracked {
            path: "brand new.txt".to_string(),
        },
        &cancel,
    )
    .await
    .unwrap();
    assert_eq!(patches.len(), 1);
    let lines = &patches[0].hunks[0].lines;
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|l| l.kind == DiffLineKind::Addition));
    assert_eq!(lines[0].text, "line 1");
}

#[tokio::test]
async fn binary_file_diff_is_flagged() {
    let mut repo = TestRepo::init();
    repo.commit_file("t.txt", "x\n", "base");
    std::fs::write(repo.path.join("blob.bin"), [0u8, 159, 146, 150, 0, 1, 2]).unwrap();
    repo.git(&["add", "--", "blob.bin"]);
    repo.git(&["commit", "-m", "add binary"]);
    let sha = repo.git(&["rev-parse", "HEAD"]);
    let parent = repo.git(&["rev-parse", "HEAD^"]);

    let (executor, cancel) = env();
    let patches = details::file_diff(
        &executor,
        &repo.path,
        &DiffTarget::Commit {
            oid: Oid::from_hex_str(&sha).unwrap(),
            parent: Some(Oid::from_hex_str(&parent).unwrap()),
            path: "blob.bin".to_string(),
            orig_path: None,
        },
        &cancel,
    )
    .await
    .unwrap();
    assert_eq!(patches.len(), 1);
    assert!(patches[0].is_binary);
    assert!(patches[0].hunks.is_empty());
}

/// Stops a merge on four kinds of conflict at once: both changed it
/// (`UU`), both added it (`AA`), we deleted / they changed (`DU`), and we
/// changed / they deleted (`UD`). The first two have two blobs to compare
/// and get a combined diff; the other two have one and get a bare
/// `* Unmerged path` line.
fn stopped_merge_of_four_kinds(repo: &mut TestRepo) {
    repo.commit_file("both.txt", "one\ntwo\nthree\n", "base");
    repo.commit_file("ours-del.txt", "base\n", "one we will drop");
    repo.commit_file("theirs-del.txt", "base\n", "one they will drop");

    repo.git(&["checkout", "-b", "side"]);
    repo.write_file("both.txt", "one\nTHEIRS\nthree\n");
    repo.write_file("ours-del.txt", "they keep editing\n");
    repo.write_file("added.txt", "their new file\n");
    std::fs::remove_file(repo.path.join("theirs-del.txt")).unwrap();
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-m", "the other side of all four"]);

    repo.git(&["checkout", "main"]);
    repo.write_file("both.txt", "one\nOURS\nthree\n");
    repo.write_file("theirs-del.txt", "we keep editing\n");
    repo.write_file("added.txt", "our new file\n");
    std::fs::remove_file(repo.path.join("ours-del.txt")).unwrap();
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-m", "our side of all four"]);

    repo.git_expect_failure(&["merge", "--no-edit", "side"]);
}

/// One stopped merge, all four kinds read back from it: the two-blob
/// conflicts (`UU`, `AA`) come out combined, the one-sided pair
/// (`DU`, `UD`) has nothing to diff and says so.
#[tokio::test]
async fn a_stopped_merge_reads_back_all_four_conflict_kinds() {
    let mut repo = TestRepo::init();
    stopped_merge_of_four_kinds(&mut repo);

    let (executor, cancel) = env();

    // UU — both changed it: the full combined shape, markers and all.
    let patches = details::file_diff(
        &executor,
        &repo.path,
        &DiffTarget::Unstaged {
            path: "both.txt".to_string(),
        },
        &cancel,
    )
    .await
    .unwrap();
    assert_eq!(patches.len(), 1);
    let patch = &patches[0];
    assert!(patch.is_combined, "a conflicted path has two old sides");
    assert!(!patch.unmerged, "this one does have a patch");
    assert_eq!(patch.path(), "both.txt");
    assert_eq!(patch.hunks.len(), 1);
    let hunk = &patch.hunks[0];
    assert_eq!(
        hunk.extra_old.len(),
        1,
        "one range per parent after the first"
    );

    // Every marker column git wrote, in order. This is the whole point of
    // the parse: without it the pane has nothing to show on the one file
    // someone opened it for.
    let markers: Vec<&str> = hunk.lines.iter().map(|l| l.markers.as_str()).collect();
    assert_eq!(
        markers,
        vec!["  ", "++", " +", "++", "+ ", "++", "  "],
        "context, then our side and theirs fenced by the markers git left"
    );
    let text: Vec<&str> = hunk.lines.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(text[2], "OURS");
    assert_eq!(text[4], "THEIRS");
    assert!(text[1].starts_with("<<<<<<<"));
    assert!(text[3].starts_with("======="));
    assert!(text[5].starts_with(">>>>>>>"));

    // The result's numbering runs unbroken down the pane — it is the file
    // on disk, markers and all.
    let result: Vec<Option<u32>> = hunk.lines.iter().map(|l| l.new_no).collect();
    assert_eq!(
        result,
        vec![
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
            Some(7)
        ]
    );
    // Our side's numbering skips what only the other side (or neither) has.
    let ours: Vec<Option<u32>> = hunk.lines.iter().map(|l| l.old_no).collect();
    assert_eq!(
        ours,
        vec![Some(1), None, Some(2), None, None, None, Some(3)]
    );

    // AA — both added it: combined too, and above all not read as a new
    // file (both sides invented the path, so neither is "the old side" —
    // being read as new would take the pane's pieces away for the wrong
    // reason).
    let patches = details::file_diff(
        &executor,
        &repo.path,
        &DiffTarget::Unstaged {
            path: "added.txt".to_string(),
        },
        &cancel,
    )
    .await
    .unwrap();
    let patch = &patches[0];
    assert!(patch.is_combined);
    assert!(patch.old_path.is_some());
    assert!(patch.new_path.is_some());
    assert!(!patch.hunks.is_empty());

    // DU / UD — one side left: named but not diffed.
    for path in ["ours-del.txt", "theirs-del.txt"] {
        let patches = details::file_diff(
            &executor,
            &repo.path,
            &DiffTarget::Unstaged {
                path: path.to_string(),
            },
            &cancel,
        )
        .await
        .unwrap();
        assert_eq!(patches.len(), 1, "{path}");
        assert!(patches[0].unmerged, "{path} is named but not diffed");
        assert!(!patches[0].is_combined, "{path}");
        assert!(patches[0].hunks.is_empty(), "{path}");
        assert_eq!(patches[0].path(), path);
    }
}
