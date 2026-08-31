//! Offline "remote" fixtures: a bare repository served over `file://`
//! (実装計画 §11.3).

use std::path::PathBuf;

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

/// A `--depth`-limited clone of a fresh linear history: the source, the
/// clone's path, and the commits the clone actually holds, oldest first.
///
/// The first of those is the edge, which git grafts as parentless — `%P`
/// comes back empty and `<edge>~1` exits 1, the same answers the real
/// first commit gives (実測 2.55). The clone sits beside the source's own
/// work tree, so the returned `TestRepo` is what keeps it on disk.
/// `--depth` needs a `file://` URL; git ignores it on a plain local path.
pub fn shallow_clone(commits: usize, depth: usize) -> (TestRepo, PathBuf, Vec<String>) {
    let mut source = TestRepo::init();
    for i in 1..=commits {
        source.commit_file("f.txt", &format!("{i}\n"), &format!("c{i}"));
    }
    let beside = source
        .path
        .parent()
        .expect("the repo sits inside its temp dir")
        .to_path_buf();
    let url = source.file_url();
    source.git_in(
        &beside,
        &["clone", "--depth", &depth.to_string(), &url, "shallow"],
    );
    let clone = beside.join("shallow");
    let held = source.git_in(&clone, &["rev-list", "--reverse", "HEAD"]);
    let held = held.lines().map(str::to_string).collect();
    (source, clone, held)
}
