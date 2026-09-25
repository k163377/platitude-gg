//! Building the corpus: the import, into a directory of its own, and the
//! move that puts it where every seat reads it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::{git, remotes, shape, stream, tree};

/// Builds it somewhere else and moves it into place.
///
/// **What is at `at` is either finished or absent.** `git init` makes
/// `.git` in milliseconds and the import that follows takes a hundred
/// seconds; a build interrupted anywhere in between would otherwise
/// leave a directory that every later run reads as "already built", and
/// the first thing to notice would be a measurement of half a corpus.
pub(super) fn build(at: &Path) -> Result<(), String> {
    sweep(at)?;
    // Named for this process, so two seats building at once are two
    // builds, each in a tree of its own.
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
    // The hash algorithm is pinned: `init` would otherwise take it
    // from `init.defaultObjectFormat`, and the same stream under
    // sha256 produces different object ids — so a corpus built on such
    // a machine could never match a recorded token.
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
    // where the wall clock goes.
    git(at, &["config", "checkout.workers", "0"])?;
    // **This corpus holds still under a measurement.** `git
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

/// How the object database is reached, an axis of its own beside its
/// size.
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
/// and still paid for.
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

/// How much of the stream is held before it is handed to git. The
/// stream is written a line at a time and is tens of gigabytes, so
/// without a buffer this is one write syscall per line.
const PIPE_BUFFER: usize = 4 * 1024 * 1024;

/// How many `fast-import` processes the blobs go through at once.
///
/// **The blobs are two thirds of the import and the only part of it
/// that parallelises.** `fast-import` reads its stream on one thread,
/// and a stream that spells every body out in its commit puts tens of
/// gigabytes through that one thread — the bodies are what the wait
/// is (times:
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
/// **`--depth=0` costs pack size.** It skips the delta attempt on
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
/// is a build that hangs.
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

#[cfg(test)]
mod tests {
    use super::{partial_owner, partial_path};

    /// The name a build writes and the name the sweep reads have to be
    /// one name, or an abandoned build is never found — and only that
    /// name is ever read as a partial build.
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
}
