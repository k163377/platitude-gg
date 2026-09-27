//! Presets about tags: every state one can be in against a remote, and
//! enough of them to measure a wall of them with.

use super::basic::basic;
use super::repo::{DemoRepo, file_url};

/// Every state a tag can be in against the remote, so the badge and the
/// name colour can be read side by side
/// (デザイン規約 §グラフ行のダブルクリック). Nothing shows until a fetch
/// has run `ls-remote --tags` — the one the window fires on opening, or
/// the `fetch` verb's.
///
/// | tag          | here          | on origin                    |
/// |--------------|---------------|------------------------------|
/// | `v1.0`       | second commit | the same commit              |
/// | `v2.0-local` | HEAD          | nowhere                      |
/// | `v1.5`       | HEAD          | the second commit — a drift  |
/// | `v0.9-theirs`| —             | a commit no branch there has |
///
/// `v0.9-theirs` is pushed by the seeder on a branch deleted straight
/// after, so no fetch brings it down: auto-following only takes tags whose
/// commits it downloads.
pub(super) fn tags(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# tags\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.git(&["tag", "-a", "v1.0", "-m", "first release"])?;
    repo.git(&["tag", "v1.5"])?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.git(&["push", "origin", "v1.0", "v1.5"])?;

    repo.commit("src/app.txt", "app v2\n", "feat: rework the app")?;
    // Never pushed…
    repo.git(&["tag", "v2.0-local"])?;
    // …and one moved here after it was published, which no fetch undoes.
    repo.git(&["tag", "-f", "v1.5"])?;

    let seeder = repo.root.join("seeder");
    let url = file_url(&repo.root.join("origin.git"));
    let root = repo.root.clone();
    repo.git_at(&root, &["clone", &url, "seeder"])?;
    for (key, value) in [
        ("user.name", "Away Colleague"),
        ("user.email", "away@example.com"),
    ] {
        repo.git_at(&seeder.clone(), &["config", key, value])?;
    }
    repo.git_at(&seeder.clone(), &["switch", "--create", "gone"])?;
    std::fs::write(seeder.join("side.txt"), "theirs\n").map_err(|e| e.to_string())?;
    repo.git_at(&seeder.clone(), &["add", "--", "side.txt"])?;
    repo.git_at(&seeder.clone(), &["commit", "-m", "feat: their side"])?;
    repo.git_at(&seeder.clone(), &["tag", "v0.9-theirs"])?;
    repo.git_at(&seeder.clone(), &["push", "origin", "gone", "v0.9-theirs"])?;
    repo.git_at(&seeder.clone(), &["push", "origin", "--delete", "gone"])?;
    Ok(())
}

/// The same states across three remotes: a tag's row opens on one line per
/// remote (デザイン規約 §左メニューの所作).
///
/// | tag           | here          | on origin        | on fork       | on mirror |
/// |---------------|---------------|------------------|---------------|-----------|
/// | `v1.0`        | second commit | the same commit  | the same one  | —         |
/// | `v1.5`        | HEAD          | the second one   | HEAD          | —         |
/// | `v2.0-local`  | HEAD          | nowhere          | nowhere       | —         |
/// | `v0.9-theirs` | —             | nowhere          | a commit no branch there has | — |
/// | `v3.0-pair`   | —             | nowhere          | that commit   | that commit |
/// | `v3.1-moved`  | HEAD          | nowhere          | second commit | second commit |
/// | `v3.2-split`  | second commit | nowhere          | second commit | HEAD      |
///
/// `v1.5` is why this preset exists: the one row where a remote that
/// agrees and one that has the name elsewhere stand under one name. The
/// `v3` names are origin's silence: two remotes agreeing decide
/// (`v3.1-moved` — the copy here is the odd one out), two disagreeing
/// leave every holder apart (`v3.2-split`), and two carrying a name with
/// no reference among them leave a menu no remote to reach unasked
/// (`v3.0-pair`; `v0.9-theirs` has the one). As with `tags`, nothing shows
/// until a fetch, so its verb asks for one first (`nav-open-tag`).
pub(super) fn tagremotes(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# tags\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.git(&["tag", "-a", "v1.0", "-m", "first release"])?;
    repo.git(&["tag", "v1.5"])?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.git(&["push", "origin", "v1.0", "v1.5"])?;

    // A second remote with the same two names on it.
    let fork = repo.root.join("fork.git");
    std::fs::create_dir_all(&fork).map_err(|e| e.to_string())?;
    repo.git_at(&fork.clone(), &["init", "--bare", "-b", "main"])?;
    let fork_url = file_url(&fork);
    repo.git(&["remote", "add", "fork", &fork_url])?;
    repo.git(&["push", "fork", "main"])?;
    repo.git(&["push", "fork", "v1.0", "v1.5"])?;

    repo.commit("src/app.txt", "app v2\n", "feat: rework the app")?;
    // Never sent anywhere…
    repo.git(&["tag", "v2.0-local"])?;
    // …and one moved after it was published, then force-pushed to the fork
    // only: HEAD here and on the fork, the old commit on origin.
    repo.git(&["tag", "-f", "v1.5"])?;
    repo.git(&["push", "fork", "main"])?;
    repo.git(&["push", "--force", "fork", "refs/tags/v1.5:refs/tags/v1.5"])?;

    // A name only the fork has, on a branch deleted straight after, as in
    // `tags`.
    let seeder = repo.root.join("seeder");
    let root = repo.root.clone();
    repo.git_at(&root, &["clone", &fork_url, "seeder"])?;
    for (key, value) in [
        ("user.name", "Away Colleague"),
        ("user.email", "away@example.com"),
    ] {
        repo.git_at(&seeder.clone(), &["config", key, value])?;
    }
    repo.git_at(&seeder.clone(), &["switch", "--create", "gone"])?;
    std::fs::write(seeder.join("side.txt"), "theirs\n").map_err(|e| e.to_string())?;
    repo.git_at(&seeder.clone(), &["add", "--", "side.txt"])?;
    repo.git_at(&seeder.clone(), &["commit", "-m", "feat: their side"])?;
    repo.git_at(&seeder.clone(), &["tag", "v0.9-theirs"])?;
    repo.git_at(&seeder.clone(), &["push", "origin", "gone", "v0.9-theirs"])?;
    repo.git_at(&seeder.clone(), &["push", "origin", "--delete", "gone"])?;

    // A third remote, and the names origin says nothing of.
    let mirror = repo.root.join("mirror.git");
    std::fs::create_dir_all(&mirror).map_err(|e| e.to_string())?;
    repo.git_at(&mirror.clone(), &["init", "--bare", "-b", "main"])?;
    let mirror_url = file_url(&mirror);
    repo.git(&["remote", "add", "mirror", &mirror_url])?;
    repo.git(&["push", "mirror", "main"])?;
    // On the seeder's commit as well, so no fetch brings it down either.
    repo.git_at(&seeder.clone(), &["remote", "add", "mirror", &mirror_url])?;
    repo.git_at(&seeder.clone(), &["tag", "v3.0-pair"])?;
    for remote in ["origin", "mirror"] {
        repo.git_at(&seeder.clone(), &["push", remote, "gone", "v3.0-pair"])?;
        repo.git_at(&seeder.clone(), &["push", remote, "--delete", "gone"])?;
    }
    let second = repo.git(&["rev-parse", "HEAD~1"])?;
    // Both remotes on the second commit, the copy here moved on to HEAD.
    repo.git(&["tag", "v3.1-moved", second.trim()])?;
    repo.git(&["push", "fork", "refs/tags/v3.1-moved"])?;
    repo.git(&["push", "mirror", "refs/tags/v3.1-moved"])?;
    repo.git(&["tag", "-f", "v3.1-moved"])?;
    // Mirror on HEAD; fork and the copy here on the second commit.
    repo.git(&["tag", "v3.2-split"])?;
    repo.git(&["push", "mirror", "refs/tags/v3.2-split"])?;
    repo.git(&["tag", "-f", "v3.2-split", second.trim()])?;
    repo.git(&["push", "fork", "refs/tags/v3.2-split"])?;
    Ok(())
}

/// How many tags `manytags` puts on: enough that TAGS cannot fit in the
/// pane at any window height.
const MANY_TAGS: usize = 2000;

/// `basic`, buried in tags: the only preset where a folded rail's section
/// outgrows the pane — with a handful of tags the peek is content-sized
/// and every placement rule looks alike (デザイン規約 §左メニューを畳む).
pub(super) fn manytags(repo: &mut DemoRepo) -> Result<(), String> {
    basic(repo)?;
    let head = repo.git(&["rev-parse", "HEAD"])?;
    let mut batch = String::new();
    for n in 0..MANY_TAGS {
        // Flat names on purpose: a `/` in a tag name is a folder in the
        // list, and folded folders are fewer rows than tags.
        batch.push_str(&format!(
            "create refs/tags/v1.{}.{} {head}\n",
            n / 100,
            n % 100
        ));
    }
    repo.git_stdin(&["update-ref", "--stdin"], &batch)?;
    Ok(())
}

/// A commit no branch reaches, held by a tag alone: the one shape where
/// hiding tags from the graph takes a row with them (`tags-eye`). A tag a
/// branch also reaches keeps its row, and a picture of that looks exactly
/// like a toggle that never answered.
///
/// `v1.0` is the control, a tag on the trunk whose row stays; annotated,
/// so the name index peels one tag of each kind (`refs::REFS_FORMAT_ARG`).
///
/// No remote, so opening fires no fetch — nothing here is about one.
pub(super) fn tagonly(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# tags\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.git(&["tag", "-a", "v1.0", "-m", "first release"])?;

    // Written on a detached HEAD and named by the tag alone, so `main`
    // never carries it.
    repo.git(&["switch", "--detach"])?;
    repo.commit(
        "src/kept.txt",
        "kept by a tag\n",
        "feat: keep this by a tag alone",
    )?;
    repo.git(&["tag", "v1.1-kept"])?;

    // The trunk goes on past it, so the row the eye takes out sits
    // inside the graph.
    repo.git(&["switch", "main"])?;
    repo.commit("src/lib.txt", "lib v1\n", "feat: add the library")?;
    Ok(())
}
