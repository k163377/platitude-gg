//! The measurement rig: the commit that was asked for, built in a tree
//! nobody edits, and the build shelved by commit so it is never built
//! twice.
//!
//! What a seat's own exe cannot answer is *which* source a number was
//! taken of: the tree carries whatever the session edited since its last
//! commit, its target/ rebuilds the app every time a source file moves,
//! and an A/B against main means switching the seat back and forth with
//! a release build each way. The rig is the one tree under the roster's
//! directory that is no seat (`seats::RIG`): `perf --at <rev>` resolves
//! the commit, checks it out there, builds it with the feature set the
//! measurement asked for, and copies the exe onto a shelf under the rig's
//! own target/ by commit and feature set. A second measurement of the
//! same commit — the other side of an A/B, the next stage table of a
//! record — builds nothing.
//!
//! **The rig is claimed while it is switched and built**, the way a seat
//! is claimed (`seats::take_seat`), so two invocations cannot check two
//! commits out into one tree at once — and claimed by the measuring
//! process: a second `perf --at` from the same session is refused
//! too, and a claim whose process is gone is cleared by the next
//! one, from any terminal. The claim is released once the exe is on
//! the shelf: the measurement runs off the copy and needs the tree
//! for nothing.
//!
//! **A dirty rig refuses.** Nothing here resets a tree — an edit in the
//! rig is somebody's, however wrong it was to make it there — so the
//! measurement stops and says whose problem it is. The write hook refuses
//! the edit in the first place (hook/seat.rs).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::seats::{
    Held, Identity, Standing, primary_checkout, rig_path, same_tree, slashed, take_seat,
    unlock_seat,
};
use crate::subprocess::git_query;

use super::Options;

/// How many builds the shelf keeps. Every entry is one exe of some sixty
/// megabytes; an A/B needs two, a record's tables three (the harness, the
/// harness with memprobe, the shipped set), and the rest is room for the
/// last few commits measured.
const KEEP: usize = 8;

/// The directory under the rig's target/ the builds are shelved in. Under
/// target/ so that git never sees it: the rig has to answer `status` with
/// nothing, or it is a tree somebody edited.
const SHELF: &str = "shelf";

/// A build the measurement can run: where the exe is, which commit it is
/// of, and the tree the run is started from.
pub(super) struct Built {
    pub(super) exe: PathBuf,
    pub(super) commit: String,
    pub(super) tree: PathBuf,
}

impl Built {
    /// The commit as the report and the shelf name it.
    pub(super) fn short(&self) -> &str {
        short(&self.commit)
    }
}

/// The build of `rev`, off the rig's shelf or freshly made there.
///
/// `caller` is the tree the command runs in: the rev is resolved there,
/// which is what lets a seat name its own branch. `opts.build` false takes
/// the shelf or nothing — a `--no-build` that would have to build is
/// refused.
///
/// Called under the measurement's own announcement (`perf::run`), so no
/// measurement holds the machine while this switches, reaps and builds.
pub(super) fn build_at(
    caller: &Path,
    rev: &str,
    path: &std::ffi::OsStr,
    opts: &Options,
) -> Result<Built, String> {
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
    let set = opts.feature_slug();
    let exe = shelf(Path::new(&rig), &commit, &set).join(crate::tree::exe_name());
    if exe.is_file() {
        println!(
            "rig: {} ({set}) is on the shelf — nothing to build",
            short(&commit)
        );
        return Ok(Built {
            exe,
            commit,
            tree: PathBuf::from(rig),
        });
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
    // A run of the rig's exe left standing holds the shelf file against
    // the copy below and the rig's store against the build; a seat's
    // `kill` reaps its own tree and never this one. No measurement is
    // running off the shelf right now — this whole build is announced,
    // and a measurement's hold is what an announcement waits for.
    for (pid, stale) in crate::gui::reap_under(Path::new(&rig))? {
        println!("rig: reaped a stale run first: {pid} ({stale})");
    }
    switch(&rig, &commit)?;
    println!("rig: building {} ({set}) in {rig}", short(&commit));
    let fresh = super::build_in(Path::new(&rig), path, true, opts)?;
    shelve(&fresh, &exe)?;
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
    /// seat is (`seats::create_seat`): no moment between the tree
    /// existing and being claimed for a second invocation to arrive in.
    ///
    /// The claim names this process: a session runs one measurement at
    /// a time, and a claim left by a killed one is litter its dead pid
    /// gives away (`seats::standing`), whichever terminal meets it
    /// next. That is why this claim is read as `Held::ByRunner` — a
    /// seat's claim is a conversation's and its number is never asked,
    /// while a measurement is one process, and a claim outliving it
    /// would keep the rig from every measurement after.
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

/// One exe onto the shelf, under its commit and feature set: copied
/// beside its place and renamed into it, so a copy that was interrupted
/// never stands where a finished build would be read; and stamped with
/// the time it was shelved, because a Windows copy carries the source's
/// own time and the shelf is swept by that.
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

/// The shelf entries past the newest `keep`, oldest first by when their
/// exe was shelved — the exe's own time, set as it was shelved, because
/// a directory's is not something std can set on Windows; an entry with
/// no exe in it is older than any that has one. A shelf that cannot be
/// read has nothing to take off.
fn stale_builds(shelf: &Path, keep: usize) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(shelf) else {
        return Vec::new();
    };
    let mut builds: Vec<(SystemTime, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            let shelved = std::fs::metadata(entry.path().join(crate::tree::exe_name()))
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

/// Where a build of `commit` with feature set `set` sits on the shelf.
fn shelf(rig: &Path, commit: &str, set: &str) -> PathBuf {
    rig.join("target")
        .join(SHELF)
        .join(format!("{}-{set}", short(commit)))
}

fn short(commit: &str) -> &str {
    commit.get(..12).unwrap_or(commit)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{shelf, short, stale_builds};

    /// One shelf name per build the record tells apart, so the harness
    /// build and the shipped one of a commit never answer for each other.
    #[test]
    fn a_build_is_shelved_by_commit_and_feature_set() {
        let rig = Path::new("C:/x/platitude-gg/.claude/worktrees/rig");
        let commit = "3443a122abcdef0123456789abcdef0123456789";
        assert_eq!(
            shelf(rig, commit, "automation"),
            rig.join("target/shelf/3443a122abcd-automation")
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
            std::fs::File::create(build.join(crate::tree::exe_name()))
                .and_then(|file| file.set_modified(shelved))
                .expect("a shelved exe");
        }
        // An entry that lost its exe is older than any that kept one.
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
