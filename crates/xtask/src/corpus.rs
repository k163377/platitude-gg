//! `cargo xtask corpus` — the repository the performance record is
//! measured against, built here.
//!
//! **Why it is generated.** A live clone of `JetBrains/kotlin` is
//! somebody's working copy: an editor fetches it, and a fetch changes
//! the rows the graph draws, the ref tables the memory is mostly made
//! of, and the commit whose diff is timed — all while `HEAD` holds
//! still. A record taken against it stops
//! being comparable without anything visibly happening
//! (ci/baseline/perf-windows-x64.md §この記録の読み方 2).
//!
//! **Why the generator is what git holds.** It is 200,000 commits,
//! 50,000 refs and a hundred thousand tracked files — seven and a half
//! gigabytes on disk, built in six minutes from `shape`'s constants.
//! The generator is the thing worth keeping.
//!
//! **What the six minutes is.** Nine and a half million objects through
//! `git fast-import`, which reads a stream on one thread. The bodies
//! are two thirds of that thread's work and go through four processes
//! at once (`BLOB_IMPORTS`); the trees and the commits are one history
//! and go through one, and that one is most of what is left. What would
//! move it further is asking for fewer objects, which is a fidelity
//! decision (`shape`, `tree`).
//!
//! **Why it is built once.** Every seat measures the same corpus, so it
//! sits beside the primary checkout's `.git`, and a run that finds it
//! already there does nothing. Delete it and the next `corpus` builds
//! it again — to the same object ids, because the dates and the
//! strings are fixed (`shape`).

mod build;
mod copies;
mod probe;
mod readings;
mod remotes;
mod shape;
mod stream;
mod tree;

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

/// The directory name, ignored and beside the primary checkout's `.git`
/// like the shot board and the chip ledger: one for all six seats, in
/// the project and outside every seat's own tree (inside one it would
/// be six copies, one `cargo clean` short of gone).
///
/// **A `git clean -xfd` in the primary checkout takes it**, as it takes
/// the board — it is ignored, which is what an ignored directory means.
/// Building it again is the whole recovery.
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
            "--copies" => {
                i += 1;
                wanted = Some(args.get(i).and_then(|n| n.parse::<usize>().ok()).ok_or(
                    "--copies takes how many working copies stand beside the corpus, 0 for none",
                )?);
            }
            other => return Err(format!("corpus does not take {other:?}")),
        }
        i += 1;
    }
    if let Some(other) = reference {
        if force || path.is_some() || wanted.is_some() {
            return Err("--against only reads: drop --force, --path or --copies".to_string());
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
        // Minutes of fast-import on every core: announced like a build
        // (`still`), and only on this path — a corpus already there is
        // only read.
        let _busy = crate::still::busy(&crate::tree::workspace_root(), "corpus")?;
        build(&at)?;
    }
    if let Some(count) = wanted {
        copies::set_copies(&at, count)?;
    }
    if probe {
        return probe::run(&at, &copies::copy_path(&at, 1));
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

/// The `--path` a caller gave, made absolute.
///
/// **A relative path is written into the corpus and read from
/// somewhere else.** The mirrors point at the corpus's objects through
/// `objects/info/alternates`, which git resolves against the mirror's
/// own `objects/`, and the remote URLs are resolved against wherever
/// the git process runs — so a corpus built at `target/corpus` had
/// mirrors looking for `…/pgg-remotes/JetBrains.git/objects/target/corpus/.git/objects`
/// and failed on the first ref written through them (measured). The
/// default path is absolute already (`default_path`); this makes a
/// given one the same.
fn given_path(path: PathBuf) -> Result<PathBuf, String> {
    std::path::absolute(&path).map_err(|e| format!("could not resolve {}: {e}", path.display()))
}

/// In the primary checkout, whichever seat is asking.
///
/// `--git-common-dir` answers the *shared* git directory, so six seats
/// resolve to one corpus — the same call the shot board is placed by
/// (`shots::board::board_dir`).
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

/// What a caller needs to measure against it: where it is, what it is,
/// and the two files the record opens.
fn report(at: &Path) -> Result<(), String> {
    let refs = git(at, &["show-ref"])?;
    let commits = git(at, &["rev-list", "--all", "--count"])?;
    // The copies' branches (`copies::stand_copies`) are refs of this
    // generator's own making: counted out here, and still in the token,
    // which is what a run is compared under.
    let counted = refs
        .lines()
        .filter(|l| !l.is_empty() && !l.contains(" refs/heads/pgg-copy-"))
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

/// The same readings, taken of a repository this generator did not
/// build.
///
/// **A distance table is only worth reading if both of its columns came
/// from here.** Every row of one is a definition — which commits the
/// window holds, what counts as an open lane, which of them carry a
/// chip — and two implementations of a definition are two definitions,
/// which is how a table comes to compare a corpus against a reading of
/// the reference repository that was never taken the same way.
///
/// **It only reads.** The reference repository is somebody's working
/// clone: nothing here writes to it, down to the status (`worktree`).
fn against(at: &Path) -> Result<(), String> {
    let refs = git(at, &["show-ref"])?;
    let commits = git(at, &["rev-list", "--all", "--count"])?;
    let counted = refs.lines().filter(|line| !line.is_empty()).count();
    println!("reference: {}", at.display());
    println!("  commits {} | refs {counted}", commits.trim());
    readings(at)
}

/// Whether what is on disk is what this generator describes.
///
/// A corpus that is merely *present* is not one that can be measured
/// against: the generator's constants move, and a directory built
/// before they did answers every other check — it has a `.git`, it has
/// refs, it prints a token. The counts are what say it is stale, and
/// the answer is always the same one, so it is in the message.
///
/// **Three counts.** A generator whose lane
/// widths or file sizes moved leaves all three standing, and what says
/// *that* is the token the record carries. These are the ones a
/// half-finished or superseded build gets wrong on its own.
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
    // The tracked count is the third leg because it is the one the
    // record's startup number rests on, and the one a half-written
    // checkout gets wrong while the counts above still answer.
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

    /// A `--path` reaches the mirrors' `alternates` and the remote URLs,
    /// which git resolves against directories of its own choosing — so
    /// a relative one has to be made absolute before anything is
    /// written from it.
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

    /// One corpus in the primary checkout, whichever seat asks — the
    /// answer comes from the shared directory alone, or six seats
    /// would build six of it. `--git-common-dir` is what makes that
    /// true: it answers that directory, so every seat is handed the
    /// same string this works from.
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

    /// `--force` removes what `--path` names, so it may only ever name
    /// a corpus. A seat that typed `--path .` would otherwise delete
    /// its own checkout.
    #[test]
    fn force_will_not_delete_something_that_is_not_a_corpus() {
        assert!(clearable(std::path::Path::new("/tmp/.pgg-perf-corpus")).is_ok());
        let refused = clearable(std::path::Path::new("/home/x/platitude-gg"))
            .expect_err("a checkout is not a corpus");
        assert!(refused.contains(".pgg-perf-corpus"), "{refused}");
    }
}
