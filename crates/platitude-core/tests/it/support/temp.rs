//! Where a test's throwaway folders go: under the build's own `target`,
//! inside the project the virus scanner is told to leave alone, never the
//! system temp folder.
//!
//! The project around them is a git checkout, so git looking upward from
//! a folder there that holds no repository would find it. Every git a test
//! runs carries [`ceiling`], which stops that search at [`root`].

use std::ffi::OsString;
use std::path::Path;

/// The folder every throwaway folder of the suite goes in (cargo makes it).
pub fn root() -> &'static Path {
    Path::new(env!("CARGO_TARGET_TMPDIR"))
}

/// A folder of the test's own under [`root`], gone when dropped.
pub fn dir() -> tempfile::TempDir {
    tempfile::tempdir_in(root()).expect("a folder under target/tmp")
}

/// The variable that keeps git from looking above [`root`] for a
/// repository, so a test's folder that holds none is not one.
pub fn ceiling() -> (OsString, OsString) {
    (
        OsString::from("GIT_CEILING_DIRECTORIES"),
        root().as_os_str().to_os_string(),
    )
}
