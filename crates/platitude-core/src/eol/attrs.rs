//! What git's own settings decide about a path before anything is
//! sampled.

use std::collections::HashMap;
use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// What git's own settings decide about a path before anything is sampled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ruling {
    /// `.gitattributes` says the path is not text — the only exclusion;
    /// the app keeps no path list of its own.
    NotText,
    /// git decides the stored endings itself, so a new file cannot disagree
    /// with its neighbours and there is nothing to compare.
    Normalised,
    /// Nothing decides it. The files around this one are the only answer.
    Open,
}

/// How many paths one `check-attr` is asked about at a time.
const ATTR_BATCH: usize = 200;

/// What git's settings say about one path.
pub async fn ruling(
    executor: &GitExecutor,
    workdir: &Path,
    path: &str,
    cancel: &CancellationToken,
) -> Result<Ruling, GitError> {
    let one = [path.to_string()];
    Ok(rulings(executor, workdir, &one, cancel)
        .await?
        .into_iter()
        .next()
        .unwrap_or(Ruling::Open))
}

/// [`ruling`] for many paths: one `config` read for the repository and one
/// `check-attr` per batch.
pub async fn rulings(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<Vec<Ruling>, GitError> {
    let converting = normalises(executor, workdir, cancel).await?;
    rulings_given(executor, workdir, paths, converting, cancel).await
}

/// [`rulings`] for a caller that already knows whether git normalises —
/// a property of the configuration a session reads once
/// (`RepoSession::normalising`).
pub async fn rulings_given(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    converting: bool,
    cancel: &CancellationToken,
) -> Result<Vec<Ruling>, GitError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::with_capacity(paths.len());
    // Chunked to stay under the command-line length limit; batches stay
    // large because the process is most of the price
    // (ci/baseline/code-costs-windows-x64.md).
    for batch in paths.chunks(ATTR_BATCH) {
        for attrs in attributes(executor, workdir, batch, cancel).await? {
            out.push(match attrs {
                _ if attrs.not_text => Ruling::NotText,
                _ if attrs.decided || converting => Ruling::Normalised,
                _ => Ruling::Open,
            });
        }
    }
    Ok(out)
}

struct Attributes {
    /// `-text`: git is told this path is not text.
    not_text: bool,
    /// `text` or `eol` is spelled out, so what gets stored is settled.
    decided: bool,
}

async fn attributes(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<Vec<Attributes>, GitError> {
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["check-attr", "-z", "text", "eol", "--"]);
    for p in paths {
        cmd = cmd.arg(p.as_str());
    }
    let out = executor.run(cmd, cancel).await?;
    // `-z` prints a `path\0attr\0value\0` triple per path per attribute.
    // Keyed by path: nothing promises the order, and a dropped path would
    // shift every answer after it.
    let mut found: HashMap<String, Attributes> = HashMap::new();
    let fields: Vec<&[u8]> = out.stdout.split(|b| *b == 0).collect();
    for triple in fields.chunks(3) {
        let [path, attr, value] = triple else {
            continue;
        };
        let entry = found
            .entry(String::from_utf8_lossy(path).into_owned())
            .or_insert(Attributes {
                not_text: false,
                decided: false,
            });
        match (
            String::from_utf8_lossy(attr).as_ref(),
            String::from_utf8_lossy(value).as_ref(),
        ) {
            ("text", "unset") => entry.not_text = true,
            ("text", "set") => entry.decided = true,
            ("eol", "lf" | "crlf") => entry.decided = true,
            _ => {}
        }
    }
    Ok(paths
        .iter()
        .map(|p| {
            found.remove(p).unwrap_or(Attributes {
                not_text: false,
                decided: false,
            })
        })
        .collect())
}

/// Whether `core.autocrlf` converts on the way into the index.
///
/// Read through [`super::setting`] so the notice and the settings screen
/// cannot give two answers about one key. `core.eol` is not consulted: it
/// only takes effect where the attribute or `autocrlf` already made a path
/// text.
pub async fn normalises(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    Ok(super::setting::effective(executor, workdir, cancel)
        .await?
        .is_some_and(super::setting::AutoCrlf::normalises))
}
