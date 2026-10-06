//! Building the corpus: the import, into a directory of its own, and the
//! move that puts it where every seat reads it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::{git, remotes, shape, stream, tree};

/// Builds it beside `at` and moves it into place, so `at` is finished or
/// absent: a build interrupted in place would leave a `.git` every later
/// run reads as "already built".
pub(super) fn build(at: &Path) -> Result<(), String> {
    sweep(at)?;
    // Named for this process, so two seats building at once do not share
    // a tree.
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
    // After the move: a remote records where it was told to look, so
    // before it every URL would name the partial directory (and the proof
    // would pass, reading that too).
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

/// Clears the partial builds nobody is behind any more: each is named for
/// its process, so the next build (another process) would otherwise leave
/// a dead one's gigabytes beside the corpus. One whose process still runs
/// is another seat building, and is left alone.
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

/// Moves the finished corpus into place; `false` when another build got
/// there first, whose corpus is then the one to report and this tree is
/// thrown away.
///
/// On Windows the rename is refused for a while — the scanner or indexer
/// walking the fresh files holds handles into the tree — so it is
/// retried. A `.git` already at the destination is another build that
/// finished first, not a held handle; anything else there is named at
/// once.
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
            // Not this run's failure: the next build sweeps it (`sweep`).
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

/// How long the move is given; generous, since running out fails a
/// finished build.
const RENAME_CEILING: std::time::Duration = std::time::Duration::from_secs(120);

/// What each phase of the build cost, printed so a long wait says where
/// it went.
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
    // Pinned: an `init.defaultObjectFormat=sha256` machine would build
    // other object ids and never match a recorded token.
    git(
        at,
        &[
            "init",
            "--quiet",
            "--initial-branch=main",
            "--object-format=sha1",
        ],
    )?;
    // A machine with no global identity must still be able to build it.
    git(at, &["config", "user.name", "Corpus"])?;
    git(at, &["config", "user.email", "corpus@example.invalid"])?;
    // The checkout has to be the same bytes everywhere: Git for Windows'
    // system `core.autocrlf=true` would check out CRLF here and LF
    // elsewhere from identical object ids.
    git(at, &["config", "core.autocrlf", "false"])?;
    // With the corpus's prefix the deepest paths pass the ANSI Windows
    // limit; without this the checkout fails on them and leaves a partial
    // corpus that passes every other check.
    git(at, &["config", "core.longpaths", "true"])?;
    // fast-import deflates every blob on one thread, which at the default
    // level is most of the build; the application's inflating does not
    // depend on the level.
    git(at, &["config", "pack.compression", "1"])?;
    // The checkout is a hundred thousand files, which on Windows is
    // where the wall clock goes.
    git(at, &["config", "checkout.workers", "0"])?;
    // Holds still under a measurement: the application's fetches would
    // run `gc --auto` mid-run (minutes at this size), read as the
    // application being slow, and rewrite the pack the record names.
    git(at, &["config", "gc.auto", "0"])?;
    git(at, &["config", "maintenance.auto", "false"])?;
    git(at, &["config", "fetch.writeCommitGraph", "false"])?;
    let newest = import(at, clock)?;
    // fast-import writes no working tree: without this every tracked file
    // reads as deleted.
    git(at, &["reset", "--hard", "main"])?;
    clock.mark("checkout");
    // The chain the reference repository has. Startup reads it, so a
    // corpus without one is measuring a different walk.
    git(at, &["commit-graph", "write", "--reachable", "--split"])?;
    // Loose refs are a file each (ci/baseline/code-costs-windows-x64.md
    // §メモリの形).
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

/// A multi-pack-index over the build's packs, as a worked-in repository
/// has: every git process maps them before it resolves anything.
///
/// The split is the build's own — one pack per blob pass and one for the
/// commits and trees ([`BLOB_IMPORTS`]). `repack -a -d` would be a second
/// pass over every object, and it produces a single pack.
fn packs(at: &Path) -> Result<(), String> {
    git(at, &["multi-pack-index", "write"])?;
    Ok(())
}

/// The ignored build output a working repository accumulates, still paid
/// for: `-uall` (`status::load`) matches each path against the ignore
/// rules. More than the reference clone has on purpose — nothing was built
/// in it. Its share of the status time is unmeasured.
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

/// The stream is written a line at a time; unbuffered, that is a syscall
/// per line.
const PIPE_BUFFER: usize = 4 * 1024 * 1024;

/// How many `fast-import` processes the blobs go through at once. The
/// bodies dominate the import (ci/baseline/code-costs-windows-x64.md
/// §コーパス生成) and are the only part that parallelises: they hash to
/// the same ids in any process, so they go in under marks the commit
/// stream names (`stream::Bodies`).
///
/// Four, because each writes a pack: four plus the commits' and trees'
/// is the reference repository's five. More would add a pack each.
///
/// Not `--depth=0`: it also skips the deltas between the trees' versions,
/// and the pack grows by two thirds (same §).
const BLOB_IMPORTS: usize = 4;

/// The import: the blobs through [`BLOB_IMPORTS`] processes at once,
/// then the commits through one that reads their marks.
fn import(at: &Path, clock: &mut Clock) -> Result<stream::Newest, String> {
    let tree = tree::build();
    let placements =
        stream::placements(&tree).map_err(|e| format!("could not lay out the history: {e}"))?;
    // Under `.git`, so the checkout does not carry them; removed after
    // the commit pass reads them.
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

/// Runs one `fast-import` fed by `feed`, and answers what the writer
/// answered.
///
/// A write error is reported only when git said nothing: a git that
/// rejects the stream exits mid-write, and the broken pipe says nothing
/// about why. Standard error is drained by a thread, or a git with a full
/// error pipe stops reading the stream and both sides hang.
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
                // A pipe that broke mid-message still keeps what arrived.
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
    // Dropping the writer above closed the pipe, which ends the stream;
    // waiting before that would hang.
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
