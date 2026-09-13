//! Real repositories in temp directories, built with the system git CLI,
//! isolated from the developer's global config and fully deterministic
//! (fixed identities and timestamps → stable SHAs).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Base timestamp for deterministic commits (arbitrary fixed epoch).
const BASE_EPOCH: u64 = 1_700_000_000;

/// The argument [`TestRepo::write_hook`] runs a freshly installed hook
/// with to find out whether the kernel will let git run it at all. Every
/// hook carries a line that exits on it, so that run does none of the
/// hook's work: git hands `pre-receive` and `pre-commit` no arguments and
/// `pre-push` the remote's name and URL, so nothing git runs is ever
/// given this.
const HOOK_PROBE_ARG: &str = "--test-repo-probe";

pub struct TestRepo {
    // Kept alive for the lifetime of the repo; dropped last.
    _dir: tempfile::TempDir,
    /// Replaces the developer's global config for TestRepo-spawned git only.
    global_config: PathBuf,
    /// Keeps Git's default excludes lookup away from the developer's XDG
    /// config directory for setup commands too.
    xdg_config: PathBuf,
    pub path: PathBuf,
    tick: u64,
}

/// Written into `.git/config` right after init — one file write instead of
/// five `git config` spawns per repo (process spawns dominate suite time on
/// Windows). Must stay repo-local: the code under test does not see
/// `global_config`, only this file. `{autocrlf}` is filled by
/// [`TestRepo::init`] / [`TestRepo::init_autocrlf`].
const REPO_CONFIG: &str = "\
[user]
\tname = Test User
\temail = test@example.com
[commit]
\tgpgsign = false
[tag]
\tgpgSign = false
[core]
\tautocrlf = {autocrlf}
";

/// Test-only speed knobs for TestRepo-spawned git (setup commits are the
/// bulk of the suite's writes): no fsync — repos are throwaway — and no
/// auto-gc mid-test. Unknown keys are ignored by older git, so nothing here
/// is a compatibility constraint.
const GLOBAL_CONFIG: &str = "\
[core]
\tfsync = none
[gc]
\tauto = 0
";

impl TestRepo {
    /// A repository with `core.autocrlf=false`: git stores and checks out
    /// bytes verbatim, so what a test writes is what the tests see.
    pub fn init() -> Self {
        Self::init_with_autocrlf("false")
    }

    /// A repository with `core.autocrlf=true` — the setting the Git for
    /// Windows installer offers by default, where git converts CRLF to LF on
    /// the way into the index and back on the way out. Pinning behaviour
    /// under it is the only way the conversion path gets walked at all; a
    /// suite that only ever runs with `false` has never seen what most
    /// Windows checkouts do.
    pub fn init_autocrlf() -> Self {
        Self::init_with_autocrlf("true")
    }

    fn init_with_autocrlf(autocrlf: &str) -> Self {
        let dir = tempfile::tempdir().expect("create tempdir");
        let path = dir.path().join("repo");
        let global_config = dir.path().join("global-config");
        let xdg_config = dir.path().join("xdg-config");
        std::fs::write(&global_config, GLOBAL_CONFIG).expect("write global config");
        std::fs::create_dir(&xdg_config).expect("create xdg config dir");
        std::fs::create_dir(&path).expect("create repo dir");
        let mut repo = Self {
            _dir: dir,
            global_config,
            xdg_config,
            path,
            tick: 0,
        };
        repo.git(&["init", "-b", "main"]);
        let config = repo.path.join(".git").join("config");
        let existing = std::fs::read_to_string(&config).expect("read repo config");
        let repo_config = REPO_CONFIG.replace("{autocrlf}", autocrlf);
        std::fs::write(&config, format!("{existing}{repo_config}")).expect("write repo config");
        repo
    }

    /// The file this repository's git reads and writes `--global` in. A
    /// test of a `--global` write needs a file of its own: the one the
    /// executor under test is isolated with is shared by the whole suite,
    /// and the suite runs in parallel (`support::exec::logged_global`).
    pub fn global_config(&self) -> &Path {
        &self.global_config
    }

    /// Writes one of the repository's own hooks and makes it runnable —
    /// the shebang is this method's, so `body` is the script alone. One
    /// spelling of the mechanics for every test that needs a hook: the
    /// exec bit is the half that only matters on machines the author is
    /// not on, and a copy that forgot it passes everywhere but the
    /// container.
    ///
    /// **Runnable is waited for, not assumed.** The write above leaves the
    /// hook exec-able by nobody for as long as a neighbouring fork holds
    /// the descriptor it was written through (`ETXTBSY` —
    /// `support::busy`), and git meeting that window declines the push
    /// *silently*: `receive-pack` writes its own `cannot exec` to the
    /// stderr it inherited rather than over the sideband, so the ref line's
    /// generic `(pre-receive hook declined)` is all that reaches this end
    /// and the report carries git's parenthetical where the hook's own
    /// words belong (measured). Nothing downstream can tell that from a
    /// hook that ran and said nothing, so the install is what waits: it
    /// runs the hook once, with [`HOOK_PROBE_ARG`] to keep that run from
    /// doing the hook's work, and returns when the kernel lets it.
    pub fn write_hook(&self, name: &str, body: &str) {
        self.write_hook_watched(name, body, || {});
    }

    /// [`TestRepo::write_hook`], telling `on_busy` the first time the hook
    /// it has just written cannot be executed yet — for the one test that
    /// holds that window open on purpose and has to know the install
    /// reached it before it lets go.
    pub fn write_hook_watched(&self, name: &str, body: &str, on_busy: impl FnOnce()) {
        let hooks = self.path.join(".git").join("hooks");
        std::fs::create_dir_all(&hooks).expect("create hooks dir");
        let hook = hooks.join(name);
        std::fs::write(
            &hook,
            format!("#!/bin/sh\nif [ \"$1\" = \"{HOOK_PROBE_ARG}\" ]; then exit 0; fi\n{body}"),
        )
        .expect("write the hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
                .expect("make the hook executable");
            let mut probe = Command::new(&hook);
            probe
                .arg(HOOK_PROBE_ARG)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            let ran = super::busy::run_once_it_is_not_busy(&mut probe, on_busy)
                .expect("the hook never became runnable");
            // The guard is the hook's first statement, so a probe that
            // reaches the interpreter at all leaves 0. Anything else is an
            // install nobody can run — said here, rather than as a refusal
            // with no words downstream.
            assert!(
                ran.status.success(),
                "the hook as installed does not run: {ran:?}"
            );
        }
        // Windows has no such window: the handle `fs::write` opened is
        // not inheritable, and nothing there refuses to run a file over
        // an open writer.
        #[cfg(not(unix))]
        drop(on_busy);
    }

    /// Runs git in the repo and panics on failure. Returns trimmed stdout.
    pub fn git(&mut self, args: &[&str]) -> String {
        let dir = self.path.clone();
        self.git_in(&dir, args)
    }

    /// Like [`TestRepo::git`] but returns raw stdout bytes (for capturing
    /// parser fixtures byte-exactly).
    pub fn git_raw(&mut self, args: &[&str]) -> Vec<u8> {
        let dir = self.path.clone();
        let out = self.run(&dir, args);
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        out.stdout
    }

    /// Runs git and reports only whether it succeeded — for the commands
    /// whose exit code *is* the answer (`check-ref-format`).
    pub fn git_ok(&mut self, args: &[&str]) -> bool {
        let dir = self.path.clone();
        self.run(&dir, args).status.success()
    }

    /// Runs git expecting a non-zero exit (e.g. a conflicting merge).
    pub fn git_expect_failure(&mut self, args: &[&str]) {
        let dir = self.path.clone();
        let out = self.run(&dir, args);
        assert!(!out.status.success(), "git {args:?} unexpectedly succeeded");
    }

    /// Runs git in an arbitrary directory (e.g. for `clone`).
    pub fn git_in(&mut self, dir: &Path, args: &[&str]) -> String {
        let out = self.run(dir, args);
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn run(&mut self, dir: &Path, args: &[&str]) -> Output {
        self.tick += 1;
        let stamp = format!("{} +0000", BASE_EPOCH + self.tick * 60);
        Command::new("git")
            .args(args)
            .current_dir(dir)
            // Isolate from developer/global configuration.
            .env("GIT_CONFIG_GLOBAL", &self.global_config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("XDG_CONFIG_HOME", &self.xdg_config)
            .env("GIT_TERMINAL_PROMPT", "0")
            // Nothing a fixture runs may open an editor: a continue that
            // wants a message must take the recorded one (`true` exits 0
            // without writing — the same pin the executor under test uses).
            .env("GIT_EDITOR", "true")
            .env("LC_ALL", "C")
            // Deterministic identities and times.
            .env("GIT_AUTHOR_NAME", "Test User")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test User")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .env("GIT_AUTHOR_DATE", &stamp)
            .env("GIT_COMMITTER_DATE", &stamp)
            .output()
            .expect("spawn git")
    }

    /// Writes a file (creating parent dirs) relative to the work tree.
    pub fn write_file(&self, rel: &str, content: &str) {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("create parent dirs");
        }
        std::fs::write(p, content).expect("write file");
    }

    /// Writes + stages + commits a file. Two spawns; the suite makes some
    /// four hundred of these, so the commit id is a separate ask
    /// ([`Self::commit_file_id`]) rather than a third spawn nobody reads.
    pub fn commit_file(&mut self, rel: &str, content: &str, message: &str) {
        self.write_file(rel, content);
        self.git(&["add", "--", rel]);
        self.git(&["commit", "-m", message]);
    }

    /// [`Self::commit_file`], answering the new commit's id.
    pub fn commit_file_id(&mut self, rel: &str, content: &str, message: &str) -> String {
        self.commit_file(rel, content, message);
        self.git(&["rev-parse", "HEAD"])
    }

    /// `file://` URL of this repository, so remote-protocol paths can be
    /// exercised without a network (実装計画 §11.3).
    pub fn file_url(&self) -> String {
        let mut p = self.path.to_string_lossy().replace('\\', "/");
        // Windows paths start with a drive letter; the URL needs a root.
        if !p.starts_with('/') {
            p.insert(0, '/');
        }
        format!("file://{p}")
    }
}

/// A hook body that holds git where it stands until `release` exists.
///
/// The only way a test can have a write provably still running while it
/// drives everything around it: a sleep would put time in place of the
/// fact, and the fact wanted here is that git has not returned yet.
pub fn barrier_hook(release: &std::path::Path) -> String {
    let release = release.to_string_lossy().replace('\\', "/");
    format!(
        "i=0\nwhile [ ! -f \"{release}\" ]; do\n  i=$((i+1))\n  [ \"$i\" -gt 6000 ] && exit 1\n  sleep 0.1\ndone\n"
    )
}

/// A clean filter that holds `git add` where it stands until `release`
/// exists, then hands the content through unchanged.
///
/// The staging counterpart of [`barrier_hook`], and the only one there
/// is: `git add` runs no hook, and a write that only touches the index is
/// exactly the one whose follow-up reads leave the refs alone. Wire it
/// with `filter.<name>.clean` **and `filter.<name>.required`** — without
/// the second, a filter git cannot run is one it warns about, stages the
/// raw bytes for and exits 0 from, which is a barrier that silently
/// is not one.
///
/// **It spins rather than sleeps**: the shell git runs a filter in has no
/// `sleep` on the PATH the suite hands it, and a loop that calls one
/// leaves through its cap at once instead of waiting.
pub fn barrier_filter(release: &std::path::Path) -> String {
    let release = release.to_string_lossy().replace('\\', "/");
    format!(
        "i=0; until [ -f \"{release}\" ]; do i=$((i+1)); [ \"$i\" -gt 200000000 ] && exit 1; done; cat"
    )
}

/// The opened [`platitude_core::repo::RepoInfo`] of a test repository —
/// what every write API takes instead of a bare path.
pub async fn info(repo: &TestRepo) -> platitude_core::repo::RepoInfo {
    let (exec, cancel) = super::exec::env();
    platitude_core::repo::open(&exec, &repo.path, &cancel)
        .await
        .expect("open repo")
}
