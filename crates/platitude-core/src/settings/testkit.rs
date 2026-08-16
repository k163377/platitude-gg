//! Shared by the test modules beside it.

use std::path::Path;

use super::Store;

pub(super) fn dir_store(dir: &Path) -> Store {
    Store::at(dir)
}
