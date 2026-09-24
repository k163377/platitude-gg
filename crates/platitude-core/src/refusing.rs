//! An executor that cannot run anything, over a repository that is not
//! there: the unit-test ground for a call that answers before any git
//! process runs, and for the command line a call would have asked for.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::operation::OperationId;
use crate::process::{CommandEnd, CommandObserver, GitExecutor, Kept};
use crate::repo::{ObjectFormat, RepoInfo};

/// Every command asked for, as the command log would show it. The
/// observer hears of a command at the ask — before the queue in front of
/// it and before any spawn — so an empty list says no git was started.
#[derive(Default)]
pub(crate) struct Asked(Mutex<Vec<String>>);

impl Asked {
    pub(crate) fn count(&self) -> usize {
        self.displays().len()
    }

    pub(crate) fn displays(&self) -> Vec<String> {
        self.0
            .lock()
            .expect("no test panics while holding it")
            .clone()
    }
}

impl CommandObserver for Asked {
    fn records(&self, _kept: Kept) -> bool {
        true
    }

    fn started(
        &self,
        display: &str,
        _full: &str,
        _kept: Kept,
        _operation: Option<OperationId>,
    ) -> u64 {
        let mut asked = self.0.lock().expect("no test panics while holding it");
        asked.push(display.to_string());
        asked.len() as u64
    }

    fn finished(
        &self,
        _id: u64,
        _end: CommandEnd,
        _waited_ms: u64,
        _elapsed_ms: u64,
        _message: &str,
    ) {
    }
}

/// An executor whose program is not there, heard by a fresh [`Asked`].
/// **The second witness**, beside the count: a call that slipped past its
/// early answer fails on the spawn instead of passing against a working
/// tree the test never made.
pub(crate) fn git() -> (GitExecutor, Arc<Asked>) {
    let asked = Arc::new(Asked::default());
    let exec = GitExecutor::with_program("no-such-git-for-this-test")
        .observed(Arc::clone(&asked) as Arc<dyn CommandObserver>, Kept::Asked);
    (exec, asked)
}

/// A working tree that is this crate's `Cargo.toml` — a file, so nothing
/// that slipped through can make a directory or a scratch file under it.
pub(crate) fn nowhere() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

/// A repository whose working tree is [`nowhere`].
pub(crate) fn repo() -> RepoInfo {
    let workdir = nowhere();
    RepoInfo {
        git_dir: workdir.join(".git"),
        config_path: workdir.join(".git").join("config"),
        workdir,
        object_format: ObjectFormat::Sha1,
    }
}
