//! Git LFS, as far as this app has to know it: which pending files the
//! `lfs` filter would clean on their way into the index, and whether git
//! can run that filter here. Nothing is converted or fetched — the filter
//! is git's and Git LFS's (デザイン規約 §ウィンドウの縁 の `NO LFS`).
//!
//! Without Git LFS the filter is either undefined (git stores the whole
//! file where a pointer belonged) or defined and failing (`git add`
//! stops). Either way the files are the ones `.gitattributes` gives
//! `filter=lfs`, so that is what is counted; which of the two happens is
//! git's to say when it happens.

use std::collections::HashSet;
use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};
use crate::status::StatusItem;

/// How many bytes of paths one `check-attr` is handed. Under Windows'
/// 32767-character command line with room for git's own words: a byte is
/// at most one UTF-16 unit, so a path is never longer there than here.
/// Batches stay large because the process is most of the price.
const ATTR_BYTES: usize = 24_000;

/// Whether the working tree holds content of this entry still to be
/// staged — what passes the clean filter. A staged side is in the index
/// already (whole, where it was staged without LFS), a rename or copy
/// with nothing unstaged carries the blob it had, and a removal stores
/// nothing; none of them runs the filter again.
pub fn stages_content(item: &StatusItem) -> bool {
    match item {
        StatusItem::Tracked { unstaged, .. } => !matches!(unstaged, 'D' | '.'),
        StatusItem::Unmerged { ours, theirs, .. } => !(*ours == 'D' && *theirs == 'D'),
        StatusItem::Untracked { .. } => true,
    }
}

/// How many of `paths` git's attributes give `filter=lfs`.
///
/// Every attribute source counts (`.gitattributes` at any depth,
/// `info/attributes`, `core.attributesFile`), which is why git is asked
/// rather than the files read.
pub async fn filtered(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<usize, GitError> {
    let mut count = 0;
    let mut rest = paths;
    while !rest.is_empty() {
        let mut bytes = 0;
        let take = rest
            .iter()
            .take_while(|p| {
                bytes += p.len() + 1;
                bytes <= ATTR_BYTES
            })
            .count()
            // A path past the budget on its own still goes, alone.
            .max(1);
        let (batch, later) = rest.split_at(take);
        rest = later;
        let mut cmd = GitCommand::new()
            .cwd(workdir)
            .args(["check-attr", "-z", "filter", "--"]);
        for p in batch {
            cmd = cmd.arg(p.as_str());
        }
        let out = executor.run(cmd, cancel).await?;
        // `-z` prints a `path\0filter\0value\0` triple per path. A path
        // named twice is counted once.
        let mut seen: HashSet<&[u8]> = HashSet::new();
        let fields: Vec<&[u8]> = out.stdout.split(|b| *b == 0).collect();
        for triple in fields.chunks(3) {
            if let [path, _, b"lfs"] = triple {
                seen.insert(path);
            }
        }
        count += seen.len();
    }
    Ok(count)
}

/// Whether git can run Git LFS here: `git lfs version` answers, where a
/// missing one is git's `'lfs' is not a git command` and exit 1.
///
/// Any other exit is no better — an LFS that cannot say its version will
/// not clean a file either. A cancel or a git that did not start is an
/// error: nothing was learned about LFS.
///
/// The session tests mock this (`mock_runs`); mry copies what a mock
/// matches on, so the borrowed arguments are skipped.
#[mry::mry(skip_args(GitExecutor, Path, CancellationToken))]
pub async fn runs(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .answers_by_code(1)
        .args(["lfs", "version"]);
    Ok(executor.run_unchecked(cmd, cancel).await?.code == 0)
}

#[cfg(test)]
mod tests {
    use super::stages_content;
    use crate::status::StatusItem;

    fn tracked(staged: char, unstaged: char) -> StatusItem {
        StatusItem::Tracked {
            staged,
            unstaged,
            path: "a.psd".to_string(),
            orig_path: None,
        }
    }

    fn unmerged(ours: char, theirs: char) -> StatusItem {
        StatusItem::Unmerged {
            ours,
            theirs,
            path: "c.psd".to_string(),
        }
    }

    /// Only the working tree's side is read: what is staged, renamed or
    /// removed is past the filter, and a new file, a change not staged
    /// yet and a conflict still to be resolved are before it.
    #[test]
    fn only_content_the_working_tree_has_to_stage_passes_the_filter() {
        for past in [
            tracked('M', '.'),
            tracked('A', '.'),
            tracked('R', '.'),
            tracked('C', '.'),
            tracked('T', '.'),
            tracked('D', '.'),
            tracked('.', 'D'),
            tracked('A', 'D'),
            unmerged('D', 'D'),
        ] {
            assert!(!stages_content(&past), "{past:?}");
        }
        for before in [
            tracked('.', 'M'),
            tracked('M', 'M'),
            tracked('A', 'M'),
            tracked('R', 'M'),
            tracked('.', 'T'),
            StatusItem::Untracked {
                path: "b.psd".to_string(),
            },
            unmerged('U', 'U'),
            unmerged('A', 'A'),
            unmerged('D', 'U'),
            unmerged('U', 'D'),
        ] {
            assert!(stages_content(&before), "{before:?}");
        }
    }
}
