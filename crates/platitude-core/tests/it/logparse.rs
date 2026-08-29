//! Validates the log format + streaming parser against real `git log`
//! output, including the `-z` record-terminator behavior.

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::GitCommand;
use platitude_core::parse::log::{LOG_FORMAT_ARG, LogParser};

/// Builds: root ── a ── merge(main) with a side branch, plus a unicode
/// subject and an empty-message-ish subject case.
fn scenario() -> (TestRepo, Vec<String>) {
    let mut repo = TestRepo::init();
    let root = repo.commit_file_id("f.txt", "0\n", "root commit");
    let a = repo.commit_file_id("f.txt", "1\n", "日本語のメッセージ 🎌 with spaces");
    repo.git(&["checkout", "-b", "side", &root]);
    let b = repo.commit_file_id("g.txt", "s\n", "side work");
    repo.git(&["checkout", "main"]);
    repo.git(&["merge", "--no-ff", "-m", "merge side into main", "side"]);
    let m = repo.git(&["rev-parse", "HEAD"]);
    (repo, vec![m, b, a, root])
}

#[tokio::test]
async fn real_git_log_streams_through_the_parser() {
    let (repo, shas) = scenario();

    let (executor, cancel) = env();
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

/// Runs the log through the parser and hands back what came out.
// The `allow-*-in-tests` clippy options only reach `#[test]` functions, and
// a helper that cannot read the log it was asked for is the test failing.
#[expect(
    clippy::unwrap_used,
    reason = "test helper; panicking on setup failure is the point"
)]
async fn parse_log(
    repo: &TestRepo,
    extra: &[&str],
) -> (Vec<platitude_core::CommitMeta>, LogParser) {
    let (executor, cancel) = env();
    let mut args: Vec<&str> = vec!["log", "-z", "--date-order", LOG_FORMAT_ARG];
    args.extend_from_slice(extra);
    let cmd = GitCommand::new().cwd(&repo.path).args(args);
    let out = executor.run(cmd, &cancel).await.unwrap();
    let mut parser = LogParser::new();
    let mut commits = Vec::new();
    parser.feed(&out.stdout, &mut commits).unwrap();
    parser.finish().unwrap();
    (commits, parser)
}

/// What `.mailmap` is for, and the reason the format asks for `%aN` /
/// `%aE` rather than the raw pair: one person with two addresses comes
/// back as one person, and the answer is git's rather than a second one
/// of our own.
#[tokio::test]
async fn the_log_reads_the_authors_through_mailmap() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "before");
    // `--author`, not `-c user.email`: the harness pins GIT_AUTHOR_EMAIL
    // for reproducible ids, and that beats configuration — a commit made
    // the other way would quietly carry the harness's address and leave
    // this test agreeing with itself.
    repo.git(&[
        "commit",
        "--allow-empty",
        "--author=Other Name <other@example.com>",
        "-m",
        "under another address",
    ]);
    repo.write_file(
        ".mailmap",
        "Test User <test@example.com> <other@example.com>\n",
    );
    repo.git(&["add", "--", ".mailmap"]);
    repo.git(&["commit", "-m", "add mailmap"]);

    let (commits, parser) = parse_log(&repo, &["--all"]).await;
    let names: Vec<&str> = commits
        .iter()
        .map(|c| parser.pool().get(c.author))
        .collect();
    assert!(
        names.iter().all(|n| *n == "Test User"),
        "every commit reads as the one person: {names:?}"
    );
    let emails: Vec<&str> = commits
        .iter()
        .map(|c| parser.pool().get(c.author_email))
        .collect();
    assert!(
        emails.iter().all(|e| *e == "test@example.com"),
        "and under the one address: {emails:?}"
    );
    // One address, one pool entry — which is what makes it usable as the
    // key a picture is filed under.
    assert_eq!(commits[0].author_email, commits[1].author_email);
}

/// git hands the address back exactly as the commit spelled it — mailmap
/// matches without regard to case but does not rewrite what it did not
/// map. Lowercasing is ours to do, or the same person shouting once would
/// file under a second key.
#[tokio::test]
async fn an_address_is_lowercased_however_the_commit_spelled_it() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "first");
    repo.git(&[
        "commit",
        "--allow-empty",
        "--author=Test User <TEST@Example.COM>",
        "-m",
        "shouting",
    ]);

    let raw = repo.git(&["log", "-1", "--format=%aE"]);
    assert_eq!(raw, "TEST@Example.COM", "git keeps the spelling");

    let (commits, parser) = parse_log(&repo, &[]).await;
    assert_eq!(
        parser.pool().get(commits[0].author_email),
        "test@example.com"
    );
    assert_eq!(
        commits[0].author_email, commits[1].author_email,
        "both spellings land on one key"
    );
}

#[tokio::test]
async fn parser_handles_tiny_chunks_from_real_output() {
    let (repo, _) = scenario();

    // Capture the raw bytes once, then re-parse with pathological chunking.
    let (executor, cancel) = env();
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
