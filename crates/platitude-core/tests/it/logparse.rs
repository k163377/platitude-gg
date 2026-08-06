//! Validates the log format + streaming parser against real `git log`
//! output, including the `-z` record-terminator behavior.

use crate::support::TestRepo;
use platitude_core::parse::log::{LOG_FORMAT_ARG, LogParser};
use platitude_core::{GitCommand, GitExecutor};
use tokio_util::sync::CancellationToken;

/// Builds: root ── a ── merge(main) with a side branch, plus a unicode
/// subject and an empty-message-ish subject case.
fn scenario() -> (TestRepo, Vec<String>) {
    let mut repo = TestRepo::init();
    let root = repo.commit_file("f.txt", "0\n", "root commit");
    let a = repo.commit_file("f.txt", "1\n", "日本語のメッセージ 🎌 with spaces");
    repo.git(&["checkout", "-b", "side", &root]);
    let b = repo.commit_file("g.txt", "s\n", "side work");
    repo.git(&["checkout", "main"]);
    repo.git(&["merge", "--no-ff", "-m", "merge side into main", "side"]);
    let m = repo.git(&["rev-parse", "HEAD"]);
    (repo, vec![m, b, a, root])
}

#[tokio::test]
async fn real_git_log_streams_through_the_parser() {
    let (repo, shas) = scenario();

    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
    let cmd = GitCommand::new().cwd(&repo.path).args([
        "log",
        "-z",
        "--topo-order",
        LOG_FORMAT_ARG,
        "--all",
    ]);

    let mut parser = LogParser::new();
    let mut commits = Vec::new();
    let mut parse_err = None;
    executor
        .run_streaming(cmd, &cancel, &mut |chunk| {
            if parse_err.is_none()
                && let Err(e) = parser.feed(chunk, &mut commits)
            {
                parse_err = Some(e);
            }
        })
        .await
        .unwrap();
    assert_eq!(parse_err, None);
    parser.finish().unwrap();

    assert_eq!(commits.len(), 4);
    let got: Vec<String> = commits.iter().map(|c| c.oid.to_hex()).collect();
    let mut got_sorted = got.clone();
    let mut expected_sorted = shas.clone();
    got_sorted.sort();
    expected_sorted.sort();
    assert_eq!(got_sorted, expected_sorted);

    // The property the lane engine relies on: children before parents.
    let index_of = |hex: &str| got.iter().position(|g| g == hex);
    for (i, c) in commits.iter().enumerate() {
        for p in &c.parents {
            let pi = index_of(&p.to_hex()).expect("parent present in --all log");
            assert!(pi > i, "parent {} must come after child {}", p, c.oid);
        }
    }

    let merge = &commits[0];
    assert_eq!(merge.parents.len(), 2);
    assert_eq!(&*merge.subject, "merge side into main");
    assert_eq!(parser.pool().get(merge.author), "Test User");

    let unicode = commits
        .iter()
        .find(|c| c.subject.contains("日本語"))
        .expect("unicode subject commit present");
    assert_eq!(&*unicode.subject, "日本語のメッセージ 🎌 with spaces");

    let root = &commits[3];
    assert!(root.parents.is_empty());
    assert!(root.time > 0);
}

#[tokio::test]
async fn parser_handles_tiny_chunks_from_real_output() {
    let (repo, _) = scenario();

    // Capture the raw bytes once, then re-parse with pathological chunking.
    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
    let cmd = GitCommand::new().cwd(&repo.path).args([
        "log",
        "-z",
        "--topo-order",
        LOG_FORMAT_ARG,
        "--all",
    ]);
    let out = executor.run(cmd, &cancel).await.unwrap();

    let mut parser = LogParser::new();
    let mut commits = Vec::new();
    for chunk in out.stdout.chunks(3) {
        parser.feed(chunk, &mut commits).unwrap();
    }
    parser.finish().unwrap();
    assert_eq!(commits.len(), 4);
}
