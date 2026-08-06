//! `cargo xtask demo-repo` — throwaway repositories in known states.
//!
//! Every invocation builds a fresh repository under the system temp dir
//! (or `--at <dir>`), isolated from the developer's git configuration, and
//! prints its path. Nothing is ever reused: a sandbox whose state has
//! drifted is worse than none, so the sandbox is always newly made.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn run(args: &[String]) -> Result<PathBuf, String> {
    let mut preset: Option<&str> = None;
    let mut at: Option<PathBuf> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--at" => {
                let dir = it.next().ok_or("--at needs a directory")?;
                at = Some(PathBuf::from(dir));
            }
            other if preset.is_none() => preset = Some(other),
            other => return Err(format!("unexpected argument: {other}")),
        }
    }
    let preset = preset.ok_or("demo-repo needs a preset (see `cargo xtask`)")?;
    create(preset, at)
}

/// Builds `preset` and returns the work-tree path.
pub fn create(preset: &str, at: Option<PathBuf>) -> Result<PathBuf, String> {
    let root = match at {
        Some(dir) => dir,
        None => {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos();
            std::env::temp_dir()
                .join("pg-demo")
                .join(format!("{preset}-{nanos}"))
        }
    };
    let mut repo = DemoRepo::init(&root)?;
    match preset {
        "basic" => basic(&mut repo)?,
        "dirty" => dirty(&mut repo)?,
        "conflict" => conflict(&mut repo)?,
        "stashes" => stashes(&mut repo)?,
        "detached" => detached(&mut repo)?,
        "behind" => behind(&mut repo)?,
        "empty" => {}
        other => return Err(format!("unknown preset: {other}")),
    }
    Ok(repo.work)
}

/// A repository under construction. Commits get ascending timestamps
/// (past → now) so the graph looks like history, not like a fixture.
struct DemoRepo {
    root: PathBuf,
    work: PathBuf,
    global_config: PathBuf,
    base_epoch: u64,
    tick: u64,
}

/// Spacing between commit timestamps.
const TICK_SECS: u64 = 30 * 60;
/// How far in the past the history starts.
const HISTORY_SECS: u64 = 40 * 60 * 60;

impl DemoRepo {
    fn init(root: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(root).map_err(|e| format!("creating {}: {e}", root.display()))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        let mut repo = Self {
            root: root.to_path_buf(),
            work: root.join("repo"),
            global_config: root.join("no-global-config"),
            base_epoch: now.saturating_sub(HISTORY_SECS),
            tick: 0,
        };
        std::fs::create_dir_all(&repo.work).map_err(|e| e.to_string())?;
        repo.git(&["init", "-b", "main"])?;
        for (key, value) in [
            ("user.name", "Demo User"),
            ("user.email", "demo@example.com"),
            ("commit.gpgsign", "false"),
            ("tag.gpgSign", "false"),
            ("core.autocrlf", "false"),
        ] {
            repo.git(&["config", key, value])?;
        }
        Ok(repo)
    }

    fn command(&mut self, dir: &Path, args: &[&str]) -> Command {
        self.tick += 1;
        let stamp = format!("{} +0000", self.base_epoch + self.tick * TICK_SECS);
        let mut cmd = Command::new("git");
        cmd.args(args)
            .current_dir(dir)
            .env("GIT_CONFIG_GLOBAL", &self.global_config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_AUTHOR_DATE", &stamp)
            .env("GIT_COMMITTER_DATE", &stamp);
        cmd
    }

    fn git(&mut self, args: &[&str]) -> Result<String, String> {
        let dir = self.work.clone();
        self.git_at(&dir, args)
    }

    fn git_at(&mut self, dir: &Path, args: &[&str]) -> Result<String, String> {
        let out = crate::run_captured(&mut self.command(dir, args))?;
        if !out.status.success() {
            return Err(format!(
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// For commands whose non-zero exit is the state we want (a merge
    /// stopping on conflicts). Only a spawn failure is an error.
    fn git_expecting_stop(&mut self, args: &[&str]) -> Result<(), String> {
        let dir = self.work.clone();
        crate::run_captured(&mut self.command(&dir, args)).map(drop)
    }

    fn write(&self, rel: &str, content: &str) -> Result<(), String> {
        let path = self.work.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, content).map_err(|e| format!("writing {rel}: {e}"))
    }

    fn commit(&mut self, rel: &str, content: &str, message: &str) -> Result<(), String> {
        self.write(rel, content)?;
        self.git(&["add", "--", rel])?;
        self.git(&["commit", "-m", message])?;
        Ok(())
    }

    /// Creates `<root>/origin.git` (bare), wires it as `origin`.
    fn add_origin(&mut self) -> Result<(), String> {
        let bare = self.root.join("origin.git");
        std::fs::create_dir_all(&bare).map_err(|e| e.to_string())?;
        self.git_at(&bare.clone(), &["init", "--bare", "-b", "main"])?;
        let url = file_url(&bare);
        self.git(&["remote", "add", "origin", &url])?;
        Ok(())
    }
}

fn file_url(path: &Path) -> String {
    let mut p = path.to_string_lossy().replace('\\', "/");
    if !p.starts_with('/') {
        p.insert(0, '/');
    }
    format!("file://{p}")
}

/// Branches, a remote one commit behind, tags in both places, a stash,
/// and a dirty working tree — the state most verbs can act on.
fn basic(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nA repository for kicking tires.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.commit("src/lib.txt", "lib v1\n", "feat: add the library")?;
    repo.commit(
        "src/app.txt",
        "app v1\nwith settings\n",
        "feat: read settings",
    )?;
    repo.git(&["tag", "-a", "v0.1", "-m", "first cut"])?;
    repo.commit("docs/guide.md", "guide v1\n", "docs: add a guide")?;
    repo.commit("src/lib.txt", "lib v2\n", "fix: harden the library")?;

    repo.git(&["switch", "--create", "feature/topic-a"])?;
    repo.commit("src/topic.txt", "topic draft\n", "feat: draft the topic")?;
    repo.commit("src/topic.txt", "topic ready\n", "feat: finish the topic")?;
    repo.git(&["switch", "main"])?;

    repo.commit(
        "src/app.txt",
        "app v2\nwith settings\n",
        "feat: rework the app",
    )?;
    repo.git(&["tag", "v0.2"])?;

    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.git(&["push", "origin", "feature/topic-a", "v0.1", "v0.2"])?;

    // A branch that exists only on the remote: push it, drop the local.
    repo.git(&["switch", "--create", "feature/remote-only", "main"])?;
    repo.commit("src/remote.txt", "remote work\n", "feat: work kept remote")?;
    repo.git(&["push", "origin", "feature/remote-only"])?;
    repo.git(&["switch", "main"])?;
    repo.git(&["branch", "-D", "feature/remote-only"])?;

    // Ahead of origin/main by one, so push has something to do.
    repo.commit("docs/guide.md", "guide v2\n", "docs: extend the guide")?;
    // A tag only this clone has.
    repo.git(&["tag", "v0.3-local"])?;

    // One stash…
    repo.write("src/lib.txt", "lib v2\nstashed experiment\n")?;
    repo.git(&["stash", "push", "-m", "experiment on the library"])?;
    // …and a dirty working tree: staged, unstaged, untracked.
    repo.write("src/app.txt", "app v2\nwith settings\nstaged line\n")?;
    repo.git(&["add", "--", "src/app.txt"])?;
    repo.write("docs/guide.md", "guide v2\nunstaged line\n")?;
    repo.write("notes.txt", "untracked scratch\n")?;
    Ok(())
}

/// Every WIP bucket at once: staged, unstaged, staged+unstaged on one
/// file, a staged rename, and an untracked file.
fn dirty(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("a.txt", "a v1\n", "feat: a")?;
    repo.commit("b.txt", "b v1\n", "feat: b")?;
    repo.commit("c.txt", "c v1\n", "feat: c")?;
    repo.commit(
        "renamed-from.txt",
        "moves around\n",
        "feat: file that moves",
    )?;

    repo.write("a.txt", "a v1\nstaged\n")?;
    repo.git(&["add", "--", "a.txt"])?;
    repo.write("b.txt", "b v1\nunstaged\n")?;
    repo.write("c.txt", "c v1\nstaged\n")?;
    repo.git(&["add", "--", "c.txt"])?;
    repo.write("c.txt", "c v1\nstaged\nand unstaged on top\n")?;
    repo.git(&["mv", "renamed-from.txt", "renamed-to.txt"])?;
    repo.write("untracked.txt", "new file\n")?;
    Ok(())
}

/// A merge stopped on conflicts: MERGE_HEAD present, one unmerged path.
fn conflict(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("shared.txt", "base\n", "feat: shared base")?;
    repo.commit("other.txt", "calm\n", "feat: untouched elsewhere")?;
    repo.git(&["switch", "--create", "feature/clash"])?;
    repo.commit("shared.txt", "feature side\n", "feat: change shared")?;
    repo.git(&["switch", "main"])?;
    repo.commit("shared.txt", "main side\n", "fix: change shared too")?;
    repo.git_expecting_stop(&["merge", "--no-edit", "feature/clash"])?;
    Ok(())
}

/// Three stashes; the newest carries an untracked file.
fn stashes(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("work.txt", "v1\n", "feat: work")?;
    repo.commit("other.txt", "v1\n", "feat: other")?;
    repo.write("work.txt", "v1\nfirst try\n")?;
    repo.git(&["stash", "push", "-m", "first try"])?;
    repo.write("other.txt", "v1\nsecond angle\n")?;
    repo.git(&["stash", "push", "-m", "second angle"])?;
    repo.write("work.txt", "v1\nthird pass\n")?;
    repo.write("sketch.txt", "untracked sketch\n")?;
    repo.git(&[
        "stash",
        "push",
        "--include-untracked",
        "-m",
        "third, with a sketch",
    ])?;
    Ok(())
}

/// HEAD detached at a tagged commit.
fn detached(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("a.txt", "v1\n", "feat: one")?;
    repo.commit("a.txt", "v2\n", "feat: two")?;
    repo.commit("a.txt", "v3\n", "feat: three")?;
    repo.git(&["tag", "v1.0", "HEAD~1"])?;
    repo.commit("a.txt", "v4\n", "feat: four")?;
    repo.git(&["switch", "--detach", "v1.0"])?;
    Ok(())
}

/// The remote holds commits a fetch would bring in (made by a second
/// clone). This repo has not fetched yet.
fn behind(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("a.txt", "v1\n", "feat: one")?;
    repo.commit("a.txt", "v2\n", "feat: two")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;

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
    std::fs::write(seeder.join("a.txt"), "v2\nremote work\n").map_err(|e| e.to_string())?;
    repo.git_at(&seeder.clone(), &["add", "--", "a.txt"])?;
    repo.git_at(
        &seeder.clone(),
        &["commit", "-m", "feat: pushed while you slept"],
    )?;
    repo.git_at(&seeder.clone(), &["push"])?;
    Ok(())
}
