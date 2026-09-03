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

mod remotes;
mod shape;
mod stream;
mod tree;

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
    // Named for this process, so two seats building at once are two
    // builds rather than one destroyed twice.
    let partial = at.with_extension(format!("partial-{}", std::process::id()));
    if partial.exists() {
        std::fs::remove_dir_all(&partial)
            .map_err(|e| format!("could not clear {}: {e}", partial.display()))?;
    }
    fill(&partial)?;
    settle(&partial, at)?;
    // **After the move, because a remote records where it was told to
    // look.** Configured before it, every URL would name the scratch
    // directory the move then takes away — and the proof would pass,
    // because the proof would be reading the scratch copy too. The
    // corpus that gets measured is this one.
    let mut clock = Clock::start();
    remotes::configure(at)?;
    clock.mark("remotes");
    clock.say();
    Ok(())
}

/// Moves the finished corpus into place, waiting out whatever is still
/// holding it.
///
/// **A rename can be refused for a while.** The build has just written
/// a hundred and eighty thousand files, and on Windows the scanner or
/// the indexer walking them keeps handles open into the tree for
/// seconds afterwards — long enough that the rename fails with an
/// access error against a destination that does not exist (measured,
/// after a ten-minute build). Waiting is the whole fix.
fn settle(partial: &Path, at: &Path) -> Result<(), String> {
    let deadline = std::time::Instant::now() + RENAME_CEILING;
    loop {
        let Err(refused) = std::fs::rename(partial, at) else {
            return Ok(());
        };
        if std::time::Instant::now() >= deadline {
            return Err(format!(
                "could not move the corpus into {} after {}s: {refused} — something is holding \
                 the tree open. It is built and complete at {}; moving it by hand finishes the \
                 job.",
                at.display(),
                RENAME_CEILING.as_secs(),
                partial.display()
            ));
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
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
            began: std::time::Instant::now(),
            phases: Vec::new(),
        }
    }

    fn mark(&mut self, phase: &'static str) {
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

fn fill(at: &Path) -> Result<(), String> {
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
    let mut clock = Clock::start();
    let newest = import(at)?;
    clock.mark("import");
    // fast-import writes refs and nothing else: without this the work
    // tree is the empty one `init` left, and every tracked file reads as
    // deleted — which the application would show as an enormous
    // uncommitted change.
    git(at, &["reset", "--hard", "main"])?;
    clock.mark("checkout");
    // The chain the reference repository has. Startup reads it, so a
    // corpus without one is measuring a different walk.
    git(at, &["commit-graph", "write", "--reachable", "--split"])?;
    // 50,000 loose refs cost 80MB of slack and slow every walk that
    // reads them; packed they are 6MB (measured).
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
    clock.say();
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
    worktree(at)?;
    branch_tree(at)?;
    graph(at)?;
    window(at)?;
    println!(
        "  measure it with: PG_ALLOW_GUI=1 cargo xtask perf --repo {} --runs 5",
        at.display()
    );
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
/// **The split is fast-import's own** (`--max-pack-size`), taken while
/// it is already writing. Asking `repack -a -d` for it afterwards costs
/// a second pass over every object in the database — measured at over
/// forty minutes for this one, and it produced a single pack anyway.
fn packs(at: &Path) -> Result<(), String> {
    git(at, &["multi-pack-index", "write"])?;
    Ok(())
}

/// The build output a working repository accumulates, which is ignored
/// and is not therefore free.
///
/// **`-uall` walks it.** `status::read` asks for every untracked path,
/// so git stats each of these and matches it against the ignore rules
/// before deciding it has nothing to report. The reference repository
/// carries about 78,000 of them and that is a large part of why its
/// status takes a second where a corpus without them takes half of one.
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
/// The reference repository is 106,581 files, a 19MB index and a
/// one-second status, and a corpus that is a fraction of that is
/// measuring a fraction of the startup it claims to.
fn worktree(at: &Path) -> Result<(), String> {
    let tracked = git(at, &["ls-files"])?.lines().count();
    let index = std::fs::metadata(at.join(".git").join("index"))
        .map(|meta| meta.len())
        .unwrap_or_default();
    let began = std::time::Instant::now();
    git(at, &["status", "--porcelain=v2", "-z", "--branch", "-uall"])?;
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
    let (mut chips, mut carrying, mut busiest) = (0, 0, 0);
    for row in rows.lines().filter(|row| !row.is_empty()) {
        let mut field = row.split('\x1f');
        let oid = field.next().unwrap_or_default();
        let parents = field.next().unwrap_or_default();
        let refs = field.next().unwrap_or_default();
        open.remove(oid);
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
         busiest {busiest}",
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
    for row in log.split('\x1e').filter(|row| !row.trim().is_empty()) {
        let mut field = row.trim_start_matches('\n').split('\x1f');
        let (author, committer) = (field.next().unwrap_or_default(), field.next());
        let subject = field.next().unwrap_or_default();
        let credits = field.next().unwrap_or_default();
        let body = field.next().unwrap_or_default();
        rows += 1;
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
        "  window {rows} rows | body {body_bytes}B | {credited} credited | {applied} applied by \
         another | {wide} needing a fallback font"
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
    let wanted_refs = shape::REFS as usize;
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

/// How much of the stream is held before it is handed to git. The
/// stream is written a line at a time and is tens of gigabytes, so
/// without a buffer this is one write syscall per line.
const PIPE_BUFFER: usize = 4 * 1024 * 1024;

/// Runs `fast-import` with the stream written straight into it.
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
fn import(at: &Path) -> Result<stream::Newest, String> {
    let mut child = Command::new("git")
        .current_dir(at)
        .args([
            "fast-import",
            "--quiet",
            "--force",
            &format!("--max-pack-size={}", shape::MAX_PACK_SIZE),
        ])
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
            stream::write(&mut buffered).and_then(|newest| {
                std::io::Write::flush(&mut buffered)?;
                Ok(newest)
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
        Ok(newest) if status.success() => Ok(newest),
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
