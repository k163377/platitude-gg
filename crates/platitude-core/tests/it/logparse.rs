//! What `git log` makes of `.mailmap` under [`LOG_FORMAT_ARG`]. The format's
//! fields are pinned by `parse::log`'s unit tests and every graph test
//! streams real `git log` through the parser; the mailmap fold is git's,
//! so it is [`periodic`].

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::GitCommand;
use platitude_core::parse::log::{LOG_FORMAT_ARG, LogParser};

mod periodic {
    use super::*;

    // The `allow-*-in-tests` clippy options only reach `#[test]` functions.
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

    /// The second address is spelled loudly so the map has to match it as
    /// git does — without regard to case.
    #[tokio::test]
    #[ignore = "git's own mailmap fold: not worth the pre-merge run"]
    async fn the_log_reads_the_authors_through_mailmap() {
        let mut repo = TestRepo::init();
        repo.commit_file("f.txt", "0\n", "before");
        // `--author`, not `-c user.email`: the harness's GIT_AUTHOR_EMAIL
        // beats config, so the commit would carry the harness's address.
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

        // `%ae` is raw even under a mailmap: git keeps the commit's spelling,
        // and lowercasing is the parser's (unit-pinned).
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
        // One pool entry: the key an avatar is filed under.
        assert_eq!(commits[0].author_email, commits[1].author_email);
    }
}
