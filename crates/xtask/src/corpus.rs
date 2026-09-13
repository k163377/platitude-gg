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
//! **Why it is not in git.** It is 200,000 commits, 50,000 refs and a
//! hundred thousand tracked files — seven and a half gigabytes on disk,
//! built in six minutes from `shape`'s constants. The generator is the
//! thing worth keeping, not its output.
//!
//! **What the six minutes is.** Nine and a half million objects through
//! `git fast-import`, which reads a stream on one thread. The bodies
//! are two thirds of that thread's work and go through four processes
//! at once (`BLOB_IMPORTS`); the trees and the commits are one history
//! and go through one, and that one is most of what is left. What would
//! move it further is asking for fewer objects, which is a fidelity
//! decision rather than a speed one (`shape`, `tree`).
//!
//! **Why it is built once.** Every seat measures the same corpus, so it
//! sits beside the primary checkout's `.git` rather than in any one
//! tree, and a run that finds it already there does nothing. Delete it
//! and the next `corpus` builds it again — to the same object ids,
//! because the dates and the strings are fixed (`shape`).

mod probe;
mod remotes;
mod shape;
mod stream;
mod tree;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::command::{self, Permission, Where};

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
/// the project rather than outside it, and never inside a seat's own
/// tree (six copies, and one `cargo clean` short of gone).
///
/// **A `git clean -xfd` in the primary checkout takes it**, as it takes
/// the board — it is ignored, which is what an ignored directory means.
/// Building it again is the whole recovery.
const DIR_NAME: &str = ".pgg-perf-corpus";

pub fn run(args: &[String]) -> Result<(), String> {
    let mut force = false;
    let mut path = None;
    let mut reference = None;
    let mut copies = None;
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
                copies =
                    Some(args.get(i).and_then(|n| n.parse::<usize>().ok()).ok_or(
                        "--copies takes how many working copies to stand beside the corpus",
                    )?);
            }
            other => return Err(format!("corpus does not take {other:?}")),
        }
        i += 1;
    }
    if let Some(other) = reference {
        if force || path.is_some() || copies.is_some() {
            return Err(
                "--against only reads; it takes neither --force, --path nor --copies".to_string(),
            );
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
        // read, not made.
        let _busy = crate::still::busy(&crate::tree::workspace_root(), "corpus")?;
        build(&at)?;
    }
    if let Some(copies) = copies {
        stand_copies(&at, copies)?;
    }
    if probe {
        return probe::run(&at, &copy_path(&at, 1));
    }
    report(&at)
}

/// Where the `nth` working copy of the corpus at `at` stands
/// (`stand_copies`).
fn copy_path(at: &Path, nth: usize) -> PathBuf {
    let name = at
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    at.with_file_name(format!("{name}-copy-{nth}"))
}

/// The other working copies the slots measurement reads beside the
/// corpus (ci/baseline/git-slots-windows-x64.md): `count` linked
/// working trees of it, each holding one untracked file so the pass
/// that reads them has a row to find, and named for their number beside
/// the corpus (`<corpus>-copy-<n>`, ignored like the corpus itself).
///
/// **Each on a branch of its own (`pgg-copy-<n>`), not detached.** A
/// copy standing on no branch is a row only the walk can draw
/// (`session::joins::WorktreeNews`), so the opening's listing asks for a
/// rebuild that takes the opening stream over before its first chunk —
/// and a run the harness cannot see the walk of is no reading at all.
/// The branches are refs, so **the corpus token moves by eight**: a run
/// against a corpus with copies is compared with runs against the same,
/// and the record says which token it was taken under. Taking the
/// copies down puts the token back (`git worktree remove` each, then
/// `git branch -D pgg-copy-<n>`).
///
/// A copy already standing is kept and only put on its branch where it
/// is detached — a `checkout -b` at the same commit moves no file. One
/// asked for beyond what stands is added; none is ever taken away here.
fn stand_copies(at: &Path, count: usize) -> Result<(), String> {
    for nth in 1..=count {
        let copy = copy_path(at, nth);
        let copy_text = copy.to_string_lossy().replace('\\', "/");
        let branch = format!("pgg-copy-{nth}");
        if !copy.join(".git").exists() {
            println!("standing copy {nth}: {copy_text}");
            git(at, &["worktree", "add", "-b", &branch, &copy_text, "HEAD"])?;
            // One untracked file: the cheapest dirt there is, and enough
            // for the read to count the copy as carrying something.
            std::fs::write(
                copy.join(format!("carried-by-copy-{nth}.txt")),
                format!("copy {nth}\n"),
            )
            .map_err(|e| format!("could not dirty {copy_text}: {e}"))?;
            continue;
        }
        let standing = git(&copy, &["symbolic-ref", "-q", "--short", "HEAD"]).unwrap_or_default();
        if standing.trim().is_empty() {
            println!("putting copy {nth} on {branch}");
            git(&copy, &["checkout", "-b", &branch])?;
        }
    }
    Ok(())
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
    sweep(at)?;
    // Named for this process, so two seats building at once are two
    // builds rather than one destroyed twice.
    let partial = partial_path(at, std::process::id());
    if partial.exists() {
        std::fs::remove_dir_all(&partial)
            .map_err(|e| format!("could not clear {}: {e}", partial.display()))?;
    }
    let mut clock = Clock::start();
    fill(&partial, &mut clock)?;
    let ours = settle(&partial, at)?;
    clock.mark("move");
    if !ours {
        clock.say();
        return Ok(());
    }
    // **After the move, because a remote records where it was told to
    // look.** Configured before it, every URL would name the scratch
    // directory the move then takes away — and the proof would pass,
    // because the proof would be reading the scratch copy too. The
    // corpus that gets measured is this one.
    remotes::configure(at)?;
    clock.mark("remotes");
    clock.say();
    Ok(())
}

/// Where a build in progress sits: beside its destination, under the
/// destination's own name and the process building it.
fn partial_path(at: &Path, pid: u32) -> PathBuf {
    let name = at
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    at.with_file_name(format!("{name}.partial-{pid}"))
}

/// The process a directory beside `at` was being built by, if it is a
/// partial build of `at` at all.
fn partial_owner(at: &Path, entry: &Path) -> Option<u32> {
    let prefix = format!("{}.partial-", at.file_name()?.to_string_lossy());
    entry
        .file_name()?
        .to_str()?
        .strip_prefix(&prefix)?
        .parse()
        .ok()
}

/// Clears the partial builds nobody is behind any more.
///
/// **A build that died leaves seven gigabytes under a name nothing
/// looks for.** The partial is named for its process so that two builds
/// cannot destroy each other, but the next build is a different process
/// and only ever clears its own name — so an interrupted build's tree
/// would sit beside the corpus, ignored by git and read by no later
/// run, until somebody wondered where the disk went. Each is asked
/// about by pid: a process still running is another seat building, and
/// is left alone.
fn sweep(at: &Path) -> Result<(), String> {
    let parent = match at.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Ok(());
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Some(pid) = partial_owner(at, &path) else {
            continue;
        };
        if pid == std::process::id() {
            continue;
        }
        if crate::subprocess::process_exists(pid) {
            println!(
                "  another build is running at {} (pid {pid}) — leaving it",
                path.display()
            );
            continue;
        }
        std::fs::remove_dir_all(&path)
            .map_err(|e| format!("could not clear {}: {e}", path.display()))?;
        println!(
            "  cleared {}, left by a build that did not finish (pid {pid} is gone)",
            path.display()
        );
    }
    Ok(())
}

/// Moves the finished corpus into place, waiting out whatever is still
/// holding it. Answers whether the corpus at `at` is this build's:
/// `false` when another build got there first, in which case this one's
/// tree has been thrown away and theirs is the one to report.
///
/// **A rename can be refused for a while.** The build has just written
/// a hundred and eighty thousand files, and on Windows the scanner or
/// the indexer walking them keeps handles open into the tree for
/// seconds afterwards — long enough that the rename fails with an
/// access error against a destination that does not exist (measured,
/// after a ten-minute build). Waiting is the whole fix.
///
/// **And it can be refused for good.** Two seats that started building
/// at once both finish, and the second rename lands on a corpus that is
/// already there — which would otherwise be retried for two minutes and
/// then reported as a handle somebody left open. A `.git` at the
/// destination says which of the two this is; anything else sitting
/// there is not a corpus and is named at once.
fn settle(partial: &Path, at: &Path) -> Result<bool, String> {
    let mut wait = crate::wait::Wait::new(
        "the finished corpus",
        crate::wait::Budget::whole(RENAME_CEILING),
        crate::wait::LOOK_AGAIN,
    );
    loop {
        let Err(refused) = std::fs::rename(partial, at) else {
            return Ok(true);
        };
        if at.join(".git").is_dir() {
            println!(
                "  another build finished first at {} — discarding this one",
                at.display()
            );
            // A failure here is not this run's: the next build sweeps
            // what its process left (`sweep`), and the corpus to report
            // is already in place.
            if let Err(e) = std::fs::remove_dir_all(partial) {
                println!(
                    "  could not clear {}: {e} — the next build clears it",
                    partial.display()
                );
            }
            return Ok(false);
        }
        if at.exists() {
            return Err(format!(
                "{} is in the way and is not a corpus. The build is complete at {}; move it \
                 there once that is out of the way.",
                at.display(),
                partial.display()
            ));
        }
        wait.saw(&refused);
        wait.look_again(&format!("its move into {}", at.display()))
            .map_err(|expired| {
                format!(
                    "{expired} — something is holding the tree open. It is built and complete \
                     at {}; moving it by hand finishes the job.",
                    partial.display()
                )
            })?;
    }
}

/// How long the move is given. Generous, because the alternative is
/// throwing away the build that produced what is being moved.
const RENAME_CEILING: std::time::Duration = std::time::Duration::from_secs(120);

/// What each phase of the build cost.
///
/// **Printed, because an invisible cost is one nobody optimises.** A
/// tool a person waits ten minutes for says where the ten minutes
/// went, or the next person to wonder has to measure it from outside.
struct Clock {
    began: std::time::Instant,
    phases: Vec<(&'static str, std::time::Duration)>,
}

impl Clock {
    fn start() -> Self {
        Self {
            // waits(measured): the phase clock this struct prints, judged by nothing
            began: std::time::Instant::now(),
            phases: Vec::new(),
        }
    }

    fn mark(&mut self, phase: &'static str) {
        // waits(measured): the phase's end, for the same print
        let now = std::time::Instant::now();
        self.phases.push((phase, now - self.began));
        self.began = now;
    }

    fn say(&self) {
        let whole: std::time::Duration = self.phases.iter().map(|(_, took)| *took).sum();
        let each: Vec<String> = self
            .phases
            .iter()
            .map(|(phase, took)| format!("{phase} {:.0}s", took.as_secs_f64()))
            .collect();
        println!(
            "  built in {:.1} min — {}",
            whole.as_secs_f64() / 60.0,
            each.join(" | ")
        );
    }
}

fn fill(at: &Path, clock: &mut Clock) -> Result<(), String> {
    println!(
        "building the corpus at {} — {} commits, {} refs",
        at.display(),
        shape::COMMITS,
        shape::REFS
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
    // **The checkout has to be the same bytes everywhere.** Git for
    // Windows ships `core.autocrlf=true` in its system config, a fresh
    // `init` inherits it, and the corpus then checks out CRLF here and
    // LF on every other machine — a different working tree, a different
    // `git status` to read, and different bytes in the diff pane, from
    // object ids that are identical (fast-import writes blobs past the
    // filter, so only the checkout moves). Measured on this machine
    // before this line existed.
    git(at, &["config", "core.autocrlf", "false"])?;
    // The reference repository's paths run to 269 characters, and this
    // one's are no shorter; with the corpus's own prefix in front of
    // them that is past what the ANSI Windows API takes, so git is told
    // to use the Unicode one. Without it the checkout fails on the
    // deepest files and the corpus is a partial one that passes every
    // other check.
    git(at, &["config", "core.longpaths", "true"])?;
    // **The pack is deflated at the cheapest level.** fast-import
    // compresses every blob as it reads it, single-threaded, and the
    // stream is tens of gigabytes — at the default level that is most
    // of the build. The corpus already carries twice the reference
    // repository's pack, so trading more bytes for less time costs
    // nothing that is being measured: inflating is what the application
    // does, and its speed does not depend on the level.
    git(at, &["config", "pack.compression", "1"])?;
    // The checkout is a hundred thousand files, which on Windows is
    // where the wall clock goes rather than in the reading.
    git(at, &["config", "checkout.workers", "0"])?;
    // **Nothing may repack this corpus behind the measurement.** `git
    // fetch` runs `gc --auto` after itself, and against nine million
    // objects that is four minutes (measured — it was the whole of what
    // looked like a slow fetch). The application fetches at open and
    // every minute, so left on, maintenance would land in the middle of
    // a run and be read as the application being slow. It would also
    // rewrite the pack the record names.
    git(at, &["config", "gc.auto", "0"])?;
    git(at, &["config", "maintenance.auto", "false"])?;
    git(at, &["config", "fetch.writeCommitGraph", "false"])?;
    let newest = import(at, clock)?;
    // fast-import writes refs and nothing else: without this the work
    // tree is the empty one `init` left, and every tracked file reads as
    // deleted — which the application would show as an enormous
    // uncommitted change.
    git(at, &["reset", "--hard", "main"])?;
    clock.mark("checkout");
    // The chain the reference repository has. Startup reads it, so a
    // corpus without one is measuring a different walk.
    git(at, &["commit-graph", "write", "--reachable", "--split"])?;
    // Tens of thousands of loose refs are a file each — slack on every
    // one, and a walk that reads them all; packed they are one file
    // (ci/baseline/code-costs-windows-x64.md §メモリの形).
    git(at, &["pack-refs", "--all"])?;
    packs(at)?;
    clock.mark("indexes");
    spill(
        at,
        &git(at, &["ls-files"])?
            .lines()
            .map(String::from)
            .collect::<Vec<_>>(),
    )?;
    clock.mark("spill");
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
    // The copies' branches (`stand_copies`) are refs of this generator's
    // own making, not of the corpus's shape: counted out here, and still
    // in the token, which is what a run is compared under.
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

/// What both columns of the distance table are made of.
fn readings(at: &Path) -> Result<(), String> {
    println!("profile: refs (counts and navigation structure; no aggregate workload score)");
    tags(at)?;
    branch_tree(at)?;
    println!("profile: startup/status (tracked tree, index and ignored paths)");
    worktree(at)?;
    println!("profile: graph/scroll (lanes, chips, visible text and font coverage)");
    graph(at)?;
    window(at)?;
    println!("profile: diff (changed-file count and source-size distribution)");
    diffs(at)
}

/// What the timed diff costs — over the whole window, not at the tip.
///
/// **The harness opens the first changed file of the first row**
/// (`--selection first`, `perf::options`), so one file's size is the
/// operation-response number. Read at the tip alone it says nothing
/// about a repository whose tip moves: the reference repository's
/// newest row changes two files with a 56KB first file, and the same
/// question asked a week earlier answered seventy-five files and 25KB.
/// The window's distribution is the part that holds still, and it is
/// what a generated tip has to sit inside.
fn diffs(at: &Path) -> Result<(), String> {
    let raw = git(
        at,
        &[
            "log",
            "--max-count=2000",
            "--date-order",
            "HEAD",
            "--branches",
            "--remotes",
            "--tags",
            "--format=commit %H",
            "--raw",
            "--no-abbrev",
            "--no-renames",
        ],
    )?;
    let mut counts: Vec<u64> = Vec::new();
    let mut opened: Vec<&str> = Vec::new();
    let mut here = 0;
    let mut first = None;
    // The two newest rows with a file to open, as `perf --cases` takes
    // them: what a run that has to keep clicking while something else
    // runs — the slots measurement — is driven with.
    let mut cases: Vec<(String, String)> = Vec::new();
    let mut commit = "";
    for line in raw.lines() {
        if let Some(oid) = line.strip_prefix("commit ") {
            if here > 0 {
                counts.push(here);
                if let Some(oid) = first {
                    opened.push(oid);
                }
            }
            here = 0;
            first = None;
            commit = oid;
        } else if let Some(fields) = line.strip_prefix(':') {
            here += 1;
            // `:<srcmode> <dstmode> <srcsha> <dstsha> <status>\t<path>`,
            // and a deletion's destination is all zeroes — there is no
            // file to open for it.
            if first.is_none() {
                first = fields
                    .split_whitespace()
                    .nth(3)
                    .filter(|oid| !oid.bytes().all(|byte| byte == b'0'));
                if first.is_some()
                    && cases.len() < 2
                    && let Some((_, path)) = fields.split_once('\t')
                {
                    cases.push((commit.to_string(), path.to_string()));
                }
            }
        }
    }
    if here > 0 {
        counts.push(here);
        if let Some(oid) = first {
            opened.push(oid);
        }
    }
    println!("  perf cases (the two newest rows with a file to open, as --cases takes them):");
    for (name, (oid, path)) in ["newest", "second"].iter().zip(&cases) {
        println!("    {name}\t{oid}\t{path}\traw");
    }
    let mut bytes = sizes(at, &opened)?;
    counts.sort_unstable();
    bytes.sort_unstable();
    let at_percent = |of: &[u64], percent: usize| {
        of.get(of.len() * percent / 100)
            .copied()
            .unwrap_or_default()
    };
    println!(
        "  diffs {} rows | files/commit p50 {} p90 {} max {} | opened file p50 {}B p90 {}B max {}B",
        counts.len(),
        at_percent(&counts, 50),
        at_percent(&counts, 90),
        counts.last().copied().unwrap_or_default(),
        at_percent(&bytes, 50),
        at_percent(&bytes, 90),
        bytes.last().copied().unwrap_or_default(),
    );
    Ok(())
}

/// How large each of those blobs is, asked once rather than once each:
/// two thousand `cat-file` processes cost more than the reading is
/// worth, and `--batch-check` answers them all down one pipe.
fn sizes(at: &Path, oids: &[&str]) -> Result<Vec<u64>, String> {
    if oids.is_empty() {
        return Ok(Vec::new());
    }
    let mut child = Command::new("git")
        .current_dir(at)
        .args(["cat-file", "--batch-check=%(objectsize)"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to run git cat-file: {e}"))?;
    let asking = child.stdin.take().ok_or("git cat-file took no input")?;
    let mut asking = std::io::BufWriter::new(asking);
    for oid in oids {
        // **One `\n` a line, never the platform's ending.** git takes
        // the whole line as the name of an object, so a carriage return
        // makes every one of them `missing`.
        writeln!(asking, "{oid}").map_err(|e| format!("could not ask about {oid}: {e}"))?;
    }
    drop(asking);
    let out = child
        .wait_with_output()
        .map_err(|e| format!("git cat-file failed: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect())
}

/// What the tags are, and how many remotes there are to read them from.
///
/// **An annotated tag is a second object and a second advertisement.**
/// `ls-remote` gives it a line of its own and a peeled one, which is the
/// pairing `remote::parse_ls_remote_tags` exists to do and the size
/// `session::RemoteTagIndex` is built at; a repository whose tags are
/// lightweight reaches neither, and one with no remote configured never
/// asks (`session::auto_fetch::known_to_have_no_remote`).
fn tags(at: &Path) -> Result<(), String> {
    let kinds = git(at, &["for-each-ref", "--format=%(objecttype)", "refs/tags"])?;
    let total = kinds.lines().filter(|kind| !kind.is_empty()).count();
    let annotated = kinds.lines().filter(|kind| *kind == "tag").count();
    let remotes = git(at, &["remote"])?
        .lines()
        .filter(|name| !name.is_empty())
        .count();
    let share = if total == 0 {
        0.0
    } else {
        annotated as f64 * 100.0 / total as f64
    };
    println!("  tags {total} | {annotated} annotated ({share:.1}%) | {remotes} remotes configured");
    Ok(())
}

/// How the object database is reached, which is not the same axis as
/// how large it is.
///
/// **Every git process maps this before it resolves anything**, and an
/// opening spawns about fifteen of them. A repository that has been
/// worked in has several packs and a multi-pack-index over them; one
/// that fast-import wrote has exactly one pack and no index over it.
///
/// **The split is the build's own**: one pack per blob pass and one for
/// the commits and trees ([`BLOB_IMPORTS`]), with `--max-pack-size` as
/// the ceiling on any one of them. Asking `repack -a -d` for it
/// afterwards costs a second pass over every object in the database —
/// measured at over forty minutes for this one, and it produced a
/// single pack anyway.
fn packs(at: &Path) -> Result<(), String> {
    git(at, &["multi-pack-index", "write"])?;
    Ok(())
}

/// The build output a working repository accumulates, which is ignored
/// and is not therefore free.
///
/// **`-uall` walks it.** `status::read` asks for every untracked path,
/// so git stats each of these and matches it against the ignore rules
/// before deciding it has nothing to report.
///
/// **Deliberately more than the reference repository has.** That clone
/// carries 278 ignored paths, because nothing has been built in it; a
/// checkout somebody works in carries the output of every build, and
/// this is the axis the application pays for on a machine in use. The
/// corpus is heavier here on purpose; how much of its status this
/// accounts for has not been measured on its own.
fn spill(at: &Path, tracked: &[String]) -> Result<(), String> {
    if tracked.is_empty() {
        return Ok(());
    }
    let stride = tracked.len() as u64 / shape::IGNORED_FILES.max(1) + 1;
    for n in 0..shape::IGNORED_FILES {
        let beside = &tracked[((n * stride) as usize) % tracked.len()];
        let Some((dir, _)) = beside.rsplit_once('/') else {
            continue;
        };
        let file = at.join(dir).join(format!("{}{n}.class", shape::word(n)));
        std::fs::write(&file, format!("{n}\n"))
            .map_err(|e| format!("could not write {}: {e}", file.display()))?;
    }
    Ok(())
}

/// What the working tree costs to read, which is most of what startup
/// is.
///
/// `status::read` runs `git status --porcelain=v2 -z --branch -uall` and
/// pays one `lstat` per tracked file; the index carries one entry each.
/// The reference repository is 106,581 files and a 17MB index, and a
/// corpus that is a fraction of that is measuring a fraction of the
/// startup it claims to.
///
/// **The time is a reading of the walk, not of a clone's settings.** On
/// the reference repository `core.fsmonitor` — which is how that clone is
/// configured — makes the status slower, not faster
/// (ci/baseline/code-costs-windows-x64.md §コーパス生成), so a corpus that
/// copied the setting would be measuring a daemon's health on the day.
fn worktree(at: &Path) -> Result<(), String> {
    let tracked = git(at, &["ls-files"])?.lines().count();
    let index = std::fs::metadata(at.join(".git").join("index"))
        .map(|meta| meta.len())
        .unwrap_or_default();
    // waits(measured): the status's time, one of the readings the corpus is described by
    let began = std::time::Instant::now();
    // **`--no-optional-locks`, because the application never runs a
    // status without it** (`process::executor::FIXED_ARGS`), and
    // because this same reading is taken of repositories that are only
    // being read — a status without it rewrites their index. It is not
    // a timing device: warm, with and without read the same
    // (ci/baseline/code-costs-windows-x64.md §コーパス生成).
    git(
        at,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "-uall",
        ],
    )?;
    let status = began.elapsed();
    let objects = git(at, &["count-objects", "-vH"])?;
    let read = |key: &str| {
        objects
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .unwrap_or("?")
            .trim()
            .to_string()
    };
    let dirs = git(at, &["ls-tree", "-r", "-d", "--name-only", "HEAD"])?
        .lines()
        .count();
    // The size histogram, because "a hundred thousand files" is not one
    // reading: a tree of that many stubs and a tree of that many
    // sources cost different amounts to check out, to status and to
    // open, and the record's claim is about the histogram rather than
    // the count.
    let mut bytes: Vec<u64> = git(at, &["ls-tree", "-r", "--format=%(objectsize)", "HEAD"])?
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect();
    bytes.sort_unstable();
    let at_percent = |percent: usize| {
        bytes
            .get(bytes.len() * percent / 100)
            .copied()
            .unwrap_or_default()
    };
    let total: u64 = bytes.iter().sum();
    let packs = std::fs::read_dir(at.join(".git").join("objects").join("pack"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "pack"))
                .count()
        })
        .unwrap_or_default();
    let midx = at
        .join(".git")
        .join("objects")
        .join("pack")
        .join("multi-pack-index")
        .exists();
    println!(
        "  packs {packs}{} — every git process maps these first",
        if midx { " + a multi-pack-index" } else { "" }
    );
    println!(
        "  worktree {tracked} files in {dirs} dirs | index {}MB | status {:.2}s | {} objects in {}",
        index / 1_000_000,
        status.as_secs_f64(),
        read("in-pack:"),
        read("size-pack:")
    );
    println!(
        "  file bytes p25 {} p50 {} p75 {} p90 {} max {} | {}MB of tree",
        at_percent(25),
        at_percent(50),
        at_percent(75),
        at_percent(90),
        bytes.last().copied().unwrap_or_default(),
        total / 1_000_000,
    );
    Ok(())
}

/// How many rows the sidebar's remotes section would build.
///
/// `nav::tree::build_tree` makes one `NavItem` per distinct directory
/// prefix — two heap strings each — and rebuilds all of them on every
/// arrange, which is every refs snapshot. It does not compact
/// single-child chains, so the count here is the count of rows.
fn branch_tree(at: &Path) -> Result<(), String> {
    let names = git(
        at,
        &[
            "for-each-ref",
            "--format=%(refname:lstrip=2)",
            "refs/remotes",
        ],
    )?;
    let mut folders = std::collections::BTreeSet::new();
    let mut leaves = 0;
    let mut deepest = 0;
    for name in names.lines().filter(|name| !name.is_empty()) {
        leaves += 1;
        let mut at = 0;
        let mut depth = 0;
        while let Some(slash) = name[at..].find('/') {
            at += slash;
            folders.insert(name[..at].to_string());
            at += 1;
            depth += 1;
        }
        deepest = deepest.max(depth);
    }
    println!(
        "  remotes {leaves} leaves | {} folder rows | {deepest} deep",
        folders.len()
    );
    Ok(())
}

/// The shape of the graph the window draws: how many refs land on its
/// rows, and how wide it gets.
///
/// **A lane is open from a row with a not-yet-emitted parent until that
/// parent is emitted**, which is the walk `GraphBuilder` does. The width
/// is what the row canvas is sized by and what the renderer has to cope
/// with; a constant width demands nothing of it.
fn graph(at: &Path) -> Result<(), String> {
    let rows = git(
        at,
        &[
            "log",
            "--max-count=2000",
            "--date-order",
            "HEAD",
            "--branches",
            "--remotes",
            "--tags",
            "--format=%H\x1f%P\x1f%D",
        ],
    )?;
    let mut open: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut widths = Vec::new();
    let (mut chips, mut carrying, mut busiest, mut merges) = (0, 0, 0, 0);
    for row in rows.lines().filter(|row| !row.is_empty()) {
        let mut field = row.split('\x1f');
        let oid = field.next().unwrap_or_default();
        let parents = field.next().unwrap_or_default();
        let refs = field.next().unwrap_or_default();
        open.remove(oid);
        // A merge is a second parent, and a second edge into the row —
        // the reference repository's window has none, because its
        // newest rows are unmerged review branches.
        if parents.split_whitespace().nth(1).is_some() {
            merges += 1;
        }
        for parent in parents.split_whitespace() {
            open.insert(parent.to_string());
        }
        widths.push(open.len());
        let here = refs
            .split(',')
            .filter(|name| !name.trim().is_empty())
            .count();
        if here > 0 {
            carrying += 1;
            chips += here;
            busiest = busiest.max(here);
        }
    }
    widths.sort_unstable();
    let at_percent = |p: usize| widths.get(widths.len() * p / 100).copied().unwrap_or(0);
    println!(
        "  graph lanes p25 {} / p50 {} / p75 {} / max {} | {chips} chips on {carrying} rows, \
         busiest {busiest} | {merges} merges",
        at_percent(25),
        at_percent(50),
        at_percent(75),
        widths.last().copied().unwrap_or(0)
    );
    Ok(())
}

/// How many rows the graph's window would draw, and what they carry.
///
/// **The window is the measurement.** Everything below rides on all
/// 2,000 rows at once — the body and the credits `parse::log` asks for,
/// the fallback font the first non-ASCII glyph loads, the second
/// identity the details card shows — and every one of them is a
/// dimension the corpus once had none of. Printed so a person can see
/// what the corpus carries without opening the application, and so a
/// generator that quietly stopped carrying one is visible here.
fn window(at: &Path) -> Result<(), String> {
    const ROWS: &str = "--max-count=2000";
    let log = git(
        at,
        &[
            "log",
            ROWS,
            "--date-order",
            "HEAD",
            "--branches",
            "--remotes",
            "--tags",
            "--format=%aN\x1f%cN\x1f%s\x1f%(trailers:key=Co-authored-by,valueonly)\x1f%b\x1e",
        ],
    )?;
    let mut rows = 0;
    let mut wide = 0;
    let mut credited = 0;
    let mut applied = 0;
    let mut body_bytes = 0;
    // Distinct names, because a name is a string the rows hold once
    // each and the details card looks up — a window written by a
    // hundred people and one written by ten thousand are different
    // amounts of text however many rows they have.
    let mut authors = std::collections::BTreeSet::new();
    for row in log.split('\x1e').filter(|row| !row.trim().is_empty()) {
        let mut field = row.trim_start_matches('\n').split('\x1f');
        let (author, committer) = (field.next().unwrap_or_default(), field.next());
        let subject = field.next().unwrap_or_default();
        let credits = field.next().unwrap_or_default();
        let body = field.next().unwrap_or_default();
        rows += 1;
        authors.insert(author);
        if !author.is_ascii() || !subject.is_ascii() {
            wide += 1;
        }
        if !credits.trim().is_empty() {
            credited += 1;
        }
        if committer.is_some_and(|by| by != author) {
            applied += 1;
        }
        body_bytes += body.trim().len();
    }
    println!(
        "  window {rows} rows | {} authors | body {body_bytes}B | {credited} credited | \
         {applied} applied by another | {wide} needing a fallback font",
        authors.len()
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
///
/// **Three counts, not the whole shape.** A generator whose lane
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

/// How much of the stream is held before it is handed to git. The
/// stream is written a line at a time and is tens of gigabytes, so
/// without a buffer this is one write syscall per line.
const PIPE_BUFFER: usize = 4 * 1024 * 1024;

/// How many `fast-import` processes the blobs go through at once.
///
/// **The blobs are two thirds of the import and the only part of it
/// that parallelises.** `fast-import` reads its stream on one thread,
/// and a stream that spells every body out in its commit puts tens of
/// gigabytes through that one thread — the bodies, not the number of
/// commits, are what the wait is (times:
/// ci/baseline/code-costs-windows-x64.md §コーパス生成; the generator
/// writing the whole stream is a rounding error beside it,
/// `stream::tests::time_the_generator_alone`). The bodies hash to the
/// same ids whichever process reads them, so they go through this many
/// at once under marks, and the commit stream names the marks
/// (`stream::Bodies`).
///
/// **Four, because each writes a pack.** The reference repository
/// carries five packs and a multi-pack-index, and every git process the
/// application spawns maps them before it resolves anything; four blob
/// packs and the one the commits and trees make is that shape. More
/// would shave seconds and add a pack each.
///
/// **`--depth=0` is not a shortcut.** It skips the delta attempt on
/// every blob, which is about a quarter of a single-process import
/// (ci/baseline/code-costs-windows-x64.md §コーパス生成), but also the
/// deltas between versions of the big directories' trees, and the pack
/// comes out 10.7GiB against 6.5.
const BLOB_IMPORTS: usize = 4;

/// The import: the blobs through [`BLOB_IMPORTS`] processes at once,
/// then the commits through one that reads their marks.
fn import(at: &Path, clock: &mut Clock) -> Result<stream::Newest, String> {
    let tree = tree::build();
    let placements =
        stream::placements(&tree).map_err(|e| format!("could not lay out the history: {e}"))?;
    // Under `.git`, so the checkout does not carry them; each is a mark
    // and an id per placement, read once by the commit pass and removed.
    let marks: Vec<String> = (0..BLOB_IMPORTS)
        .map(|shard| format!(".git/pgg-marks-{shard}"))
        .collect();
    std::thread::scope(|scope| {
        let shards: Vec<_> = marks
            .iter()
            .enumerate()
            .map(|(shard, file)| {
                let (tree, placements) = (&tree, &placements);
                scope.spawn(move || {
                    let export = format!("--export-marks={file}");
                    fast_import(at, &[export.as_str()], |out| {
                        stream::blobs(out, tree, placements, shard, BLOB_IMPORTS)
                    })
                })
            })
            .collect();
        shards
            .into_iter()
            .map(|shard| {
                shard
                    .join()
                    .unwrap_or_else(|_| Err("a blob import thread panicked".to_string()))
            })
            .collect::<Result<Vec<()>, String>>()
    })?;
    clock.mark("blobs");
    let imports: Vec<String> = marks
        .iter()
        .map(|file| format!("--import-marks={file}"))
        .collect();
    let args: Vec<&str> = imports.iter().map(String::as_str).collect();
    let newest = fast_import(at, &args, |out| {
        stream::write(out, &tree, &mut stream::Marked::default())
    })?;
    for file in &marks {
        std::fs::remove_file(at.join(file)).map_err(|e| format!("could not remove {file}: {e}"))?;
    }
    clock.mark("commits");
    Ok(newest)
}

/// Runs one `fast-import` with a stream written straight into it, and
/// answers what the writer answered.
///
/// **The write is allowed to fail.** A git that rejects the stream exits
/// while the rest of it is still being written, and the broken pipe that
/// follows says nothing about why — so it is kept and only reported if
/// git itself turns out to have had nothing to say.
///
/// **Its standard error is drained by a thread**, because both pipes are
/// live at once for as long as the stream takes: a git that filled its
/// error pipe would stop reading the stream, and a writer that never
/// stops writing would never read the error. Each waiting for the other
/// is a build that hangs rather than one that says why.
fn fast_import<T>(
    at: &Path,
    args: &[&str],
    feed: impl FnOnce(&mut dyn Write) -> std::io::Result<T>,
) -> Result<T, String> {
    let mut child = Command::new("git")
        .current_dir(at)
        .args([
            "fast-import",
            "--quiet",
            "--force",
            &format!("--max-pack-size={}", shape::MAX_PACK_SIZE),
        ])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn git fast-import: {e}"))?;
    let drain = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut said = String::new();
            match std::io::Read::read_to_string(&mut pipe, &mut said) {
                // A pipe that broke mid-message still carries the part
                // of it that arrived, which is the part worth printing.
                Ok(_) | Err(_) => said,
            }
        })
    });
    let wrote = match child.stdin.take() {
        Some(pipe) => {
            let mut buffered = std::io::BufWriter::with_capacity(PIPE_BUFFER, pipe);
            feed(&mut buffered).and_then(|answer| {
                buffered.flush()?;
                Ok(answer)
            })
        }
        None => Err(std::io::Error::other("fast-import took no standard input")),
    };
    // The pipe closes with the writer above, which is what tells
    // fast-import the stream is over; waiting before that would wait
    // forever.
    let status = child.wait().map_err(|e| e.to_string())?;
    let said = match drain.map(std::thread::JoinHandle::join) {
        Some(Ok(said)) => said.trim().to_string(),
        Some(Err(_)) | None => String::new(),
    };
    match wrote {
        Ok(answer) if status.success() => Ok(answer),
        wrote => Err(format!(
            "git fast-import failed: {}",
            if said.is_empty() {
                wrote.err().map(|e| e.to_string()).unwrap_or_default()
            } else {
                said
            }
        )),
    }
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
    use super::{beside, clearable, given_path, partial_owner, partial_path};

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

    /// The name a build writes and the name the sweep reads have to be
    /// one name, or an abandoned build is never found — and a directory
    /// that merely sits beside the corpus must never be read as one.
    #[test]
    fn a_partial_build_is_found_under_the_name_it_was_given() {
        let at = std::path::Path::new("/home/x/platitude-gg/.pgg-perf-corpus");
        let partial = partial_path(at, 4242);
        assert_eq!(partial.parent(), at.parent(), "beside its destination");
        assert_eq!(
            partial.file_name().and_then(|name| name.to_str()),
            Some(".pgg-perf-corpus.partial-4242")
        );
        assert_eq!(partial_owner(at, &partial), Some(4242));
        for other in [
            "/home/x/platitude-gg/.pgg-perf-corpus",
            "/home/x/platitude-gg/.pgg-perf-corpus.partial-x",
            "/home/x/platitude-gg/.pgg-perf-corpus-notes",
            "/home/x/platitude-gg/target",
        ] {
            assert_eq!(
                partial_owner(at, std::path::Path::new(other)),
                None,
                "{other}"
            );
        }
    }

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
