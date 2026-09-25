//! The longest strings git will hand over, with Japanese in all of them,
//! carried end to end through the readers the UI feeds on. The short end
//! (the table's last row) needs no walk: no parser has a minimum. git's own
//! side is in [`periodic`].
//!
//! The walls, measured on both systems:
//!
//! | what | wall |
//! |---|---|
//! | a ref name's last component | 250 bytes on Linux — a loose ref is the file `<name>.lock` against a 255-byte filename. NTFS counts UTF-16 units and took 301 bytes of kanji, so a repository can hold a ref only Windows can spell |
//! | a path component | 255 bytes on both |
//! | subject, body, author name, address, URL | none. git took a megabyte of each |
//! | the short end | one character of anything, an empty subject, and an empty address — but *not* an empty author name, which git alone refuses |
//!
//! The ref names below stand on that wall and end with a multi-byte
//! character, the byte a slice cuts in half; the walls git does not have
//! are walked with long-but-arbitrary lengths.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::expect_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::parse::log::{LOG_FORMAT_ARG, LogParser};
use platitude_core::refs::{self, RefKind};
use platitude_core::{GitCommand, status};

/// Bytes a ref's last component may have, from the measurement above.
const REF_WALL: usize = 250;

/// `head`, then kanji until the whole is exactly `bytes` long, so the last
/// character ends on the wall.
fn to_the_byte(head: &str, bytes: usize) -> String {
    const FILL: [char; 5] = ['長', 'い', '名', '前', 'の'];
    let mut s = String::from(head);
    let mut i = 0;
    while s.len() + 3 <= bytes {
        s.push(FILL[i % FILL.len()]);
        i += 1;
    }
    while s.len() < bytes {
        s.push('x');
    }
    s
}

/// A paragraph in both scripts, at least `bytes` long.
fn pasted(bytes: usize) -> String {
    let unit = "この行は長い日本語の文章で、折り返しと省略の両方を試すために置いてある。 \
                And an English clause rides along so the run of Latin text is measured too. ";
    let mut s = String::new();
    while s.len() < bytes {
        s.push_str(unit);
    }
    s.push_str("終端");
    s
}

/// Reads the whole log through the parser, the way the graph walk does.
#[expect(
    clippy::unwrap_used,
    reason = "test helper; panicking on setup failure is the point"
)]
async fn walk(repo: &TestRepo) -> (Vec<platitude_core::CommitMeta>, LogParser) {
    let (executor, cancel) = env();
    let cmd = GitCommand::new().cwd(&repo.path).args([
        "log",
        "-z",
        "--date-order",
        LOG_FORMAT_ARG,
        "--all",
    ]);
    let out = executor.run(cmd, &cancel).await.unwrap();
    let mut parser = LogParser::new();
    let mut commits = Vec::new();
    parser.feed(&out.stdout, &mut commits).unwrap();
    parser.finish().unwrap();
    (commits, parser)
}

#[tokio::test]
async fn a_ref_whose_last_character_ends_on_the_wall_comes_back_whole() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "root");
    let branch = to_the_byte("b", REF_WALL);
    let tag = to_the_byte("t", REF_WALL);
    assert_eq!(branch.len(), REF_WALL, "the fixture sits on the wall");
    assert!(
        !branch.is_char_boundary(REF_WALL - 1),
        "and the last character straddles it, which is the point"
    );
    repo.git(&["branch", &branch]);
    repo.git(&["tag", &tag]);

    let (executor, cancel) = env();
    let listed = refs::load(&executor, &repo.path, &cancel)
        .await
        .expect("refs load");

    let found =
        |kind: RefKind, short: &str| listed.iter().any(|r| r.kind == kind && r.short == short);
    assert!(
        found(RefKind::LocalBranch, &branch),
        "the branch reads back"
    );
    assert!(found(RefKind::Tag, &tag), "and so does the tag");
}

/// The credit trailer comes after the long body, so the trailer scan has
/// to reach the end.
#[tokio::test]
async fn a_pasted_paragraph_survives_the_walk_and_the_trailer_after_it() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "root");
    let subject = pasted(2000);
    let body = format!(
        "{}\n\nCo-authored-by: {} <{}@example.com>\n",
        pasted(1500),
        pasted(120),
        to_the_byte("c", 40)
    );
    repo.git(&["commit", "--allow-empty", "-m", &subject, "-m", &body]);

    let (commits, _) = walk(&repo).await;
    let newest = &commits[0];
    assert_eq!(&*newest.subject, subject, "the subject is not trimmed");
    assert!(
        newest.body.contains("終端"),
        "the body keeps its far end: {} bytes",
        newest.body.len()
    );
    assert!(
        !newest.body.contains("Co-authored-by"),
        "and the credit trailer is lifted out of it, however long the body"
    );
}

/// git's side: a path of kanji and spaces through `status::load`, and the
/// empty `%s` / `%aE`. Our parsers read those from the committed `-z`
/// fixture (`status_tests::committed_status_fixture_parses`) and from
/// hand-written bytes (`log_tests::empty_subject_is_preserved`,
/// `log_tests::an_empty_address_is_kept_as_one`) before every merge, so
/// the full gate runs these (`-- --ignored ::periodic::`), not every change.
mod periodic {
    use super::*;

    /// Kanji, spaces and nesting: the shapes a status parser splits on.
    #[tokio::test]
    #[ignore = "duplicates committed_status_fixture_parses: not worth the pre-merge run"]
    async fn a_path_of_kanji_and_spaces_comes_back_as_it_was_written() {
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "1\n", "root");
        let deep = "第一階層/第二階層/第三階層/第四階層/深い場所のファイル.txt";
        let spaced = "名前に 空白 が入る.txt";
        repo.write_file(deep, "deep\n");
        repo.write_file(spaced, "space\n");
        repo.git(&["add", "--", deep]);

        let (executor, cancel) = env();
        let state = status::load(&executor, &repo.path, &cancel)
            .await
            .expect("status load");
        let paths: Vec<&str> = state.items.iter().map(|i| i.path()).collect();
        assert!(paths.contains(&deep), "staged deep path: {paths:?}");
        assert!(paths.contains(&spaced), "untracked spaced path: {paths:?}");
    }

    /// Nothing downstream may treat the empty subject as "no commit".
    #[tokio::test]
    #[ignore = "the empty %s git writes for an empty message: not worth the pre-merge run"]
    async fn a_commit_with_no_message_at_all_reads_back_empty() {
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "1\n", "root");
        repo.git(&["commit", "--allow-empty", "--allow-empty-message", "-m", ""]);

        let (commits, _) = walk(&repo).await;
        assert_eq!(commits.len(), 2);
        assert_eq!(&*commits[0].subject, "", "the empty subject survives");
        assert!(commits[0].time > 0, "and the commit is otherwise ordinary");
    }

    #[tokio::test]
    #[ignore = "the empty %aE git writes for an address nobody gave: not worth the pre-merge run"]
    async fn an_author_may_be_a_paragraph_and_an_address_may_be_nothing() {
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "1\n", "root");
        let name = pasted(300);
        repo.git(&[
            "commit",
            "--allow-empty",
            &format!("--author={name} <>"),
            "-m",
            "chore: 段落の著者",
        ]);

        let (commits, parser) = walk(&repo).await;
        let newest = &commits[0];
        assert_eq!(
            parser.pool().get(newest.author),
            name,
            "the name is handed over whole"
        );
        assert_eq!(
            parser.pool().get(newest.author_email),
            "",
            "and an address nobody wrote reads as nothing, not as a failure"
        );
    }
}
