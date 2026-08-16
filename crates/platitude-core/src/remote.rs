//! Remotes: listing, fetch and push.
//!
//! This is the only place the application causes network traffic, and it
//! causes it the same way a terminal would — by running git. Authentication
//! is git's (`GIT_TERMINAL_PROMPT=0` keeps a missing credential helper from
//! hanging the process instead of failing).
//!
//! Network commands take a timeout rather than running unbounded.
//! Cancellation is the normal way to stop one early; the timeout is the
//! backstop for a connection that neither finishes nor fails.

use std::path::Path;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

/// Default time budget for commands that talk to a remote.
///
/// Three minutes covers an ordinary fetch of a large repository over a slow
/// link without leaving a hung connection running for an hour. A user on a
/// genuinely slow line can raise it (the `network_timeout_secs` settings
/// key; the dialog's input field is what is still missing — 実装計画 §7).
pub const DEFAULT_NETWORK_TIMEOUT: Duration = Duration::from_secs(180);

/// A configured remote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Remote {
    pub name: String,
    pub fetch_url: String,
    /// `remote.<name>.pushurl` when set, otherwise the fetch URL.
    pub push_url: String,
}

/// Lists configured remotes.
///
/// Reads config keys rather than `git remote -v`, whose two-lines-per-remote
/// shape is meant for humans.
pub async fn list(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<Remote>, GitError> {
    let cmd = GitCommand::new().cwd(workdir).args([
        "config",
        "-z",
        "--get-regexp",
        r"^remote\..*\.(url|pushurl)$",
    ]);
    // Exit code 1 just means "no matching keys" (a repository with no
    // remotes), which is not a failure.
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 1 {
        return Ok(Vec::new());
    }
    if out.code != 0 {
        return Err(GitError::Failed {
            command: "git config --get-regexp remote".to_string(),
            code: out.code,
            stderr: out.failure_message(),
        });
    }
    Ok(parse_remote_config(&out.stdout))
}

/// Parses `git config -z --get-regexp`: `key\nvalue` records, NUL-terminated.
fn parse_remote_config(bytes: &[u8]) -> Vec<Remote> {
    let mut remotes: Vec<Remote> = Vec::new();
    for record in bytes.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let text = String::from_utf8_lossy(record);
        let Some((key, value)) = text.split_once('\n') else {
            continue;
        };
        // `remote.<name>.url` — the name itself may contain dots, so take
        // the first and last segments and treat the middle as the name.
        let Some(rest) = key.strip_prefix("remote.") else {
            continue;
        };
        let Some((name, field)) = rest.rsplit_once('.') else {
            continue;
        };
        let entry = match remotes.iter_mut().find(|r| r.name == name) {
            Some(existing) => existing,
            None => {
                remotes.push(Remote {
                    name: name.to_string(),
                    ..Default::default()
                });
                // The push above guarantees a last element.
                match remotes.last_mut() {
                    Some(e) => e,
                    None => continue,
                }
            }
        };
        match field {
            "url" => {
                entry.fetch_url = value.to_string();
                if entry.push_url.is_empty() {
                    entry.push_url = value.to_string();
                }
            }
            "pushurl" => entry.push_url = value.to_string(),
            _ => {}
        }
    }
    remotes.sort_by(|a, b| a.name.cmp(&b.name));
    remotes
}

/// `git fetch --prune`. `remote` of `None` fetches every remote.
pub async fn fetch(
    executor: &GitExecutor,
    workdir: &Path,
    remote: Option<&str>,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["fetch", "--prune"])
        .timeout(timeout);
    let cmd = match remote {
        Some(name) => cmd.args(["--", name]),
        None => cmd.arg("--all"),
    };
    executor.run(cmd, cancel).await.map(|_| ())
}

/// What a remote carries under `refs/tags/`: each tag name and the commit
/// it designates over there.
///
/// Nothing local records this. A branch on a remote leaves `refs/remotes/`
/// behind, so its badge survives going offline; a fetched tag lands in
/// `refs/tags/` next to the ones made here and the two become
/// indistinguishable. Asking the remote is the only way to tell them apart,
/// which is why this is on the fetch path rather than the poll: it is the
/// one place the user has already agreed to pay for the network.
///
/// An annotated tag is advertised twice — the tag object, then the commit
/// it peels to under a `^{}` suffix — and it is the commit that has to line
/// up with what the local listing peels to, so the peeled line wins wherever
/// both appear. `--refs` cannot do that filtering: it drops the peeled line
/// and keeps the tag object, which compares equal to nothing (measured).
pub async fn list_tags(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<Vec<RemoteTag>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["ls-remote", "--tags", "--", remote])
        .timeout(timeout);
    let out = executor.run(cmd, cancel).await?;
    Ok(parse_ls_remote_tags(&out.stdout))
}

/// One tag as a remote advertises it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTag {
    pub name: crate::Name,
    /// The commit it designates, annotated tags peeled — the same thing
    /// [`crate::refs::RefEntry::commit_oid`] answers for a local one, so
    /// the two can be compared directly.
    pub commit: Oid,
    /// True when the remote advertised a tag object to peel.
    pub annotated: bool,
}

/// Parses `git ls-remote --tags`: `<oid>\t<refname>` lines, sorted by name.
///
/// The `From <url>` banner goes to stderr, so stdout is only ref lines.
/// Anything outside `refs/tags/` and any unreadable oid is skipped rather
/// than failing the listing — one odd advertisement must not cost every
/// other tag its badge.
pub fn parse_ls_remote_tags(bytes: &[u8]) -> Vec<RemoteTag> {
    let mut out: Vec<RemoteTag> = Vec::new();
    for line in bytes.split(|b| *b == b'\n') {
        let Some((oid, refname)) = split_ls_remote_line(line) else {
            continue;
        };
        let Some(name) = refname.strip_prefix("refs/tags/") else {
            continue;
        };
        let (name, peeled) = match name.strip_suffix("^{}") {
            Some(base) => (base, true),
            None => (name, false),
        };
        match out.iter_mut().find(|t| t.name == name) {
            // Two lines for one name means an annotated tag: the pair is
            // the tag object and the commit under it. Only the peeled line
            // carries the commit, so it wins whichever side it arrives on.
            Some(seen) if !seen.annotated => {
                seen.commit = oid;
                seen.annotated = peeled;
            }
            Some(_) => {}
            None => out.push(RemoteTag {
                name: name.into(),
                commit: oid,
                annotated: peeled,
            }),
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn split_ls_remote_line(line: &[u8]) -> Option<(Oid, &str)> {
    let tab = line.iter().position(|b| *b == b'\t')?;
    let oid = Oid::from_hex(&line[..tab]).ok()?;
    let refname = std::str::from_utf8(&line[tab + 1..]).ok()?.trim_end();
    Some((oid, refname))
}

/// How hard a push may overwrite the remote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushForce {
    /// Fast-forward only; git refuses anything else.
    None,
    /// `--force-with-lease`. `expect` pins the remote commit the user
    /// actually saw. Without it the lease is checked against the local
    /// remote-tracking ref, which a background fetch can advance behind the
    /// user's back — turning the safety net into a plain force.
    WithLease { expect: Option<String> },
    /// `--force`: unconditional. The UI confirms before choosing this.
    Force,
}

/// What to push where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushSpec {
    pub remote: String,
    /// Local ref to push (a branch name, or `HEAD`).
    pub local: String,
    /// Branch name on the remote side.
    pub remote_branch: String,
    /// Record the pushed branch as upstream (`--set-upstream`).
    pub set_upstream: bool,
    pub force: PushForce,
}

/// Works out where the branch that is checked out should be pushed.
///
/// A branch that already tracks something goes back to exactly that. One
/// that tracks nothing goes to `fallback_remote` under its own name and
/// records the upstream, so the next push needs no decision. A detached
/// HEAD has no branch to push, and says so rather than guessing.
///
/// The upstream is read from configuration rather than parsed out of
/// `origin/main`: a remote may be named `my/fork`, and a branch name may
/// contain slashes, so splitting that string cannot be done reliably.
pub async fn plan_current_push(
    executor: &GitExecutor,
    workdir: &Path,
    fallback_remote: &str,
    force: PushForce,
    cancel: &CancellationToken,
) -> Result<PushSpec, GitError> {
    let branch = current_branch(executor, workdir, cancel).await?;
    let remote = config_value(
        executor,
        workdir,
        &format!("branch.{branch}.remote"),
        cancel,
    )
    .await?;
    let merge = config_value(executor, workdir, &format!("branch.{branch}.merge"), cancel).await?;

    match (remote, merge) {
        (Some(remote), Some(merge)) => Ok(PushSpec {
            remote,
            remote_branch: merge
                .strip_prefix("refs/heads/")
                .unwrap_or(&merge)
                .to_string(),
            local: branch,
            set_upstream: false,
            force,
        }),
        _ => {
            if fallback_remote.is_empty() {
                return Err(GitError::UnexpectedOutput {
                    command: "git push".to_string(),
                    message: "this repository has no remote to push to".to_string(),
                });
            }
            Ok(PushSpec {
                remote: fallback_remote.to_string(),
                remote_branch: branch.clone(),
                local: branch,
                set_upstream: true,
                force,
            })
        }
    }
}

/// Where the branch that is checked out should go when the user has just
/// said so, rather than when configuration already knows.
///
/// `expect` is the commit the question showed as being over there. Empty
/// sends the push fast-forward only; a commit turns it into the same
/// leased overwrite the toolbar offers a diverged branch — and the lease
/// is pinned to what was on screen, so a remote that moved since is
/// refused rather than flattened (§相手の履歴を置き換える).
pub async fn plan_publish(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    remote_branch: &str,
    expect: &str,
    cancel: &CancellationToken,
) -> Result<PushSpec, GitError> {
    let branch = current_branch(executor, workdir, cancel).await?;
    Ok(PushSpec {
        remote: remote.to_string(),
        remote_branch: remote_branch.to_string(),
        local: branch,
        set_upstream: true,
        force: if expect.is_empty() {
            PushForce::None
        } else {
            PushForce::WithLease {
                expect: Some(expect.to_string()),
            }
        },
    })
}

/// `git remote add <name> <url>`.
///
/// Nothing is contacted: git records the URL and reports success even for a
/// host that does not exist, so a bad URL is only found out by the push that
/// follows (実測). The remote survives that failure, which is why the UI
/// offers a way to correct the URL rather than undoing the add.
///
/// Names are git's to judge — it refuses `bad name` (exit 128) and a name it
/// already has (exit 3), and both refusals arrive as their own text.
pub async fn add(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    url: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["remote", "add", "--end-of-options", name, url]);
    executor.run(cmd, cancel).await?;
    Ok(())
}

/// `git remote set-url <name> <url>` — the way back from a URL typed wrong.
pub async fn set_url(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    url: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd =
        GitCommand::new()
            .cwd(workdir)
            .args(["remote", "set-url", "--end-of-options", name, url]);
    executor.run(cmd, cancel).await?;
    Ok(())
}

/// What a remote carries under this exact branch name, if anything.
///
/// Asked before a first push, because git answers two different ways to a
/// name that is already over there: it fast-forwards one that our history
/// contains — silently advancing somebody else's branch — and refuses one
/// it does not (実測, both). The commit is returned rather than a yes, so
/// the caller can tell those two apart before anything is sent.
///
/// **The pattern has to be the full `refs/heads/<name>`.** `ls-remote`
/// matches a bare name against the *tail* of a ref, so asking for `topic`
/// answers yes when the remote only has `feature/topic` (実測).
pub async fn branch_tip(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    branch: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<Option<Oid>, GitError> {
    let refname = format!("refs/heads/{branch}");
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["ls-remote", "--heads", "--end-of-options", remote, &refname])
        .timeout(timeout);
    let out = executor.run(cmd, cancel).await?;
    // A remote that has nothing to say answers with an empty stdout and
    // exit 0, so the absence is in the output rather than in the code.
    Ok(out
        .stdout
        .split(|b| *b == b'\n')
        .filter_map(split_ls_remote_line)
        .find(|(_, name)| *name == refname)
        .map(|(oid, _)| oid))
}

/// What a first push under a given name would meet on the far side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteBranchState {
    /// Nothing is there under that name: the push makes the branch.
    Free,
    /// It is there and this history contains it, so the push lands and
    /// moves it on.
    FastForward,
    /// It is there with commits this history does not have. **git refuses
    /// this push** (実測), so nothing can happen by pressing.
    Refused,
    /// It is there, and what it holds cannot be read from here — the
    /// commit it names is not in this repository, so the two histories
    /// cannot be compared without fetching it first.
    Unknown,
    /// The remote never answered: a URL typed wrong, credentials that are
    /// not there, no network. Silence is not "nothing is there".
    Unreachable,
}

impl RemoteBranchState {
    /// The wire name the UI reads. Spelled out rather than derived so the
    /// two ends cannot drift apart on a rename.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::FastForward => "fast-forward",
            Self::Refused => "refused",
            Self::Unknown => "unknown",
            Self::Unreachable => "unreachable",
        }
    }
}

/// Short name of the checked-out branch; an error when HEAD is detached.
async fn current_branch(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<String, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["symbolic-ref", "-q", "--short", "HEAD"]);
    let out = executor.run_unchecked(cmd, cancel).await?;
    let name = out.stdout_utf8().trim().to_string();
    if out.code != 0 || name.is_empty() {
        return Err(GitError::UnexpectedOutput {
            command: "git symbolic-ref --short HEAD".to_string(),
            message: "HEAD is detached, so there is no branch to push".to_string(),
        });
    }
    Ok(name)
}

/// One configuration value, or `None` when the key is unset.
async fn config_value(
    executor: &GitExecutor,
    workdir: &Path,
    key: &str,
    cancel: &CancellationToken,
) -> Result<Option<String>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["config", "--get", "--", key]);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => {
            let value = out.stdout_utf8().trim().to_string();
            Ok((!value.is_empty()).then_some(value))
        }
        // 1 is git's "no such key" — a valid empty answer. Anything else
        // (128 on an unreadable config) must not read as "unset": a push
        // planned on that misreading rewrites upstreams (実測: a bad
        // config line makes `git config --get` exit 128, not 1).
        1 => Ok(None),
        code => Err(GitError::Failed {
            command: format!("git config --get -- {key}"),
            code,
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }),
    }
}

/// `git push` for one branch.
///
/// A refusal that a fetch would answer comes back as
/// [`GitError::PushOutdated`] rather than a plain failure, so the caller can
/// go and find out what the remote actually holds.
pub async fn push(
    executor: &GitExecutor,
    workdir: &Path,
    spec: &PushSpec,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    // `--porcelain` puts a fixed per-ref result on stdout. Which kind of
    // refusal this was has to be read from somewhere, and the sentence that
    // says so on stderr is prose written for a terminal.
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["push", "--porcelain"])
        .timeout(timeout);
    if spec.set_upstream {
        cmd = cmd.arg("--set-upstream");
    }
    match &spec.force {
        PushForce::None => {}
        PushForce::WithLease { expect: None } => cmd = cmd.arg("--force-with-lease"),
        PushForce::WithLease { expect: Some(oid) } => {
            cmd = cmd.arg(format!("--force-with-lease={}:{oid}", spec.remote_branch));
        }
        PushForce::Force => cmd = cmd.arg("--force"),
    }
    let refspec = format!("{}:refs/heads/{}", spec.local, spec.remote_branch);
    cmd = cmd.args(["--", &spec.remote, &refspec]);

    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 0 {
        return Ok(());
    }
    let stderr = out.failure_message();
    if is_outdated(&out.stdout_utf8()) {
        return Err(GitError::PushOutdated {
            command,
            code: out.code,
            stderr,
        });
    }
    Err(GitError::Failed {
        command,
        code: out.code,
        stderr,
    })
}

/// Whether a `--porcelain` push result refused a ref for knowing the remote
/// only as it used to be.
///
/// The lines are `<flag>\t<from>:<to>\t<summary>`, where `!` is a refusal.
/// Three summaries say the same thing: a plain push found commits it would
/// drop (`fetch first`, or `non-fast-forward` for a ref that is not the
/// current branch's upstream), or a lease was pinned to a commit the remote
/// has since left (`stale info`). All three are answered by fetching.
///
/// Anything else — a hook, a protected branch, an unreachable host — is a
/// refusal to pass on as it is. A host that could not be reached at all
/// prints no ref lines, so it cannot be mistaken for one of these.
fn is_outdated(porcelain: &str) -> bool {
    porcelain.lines().any(|line| {
        let mut fields = line.split('\t');
        fields.next() == Some("!")
            && fields.nth(1).is_some_and(|summary| {
                summary.contains("(fetch first)")
                    || summary.contains("(stale info)")
                    || summary.contains("(non-fast-forward)")
            })
    })
}

/// `git push <remote> --delete <branch>`: removes a branch on the remote.
/// Destructive — the caller confirms first.
pub async fn delete_remote_branch(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    branch: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["push", "--delete", "--", remote, branch])
        .timeout(timeout);
    executor.run(cmd, cancel).await.map(|_| ())
}

/// Renames a branch on a remote: the composition git has no command for.
///
/// The new name is pushed **from the remote-tracking ref**, not from a
/// local branch of the same name: the question asked was about a name, and
/// a local branch that has moved on since would publish its commits as
/// well. Then the old name goes, and any local branch that tracked it is
/// pointed at the new one — `push --delete` prunes the tracking ref an
/// upstream setting names, and a stale one sends the next push straight
/// back to the name just deleted.
///
/// This is not the rename a forge offers: the far side sees a branch
/// created and a branch deleted, so whatever hung off the old name — an
/// open pull request, a protected-branch rule — does not follow it. The UI
/// warns about this before it runs.
pub async fn rename_remote_branch(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    from: &str,
    to: &str,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let spec = PushSpec {
        remote: remote.to_string(),
        local: format!("refs/remotes/{remote}/{from}"),
        remote_branch: to.to_string(),
        set_upstream: false,
        force: PushForce::None,
    };
    push(executor, workdir, &spec, timeout, cancel).await?;
    delete_remote_branch(executor, workdir, remote, from, timeout, cancel).await?;
    for branch in tracking_branches(executor, workdir, remote, from, cancel).await? {
        let cmd = GitCommand::new().cwd(workdir).args([
            "branch",
            &format!("--set-upstream-to={remote}/{to}"),
            "--end-of-options",
            &branch,
        ]);
        executor.run(cmd, cancel).await?;
    }
    Ok(())
}

/// Local branches configured to track `<remote>/<branch>`.
///
/// One read of the whole `branch.` section rather than a lookup per branch:
/// which local branches point at a remote one is not something git answers
/// directly, and the section is small.
async fn tracking_branches(
    executor: &GitExecutor,
    workdir: &Path,
    remote: &str,
    branch: &str,
    cancel: &CancellationToken,
) -> Result<Vec<String>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["config", "-z", "--get-regexp", r"^branch\."]);
    let out = executor.run_unchecked(cmd, cancel).await?;
    // 1 is "no matching keys" — a repository whose branches all stand on
    // their own.
    if out.code == 1 {
        return Ok(Vec::new());
    }
    if out.code != 0 {
        return Err(GitError::Failed {
            command: "git config --get-regexp branch".to_string(),
            code: out.code,
            stderr: out.failure_message(),
        });
    }
    Ok(parse_tracking(&out.stdout, remote, branch))
}

/// Picks the branches whose `remote` and `merge` both name the same remote
/// branch out of `git config -z --get-regexp ^branch\.` output.
fn parse_tracking(bytes: &[u8], remote: &str, branch: &str) -> Vec<String> {
    let merge_ref = format!("refs/heads/{branch}");
    let mut remotes: Vec<(String, String)> = Vec::new();
    let mut merges: Vec<(String, String)> = Vec::new();
    for record in bytes.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let text = String::from_utf8_lossy(record);
        let Some((key, value)) = text.split_once('\n') else {
            continue;
        };
        // `branch.<name>.<field>` — a branch name may contain dots, so the
        // field is the last segment and everything between is the name.
        let Some(rest) = key.strip_prefix("branch.") else {
            continue;
        };
        let Some((name, field)) = rest.rsplit_once('.') else {
            continue;
        };
        match field {
            "remote" => remotes.push((name.to_string(), value.to_string())),
            "merge" => merges.push((name.to_string(), value.to_string())),
            _ => {}
        }
    }
    remotes
        .into_iter()
        .filter(|(_, value)| value == remote)
        .map(|(name, _)| name)
        .filter(|name| {
            merges
                .iter()
                .any(|(other, value)| other == name && *value == merge_ref)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z(records: &[&str]) -> Vec<u8> {
        let mut v = Vec::new();
        for r in records {
            v.extend_from_slice(r.as_bytes());
            v.push(0);
        }
        v
    }

    #[test]
    fn parses_urls_and_push_urls() {
        let bytes = z(&[
            "remote.origin.url\nhttps://example.invalid/a.git",
            "remote.origin.pushurl\nssh://example.invalid/a.git",
            "remote.upstream.url\nhttps://example.invalid/b.git",
        ]);
        let remotes = parse_remote_config(&bytes);
        assert_eq!(remotes.len(), 2);
        assert_eq!(remotes[0].name, "origin");
        assert_eq!(remotes[0].fetch_url, "https://example.invalid/a.git");
        assert_eq!(remotes[0].push_url, "ssh://example.invalid/a.git");
        assert_eq!(remotes[1].name, "upstream");
        assert_eq!(
            remotes[1].push_url, remotes[1].fetch_url,
            "push falls back to the fetch URL"
        );
    }

    #[test]
    fn remote_names_may_contain_dots() {
        let bytes = z(&["remote.my.fork.url\nfile:///tmp/x"]);
        let remotes = parse_remote_config(&bytes);
        assert_eq!(remotes[0].name, "my.fork");
    }

    #[test]
    fn empty_config_yields_no_remotes() {
        assert!(parse_remote_config(b"").is_empty());
    }

    /// Recorded from git 2.51 pushing to a local bare repository (the
    /// refusals from a second clone pushing first, from a lease pinned to
    /// what it had left, and from a `pre-receive` hook exiting non-zero).
    /// Every line here is `--porcelain` output as git wrote it.
    const REFUSED_FETCH_FIRST: &str = "To C:/tmp/remote.git\n\
         !\trefs/heads/main:refs/heads/main\t[rejected] (fetch first)\nDone\n";
    const REFUSED_STALE_LEASE: &str = "To C:/tmp/remote.git\n\
         !\trefs/heads/main:refs/heads/main\t[rejected] (stale info)\nDone\n";
    const FORCED_UPDATE: &str = "To C:/tmp/remote.git\n\
         +\trefs/heads/main:refs/heads/main\t34158f1...6e03ce6 (forced update)\nDone\n";
    const UP_TO_DATE: &str = "To C:/tmp/remote.git\n\
         =\trefs/heads/main:refs/heads/main\t[up to date]\nDone\n";
    const NEW_BRANCH: &str = "To C:/tmp/remote.git\n\
         *\trefs/heads/side:refs/heads/side\t[new branch]\nDone\n";

    #[test]
    fn a_refusal_a_fetch_would_answer_is_recognised() {
        assert!(is_outdated(REFUSED_FETCH_FIRST));
        assert!(is_outdated(REFUSED_STALE_LEASE));
    }

    #[test]
    fn pushes_that_landed_are_not_refusals() {
        assert!(!is_outdated(FORCED_UPDATE));
        assert!(!is_outdated(UP_TO_DATE));
        assert!(!is_outdated(NEW_BRANCH));
    }

    /// A refusal the remote decided on its own terms. Fetching tells us
    /// nothing about it, so it must not be dressed up as something to
    /// retry — and neither must a host that never answered, which prints
    /// no ref lines at all.
    #[test]
    fn refusals_a_fetch_cannot_help_with_are_left_alone() {
        assert!(!is_outdated(
            "To C:/tmp/remote.git\n\
             !\trefs/heads/main:refs/heads/main\t[remote rejected] (pre-receive hook declined)\nDone\n"
        ));
        assert!(!is_outdated(""));
    }

    #[test]
    fn tracking_needs_both_halves_to_agree() {
        let bytes = z(&[
            "branch.billing.remote\norigin",
            "branch.billing.merge\nrefs/heads/billing",
            // Same name over there, but on another remote.
            "branch.mirror.remote\nupstream",
            "branch.mirror.merge\nrefs/heads/billing",
            // Same remote, another branch.
            "branch.main.remote\norigin",
            "branch.main.merge\nrefs/heads/main",
            // Half a setting is no setting.
            "branch.orphan.remote\norigin",
        ]);
        assert_eq!(parse_tracking(&bytes, "origin", "billing"), ["billing"]);
    }

    /// Recorded from git 2.51 running `ls-remote --tags` against a local
    /// repository holding one annotated tag (`ann-1`, advertised twice) and
    /// three lightweight ones. Byte for byte as git wrote it to stdout.
    const LS_REMOTE_TAGS: &[u8] = b"\
a56decf8286d58887d4e0bf7bda73adb7a7ec957\trefs/tags/ann-1\n\
4b85e9d7b2bc03f629f3331705f5049e4522ce31\trefs/tags/ann-1^{}\n\
b132edc248d8082a5019061b53fc2e3de31cc085\trefs/tags/drift\n\
4b85e9d7b2bc03f629f3331705f5049e4522ce31\trefs/tags/light-1\n\
b132edc248d8082a5019061b53fc2e3de31cc085\trefs/tags/only-remote\n";

    #[test]
    fn an_annotated_tag_is_read_at_the_commit_it_peels_to() {
        let tags = parse_ls_remote_tags(LS_REMOTE_TAGS);
        assert_eq!(
            tags.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
            ["ann-1", "drift", "light-1", "only-remote"],
            "the ^{{}} line is the same tag, not another one"
        );
        assert_eq!(
            tags[0].commit.to_hex(),
            "4b85e9d7b2bc03f629f3331705f5049e4522ce31",
            "the peeled commit, not the tag object"
        );
        assert!(tags[0].annotated);
        assert!(!tags[1].annotated, "a lightweight tag has nothing to peel");
    }

    #[test]
    fn the_peeled_line_wins_whichever_side_it_arrives_on() {
        // git advertises the tag object first, but nothing in the protocol
        // promises the order, and a reversed pair must read the same.
        let reversed = b"\
4b85e9d7b2bc03f629f3331705f5049e4522ce31\trefs/tags/ann-1^{}\n\
a56decf8286d58887d4e0bf7bda73adb7a7ec957\trefs/tags/ann-1\n";
        let tags = parse_ls_remote_tags(reversed);
        assert_eq!(tags.len(), 1);
        assert_eq!(
            tags[0].commit.to_hex(),
            "4b85e9d7b2bc03f629f3331705f5049e4522ce31"
        );
        assert!(tags[0].annotated);
    }

    #[test]
    fn a_remote_with_no_tags_lists_none() {
        // Measured: git prints nothing at all and exits 0.
        assert!(parse_ls_remote_tags(b"").is_empty());
    }

    #[test]
    fn lines_outside_the_tag_namespace_are_left_out() {
        let bytes = b"\
4b85e9d7b2bc03f629f3331705f5049e4522ce31\tHEAD\n\
4b85e9d7b2bc03f629f3331705f5049e4522ce31\trefs/heads/main\n\
b132edc248d8082a5019061b53fc2e3de31cc085\trefs/tags/v1\n\
garbage-without-a-tab\n\
nothex\trefs/tags/v2\n";
        let tags = parse_ls_remote_tags(bytes);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "v1");
    }

    #[test]
    fn a_branch_name_with_dots_keeps_its_name() {
        let bytes = z(&[
            "branch.release.8.4.remote\norigin",
            "branch.release.8.4.merge\nrefs/heads/release.8.4",
        ]);
        assert_eq!(
            parse_tracking(&bytes, "origin", "release.8.4"),
            ["release.8.4"]
        );
    }
}
