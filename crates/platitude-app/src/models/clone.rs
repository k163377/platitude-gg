//! Fetching a repository that has no tab yet.
//!
//! Its own object rather than a corner of [`TabsModel`]: a clone runs
//! **outside every session and before there is anything to open**, which
//! is the one write in the application that the tab strip has no part in.
//! What the strip does with the answer — opening the folder that came
//! down — is its ordinary job and is asked of it from the window
//! (`Main.qml`), so neither object has to know the other.
//!
//! [`TabsModel`]: super::TabsModel

use std::sync::Arc;

use qtbridge::{QObjectHolder, qobject};

use crate::hub::{CloneMsg, Feed, Hub};
use crate::urlpath::file_url_to_path;

use super::qml_register;

/// The clone in flight, and what became of the last one.
#[derive(Default)]
pub struct CloneModel {
    /// What the background task answers with. Attached the first time a
    /// clone is asked for rather than at startup: a window nobody clones
    /// in never has one to hear.
    feed: Arc<Feed<CloneMsg>>,
    attached: bool,
    cloning: bool,
    /// What stops it. Held here rather than in the dialog because the
    /// dialog is built and rebuilt with the window's overlay, and the
    /// token is not its to lose.
    cancel: Option<tokio_util::sync::CancellationToken>,
}

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl CloneModel {
    // Whether git is out fetching. The dialog binds its ring, its fields
    // and its buttons to this, so it has to be a property: a slot would
    // freeze at the value it was first read at (.claude/rules/app-ui.md).
    // Written with `//` — a doc comment on a `qproperty!` fails the build
    // (rules-refs/app-ui.md).
    qproperty!("cloning", Member = cloning, Notify = cloning_changed);

    #[qsignal]
    fn cloning_changed(&mut self);

    /// The clone came down, into the folder `path` names. Its own signal
    /// rather than a reader watching `cloning` fall: the dialog has to
    /// tell a clone that landed from one git refused, and the two answers
    /// arrive on the same property.
    #[qsignal]
    fn clone_done(&mut self, path: String);

    /// git would not make it, and `message` is what it said. The dialog
    /// that asked is still standing, which is where this lands: it is the
    /// only thing on screen this is about
    /// (デザイン規約 §リポジトリを取り寄せる).
    #[qsignal]
    fn clone_failed(&mut self, message: String);

    /// Fetches `url` into a folder called `name` inside the folder
    /// `parent_url` names (a `file://` URL, as the FolderDialog hands one
    /// over).
    ///
    /// **Nothing is checked here first.** A destination that is taken and
    /// a URL nothing answers are both git's to refuse, and unlike the
    /// picker's folder — where the refusal would arrive after the dialog
    /// had closed — this dialog stays open for the whole call and has
    /// somewhere to put what git says.
    #[qslot]
    fn clone_repository(&mut self, url: String, parent_url: String, name: String) {
        let url = url.trim().to_string();
        let name = name.trim().to_string();
        // One at a time: the dialog is modal and its accept is refused
        // while a clone is out, so a second call is a wiring accident
        // rather than something a reader can ask for.
        if url.is_empty() || name.is_empty() || self.cloning {
            return;
        }
        let parent = file_url_to_path(&parent_url);
        if parent.as_os_str().is_empty() {
            return;
        }
        if !self.attached {
            self.feed.attach(self.get_qml_method_invoker());
            self.attached = true;
        }
        let feed = Arc::clone(&self.feed);
        let into = parent.join(&name);
        tracing::info!(url = %url, into = %into.display(), "cloning");
        let Some(Some(cancel)) = Hub::with(|hub| hub.clone_repo(url, into, feed)) else {
            return;
        };
        self.cancel = Some(cancel);
        self.settle(true);
    }

    /// Stops the clone in flight, if there is one. What git left behind is
    /// left behind: this application does not delete folders.
    #[qslot]
    fn cancel_clone(&mut self) {
        let Some(cancel) = self.cancel.take() else {
            return;
        };
        cancel.cancel();
        // Cancellation is cooperative, and the answer can be on its way
        // already: git exits 0 and the task pushes in the instant the
        // press lands. Emptying the queue only catches the half of that
        // race that has already arrived — the feed itself is let go of,
        // so the push that comes after wakes nobody and dies with the
        // task holding it, and the next clone attaches a new one. Without
        // this, a clone the reader stopped opens a tab.
        self.feed.release();
        self.feed = Arc::new(Feed::default());
        self.attached = false;
        tracing::info!("clone cancelled");
        self.settle(false);
    }

    #[qslot]
    fn drain(&mut self) {
        for msg in self.feed.drain() {
            self.cancel = None;
            self.settle(false);
            match msg {
                CloneMsg::Done { path } => {
                    self.clone_done(path.to_string_lossy().into_owned());
                }
                CloneMsg::Failed { message } => {
                    tracing::info!(message = %message, "clone refused");
                    self.clone_failed(message);
                }
            }
        }
    }
}

impl CloneModel {
    /// Says whether git is out, and tells whoever is watching only when
    /// the answer changed.
    fn settle(&mut self, cloning: bool) {
        if self.cloning != cloning {
            self.cloning = cloning;
            self.cloning_changed();
        }
    }
}

qml_register!(CloneModel, "CloneModel", singleton = false);
