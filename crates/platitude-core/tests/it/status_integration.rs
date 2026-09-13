//! The working tree's own shapes, as real git spells them back.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::expect_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::status;

/// A repository sitting inside the working copy is a boundary git will
/// not cross. `-uall` opens every other new directory and lists the files
/// in it; this one comes back as the single entry `nest/`, trailing slash
/// and all, because the files under it are another repository's.
///
/// That slash is the only one a path from `status` ends with, and the
/// file lists cut paths on `/` to file a row under its directories
/// (`platitude_app::models::pathtree`) — so what git answers here decides
/// whether that row is a directory of its own or a folder with a nameless
/// row inside it.
#[tokio::test]
async fn a_repository_inside_the_working_copy_stays_one_entry() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "root");
    // A new directory git does open, for the contrast: its files are
    // listed one by one, and the directory itself is never an entry.
    repo.write_file("plain/loose.txt", "in the same repository\n");
    let nest = repo.path.join("vendor").join("nest");
    std::fs::create_dir_all(&nest).expect("create the nested repository's directory");
    repo.git_in(&nest, &["init", "-b", "main"]);
    repo.write_file("vendor/nest/inside.txt", "another repository's\n");

    let (executor, cancel) = env();
    let state = status::load(&executor, &repo.path, &cancel)
        .await
        .expect("status load");
    let untracked: Vec<&str> = state.untracked().map(|i| i.path()).collect();
    assert_eq!(untracked, ["plain/loose.txt", "vendor/nest/"]);
}
