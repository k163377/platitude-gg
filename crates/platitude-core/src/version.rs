//! The installed git version against the supported minimum. Below it is a
//! fact reported, not an error: the app keeps working and says so
//! (internal-docs/git互換性.md).

use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// Minimum supported git version; this definition alone owns the number
/// and its rationale: Ubuntu 24.04 LTS ships this git series.
/// Package evidence: <https://packages.ubuntu.com/noble/git>.
/// Support policy and the addition procedure: internal-docs/git互換性.md.
///
/// # Command compatibility audit
///
/// The forms below are available at this minimum. Evidence is the
/// versioned `git-scm.com/docs/git-<command>/<minimum>.0` manuals, gitcli(7)
/// for generic negated options and option terminators, and git.git's
/// matching `v<minimum>.0` Documentation sources when a manual redirects.
/// This is a compatibility record, not an argv template: builders own
/// the actual arguments, and real-git tests cover their combinations.
/// Git-LFS is a separately installed extension; `lfs version` probes it
/// and its availability is not guaranteed by this git minimum.
///
/// Shared arguments, environment and pathspecs:
/// - Executor: `-c color.ui=false`, `core.quotepath=false`,
///   `log.showSignature=false`, `diff.autoRefreshIndex=false`,
///   `--no-optional-locks` (2.15); `LC_ALL=C`, `GIT_EDITOR=true`,
///   `GIT_TERMINAL_PROMPT=0` (2.3), `GIT_OPTIONAL_LOCKS=0` (2.15).
/// - Rebase: `GIT_SEQUENCE_EDITOR` (1.7.8); paths: `:(literal)` (1.9).
/// - Diff shape: `-c diff.noprefix=false`, `diff.mnemonicPrefix=false`,
///   `diff.external=`; mergetool: `-c mergetool.writeToTemp=true`.
/// - Temporary index / discard records: `-c core.hooksPath=<absent>`
///   (2.9), `core.splitIndex=false` (2.12), `core.fsmonitor=false`
///   (boolean form 2.36), `core.safecrlf=false`, `index.skipHash=true`
///   (2.40); `GIT_INDEX_FILE`, `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`,
///   `GIT_AUTHOR_DATE`, `GIT_COMMITTER_NAME`, `GIT_COMMITTER_EMAIL`,
///   `GIT_COMMITTER_DATE`. `discards_integration::recording` checks the
///   isolated index and hook behavior on the minimum binary.
///
/// Subcommands and their emitted options / formats:
/// - `--version`.
/// - `add`: `--all`, `--intent-to-add`, `--force`, `--sparse` (2.34).
/// - `apply`: `--cached`, `--reverse`, `--check`, `--whitespace=nowarn`.
/// - `branch`: `-d`, `-D`, `-m`, `-M`, `--set-upstream-to=`,
///   `--end-of-options`.
/// - `cat-file`: `-s`, `blob`, `commit`; `check-attr`: `-z`, names, `--`.
/// - `cherry-pick` / `revert`: `--no-edit`, `--continue`, `--abort`,
///   `--skip`, `--quit`; `--no-edit` follows gitcli's negation rule.
/// - `clean`: `-f`, `-d`; `clone`: `[--] <repository> <directory>`.
/// - `commit`: `--amend`, `--allow-empty`, `--reset-author`, `--no-edit`,
///   `--cleanup=whitespace`, `--file`; `commit-tree`: `-p`, `-m`.
/// - `config`: `--get`, `--get-regexp`, `-z`, `--global`, `--local`,
///   `--show-scope` (2.26), `--unset`, `--type=bool` (2.18), and
///   `<key> <value>` writes. `checkout.defaultRemote` (2.19),
///   `remote.pushDefault`, and `filter.lfs.required` are supported keys.
///   `lfs_integration` checks typed bool reads; absent `--unset` exits 5.
/// - `diff`: `--cached`, `--no-ext-diff`, `--find-renames`, `--no-renames`,
///   `--name-status`, `--diff-filter=`, `-z`, `--no-index` (1.5.1),
///   `--end-of-options` (2.24).
/// - `diff-tree`: `-r`, `--no-commit-id`, `-z`, `--name-status`,
///   `--name-only`, `--diff-filter=`, `--find-renames`, `--no-renames`,
///   `--root`, `--no-ext-diff`, `-p`, `--binary`, `--full-index`,
///   `--no-color`, `--end-of-options`.
/// - `fetch`: `--prune`, `--all`; `pull`: `--no-edit`.
/// - `for-each-ref`: `--points-at=` (2.7), `--count`,
///   `--sort=refname:short`, `--format=` with `refname[:short]`,
///   `objecttype`, `objectname`, `*objectname`, `upstream[:track]`,
///   `HEAD`, `creatordate:unix`, `push`, `push:remoteref`, `push:track`.
///   `-c push.default=current` plus `refs/heads/<branch>` is covered by
///   `push_default_integration`, including the push destination fields.
/// - `log`: `-z`, `-1`, `-r`, `-p`, `--name-status`, `--date-order`,
///   `--branches`, `--remotes`,
///   `--tags`, `--max-count=`, `--ignore-missing`, `--reverse`,
///   `--format=`, `--end-of-options`, `--walk-reflogs`, `--date=unix`
///   (2.9), `--shortstat`, `--diff-merges=first-parent` (2.31),
///   `--no-walk[=unsorted]`, `--no-ext-diff`, `--find-renames`.
///   `-z` is in diff-options.txt's git-log section. Reflogs named
///   `HEAD`, `main-worktree/HEAD`, `worktrees/<id>/HEAD`, and
///   `refs/pgg/discards` follow git-worktree(1) REFS;
///   `discards_integration` checks the cross-worktree reads.
/// - `ls-files`: `-z`, `--eol` (2.8), `--unmerged`, `--others`,
///   `--exclude-standard`, pathspecs.
/// - `ls-remote`: `--tags`, `--heads`, `--end-of-options`, `--`,
///   multiple patterns. A peeled `refs/tags/<t>^{}` line requires its
///   own explicit pattern; the base tag pattern alone does not match it.
/// - `ls-tree`: `-r`, `-z`, `--name-only`, `--full-tree`,
///   `--end-of-options`.
/// - `merge`: `--no-edit`, `--no-ff`, `--ff-only`, `--squash`,
///   `--continue`, `--abort`, `--quit`; `merge-base`: `--is-ancestor`.
/// - `mergetool`: `--no-prompt`, `--gui`, `--tool=`, `--tool-help`;
///   custom recipes are read with `config --get-regexp`.
/// - `name-rev`: `--name-only`, `--refs=refs/heads/*`.
/// - `push`: `--porcelain`, `--set-upstream`, `--force-with-lease[=ref:oid]`,
///   `--force`, `--delete`. `remote_` integration tests check lease plus
///   deletion of qualified branch/tag refs: matching object deletes;
///   stale or absent object rejects with stale info and exit 1.
/// - `read-tree <tree-ish>` / `write-tree` use a temporary index.
/// - `rebase`: `--interactive`, `--no-rebase-merges`, `--onto`, `--root`,
///   `--update-refs` (2.38), `--continue`, `--abort`, `--skip`, `--quit`.
/// - `remote`: `add`, `set-url`, `--end-of-options`, `get-url --push --`.
/// - `reset`: `--quiet`, `--soft`, `--mixed`, `--hard`; see the
///   unsupported forms at the end of this file for option termination.
/// - `restore`: `--ours`, `--theirs`, `--staged`, `--worktree` (combined).
/// - `rev-list`: `--count`, `--merges`, `--not`, `--remotes`,
///   `--max-count=`, `--branches`, `--exclude=`, `--glob=`,
///   `--ignore-missing`, `--parents`, `--tags`.
/// - `rev-parse`: `--verify`, `-q` / `--quiet`, `--git-path`,
///   `--path-format=absolute` (2.31), `--show-toplevel`,
///   `--absolute-git-dir`, `--show-object-format`, `--symbolic-full-name`,
///   `--show-prefix`, `--is-bare-repository`, `--end-of-options`,
///   `<rev>^{commit}`, `<rev>^{tree}`.
/// - `rm`: `--cached`, `-r`, `-f`, `--quiet`, `--ignore-unmatch`.
/// - `show`: `-z`, `-r`, `--name-status`, `--find-renames`,
///   `--diff-merges=first-parent` (2.31), `--format=`.
/// - `stash`: `list -z --format=`, `push -m` / `--message`,
///   `--include-untracked`, `--keep-index`, `--staged` (2.35),
///   `pop --index`, `apply`, `drop`, `store -m`.
/// - `status`: `--porcelain=v2` (2.11), `-z`, `--branch`, `-uall`, `-uno`.
/// - `switch`: `--create`, `--force-create`, `--track`, `--no-track`.
///   `upstream_integration` checks `--no-track` plus `--force-create`;
///   the manual does not explicitly describe this combination.
/// - `symbolic-ref`: `-q`, `--short`; `tag`: `--delete`,
///   `--end-of-options`.
/// - `update-index`: `--add`, `--cacheinfo <mode>,<object>,<path>` (2.0),
///   `--force-remove`, `--`; `update-ref`: `--create-reflog` (2.6), `-m`,
///   `<ref> <new>`, `--end-of-options <ref> <new> ""` (create-only CAS).
/// - `worktree`: `list --porcelain -z` (`-z` 2.36), `remove -- <path>`
///   (2.17), `add [-b <name>] [--track] [--detach] -- <path> <start>`.
///   `worktree_add` tests the branch-before-directory check and the
///   option-like branch name trap; the app rejects both before writing.
///
/// Pretty formats: `%H %h %P %T %an %ae %at %aI %cn %ce %ct %cI %B %s`
/// `%gd %gD %gs %G? %GS %GK %xNN`; `%(trailers:...)` with `key=`,
/// `valueonly`, `only`, `unfold`, `separator=`. These are documented in
/// pretty-formats; `creatordate:unix` delegates to log's unix date format.
pub const MINIMUM_GIT: (u32, u32) = (2, 43);

/// [`MINIMUM_GIT`] formatted for the UI and error messages.
pub fn minimum_string() -> String {
    format!("{}.{}", MINIMUM_GIT.0, MINIMUM_GIT.1)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    /// The full version string as printed by git (for display).
    pub raw: String,
}

impl GitVersion {
    /// Parses lines like `git version 2.51.0.windows.1` or
    /// `git version 2.39.2 (Apple Git-143)`.
    pub fn parse(line: &str) -> Option<Self> {
        let rest = line.trim().strip_prefix("git version ")?;
        let mut parts = rest.split(['.', ' ']);
        let major = leading_number(parts.next()?)?;
        let minor = leading_number(parts.next()?)?;
        let patch = parts.next().and_then(leading_number).unwrap_or(0);
        Some(Self {
            major,
            minor,
            patch,
            raw: rest.trim().to_string(),
        })
    }

    pub fn supported(&self) -> bool {
        (self.major, self.minor) >= MINIMUM_GIT
    }
}

fn leading_number(part: &str) -> Option<u32> {
    let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// What one candidate git answered when asked its version. An old git
/// still runs the app, and `Missing` vs `Failed` is what a reader needs to
/// fix a typed path. The wording is the caller's; this carries only what
/// git or the OS said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probe {
    /// It ran, and named a version at or above [`MINIMUM_GIT`].
    Supported(GitVersion),
    /// It ran, and named an older one.
    Old(GitVersion),
    /// There is nothing to run at that path.
    Missing,
    /// It could not be started, or it started and did not answer like
    /// git. `message` is the reason in git's or the OS's own words.
    Failed { message: String },
}

impl Probe {
    /// Whether git answered at all — the two versions, old or not.
    pub fn answered(&self) -> bool {
        matches!(self, Self::Supported(_) | Self::Old(_))
    }

    /// The version string git printed, or empty where it printed none.
    pub fn version(&self) -> &str {
        match self {
            Self::Supported(v) | Self::Old(v) => &v.raw,
            _ => "",
        }
    }
}

/// Asks one git binary for its version, on an executor of its own,
/// leaving the running one alone. `program` is a path, or empty for
/// `PATH`'s git — as [`crate::settings::Defaults::git_path`] holds it.
pub async fn probe(program: &str, cancel: &CancellationToken) -> Probe {
    let executor = if program.is_empty() {
        GitExecutor::new()
    } else {
        GitExecutor::with_program(program)
    };
    match detect(&executor, cancel).await {
        Ok(version) if version.supported() => Probe::Supported(version),
        Ok(version) => Probe::Old(version),
        Err(GitError::GitNotFound { .. }) => Probe::Missing,
        Err(error) => Probe::Failed {
            message: error.to_string(),
        },
    }
}

pub async fn detect(
    executor: &GitExecutor,
    cancel: &CancellationToken,
) -> Result<GitVersion, GitError> {
    let cmd = GitCommand::new()
        .arg("--version")
        .timeout(Duration::from_secs(10));
    let out = executor.run(cmd, cancel).await?;
    let text = out.stdout_utf8();
    GitVersion::parse(&text).ok_or_else(|| GitError::UnexpectedOutput {
        command: "git --version".to_string(),
        message: text.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_version() {
        let v = GitVersion::parse("git version 2.40.0").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (2, 40, 0));
        assert_eq!(v.raw, "2.40.0");
    }

    #[test]
    fn parses_windows_version() {
        let v = GitVersion::parse("git version 2.51.0.windows.1\n").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (2, 51, 0));
    }

    #[test]
    fn parses_apple_suffix() {
        let v = GitVersion::parse("git version 2.39.2 (Apple Git-143)").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (2, 39, 2));
    }

    #[test]
    fn parses_rc_version() {
        let v = GitVersion::parse("git version 2.44.0-rc1").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (2, 44, 0));
    }

    #[test]
    fn rejects_garbage() {
        assert!(GitVersion::parse("not git").is_none());
        assert!(GitVersion::parse("git version x.y").is_none());
    }

    #[test]
    fn support_boundaries() {
        let mk = |maj, min| GitVersion {
            major: maj,
            minor: min,
            patch: 0,
            raw: String::new(),
        };
        let (major, minor) = MINIMUM_GIT;
        let (below_major, below_minor) = if minor > 0 {
            (major, minor - 1)
        } else {
            (major - 1, u32::MAX)
        };
        assert!(!mk(below_major, below_minor).supported());
        assert!(mk(major, minor).supported());
        assert!(mk(major, minor + 1).supported());
        assert!(mk(major + 1, 0).supported());
    }
}

// Unsupported forms at MINIMUM_GIT (newer development git can hide these):
// - `reset --end-of-options` is rejected despite gitcli's general rule.
//   Resolve names with `rev-parse --end-of-options`, then pass object ids.
//   `branch`, `switch`, `tag`, `remote add/set-url`, `ls-remote`,
//   `rev-parse --verify --quiet`, `log -1 --format=`, and `stash list`
//   accept the terminator on the minimum binary.
// - `cherry-pick --empty=...` requires 2.45; skip an empty stop instead.
// - `config get/set/unset/list` subcommands require 2.46; use old forms.
// - `for-each-ref %(is-base:...)` requires 2.47.
// - `rev-list -z` requires 2.49; `log -z` is available at the minimum.
// - The bundled `vscode` mergetool recipe requires 2.47; an explicit
//   `mergetool.vscode.cmd` works without it. `smerge` exists since 2.22.
//   Recipes only launch external tools; they do not install the tools.
// `%(ahead-behind:...)` exists since 2.41 and is allowed but not emitted.
