//! Presets about tags: every state one can be in against a remote, and
//! enough of them to measure a wall of them with.

use super::basic::basic;
use super::repo::{DemoRepo, file_url};

/// Every state a tag can be in with respect to the remote, so the badge
/// and the name colour can be read side by side (デザイン規約 §グラフ行の
/// ダブルクリック). Nothing shows until a fetch: `ls-remote --tags` is what carries
/// it, so run this preset with the `fetch` verb.
///
/// | tag          | here          | on origin                    |
/// |--------------|---------------|------------------------------|
/// | `v1.0`       | second commit | the same commit              |
/// | `v2.0-local` | HEAD          | nowhere                      |
/// | `v1.5`       | HEAD          | the second commit — a drift  |
/// | `v0.9-theirs`| —             | a commit no branch there has |
///
/// `v0.9-theirs` comes from the seeder on a branch deleted straight after,
/// which is what keeps a fetch from quietly bringing the tag down with it
/// (measured: auto-following only takes tags whose commits it downloads).
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

/// The same states read across **two** remotes, which is what a tag's row
/// opens on: the name is one row however many of them carry it, and the
/// lines under it are one to a remote (デザイン規約 §左メニューの所作).
///
/// | tag           | here          | on origin        | on fork       |
/// |---------------|---------------|------------------|---------------|
/// | `v1.0`        | second commit | the same commit  | the same one  |
/// | `v1.5`        | HEAD          | the second one   | HEAD          |
/// | `v2.0-local`  | HEAD          | nowhere          | nowhere       |
/// | `v0.9-theirs` | —             | nowhere          | a commit no branch there has |
///
/// **`v1.5` is why this preset exists**: its row is the one place where a
/// carrier that agrees and a carrier that has the name somewhere else
/// stand under one name, and the two lines have to be told apart there or
/// nowhere. `--preset tags` has one remote, so every row of it is a list
/// of one.
///
/// Nothing shows until a fetch — `ls-remote --tags` is the only carrier
/// (core.md タグのリモート状態) — so the run that reads this preset asks
/// for one first (PGG_AUTO_ACT=nav-open-tag).
pub(super) fn tagremotes(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# tags\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.git(&["tag", "-a", "v1.0", "-m", "first release"])?;
    repo.git(&["tag", "v1.5"])?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.git(&["push", "origin", "v1.0", "v1.5"])?;

    // A second place with the same two names on it, so a row can open on
    // more than one line.
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
    // …and one moved here after it was published, then sent on to one of
    // the two: the name now stands on three commits' worth of opinion —
    // here and on the fork it is HEAD, on origin it is where it was.
    repo.git(&["tag", "-f", "v1.5"])?;
    repo.git(&["push", "fork", "main"])?;
    repo.git(&["push", "--force", "fork", "refs/tags/v1.5:refs/tags/v1.5"])?;

    // A name only the fork has, on a branch deleted straight after — which
    // is what keeps a fetch from quietly bringing the tag down with it
    // (measured: auto-following only takes tags whose commits it
    // downloads).
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
    Ok(())
}

/// How many tags `manytags` puts on: enough that TAGS cannot fit in the
/// pane at any window height anybody works at.
const MANY_TAGS: usize = 2000;

/// `basic`, buried in tags. What the folded rail does when a section has
/// more rows than the pane is tall can only be read here: with a handful
/// of tags the peek is content-sized and every placement rule looks alike
/// (デザイン規約 §左メニューを畳む).
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

/// A commit no branch reaches, held by a tag alone.
///
/// **The one shape in which taking the tags out of the graph takes a row
/// with them**, which is what `tags-eye` is judged on: the walk reaches
/// `v1.1-kept`'s commit through `refs/tags` and through nothing else, so
/// its row goes on the press and comes back on the second one. A tag a
/// branch also reaches keeps its row through both, and a picture of that
/// frames exactly like a switch that never answered.
///
/// `v1.0` is the other half of that pair — a tag on the trunk, whose row
/// stays — and it is annotated, so the name index has one tag of each
/// kind to peel (`refs::REFS_FORMAT_ARG`).
///
/// No remote: nothing here is about one, and the fetch an opening fires
/// would be the only write in a run that is about a graph.
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
