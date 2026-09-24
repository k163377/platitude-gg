//! What `git log` makes of `.mailmap` under [`LOG_FORMAT_ARG`], recorded
//! in [`periodic`]. The pre-merge run holds the rest elsewhere: the format's
//! fields are pinned by `parse::log`'s unit tests, and every graph test
//! streams real `git log` output through the parser.

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::GitCommand;
use platitude_core::parse::log::{LOG_FORMAT_ARG, LogParser};

/// **What the pre-merge run leaves out**: git's mailmap fold of the authors
/// the log reads. The format asks for it (`%aN` / `%aE`, pinned in
/// `parse::log`'s unit tests) and the parser's own case folding is
/// unit-held; the mailmap fold is git's, so the full gate runs it
/// (`-- --ignored ::periodic::`) rather than every change.
mod periodic {
    use super::*;

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
    /// `%aE`: one person with two addresses comes back as one person, and
    /// the answer is git's. The second address is spelled loudly so
    /// the map has to match it the way git matches — without regard
    /// to case.
    #[tokio::test]
    #[ignore = "git's own mailmap fold: not worth the pre-merge run"]
    async fn the_log_reads_the_authors_through_mailmap() {
        let mut repo = TestRepo::init();
        repo.commit_file("f.txt", "0\n", "before");
        // `--author`: the harness pins GIT_AUTHOR_EMAIL for reproducible
        // ids, and that beats configuration — a commit made through
        // `-c user.email` would quietly carry the harness's address and
        // leave this test agreeing with itself.
        repo.git(&[
            "commit",
            "--allow-empty",
            "--author=Other Name <OTHER@Example.COM>",
            "-m",
            "under another address",
        ]);
        repo.write_file(
            ".mailmap",
            "Test User <test@example.com> <other@example.com>\n",
        );
        repo.git(&["add", "--", ".mailmap"]);
        repo.git(&["commit", "-m", "add mailmap"]);

        // `%ae` is the raw address whether a mailmap matches it or not, so
        // this pins what the parser receives: git keeps the commit's own
        // spelling. Lowercasing is the parser's to do, pinned in its unit
        // tests.
        let raw = repo.git(&["log", "--format=%ae", "-1", "HEAD^"]);
        assert_eq!(raw, "OTHER@Example.COM", "git keeps the spelling");

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
}
