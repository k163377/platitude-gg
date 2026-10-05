//! The configured remotes themselves: reading them, adding one,
//! correcting a URL, and the config reads a push plan plans from. The
//! push marks are [`super::marks`].

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

use super::marks::{OriginMarks, PushDefault, origin_marks};

/// A configured remote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Remote {
    pub name: String,
    pub fetch_url: String,
    /// `remote.<name>.pushurl` when set, otherwise the fetch URL.
    pub push_url: String,
}

/// The configured remotes and the origin marks ([`OriginMarks`]), read
/// together because one stat of the one config file drops both
/// (`RepoSession::remotes`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Remotes {
    pub list: Vec<Remote>,
    pub push_default: Option<PushDefault>,
    pub checkout_default: Option<String>,
}

/// Both reads at once — what the session keeps.
pub async fn read(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Remotes, GitError> {
    let list = list(executor, workdir, cancel).await?;
    let OriginMarks {
        push_default,
        checkout_default,
    } = origin_marks(executor, workdir, cancel).await?;
    Ok(Remotes {
        list,
        push_default,
        checkout_default,
    })
}

/// Lists configured remotes from config keys, not `git remote -v`'s
/// human-facing output.
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
        // A remote name may contain dots: the field is the last segment.
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
/// Nothing is contacted: a bad URL is found only by the next push, and the
/// remote survives it (hence [`set_url`]). Names are git's to judge; its
/// refusals arrive as its own text.
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
        // Exit 1: detached, reported by the error below.
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

    /// A name and a URL are handed over after `--end-of-options`, so a name
    /// the reader typed with a leading `-` is judged by git as a name.
    #[tokio::test]
    async fn adding_and_correcting_a_remote_name_it_and_its_url_as_values() {
        let (exec, asked) = crate::refusing::git();
        let cancel = CancellationToken::new();
        let workdir = crate::refusing::nowhere();
        add(&exec, &workdir, "-o", "file:///far", &cancel)
            .await
            .expect_err("there is no git here to add with");
        set_url(&exec, &workdir, "-o", "file:///near", &cancel)
            .await
            .expect_err("there is no git here to correct with");
        assert_eq!(
            asked.displays(),
            [
                "git remote add --end-of-options -o file:///far",
                "git remote set-url --end-of-options -o file:///near",
            ]
        );
    }
}
