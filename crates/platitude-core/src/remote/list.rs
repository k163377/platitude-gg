//! The configured remotes themselves: reading them, adding one,
//! correcting a URL, and the config reads a push plan plans from. The
//! push marks are [`super::marks`].

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

use super::marks::{PushDefault, push_default};

/// A configured remote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Remote {
    pub name: String,
    pub fetch_url: String,
    /// `remote.<name>.pushurl` when set, otherwise the fetch URL.
    pub push_url: String,
}

/// What the configuration says about remotes: the ones written down, and
/// where a push goes when no branch says otherwise.
///
/// The two are read together because they live in one file and are dropped
/// by one stat of it (`RepoSession::remotes`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Remotes {
    pub list: Vec<Remote>,
    pub push_default: Option<PushDefault>,
}

/// Both reads at once — what the session keeps.
pub async fn read(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Remotes, GitError> {
    Ok(Remotes {
        list: list(executor, workdir, cancel).await?,
        push_default: push_default(executor, workdir, cancel).await?,
    })
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
    let out = config::get_regexp(
        executor,
        workdir,
        r"^remote\..*\.(url|pushurl)$",
        "git config --get-regexp remote",
        cancel,
    )
    .await?;
    Ok(parse_remote_config(&out))
}

fn parse_remote_config(bytes: &[u8]) -> Vec<Remote> {
    let mut remotes: Vec<Remote> = Vec::new();
    for record in config::parse_z_records(bytes) {
        // A remote is its URL; a key written without one names nothing.
        let Some(value) = record.value() else {
            continue;
        };
        // `remote.<name>.url` — the name itself may contain dots, so take
        // the first and last segments and treat the middle as the name.
        let Some(rest) = record.key().strip_prefix("remote.") else {
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

/// `git remote add <name> <url>`.
///
/// Nothing is contacted: git records the URL and reports success even for a
/// host that does not exist, so a bad URL is only found out by the push that
/// follows (measured). The remote survives that failure, which is why the UI
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

/// Short name of the checked-out branch; an error when HEAD is detached.
pub(super) async fn current_branch(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<String, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        // Exit 1 is the answer "detached" — reported to the caller as the
        // error below, not raised again by the command log.
        .answers_by_code(1)
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
pub(super) async fn config_value(
    executor: &GitExecutor,
    workdir: &Path,
    key: &str,
    cancel: &CancellationToken,
) -> Result<Option<String>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        // The key not being set answers with code 1, which is an answer.
        .answers_by_code(1)
        .args(["config", "--get", "--", key]);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => {
            let value = out.stdout_utf8().trim().to_string();
            Ok((!value.is_empty()).then_some(value))
        }
        // 1 is git's "no such key" — a valid empty answer. Anything else
        // (128 on an unreadable config) must not read as "unset": a push
        // planned on that misreading rewrites upstreams (measured: a bad
        // config line makes `git config --get` exit 128, not 1).
        1 => Ok(None),
        code => Err(GitError::Failed {
            command: format!("git config --get -- {key}"),
            code,
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::z;
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
}
