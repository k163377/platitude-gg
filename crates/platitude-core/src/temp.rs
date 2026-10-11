//! Where a unit test's throwaway folders go: the workspace's `target/tmp`,
//! inside the project the virus scanner is told to leave alone, never the
//! system temp folder — the integration suite's `support::temp` folder,
//! which cargo names only for that suite.
//!
//! A git run in a folder here that holds no repository finds the checkout
//! around it: a test that runs git belongs to the integration suite, whose
//! git is stopped above its folder.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The workspace's `target/tmp`, made once.
fn root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace = crate_dir
            .parent()
            .and_then(Path::parent)
            .expect("the crate sits two folders under the workspace");
        let root = workspace.join("target").join("tmp");
        std::fs::create_dir_all(&root).expect("make target/tmp");
        root
    })
}

/// A folder of the test's own under the workspace's `target/tmp`, gone when
/// dropped.
pub(crate) fn dir() -> tempfile::TempDir {
    tempfile::tempdir_in(root()).expect("a folder under target/tmp")
}
