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
    /// `.gitattributes` says the path is not text. This is the only
    /// exclusion mechanism — the app keeps no path list of its own; build
    /// output, test data and the rest are `.gitattributes`' business.
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
/// `check-attr` per batch, rather than two spawns per path.
pub async fn rulings(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<Vec<Ruling>, GitError> {
    let converting = normalises(executor, workdir, cancel).await?;
    rulings_given(executor, workdir, paths, converting, cancel).await
}

/// [`rulings`] for a caller that already knows whether git normalises.
///
/// Whether it does is a property of the repository's configuration, not of
/// the path, so a session that has read it once can hand the answer down
/// and every diff after the first opens with one fewer process
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
    // Measured against the reference repository: a 200-path batch answers
    // in 71ms vs 42ms per one-path spawn. Chunking also keeps a long list
    // under the command-line length limit.
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
    // `-z` prints one `path\0attr\0value\0` triple per attribute asked for,
    // so a path answering for two attributes takes two triples. Keyed by
    // path rather than by position: nothing promises the order, and a path
    // git dropped would otherwise shift every answer after it.
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
/// **`--get-regexp` answers from every config level at once, lowest first,
/// and the last one is the effective value.** Reading the first match makes
/// every repository on Windows look like it normalises: the Git for Windows
/// installer writes `core.autocrlf=true` into the system config, and a
/// repository that sets `false` for itself shows up as the second record
/// (measured on this machine — system `true`, repo `false`, in that order).
///
/// `core.eol` is read in the same breath and deliberately not consulted:
/// it only takes effect where a path is already text by attribute or by
/// `autocrlf`, both of which have answered by then, so on its own it never
/// decides anything.
pub async fn normalises(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        // Neither key being set answers with code 1, which is an answer.
        .answers_by_code()
        .args(["config", "-z", "--get-regexp", r"^core\.(autocrlf|eol)$"]);
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 1 {
        return Ok(false);
    }
    if out.code != 0 {
        return Err(GitError::Failed {
            command: "git config --get-regexp core.autocrlf".to_string(),
            code: out.code,
            stderr: out.failure_message(),
        });
    }
    let mut effective = false;
    for record in out.stdout.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let text = String::from_utf8_lossy(record);
        let Some((key, value)) = text.split_once('\n') else {
            continue;
        };
        if key.trim() == "core.autocrlf" {
            // `input` converts on the way in and not on the way out, which
            // is still git deciding what gets stored.
            effective = matches!(value.trim(), "true" | "input");
        }
    }
    Ok(effective)
}
