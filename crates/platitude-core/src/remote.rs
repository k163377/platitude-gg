//! Remotes: listing, fetch and push.
//!
//! This is the only place the application causes network traffic, and it
//! causes it the same way a terminal would — by running git. Authentication
//! is git's (`GIT_TERMINAL_PROMPT=0` keeps a missing credential helper from
//! hanging the process instead of failing).
//!
//! Network commands take a timeout rather than running unbounded: a wedged
//! connection must not leave a subprocess running forever. Cancellation is
//! the normal way to stop one early; the timeout is the backstop for a
//! connection that neither finishes nor fails.

use std::path::Path;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// Default time budget for commands that talk to a remote.
///
/// Three minutes covers an ordinary fetch of a large repository over a slow
/// link without leaving a hung connection running for an hour. A user on a
/// genuinely slow line can raise it (the setting is persisted in Phase 4).
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
    if out.code != 0 {
        return Ok(None);
    }
    let value = out.stdout_utf8().trim().to_string();
    Ok((!value.is_empty()).then_some(value))
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
/// says so before this runs, and holds the answer down to mean it.
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
