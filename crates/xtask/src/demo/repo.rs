//! The repository under construction, and the one way every preset talks
//! to git.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// A repository under construction. Commits get ascending timestamps
/// (past → now) so the graph looks like history, not like a fixture.
pub(super) struct DemoRepo {
    pub(super) root: PathBuf,
    pub(super) work: PathBuf,
    global_config: PathBuf,
    base_epoch: u64,
    tick: u64,
}

const TICK_SECS: u64 = 30 * 60;
const HISTORY_SECS: u64 = 40 * 60 * 60;

impl DemoRepo {
    pub(super) fn init(root: &Path, name: &str) -> Result<Self, String> {
        std::fs::create_dir_all(root).map_err(|e| format!("creating {}: {e}", root.display()))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        let mut repo = Self {
            root: root.to_path_buf(),
            work: root.join(name),
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

    pub(super) fn command(&mut self, dir: &Path, args: &[&str]) -> Command {
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

    pub(super) fn git(&mut self, args: &[&str]) -> Result<String, String> {
        let dir = self.work.clone();
        self.git_at(&dir, args)
    }

    pub(super) fn git_at(&mut self, dir: &Path, args: &[&str]) -> Result<String, String> {
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
    pub(super) fn git_expecting_stop(&mut self, args: &[&str]) -> Result<(), String> {
        let dir = self.work.clone();
        crate::run_captured(&mut self.command(&dir, args)).map(drop)
    }

    pub(super) fn write(&self, rel: &str, content: &str) -> Result<(), String> {
        let path = self.work.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, content).map_err(|e| format!("writing {rel}: {e}"))
    }

    pub(super) fn commit(&mut self, rel: &str, content: &str, message: &str) -> Result<(), String> {
        self.write(rel, content)?;
        self.git(&["add", "--", rel])?;
        self.git(&["commit", "-m", message])?;
        Ok(())
    }

    /// Commits something written `earlier` seconds before it was
    /// committed, optionally by somebody other than this repository's
    /// own identity. `--date` beats the `GIT_AUTHOR_DATE` every command
    /// here carries — the committer date keeps the marching stamp, so
    /// the two moments come out apart by exactly what was asked for.
    pub(super) fn commit_written_earlier(
        &mut self,
        rel: &str,
        content: &str,
        message: &str,
        author: Option<&str>,
        earlier: u64,
    ) -> Result<(), String> {
        self.write(rel, content)?;
        self.git(&["add", "--", rel])?;
        // Two commands from now is what the commit itself will carry;
        // reading the base rather than the clock keeps it reproducible.
        let written = self.base_epoch + (self.tick + 2) * TICK_SECS - earlier;
        let date = format!("--date={written} +0000");
        let mut args = vec!["commit", &date];
        let author_arg = author.map(|a| format!("--author={a}"));
        if let Some(arg) = author_arg.as_deref() {
            args.push(arg);
        }
        args.extend(["-m", message]);
        self.git(&args)?;
        Ok(())
    }

    /// One process for a batch of refs: a repository of thousands of
    /// tags built a `git tag` at a time is minutes of process spawning
    /// on Windows.
    pub(super) fn git_stdin(&mut self, args: &[&str], input: &str) -> Result<String, String> {
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
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    pub(super) fn add_origin(&mut self) -> Result<(), String> {
        let bare = self.root.join("origin.git");
        std::fs::create_dir_all(&bare).map_err(|e| e.to_string())?;
        self.git_at(&bare.clone(), &["init", "--bare", "-b", "main"])?;
        let url = file_url(&bare);
        self.git(&["remote", "add", "origin", &url])?;
        Ok(())
    }
}

pub(super) fn file_url(path: &Path) -> String {
    let mut p = path.to_string_lossy().replace('\\', "/");
    if !p.starts_with('/') {
        p.insert(0, '/');
    }
    format!("file://{p}")
}
