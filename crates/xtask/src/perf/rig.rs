//! The measurement rig (`seats::RIG`): the commit that was asked for,
//! built in a tree nobody edits against the Qt that commit pins (or the
//! one `--qt` names), and the exe shelved by commit, feature set and the
//! rest of what it is built from (`perf::identity`) so it is never built
//! twice.
//!
//! The rig is claimed while it is switched and built, the way a seat is
//! (`seats::take_seat`), so two invocations cannot check two commits out
//! into one tree — but by the measuring process, so a second `perf --at`
//! from the same session is refused too. The claim is released once the
//! exe is on the shelf: the measurement runs off the copy.
//!
//! A dirty rig refuses (`switch`); the write hook refuses the edit in the
//! first place (hook/seat.rs).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::qt::Qt;
use crate::seats::{
    Held, Identity, Standing, primary_checkout, rig_path, same_tree, slashed, take_seat,
    unlock_seat,
};
use crate::subprocess::git_query;

use super::Options;
use super::identity;

/// How many builds the shelf keeps: an A/B needs two, a record's tables
/// three (the harness, the harness with memprobe, the shipped set), and
/// the rest is room for the last few commits measured.
const KEEP: usize = 8;

/// The directory under the rig's target/ the builds are shelved in. Under
/// target/ so that git never sees it: a rig whose `status` says anything
/// is a tree somebody edited.
const SHELF: &str = "shelf";

/// A build the measurement can run: where the exe is, which commit it is
/// of, the tree the run is started from, and the Qt it was built against
/// — the one its run loads.
pub(super) struct Built {
    pub(super) exe: PathBuf,
    pub(super) commit: String,
    pub(super) tree: PathBuf,
    pub(super) qt: Qt,
    /// Why this Qt: a commit's pin, or `--qt`.
    pub(super) qt_source: String,
    pub(super) identity: identity::Identity,
}

impl Built {
    /// The commit as the report and the shelf name it.
    pub(super) fn short(&self) -> &str {
        short(&self.commit)
    }
}

/// The build of `rev`, off the rig's shelf or freshly made there.
///
/// `caller` is where the rev is resolved, so a seat can name its own
/// branch. `opts.build` false takes the shelf or nothing.
///
/// Called under the measurement's own announcement (`perf::run`), so no
/// measurement holds the machine while this switches, reaps and builds.
pub(super) fn build_at(caller: &Path, rev: &str, opts: &Options) -> Result<Built, String> {
    let here = slashed(caller);
    let commit = git_query(
        &here,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &format!("{rev}^{{commit}}"),
        ],
    )
    .ok_or_else(|| format!("--at {rev}: nothing here names a commit by that"))?;
    let (primary, trees) = primary_checkout(&here)?;
    let rig = rig_path(&primary);
    let pinned =
        crate::qt::pinned_at(&|path| git_query(&here, &["show", &format!("{commit}:{path}")]))
            .ok_or_else(|| {
                format!(
                    "{} pins no Qt (in {}, or on the workflow's QT_VERSION line before that \
                     file): name one with --qt",
                    short(&commit),
                    crate::qt::PIN
                )
            })?;
    let (qt, qt_source) =
        super::qt_for(opts, &format!("{} pins {pinned}", short(&commit)), &pinned)?;
    let set = opts.feature_slug();
    let configs = identity::configs(Path::new(&rig), Some((&here, &commit)));
    // The rig's own configuration sets what it sets: none of what this
    // tree's put into this process reaches its build.
    let unset = identity::injected(&identity::configs(caller, None));
    let identity = identity::of(&commit, &opts.features(), &qt, &configs, &unset);
    let shelved = shelf(Path::new(&rig), &commit, &set, &identity.hash());
    let exe = shelved.join(crate::app_build::exe_name());
    if identity.complete && exe.is_file() {
        println!(
            "rig: {} ({set}, Qt {}) is on the shelf — nothing to build",
            short(&commit),
            qt.version
        );
        return Ok(Built {
            exe,
            commit,
            tree: PathBuf::from(rig),
            qt,
            qt_source,
            identity,
        });
    }
    if !identity.complete {
        println!(
            "rig: the compiler or rustc did not answer, so no shelved build can be matched — \
             building"
        );
    }
    if !opts.build {
        return Err(format!(
            "the rig has no build of {} with {set} on its shelf, and --no-build asked for none \
             — drop --no-build",
            short(&commit)
        ));
    }
    let exists = trees.iter().any(|tree| same_tree(&tree.path, &rig));
    let _claim = Claim::take(&primary, &rig, &commit, exists)?;
    // A run of the rig's exe left standing holds files the copy and the
    // build must replace, and a seat's `kill` never reaps this tree. Safe
    // to reap: under the announcement no measurement runs off the shelf.
    for (pid, stale) in crate::gui::reap_under(Path::new(&rig))? {
        println!("rig: reaped a stale run first: {pid} ({stale})");
    }
    switch(&rig, &commit)?;
    println!(
        "rig: building {} ({set}) in {rig} against {}",
        short(&commit),
        qt.describe()
    );
    // The rig follows one commit's Qt with another's in one target
    // directory: its qmake is named, not only put on PATH (`crate::qt`).
    let fresh = super::build_in(Path::new(&rig), &qt, true, &unset, true, opts)?;
    shelve(&fresh, &exe)?;
    std::fs::write(shelved.join("build.txt"), &identity.text).map_err(|e| {
        format!(
            "could not write {}: {e}",
            shelved.join("build.txt").display()
        )
    })?;
    let shelf = Path::new(&rig).join("target").join(SHELF);
    for gone in stale_builds(&shelf, KEEP) {
        if let Err(error) = std::fs::remove_dir_all(&gone) {
            println!(
                "rig: could not take {} off the shelf: {error}",
                gone.display()
            );
        }
    }
    Ok(Built {
        exe,
        commit,
        tree: PathBuf::from(rig),
        qt,
        qt_source,
        identity,
    })
}

/// The rig's claim for as long as it is being switched and built. Dropped
/// on every exit, so a build that failed does not leave the rig claimed.
struct Claim {
    primary: String,
    rig: String,
}

impl Claim {
    /// Locked in the same step that creates it when the rig is new, as a
    /// seat is (`seats::create_seat`), so no second invocation arrives
    /// between the tree existing and being claimed.
    ///
    /// The claim names this process and is read as `Held::ByRunner`: a
    /// claim outliving its measurement would keep the rig from every
    /// measurement after.
    fn take(primary: &str, rig: &str, commit: &str, exists: bool) -> Result<Self, String> {
        // The session mark carries the pid too: a claim's reason is read
        // back as `<session> pid <pid>`, and an empty session leaves the
        // pid unparsed (`seats::standing`).
        let me = Identity {
            session: format!("perf-{}", std::process::id()),
            pid: Some(std::process::id()),
        };
        if !exists {
            git_query(
                primary,
                &[
                    "worktree",
                    "add",
                    "--lock",
                    "--reason",
                    &me.reason(),
                    "--detach",
                    rig,
                    commit,
                ],
            )
            .ok_or_else(|| format!("could not create the rig at {rig}"))?;
            println!("rig: created at {rig}");
            return Ok(Self {
                primary: primary.to_string(),
                rig: rig.to_string(),
            });
        }
        match take_seat(primary, rig, &me, Held::ByRunner) {
            Standing::Ours => Ok(Self {
                primary: primary.to_string(),
                rig: rig.to_string(),
            }),
            Standing::Foreign(reason) | Standing::Stale(reason) => Err(format!(
                "the rig is claimed — another `perf --at` is switching or building it right now \
                 (the lock says: {reason}). Wait for it; a claim whose process is gone clears \
                 itself on the next try."
            )),
            Standing::Free => Err(format!("the rig at {rig} would not take a claim")),
        }
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        if !unlock_seat(&self.primary, &self.rig) {
            println!(
                "rig: the claim on {} did not release — `git worktree unlock {}` by hand",
                self.rig, self.rig
            );
        }
    }
}

/// Puts the rig on `commit`. A dirty rig is refused: the change is
/// somebody's, and this is the one place it would go missing without
/// a word.
fn switch(rig: &str, commit: &str) -> Result<(), String> {
    let dirty = git_query(rig, &["status", "--porcelain"])
        .ok_or_else(|| format!("the rig at {rig} would not answer git status"))?;
    if !dirty.is_empty() {
        return Err(format!(
            "the rig at {rig} has uncommitted changes, and nothing here resets a tree — it is \
             built and measured only. Clean it by hand and make the change in a \
             seat:\n{dirty}"
        ));
    }
    if git_query(rig, &["rev-parse", "HEAD"]).as_deref() == Some(commit) {
        return Ok(());
    }
    git_query(rig, &["switch", "--detach", commit])
        .map(|_| ())
        .ok_or_else(|| format!("could not put the rig on {}", short(commit)))
}

/// One exe onto the shelf: copied beside its place and renamed into it,
/// so an interrupted copy never stands where a finished build is read;
/// and stamped with the time it was shelved, because a Windows copy
/// carries the source's time and the shelf is swept by it.
fn shelve(fresh: &Path, exe: &Path) -> Result<(), String> {
    let dir = exe.parent().ok_or("the shelf has no directory")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("could not make {}: {e}", dir.display()))?;
    let staged = exe.with_extension("staged");
    std::fs::copy(fresh, &staged).map_err(|e| {
        format!(
            "could not shelve {} as {}: {e}",
            fresh.display(),
            staged.display()
        )
    })?;
    std::fs::File::options()
        .write(true)
        .open(&staged)
        .and_then(|file| file.set_modified(SystemTime::now()))
        .map_err(|e| format!("could not stamp {}: {e}", staged.display()))?;
    std::fs::rename(&staged, exe)
        .map_err(|e| format!("could not put {} on the shelf: {e}", exe.display()))?;
    println!("rig: shelved {}", exe.display());
    Ok(())
}

/// The shelf entries past the newest `keep`, oldest first by their exe's
/// time (std cannot set a directory's on Windows); an entry with no exe
/// is older than any that has one. An unreadable shelf has nothing to
/// take off.
fn stale_builds(shelf: &Path, keep: usize) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(shelf) else {
        return Vec::new();
    };
    let mut builds: Vec<(SystemTime, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            let shelved = std::fs::metadata(entry.path().join(crate::app_build::exe_name()))
                .and_then(|meta| meta.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            (shelved, entry.path())
        })
        .collect();
    builds.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    builds
        .into_iter()
        .skip(keep)
        .map(|(_, path)| path)
        .collect()
}

/// One build's place: the commit and feature set, as people read them,
/// and the fingerprint of the rest of what it was built from.
fn shelf(rig: &Path, commit: &str, set: &str, built_from: &str) -> PathBuf {
    rig.join("target").join(SHELF).join(format!(
        "{}-{set}-{}",
        short(commit),
        built_from.get(..8).unwrap_or(built_from)
    ))
}

fn short(commit: &str) -> &str {
    commit.get(..12).unwrap_or(commit)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{shelf, short, stale_builds};

    /// One shelf name per build the record tells apart, so the harness
    /// build and the shipped one of a commit — or one commit against two
    /// Qts — never answer for each other.
    #[test]
    fn a_build_is_shelved_by_commit_feature_set_and_what_it_was_built_from() {
        let rig = Path::new("C:/x/platitude-gg/.claude/worktrees/rig");
        let commit = "3443a122abcdef0123456789abcdef0123456789";
        assert_eq!(
            shelf(rig, commit, "automation", "0123456789abcdef"),
            rig.join("target/shelf/3443a122abcd-automation-01234567")
        );
        assert_ne!(
            shelf(rig, commit, "automation", "0123456789abcdef"),
            shelf(rig, commit, "automation", "fedcba9876543210")
        );
        assert_eq!(short("abc"), "abc");
    }

    #[test]
    fn the_shelf_keeps_the_newest_builds() {
        let dir = crate::yard::Yard::new("rig-shelf");
        for (name, age) in [("old", 30), ("middle", 20), ("new", 10), ("newest", 0)] {
            let build = dir.join(name);
            std::fs::create_dir_all(&build).expect("a shelf entry");
            // waits(measured): the clock is what the shelf's ages are written against, and the shelf reads no other
            let shelved = std::time::SystemTime::now() - std::time::Duration::from_secs(age);
            std::fs::File::create(build.join(crate::app_build::exe_name()))
                .and_then(|file| file.set_modified(shelved))
                .expect("a shelved exe");
        }
        std::fs::create_dir_all(dir.join("empty")).expect("a shelf entry with nothing in it");
        let stale = stale_builds(&dir, 2);
        assert_eq!(
            stale,
            vec![dir.join("middle"), dir.join("old"), dir.join("empty")]
        );
        assert!(stale_builds(&dir, 10).is_empty());
        assert!(stale_builds(&dir.join("nowhere"), 1).is_empty());
    }
}
