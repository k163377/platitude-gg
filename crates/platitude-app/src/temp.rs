//! Where a test's throwaway folders go: the workspace's `target/tmp`,
//! inside the project the virus scanner is told to leave alone, never the
//! system temp folder — the same folder `platitude-core`'s tests use.
//!
//! A git run in a folder here that holds no repository finds the checkout
//! around it: these tests run none.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The workspace's `target/tmp`, made once.
pub(crate) fn root() -> &'static Path {
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
