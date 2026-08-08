//! `cargo xtask demo-repo` — throwaway repositories in known states.
//!
//! Every invocation builds a fresh repository under the system temp dir
//! (or `--at <dir>`), isolated from the developer's git configuration, and
//! prints its path. Nothing is ever reused: a sandbox whose state has
//! drifted is worse than none, so the sandbox is always newly made.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
        "rebase-conflict" => rebase_conflict(&mut repo)?,
        "rebase-staged" => rebase_staged(&mut repo)?,
        "rebase-empty" => rebase_empty(&mut repo)?,
        "cherry-pick-conflict" => cherry_pick_conflict(&mut repo)?,
        "conflict-kinds" => conflict_kinds(&mut repo)?,
        "stashes" => stashes(&mut repo)?,
        "detached" => detached(&mut repo)?,
        "behind" => behind(&mut repo)?,
        "diverged" => diverged(&mut repo)?,
        "unpublished" => unpublished(&mut repo)?,
        "noremote" => noremote(&mut repo)?,
        "signed" => signed(&mut repo)?,
        "tags" => tags(&mut repo)?,
        "manytags" => manytags(&mut repo)?,
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

    /// Runs git with `input` on its standard input. One process for a
    /// batch of refs: a repository of thousands of tags built a `git tag`
    /// at a time is minutes of process spawning on Windows.
    fn git_stdin(&mut self, args: &[&str], input: &str) -> Result<(), String> {
        let dir = self.work.clone();
        let mut child = self
            .command(&dir, args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn git {args:?}: {e}"))?;
        let mut stdin = child.stdin.take().ok_or("git took no standard input")?;
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| format!("writing to git {args:?}: {e}"))?;
        drop(stdin);
        let out = child
            .wait_with_output()
            .map_err(|e| format!("waiting for git {args:?}: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
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
    // Japanese subject and body on purpose: every screenshot of this
    // preset then exercises CJK rendering, and 直 / 骨 make a wrong
    // (Chinese-variant) glyph visible at a glance.
    repo.commit(
        "docs/guide.md",
        "guide v1\n\n## 使い方\n直感的な操作の案内。骨組みだけ先に日本語で書く。\n",
        "docs: 利用案内の骨子を日本語で直す",
    )?;
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
    // …and a dirty working tree: staged, unstaged, untracked. The
    // unstaged edit REPLACES the first line, so the diff's body line 0 is
    // a change — the stage-line / discard-line hooks pick line 0, and a
    // context line there would select nothing.
    repo.write("src/app.txt", "app v2\nwith settings\nstaged line\n")?;
    repo.git(&["add", "--", "src/app.txt"])?;
    repo.write("docs/guide.md", "guide v2, reworded\nunstaged line\n")?;
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
    // Long enough to be read as a file rather than as a line: the diff of
    // something the repository has never seen is all additions under one
    // heading, and one line of it cannot show what that looks like.
    repo.write(
        "untracked.txt",
        "Notes for the next release\n\
         \n\
         - decide what goes in the first tag\n\
         - write the install steps down\n\
         - check the licence headers\n\
         - measure the cold start once more\n\
         \n\
         Nothing here is recorded yet.\n",
    )?;
    Ok(())
}

/// A merge stopped on conflicts: MERGE_HEAD present, one unmerged path.
///
/// The file is long enough to be read as one: a conflicted path's diff is
/// the combined form, which puts context, our side, their side and the
/// markers git wrote in one hunk — a one-line file shows none of that.
/// The two sides also disagree about one line and agree about another, so
/// both the fenced part and the part that merged cleanly are on screen.
fn conflict(repo: &mut DemoRepo) -> Result<(), String> {
    const BASE: &str = "\
Release checklist
=================

- pick the version number
- write the notes
- tag the commit
- upload the archives
";
    repo.commit("shared.txt", BASE, "feat: shared base")?;
    repo.commit("other.txt", "calm\n", "feat: untouched elsewhere")?;

    repo.git(&["switch", "--create", "feature/clash"])?;
    repo.commit(
        "shared.txt",
        &BASE
            .replace("- pick the version number", "- agree the version number")
            .replace(
                "- upload the archives",
                "- upload the archives and checksums",
            ),
        "feat: change shared",
    )?;

    repo.git(&["switch", "main"])?;
    repo.commit(
        "shared.txt",
        &BASE.replace("- pick the version number", "- decide the version number"),
        "fix: change shared too",
    )?;
    repo.git_expecting_stop(&["merge", "--no-edit", "feature/clash"])?;
    Ok(())
}

/// A rebase stopped part-way, which a stopped merge cannot stand in for:
/// it steps (so it counts `1/2` and takes `--skip` / `--quit`), and the
/// two sides swap over — the commit being replayed is "theirs".
fn rebase_conflict(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("shared.txt", "base\n", "feat: shared base")?;
    repo.commit("other.txt", "calm\n", "feat: untouched elsewhere")?;
    repo.git(&["switch", "--create", "feature/clash"])?;
    repo.commit("shared.txt", "feature side\n", "feat: change shared")?;
    // A second commit behind the conflicting one, so the count has
    // somewhere to go and `--skip` has a next step to move to.
    repo.commit("later.txt", "after\n", "feat: one more on the branch")?;
    repo.git(&["switch", "main"])?;
    repo.commit("shared.txt", "main side\n", "fix: change shared too")?;
    repo.git(&["switch", "feature/clash"])?;
    repo.git_expecting_stop(&["rebase", "main"])?;
    Ok(())
}

/// A branch that has never been sent anywhere, in a repository with two
/// remotes — so the question the first push raises has something to pick
/// between, and a name on the far side to run into.
///
/// `taken` exists on `origin` one commit behind us, which is exactly the
/// case git will not refuse: the push fast-forwards somebody else's
/// branch. The checked-out branch has no upstream at all.
fn unpublished(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;

    // A second place to send things, so the chooser has a choice to make.
    let fork = repo.root.join("fork.git");
    std::fs::create_dir_all(&fork).map_err(|e| e.to_string())?;
    repo.git_at(&fork.clone(), &["init", "--bare", "-b", "main"])?;
    let fork_url = file_url(&fork);
    repo.git(&["remote", "add", "fork", &fork_url])?;

    // Somebody else's branch, already over there and behind us.
    repo.git(&["switch", "--create", "taken"])?;
    repo.commit("src/app.txt", "app v2\n", "feat: theirs")?;
    // Pushed without `-u`, so nothing here records that it went anywhere.
    repo.git(&["push", "origin", "taken"])?;

    repo.git(&["switch", "main"])?;
    repo.git(&["switch", "--create", "feature/new-thing"])?;
    repo.commit("src/new.txt", "new\n", "feat: draft the new thing")?;
    repo.commit("src/new.txt", "new v2\n", "feat: finish the new thing")?;
    Ok(())
}

/// Commits and nowhere to send them: no remote at all. The first push
/// cannot even pick a destination here — the remote dialog opens by
/// itself, name prefilled `origin`, with the question standing behind it.
fn noremote(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    Ok(())
}

/// The same rebase, one step further on: the conflict resolved and
/// staged, so `--continue` is live and the file list has left CONFLICTS.
fn rebase_staged(repo: &mut DemoRepo) -> Result<(), String> {
    rebase_conflict(repo)?;
    repo.write("shared.txt", "main side\nfeature side\n")?;
    repo.git(&["add", "shared.txt"])?;
    Ok(())
}

/// A rebase stopped on a commit that came out empty — git's own
/// `Otherwise, please use 'git rebase --skip'`. Nothing is conflicted or
/// staged, which is what makes leaving this one out cost nothing.
///
/// Reached with `--empty=stop` rather than through an interactive
/// rebase: the state is the same one, and driving `-i` from here would
/// need a sequence editor on the PATH of three operating systems.
fn rebase_empty(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("f.txt", "a\n", "feat: root")?;
    repo.git(&["switch", "--create", "topic"])?;
    repo.commit("f.txt", "a\nX\n", "feat: adds X")?;
    repo.commit("h.txt", "keep\n", "feat: one more on the branch")?;
    repo.git(&["switch", "main"])?;
    // The same net line, arriving with company: not a clean cherry-pick
    // of "adds X", so the cherry-pick filter cannot be what drops it.
    repo.write("f.txt", "a\nX\n")?;
    repo.write("g.txt", "unrelated\n")?;
    repo.git(&["add", "-A"])?;
    repo.git(&["commit", "-m", "fix: X arrives with company"])?;
    repo.git(&["switch", "topic"])?;
    repo.git_expecting_stop(&["rebase", "--empty=stop", "main"])?;
    Ok(())
}

/// A cherry-pick stopped on a conflict: it steps the way a rebase does
/// (so it takes `--skip` and `--quit`) but keeps no count, which is what
/// tells the card's two tests apart.
fn cherry_pick_conflict(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("shared.txt", "base\n", "feat: shared base")?;
    repo.git(&["switch", "--create", "feature/clash"])?;
    repo.commit("shared.txt", "feature side\n", "feat: change shared")?;
    repo.git(&["switch", "main"])?;
    repo.commit("shared.txt", "main side\n", "fix: change shared too")?;
    repo.git_expecting_stop(&["cherry-pick", "feature/clash"])?;
    Ok(())
}

/// A merge stopped on four different kinds of conflict at once, so the
/// rows that name what each side did can be read side by side: both
/// changed it (`UU`), both added it (`AA`), deleted here and changed
/// there (`DU`), changed here and deleted there (`UD`).
fn conflict_kinds(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("both.txt", "base\n", "feat: shared base")?;
    repo.commit("ours-del.txt", "base\n", "feat: one we will drop")?;
    repo.commit("theirs-del.txt", "base\n", "feat: one they will drop")?;

    repo.git(&["switch", "--create", "feature/clash"])?;
    repo.write("both.txt", "feature side\n")?;
    repo.write("ours-del.txt", "feature keeps editing\n")?;
    repo.write("added.txt", "feature's new file\n")?;
    std::fs::remove_file(repo.work.join("theirs-del.txt")).map_err(|e| e.to_string())?;
    repo.git(&["add", "-A"])?;
    repo.git(&["commit", "-m", "feat: the feature side of all four"])?;

    repo.git(&["switch", "main"])?;
    repo.write("both.txt", "main side\n")?;
    repo.write("theirs-del.txt", "main keeps editing\n")?;
    repo.write("added.txt", "main's new file\n")?;
    std::fs::remove_file(repo.work.join("ours-del.txt")).map_err(|e| e.to_string())?;
    repo.git(&["add", "-A"])?;
    repo.git(&["commit", "-m", "fix: the main side of all four"])?;

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

/// Both sides moved on, and this repository has already seen it happen:
/// the fetch is part of the preset, so the toolbar offers the overwrite
/// (`push -f`) from the moment the window opens rather than after a verb.
///
/// Break the remote's URL afterwards (`.git/config`) and the overwrite
/// fails without the tracking refs moving — which is how the refused shape
/// of that button gets photographed.
fn diverged(repo: &mut DemoRepo) -> Result<(), String> {
    behind(repo)?;
    repo.git(&["fetch", "origin"])?;
    repo.commit("b.txt", "ours\n", "feat: work of our own")?;
    Ok(())
}

/// Every outcome the details pane can show, and a working tree set up to
/// make one more: a commit signed by a key this repository vouches for
/// (`G`), one signed by a key it has never heard of (`U` — measured, not
/// the `E` one might expect), and one not signed at all. SSH signing is
/// what a throwaway repository can do on its own: a passphrase-less key
/// needs no agent, so no pinentry can appear.
fn signed(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("a.txt", "v1\n", "feat: land before signing was on")?;

    let trusted = keygen(repo, "trusted", "demo@example.com")?;
    // Made, never vouched for: its public line goes nowhere.
    keygen(repo, "stranger", "stranger@example.com")?;

    // Only the first key is vouched for; the other one is a signature
    // git can read but cannot judge.
    let allowed = repo.root.join("allowed_signers");
    std::fs::write(&allowed, format!("demo@example.com {trusted}"))
        .map_err(|e| format!("writing allowed_signers: {e}"))?;
    for (key, value) in [
        ("gpg.format", "ssh".to_string()),
        ("gpg.ssh.allowedSignersFile", config_path(&allowed)),
        ("commit.gpgsign", "true".to_string()),
    ] {
        repo.git(&["config", key, &value])?;
    }

    let stranger_key = repo.root.join("stranger.pub");
    repo.git(&["config", "user.signingkey", &config_path(&stranger_key)])?;
    repo.commit("b.txt", "v1\n", "feat: signed by a key nobody vouched for")?;

    let trusted_key = repo.root.join("trusted.pub");
    repo.git(&["config", "user.signingkey", &config_path(&trusted_key)])?;
    repo.commit("c.txt", "v1\n", "feat: signed and verified")?;

    // Something to commit, so the editor that says "will be signed" has
    // a reason to be reachable and its button is live.
    repo.write("a.txt", "v1\nabout to be committed\n")?;
    repo.git(&["add", "--", "a.txt"])?;
    Ok(())
}

/// Writes a passphrase-less ed25519 key pair under the demo root and
/// returns the public key's one line.
fn keygen(repo: &DemoRepo, name: &str, comment: &str) -> Result<String, String> {
    let path = repo.root.join(name);
    let out = crate::run_captured(
        Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-N", "", "-C", comment, "-f"])
            .arg(&path),
    )?;
    if !out.status.success() {
        return Err(format!(
            "ssh-keygen failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    std::fs::read_to_string(path.with_extension("pub"))
        .map(|k| k.trim().to_string())
        .map_err(|e| format!("reading {name}.pub: {e}"))
}

/// git reads config values with its own parser, which takes forward
/// slashes on every platform.
fn config_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Every state a tag can be in with respect to the remote, so the badge
/// and the name colour can be read side by side (デザイン規約 §グラフ行の
/// チップ). Nothing shows until a fetch: `ls-remote --tags` is what carries
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
fn tags(repo: &mut DemoRepo) -> Result<(), String> {
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

/// How many tags `manytags` puts on. Enough that TAGS cannot fit in the
/// pane at any window height anybody works at — which is the whole point
/// of the preset, and the shape a release-tagging repository really has
/// (the reference repository carries 45,000).
const MANY_TAGS: usize = 2000;

/// `basic`, buried in tags. What the folded rail does when a section has
/// more rows than the pane is tall can only be read here: with a handful
/// of tags the peek is content-sized and every placement rule looks alike
/// (デザイン規約 §左メニューを畳む).
fn manytags(repo: &mut DemoRepo) -> Result<(), String> {
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
    repo.git_stdin(&["update-ref", "--stdin"], &batch)
}
