//! Presets with files `.gitattributes` gives to Git LFS, pending — what the
//! band's `NO LFS` counts where git cannot run LFS (`verify-ui --no-lfs`).
//! The runs' git reads no system config, so no LFS filter is defined and
//! nothing here depends on this machine having LFS.

use super::repo::DemoRepo;

/// What `git lfs track` writes for the two kinds of file below.
const TRACKED: &str = "\
*.psd filter=lfs diff=lfs merge=lfs -text
*.mp4 filter=lfs diff=lfs merge=lfs -text
";

/// A repository that tracks artwork and video through LFS, with three
/// such files pending — one changed, two new — and a text change beside
/// them that LFS has nothing to do with, so the count is not the list's.
pub(super) fn lfs(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nArtwork and the launch video.\n",
        "docs: start the readme",
    )?;
    repo.commit(".gitattributes", TRACKED, "chore: keep artwork in Git LFS")?;
    repo.commit("art/cover.psd", "8BPS cover v1\n", "feat: add the cover")?;
    repo.write("art/cover.psd", "8BPS cover v2\n")?;
    repo.write("art/logo.psd", "8BPS logo v1\n")?;
    repo.write("media/launch.mp4", "ftypisom launch v1\n")?;
    repo.write(
        "README.md",
        "# demo\n\nArtwork and the launch video, kept in Git LFS.\n",
    )?;
    Ok(())
}

/// The stopped merge of `conflict`, with one file for LFS added beside
/// it, tracked in the same stroke — every badge the page can raise at
/// once, and the count's one-file sentence.
pub(super) fn lfs_conflict(repo: &mut DemoRepo) -> Result<(), String> {
    super::conflict::conflict(repo)?;
    repo.write(".gitattributes", TRACKED)?;
    repo.write("art/cover.psd", "8BPS cover v1\n")?;
    Ok(())
}
