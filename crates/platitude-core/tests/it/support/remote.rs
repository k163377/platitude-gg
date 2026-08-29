//! Offline "remote" fixtures: a bare repository served over `file://`
//! (実装計画 §11.3).

use crate::support::TestRepo;

/// A bare repository serving as `origin`, plus a working clone of it.
pub fn origin_and_clone() -> (TestRepo, TestRepo) {
    let mut seed = TestRepo::init();
    seed.commit_file("a.txt", "one\n", "root");

    let mut bare = TestRepo::init();
    // Reuse the temp dir machinery, then turn the repo into a bare mirror.
    let bare_path = bare.path.clone();
    bare.git_in(&bare_path, &["config", "core.bare", "true"]);
    seed.git(&["remote", "add", "origin", &bare.file_url()]);
    seed.git(&["push", "origin", "main"]);
    (bare, seed)
}
