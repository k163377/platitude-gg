//! What destructive operations took away, on real repositories
//! (破棄記録仕様.md): read off git's reflogs, written to and read from
//! Platitude GG's own record, and brought back.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
// An inner attribute here covers the child modules too.
#![allow(clippy::expect_used)]

mod reading;
mod recording;
mod restoring;

use std::path::{Path, PathBuf};

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::Oid;
use platitude_core::discards::{self, CopyOf, Discard, DiscardKind, Restore};

async fn read(repo: &TestRepo) -> Vec<Discard> {
    read_at(&repo.path).await
}

/// The reading from the working copy at `path`.
async fn read_at(path: &Path) -> Vec<Discard> {
    let (executor, cancel) = env();
    discards::read_repo(&executor, path, Some(2000), &cancel)
        .await
        .expect("read the reflogs and the record")
}

fn oid(hex: &str) -> Oid {
    Oid::from_hex_str(hex.trim()).expect("an oid")
}

fn one(found: &[Discard], kind: DiscardKind) -> &Discard {
    let mut of_kind = found.iter().filter(|d| d.kind == kind);
    let first = of_kind.next().expect("a line of that kind");
    assert!(of_kind.next().is_none(), "one line of {kind:?}");
    first
}

/// The repository's own git directory, where the record's scratch goes.
fn git_dir(repo: &TestRepo) -> PathBuf {
    repo.path.join(".git")
}

/// Whether `path` names the folder `dir` is: git writes a working copy's
/// path in full where the temp folder is spelled short on Windows.
fn same_dir(path: &str, dir: &Path) -> bool {
    std::fs::canonicalize(path).ok() == std::fs::canonicalize(dir).ok()
}

/// Each part's restore, in order.
fn restores(entry: &Discard) -> Vec<&Restore> {
    entry.parts.iter().map(|part| &part.restore).collect()
}

/// Copies the work `of` names and puts the copy on the record, as a write
/// does around a discard that takes whole files back to the index; whether
/// a line went on.
async fn copy_and_record(of: &CopyOf<'_>) -> bool {
    match begin(of).await {
        Some(copied) => finish(of, &copied, true).await,
        None => false,
    }
}

/// Reads what the work `of` names holds before a write throws it away.
async fn begin(of: &CopyOf<'_>) -> Option<discards::Copied> {
    let (executor, cancel) = env();
    discards::copy_work(&executor, of, &cancel)
        .await
        .expect("copy")
}

/// Puts a begun copy on the record once the write answered (`landed`);
/// whether a line went on.
async fn finish(of: &CopyOf<'_>, copied: &discards::Copied, landed: bool) -> bool {
    let (executor, cancel) = env();
    discards::record_copy(&executor, of.workdir, of.git_dir, copied, landed, &cancel)
        .await
        .expect("record the copy")
}
