//! `cargo xtask corpus` — the repository the performance record is
//! measured against, built here rather than cloned from anywhere.
//!
//! **Why it is generated.** The record used to be taken against a live
//! clone of `JetBrains/kotlin`, which is somebody's working copy: an
//! editor fetches it, and a fetch changes the rows the graph draws, the
//! ref tables the memory is mostly made of, and the commit whose diff is
//! timed — all while `HEAD` holds still. A record taken against it stops
//! being comparable without anything visibly happening
//! (ci/baseline/perf-windows-x64.md §この記録の読み方 2).
//!
//! **Why it is not in git.** It is 200,000 commits and 50,000 refs. It
//! is built in under a minute from `shape`'s constants, so the thing
//! worth keeping is the generator, not its output.
//!
//! **Why it is built once.** Every seat measures the same corpus, so it
//! sits beside the primary checkout's `.git` rather than in any one
//! tree, and a run that finds it already there does nothing. Delete it
//! and the next `corpus` builds it again — to the same object ids,
//! because the dates and the strings are fixed (`shape`).

mod shape;
mod stream;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The directory name, ignored and beside the primary checkout's `.git`
/// like the shot board and the chip ledger: one for all six seats, in
/// the project rather than outside it, and never inside a seat's own
/// tree (six copies, and one `cargo clean` short of gone).
///
/// **A `git clean -xfd` in the primary checkout takes it**, as it takes
/// the board — it is ignored, which is what an ignored directory means.
/// Building it again is the whole recovery.
const DIR_NAME: &str = ".pg-perf-corpus";

pub fn run(args: &[String]) -> Result<(), String> {
    let mut force = false;
    let mut path = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--force" => force = true,
            "--path" => {
                i += 1;
                path = Some(PathBuf::from(
                    args.get(i).ok_or("--path needs a directory")?,
                ));
            }
            other => return Err(format!("corpus does not take {other:?}")),
        }
        i += 1;
    }
    let at = path.map_or_else(default_path, Ok)?;
    if force && at.exists() {
        clearable(&at)?;
        std::fs::remove_dir_all(&at)
            .map_err(|e| format!("could not clear {}: {e}", at.display()))?;
    }
    if !at.join(".git").is_dir() {
        build(&at)?;
    }
    report(&at)
}

/// Whether `--force` may delete this directory.
///
/// `--force` takes whatever `--path` was given and removes it whole, so
/// without this `--path .` from a seat deletes the checkout. A corpus is
/// only ever the one directory name, so that is the test: everything
/// else is somebody's own directory and is refused.
fn clearable(at: &Path) -> Result<(), String> {
    if at.file_name().is_some_and(|name| name == DIR_NAME) {
        return Ok(());
    }
    Err(format!(
        "{} is not a corpus — --force deletes what --path names, and only a directory called \
         {DIR_NAME} is that. Remove it by hand if you meant to.",
        at.display()
    ))
}

/// In the primary checkout, whichever seat is asking.
///
/// `--git-common-dir` answers the *shared* git directory, so six seats
/// resolve to one corpus rather than building six of it — the same call
/// the shot board is placed by (`shots::board::board_dir`).
fn default_path() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("no working directory: {e}"))?;
    let cwd = cwd.to_string_lossy().replace('\\', "/");
    let common = crate::subprocess::git_query(
        &cwd,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .ok_or("not in a git repository: the corpus lives in the primary checkout")?;
    beside(&common).ok_or_else(|| format!("{common} has no checkout to hold the corpus"))
}

/// Where the corpus goes, given the shared git directory: next to it,
/// which is the root of the checkout that holds it.
fn beside(common: &str) -> Option<PathBuf> {
    Path::new(common).parent().map(|root| root.join(DIR_NAME))
}

/// Builds it somewhere else and moves it into place.
///
/// **What is at `at` is either finished or absent.** `git init` makes
/// `.git` in milliseconds and the import that follows takes a hundred
/// seconds; a build interrupted anywhere in between would otherwise
/// leave a directory that every later run reads as "already built", and
/// the first thing to notice would be a measurement of half a corpus.
fn build(at: &Path) -> Result<(), String> {
    let partial = at.with_extension("partial");
    if partial.exists() {
        std::fs::remove_dir_all(&partial)
            .map_err(|e| format!("could not clear {}: {e}", partial.display()))?;
    }
    fill(&partial)?;
    std::fs::rename(&partial, at)
        .map_err(|e| format!("could not move the corpus into {}: {e}", at.display()))
}

fn fill(at: &Path) -> Result<(), String> {
    println!(
        "building the corpus at {} — {} commits, {} refs",
        at.display(),
        shape::COMMITS,
        shape::TAGS + shape::REMOTE_BRANCHES + 1
    );
    std::fs::create_dir_all(at).map_err(|e| format!("could not make {}: {e}", at.display()))?;
    // The hash algorithm is pinned rather than inherited: `init` would
    // otherwise take it from `init.defaultObjectFormat`, and the same
    // stream under sha256 produces different object ids — so a corpus
    // built on such a machine could never match a recorded token.
    git(
        at,
        &[
            "init",
            "--quiet",
            "--initial-branch=main",
            "--object-format=sha1",
        ],
    )?;
    // Nothing here is anybody's identity, and a machine whose global
    // config has none must still be able to build it.
    git(at, &["config", "user.name", "Corpus"])?;
    git(at, &["config", "user.email", "corpus@example.invalid"])?;
    let (text, newest) = stream::build();
    git_stdin(at, &["fast-import", "--quiet", "--force"], &text)?;
    // fast-import writes refs and nothing else: without this the work
    // tree is the empty one `init` left, and every tracked file reads as
    // deleted — which the application would show as an enormous
    // uncommitted change.
    git(at, &["reset", "--hard", "main"])?;
    // The chain the reference repository has. Startup reads it, so a
    // corpus without one is measuring a different walk.
    git(at, &["commit-graph", "write", "--reachable", "--split"])?;
    // 50,000 loose refs cost 80MB of slack and slow every walk that
    // reads them; packed they are 6MB (measured).
    git(at, &["pack-refs", "--all"])?;
    println!(
        "  the newest commit changes {} files; its first is {}",
        newest.paths.len() + 1,
        newest.paths.first().map_or("-", String::as_str)
    );
    println!("  the large source is {}", newest.huge);
    Ok(())
}

/// What a caller needs to measure against it: where it is, what it is,
/// and the two files the record opens.
fn report(at: &Path) -> Result<(), String> {
    let refs = git(at, &["show-ref"])?;
    let commits = git(at, &["rev-list", "--all", "--count"])?;
    let counted = refs.lines().filter(|l| !l.is_empty()).count();
    holds_its_shape(commits.trim(), counted, at)?;
    println!("corpus: {}", at.display());
    println!(
        "  commits {} | refs {} | token {}",
        commits.trim(),
        counted,
        token(at, &refs)?
    );
    println!(
        "  measure it with: PG_ALLOW_GUI=1 cargo xtask perf --repo {} --runs 5",
        at.display()
    );
    Ok(())
}

/// Whether what is on disk is what this generator describes.
///
/// A corpus that is merely *present* is not one that can be measured
/// against: the generator's constants move, and a directory built
/// before they did answers every other check — it has a `.git`, it has
/// refs, it prints a token. The counts are what say it is stale, and
/// the answer is always the same one, so it is in the message.
fn holds_its_shape(commits: &str, refs: usize, at: &Path) -> Result<(), String> {
    let wanted_refs = (shape::TAGS + shape::REMOTE_BRANCHES + 1) as usize;
    let wanted_commits = shape::COMMITS.to_string();
    if commits == wanted_commits && refs == wanted_refs {
        return Ok(());
    }
    Err(format!(
        "the corpus at {} is {commits} commits and {refs} refs, where this generator describes \
         {wanted_commits} and {wanted_refs} — it was built by another version of it, or a build \
         of it did not finish. Rebuild with: cargo xtask corpus --force",
        at.display()
    ))
}

/// The same fingerprint `perf` takes, so the two agree on what this is.
fn token(at: &Path, refs: &str) -> Result<String, String> {
    let mut child = Command::new("git")
        .current_dir(at)
        .args(["hash-object", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not fingerprint the corpus: {e}"))?;
    write_all(&mut child, refs)?;
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn git(at: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .current_dir(at)
        .args(args)
        .output()
        .map_err(|e| format!("failed to run git {args:?}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// The import stream is tens of megabytes, so it goes down a pipe while
/// git reads it rather than through a file nobody would clean up.
///
/// **The write is allowed to fail.** A git that rejects the stream exits
/// while the rest of it is still being written, and the broken pipe that
/// follows says nothing about why — so it is kept and only reported if
/// git itself turns out to have had nothing to say.
fn git_stdin(at: &Path, args: &[&str], input: &str) -> Result<(), String> {
    let mut child = Command::new("git")
        .current_dir(at)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn git {args:?}: {e}"))?;
    let wrote = write_all(&mut child, input);
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if !out.status.success() || wrote.is_err() {
        return Err(format!(
            "git {args:?} failed: {}",
            if said.is_empty() {
                wrote.err().unwrap_or_default()
            } else {
                said
            }
        ));
    }
    Ok(())
}

fn write_all(child: &mut std::process::Child, input: &str) -> Result<(), String> {
    use std::io::Write;
    child
        .stdin
        .take()
        .ok_or("git took no standard input")?
        .write_all(input.as_bytes())
        .map_err(|e| format!("writing to git: {e}"))
}

#[cfg(test)]
mod tests {
    use super::{beside, clearable};

    fn shown(common: &str) -> String {
        beside(common)
            .expect("a git directory sits in a checkout")
            .to_string_lossy()
            .replace('\\', "/")
    }

    /// One corpus in the primary checkout, whichever seat asks — a
    /// seat's own path must not reach the answer, or six seats would
    /// build six of it. `--git-common-dir` is what makes that true: it
    /// answers the shared directory, so every seat is handed the same
    /// string this works from.
    #[test]
    fn every_seat_is_pointed_at_the_same_corpus() {
        let from_seat = shown("C:/Users/x/IdeaProjects/platitude-gg/.git");
        assert_eq!(
            from_seat,
            "C:/Users/x/IdeaProjects/platitude-gg/.pg-perf-corpus"
        );
        assert!(!from_seat.contains("/.claude/worktrees/"), "{from_seat}");
        assert_eq!(
            shown("/home/x/platitude-gg/.git"),
            from_seat.replace("C:/Users/x/IdeaProjects", "/home/x")
        );
    }

    /// `--force` removes what `--path` names, so it may only ever name
    /// a corpus. A seat that typed `--path .` would otherwise delete
    /// its own checkout.
    #[test]
    fn force_will_not_delete_something_that_is_not_a_corpus() {
        assert!(clearable(std::path::Path::new("/tmp/.pg-perf-corpus")).is_ok());
        let refused = clearable(std::path::Path::new("/home/x/platitude-gg"))
            .expect_err("a checkout is not a corpus");
        assert!(refused.contains(".pg-perf-corpus"), "{refused}");
    }
}
