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
//! is claimed (`seats::take_seat`), so two sessions cannot check two
//! commits out into one tree at once. The claim is released once the exe
//! is on the shelf: the measurement runs off the copy and needs the tree
//! for nothing.
//!
//! **A dirty rig refuses.** Nothing here resets a tree — an edit in the
//! rig is somebody's, however wrong it was to make it there — so the
//! measurement stops and says whose problem it is. The write hook refuses
//! the edit in the first place (hook/seat.rs).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::seats::{
    Identity, Standing, claim_liveness, rig_path, take_seat, unlock_seat, worktree_blocks,
};
use crate::subprocess::git_query;

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
/// which is what lets a seat name its own branch. `build` false takes the
/// shelf or nothing — a `--no-build` that would have to build is refused
/// rather than quietly done.
pub(super) fn build_at(
    caller: &Path,
    rev: &str,
    path: &std::ffi::OsStr,
    build: bool,
    harness: bool,
    breakdown: bool,
) -> Result<Built, String> {
    let here = forward(caller);
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
    let listing = git_query(&here, &["worktree", "list", "--porcelain"])
        .ok_or("git worktree list failed — is git on PATH and this a repository?")?;
    let trees = worktree_blocks(&listing);
    let primary = trees
        .first()
        .ok_or("git worktree list named no tree at all")?
        .path
        .clone();
    let rig = rig_path(&primary);
    let set = feature_set(harness, breakdown);
    let exe = shelf(Path::new(&rig), &commit, set).join(exe_name());
    // A run of the rig's exe left standing holds the file against the
    // copy below and would open on the store's refusal gate; a seat's
    // `kill` reaps its own tree and never this one.
    for (pid, stale) in crate::gui::reap_under(Path::new(&rig))? {
        println!("rig: reaped a stale run first: {pid} ({stale})");
    }
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
    if !build {
        return Err(format!(
            "the rig has no build of {} with {set} on its shelf, and --no-build asked for none \
             — drop --no-build",
            short(&commit)
        ));
    }
    let exists = trees.iter().any(|tree| same_tree(&tree.path, &rig));
    let _claim = Claim::take(&primary, &rig, &commit, exists)?;
    switch(&rig, &commit)?;
    println!("rig: building {} ({set}) in {rig}", short(&commit));
    let fresh = if harness {
        let mut extra = vec!["-p", "platitude-app"];
        if breakdown {
            extra.extend(["--features", "memprobe"]);
        }
        crate::tree::app_exe(Path::new(&rig), path, true, &extra)?
    } else {
        crate::tree::shipped_exe(Path::new(&rig), path, true)?
    };
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
    /// existing and being claimed for a second session to arrive in.
    fn take(primary: &str, rig: &str, commit: &str, exists: bool) -> Result<Self, String> {
        let me = Identity::current(None);
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
        match take_seat(primary, rig, &me) {
            Standing::Ours => Ok(Self {
                primary: primary.to_string(),
                rig: rig.to_string(),
            }),
            Standing::Foreign(reason) | Standing::Stale(reason) => Err(format!(
                "the rig is claimed — another measurement is switching or building it. The lock \
                 says: {reason}. {} Wait for it, or if that session is gone, `git worktree \
                 unlock {rig}` by hand.",
                claim_liveness(&reason)
            )),
            Standing::Free => Err(format!("the rig at {rig} would not take a claim")),
        }
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        unlock_seat(&self.primary, &self.rig);
    }
}

/// Puts the rig on `commit`. A dirty rig is refused rather than reset:
/// the change is somebody's, and this is the one place it would go
/// missing without a word.
fn switch(rig: &str, commit: &str) -> Result<(), String> {
    let dirty = git_query(rig, &["status", "--porcelain"])
        .ok_or_else(|| format!("the rig at {rig} would not answer git status"))?;
    if !dirty.is_empty() {
        return Err(format!(
            "the rig at {rig} has uncommitted changes, and nothing here resets a tree — it is \
             built and measured, never edited. Clean it by hand and make the change in a seat \
             instead:\n{dirty}"
        ));
    }
    if git_query(rig, &["rev-parse", "HEAD"]).as_deref() == Some(commit) {
        return Ok(());
    }
    git_query(rig, &["switch", "--detach", commit])
        .map(|_| ())
        .ok_or_else(|| format!("could not put the rig on {}", short(commit)))
}

/// One exe onto the shelf, under its commit and feature set.
fn shelve(fresh: &Path, exe: &Path) -> Result<(), String> {
    let dir = exe.parent().ok_or("the shelf has no directory")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("could not make {}: {e}", dir.display()))?;
    std::fs::copy(fresh, exe).map_err(|e| {
        format!(
            "could not shelve {} as {}: {e}",
            fresh.display(),
            exe.display()
        )
    })?;
    println!("rig: shelved {}", exe.display());
    Ok(())
}

/// The shelf entries past the newest `keep`, oldest first by when their
/// exe was shelved — the exe's own time, because a directory's is not
/// something std can set on Windows, and an entry with no exe in it is
/// older than any that has one. A shelf that cannot be read has nothing
/// to take off.
fn stale_builds(shelf: &Path, keep: usize) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(shelf) else {
        return Vec::new();
    };
    let mut builds: Vec<(SystemTime, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            let shelved = std::fs::metadata(entry.path().join(exe_name()))
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

/// The feature set as the shelf names it, one word per build the record
/// distinguishes (`perf::Options::features`).
fn feature_set(harness: bool, breakdown: bool) -> &'static str {
    match (harness, breakdown) {
        (true, true) => "automation+memprobe",
        (true, false) => "automation",
        (false, _) => "shipped",
    }
}

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "platitude-gg.exe"
    } else {
        "platitude-gg"
    }
}

fn short(commit: &str) -> &str {
    commit.get(..12).unwrap_or(commit)
}

fn forward(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Whether two paths name one tree. Windows spells a path in whatever
/// case the writer used, so the comparison there is case-blind.
fn same_tree(left: &str, right: &str) -> bool {
    let trim = |path: &str| path.replace('\\', "/").trim_end_matches('/').to_string();
    let (left, right) = (trim(left), trim(right));
    if cfg!(windows) {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{feature_set, same_tree, shelf, short, stale_builds};

    /// One shelf name per build the record tells apart, so the harness
    /// build and the shipped one of a commit never answer for each other.
    #[test]
    fn a_build_is_shelved_by_commit_and_feature_set() {
        let rig = Path::new("C:/x/platitude-gg/.claude/worktrees/rig");
        let commit = "3443a122abcdef0123456789abcdef0123456789";
        assert_eq!(
            shelf(rig, commit, feature_set(true, false)),
            rig.join("target/shelf/3443a122abcd-automation")
        );
        assert_eq!(feature_set(true, true), "automation+memprobe");
        assert_eq!(feature_set(false, true), "shipped");
        assert_eq!(short("abc"), "abc");
    }

    #[test]
    fn the_shelf_keeps_the_newest_builds() {
        let dir = std::env::temp_dir().join(format!("pg-rig-shelf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (name, age) in [("old", 30), ("middle", 20), ("new", 10), ("newest", 0)] {
            let build = dir.join(name);
            std::fs::create_dir_all(&build).expect("a shelf entry");
            let shelved = std::time::SystemTime::now() - std::time::Duration::from_secs(age);
            std::fs::File::create(build.join(super::exe_name()))
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
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn one_tree_spelled_two_ways_is_one_tree() {
        assert!(same_tree("C:/x/rig/", "C:\\x\\rig"));
        assert_eq!(same_tree("C:/x/RIG", "C:/x/rig"), cfg!(windows));
        assert!(!same_tree("C:/x/rig", "C:/x/rigging"));
    }
}
