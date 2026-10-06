//! `cargo xtask corpus` — builds the repository the performance record is
//! measured against.
//!
//! Generated, not a live clone of `JetBrains/kotlin`: a fetch changes the
//! rows, the ref tables and the timed commit while `HEAD` holds still, and
//! the record stops being comparable without anything visibly happening
//! (ci/baseline/perf-windows-x64.md §この記録の読み方 2). The output is
//! gigabytes, so git holds the generator.
//!
//! The cost is millions of objects through `git fast-import`, which reads
//! its stream on one thread; the bodies go through four processes at once
//! (`BLOB_IMPORTS`), the commits and trees through one. Going further means
//! asking for fewer objects — a fidelity decision (`shape`, `tree`).
//!
//! Built once, beside the primary checkout's `.git`, for every seat; a run
//! that finds it does nothing. A rebuild gives the same object ids, because
//! the dates and strings are fixed (`shape`).

mod build;
mod probe;
mod readings;
mod remotes;
mod shape;
mod stream;
mod tree;
mod worktrees;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::command::{self, Permission, Where};
use build::build;
use readings::readings;

pub(crate) static CORPUS: command::Command = command::Command {
    id: "corpus.build",
    call: "corpus",
    purpose: "generate the synthetic repository the performance record is measured against",
    run_in: Where::Seat,
    needs: &["room for the generated repository, and the minutes it takes"],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&CORPUS];

/// Ignored, beside the primary checkout's `.git` like the shot board: one
/// for all seats, outside every seat's own tree (inside, it would be one
/// per seat and one `cargo clean` from gone). A `git clean -xfd` in the
/// primary checkout takes it; rebuilding is the recovery.
const DIR_NAME: &str = ".pgg-perf-corpus";

pub fn run(args: &[String]) -> Result<(), String> {
    let mut force = false;
    let mut path = None;
    let mut reference = None;
    let mut wanted = None;
    let mut probe = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--force" => force = true,
            "--probe" => probe = true,
            "--path" => {
                i += 1;
                path = Some(PathBuf::from(
                    args.get(i).ok_or("--path needs a directory")?,
                ));
            }
            "--against" => {
                i += 1;
                reference = Some(PathBuf::from(
                    args.get(i).ok_or("--against needs a repository")?,
                ));
            }
            "--worktrees" => {
                i += 1;
                wanted = Some(args.get(i).and_then(|n| n.parse::<usize>().ok()).ok_or(
                    "--worktrees takes how many worktrees stand beside the corpus, 0 for none",
                )?);
            }
            other => return Err(format!("corpus does not take {other:?}")),
        }
        i += 1;
    }
    if let Some(other) = reference {
        if force || path.is_some() || wanted.is_some() {
            return Err("--against only reads: drop --force, --path or --worktrees".to_string());
        }
        return against(&other);
    }
    let at = path.map_or_else(default_path, given_path)?;
    if force && at.exists() {
        clearable(&at)?;
        std::fs::remove_dir_all(&at)
            .map_err(|e| format!("could not clear {}: {e}", at.display()))?;
    }
    if !at.join(".git").is_dir() {
        // Announced like a build (`still`) only here: a corpus already
        // there is only read.
        let _busy = crate::still::busy(&crate::tree::workspace_root(), "corpus")?;
        build(&at)?;
    }
    if let Some(count) = wanted {
        worktrees::set_worktrees(&at, count)?;
    }
    if probe {
        return probe::run(&at, &worktrees::nth_worktree_path(&at, 1));
    }
    report(&at)
}

/// Whether `--force` may delete this directory: only one named
/// `DIR_NAME`, or `--path .` from a seat would delete the checkout.
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

/// The `--path` a caller gave, made absolute: it is written into the
/// mirrors' `objects/info/alternates` (resolved against the mirror's own
/// `objects/`) and the remote URLs (resolved against git's working
/// directory), so a relative one points somewhere else.
fn given_path(path: PathBuf) -> Result<PathBuf, String> {
    std::path::absolute(&path).map_err(|e| format!("could not resolve {}: {e}", path.display()))
}

/// In the primary checkout, whichever seat asks: `--git-common-dir` is the
/// shared git directory, as for the shot board (`shots::board::board_dir`).
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

/// The corpus beside the shared git directory, in its checkout's root.
fn beside(common: &str) -> Option<PathBuf> {
    Path::new(common).parent().map(|root| root.join(DIR_NAME))
}

/// Where the corpus is, what it holds, and how to measure against it.
fn report(at: &Path) -> Result<(), String> {
    let refs = git(at, &["show-ref"])?;
    let commits = git(at, &["rev-list", "--all", "--count"])?;
    // The worktrees' branches (`worktrees::stand_worktrees`) are not counted, but
    // stay in the token a run is compared under.
    let counted = refs
        .lines()
        .filter(|l| !l.is_empty() && !l.contains(" refs/heads/pgg-worktree-"))
        .count();
    holds_its_shape(commits.trim(), counted, at)?;
    println!("corpus: {}", at.display());
    println!(
        "  commits {} | refs {} | token {}",
        commits.trim(),
        counted,
        token(at, &refs)?
    );
    readings(at)?;
    println!(
        "  measure it with: PGG_ALLOW_GUI=1 cargo xtask perf --repo {} --runs 5",
        at.display()
    );
    Ok(())
}

/// The same readings of a repository this generator did not build, so
/// both columns of a distance table come from one implementation of each
/// definition (window, lane, chip).
///
/// Read only: the reference is somebody's working clone, so nothing here
/// writes to it, down to the status (`readings::working_tree`).
fn against(at: &Path) -> Result<(), String> {
    let refs = git(at, &["show-ref"])?;
    let commits = git(at, &["rev-list", "--all", "--count"])?;
    let counted = refs.lines().filter(|line| !line.is_empty()).count();
    println!("reference: {}", at.display());
    println!("  commits {} | refs {counted}", commits.trim());
    readings(at)
}

/// Whether what is on disk is what this generator describes: a corpus
/// built before the constants moved still has a `.git`, refs and a token,
/// and only the counts say it is stale. They catch a half-finished or
/// superseded build; moved lane widths or file sizes leave them standing,
/// and the record's token says that.
fn holds_its_shape(commits: &str, refs: usize, at: &Path) -> Result<(), String> {
    let wanted_refs = shape::REFS as usize;
    let wanted_commits = shape::COMMITS.to_string();
    let tracked = git(at, &["ls-files"])?.lines().count();
    let wanted_tracked = tree::TRACKED as usize;
    let off = |what: &str, is: String, wants: String| {
        format!(
            "the corpus at {} has {is} {what} where this generator describes {wants} — it was \
             built by another version of it, or a build of it did not finish. Rebuild with: \
             cargo xtask corpus --force",
            at.display()
        )
    };
    if commits != wanted_commits {
        return Err(off("commits", commits.to_string(), wanted_commits));
    }
    if refs != wanted_refs {
        return Err(off("refs", refs.to_string(), wanted_refs.to_string()));
    }
    // A half-written checkout gets this wrong while the counts above
    // still answer.
    if tracked.abs_diff(wanted_tracked) > tree::SPARE as usize {
        return Err(off(
            "tracked files",
            tracked.to_string(),
            wanted_tracked.to_string(),
        ));
    }
    Ok(())
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
    use super::{beside, clearable, given_path};

    #[test]
    fn a_given_path_is_absolute_before_it_is_written_anywhere() {
        let given = given_path(std::path::PathBuf::from("target/corpus-x"))
            .expect("the working directory resolves");
        assert!(given.is_absolute(), "{}", given.display());
        assert!(given.ends_with("target/corpus-x"), "{}", given.display());
        let absolute = std::env::current_dir().expect("a working directory");
        assert_eq!(
            given_path(absolute.clone()).expect("an absolute path resolves"),
            absolute
        );
    }

    fn shown(common: &str) -> String {
        beside(common)
            .expect("a git directory sits in a checkout")
            .to_string_lossy()
            .replace('\\', "/")
    }

    /// The answer comes from the shared git directory alone, so every
    /// seat is pointed at the primary checkout's corpus.
    #[test]
    fn every_seat_is_pointed_at_the_same_corpus() {
        let from_seat = shown("C:/Users/x/IdeaProjects/platitude-gg/.git");
        assert_eq!(
            from_seat,
            "C:/Users/x/IdeaProjects/platitude-gg/.pgg-perf-corpus"
        );
        assert!(!from_seat.contains("/.claude/worktrees/"), "{from_seat}");
        assert_eq!(
            shown("/home/x/platitude-gg/.git"),
            from_seat.replace("C:/Users/x/IdeaProjects", "/home/x")
        );
    }

    #[test]
    fn force_will_not_delete_something_that_is_not_a_corpus() {
        assert!(clearable(std::path::Path::new("/tmp/.pgg-perf-corpus")).is_ok());
        let refused = clearable(std::path::Path::new("/home/x/platitude-gg"))
            .expect_err("a checkout is not a corpus");
        assert!(refused.contains(".pgg-perf-corpus"), "{refused}");
    }
}
